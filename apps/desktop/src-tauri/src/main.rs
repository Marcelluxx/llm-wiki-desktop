use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::{Command as StdCommand, Stdio},
    sync::{Arc, Mutex},
};

use llm_wiki_app_core::{
    CatalogError, IpcEnvelope, JobState, JobSummary, MessageType, RegistrySnapshot, RegistryStore,
    WikiCatalog, WikiRegistration, WikiSettings,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{Manager, State, ipc::Channel};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::watch,
};
use uuid::Uuid;

struct AppState {
    registry: Mutex<RegistryStore>,
    active_jobs: Arc<Mutex<HashMap<String, watch::Sender<bool>>>>,
}

#[derive(Debug, Serialize)]
struct CommandError {
    code: &'static str,
    message: String,
}

impl From<llm_wiki_app_core::RegistryError> for CommandError {
    fn from(error: llm_wiki_app_core::RegistryError) -> Self {
        Self {
            code: error.code(),
            message: error.to_string(),
        }
    }
}

impl From<CatalogError> for CommandError {
    fn from(error: CatalogError) -> Self {
        Self {
            code: error.code(),
            message: error.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WikiInput {
    display_name: String,
    root: String,
    note_language: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct JobEvent {
    job_id: String,
    state: JobState,
    progress: f64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JobLogEntry {
    timestamp: String,
    level: String,
    job_id: String,
    state: String,
    message: String,
    source: Option<String>,
    detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct PerformanceStatus {
    nvidia_present: bool,
    cuda_enabled: bool,
    device_name: Option<String>,
}

fn with_registry<T>(
    state: State<'_, AppState>,
    action: impl FnOnce(&RegistryStore) -> Result<T, llm_wiki_app_core::RegistryError>,
) -> Result<T, CommandError> {
    let registry = state.registry.lock().map_err(|_| CommandError {
        code: "unavailable",
        message: "The local registry is temporarily unavailable".to_owned(),
    })?;
    action(&registry).map_err(CommandError::from)
}

#[tauri::command]
fn get_registry(state: State<'_, AppState>) -> Result<RegistrySnapshot, CommandError> {
    with_registry(state, RegistryStore::snapshot)
}

#[tauri::command]
fn set_interface_language(
    language: String,
    state: State<'_, AppState>,
) -> Result<RegistrySnapshot, CommandError> {
    with_registry(state, |registry| registry.set_interface_language(&language))
}

#[tauri::command]
fn create_wiki(
    request: WikiInput,
    state: State<'_, AppState>,
) -> Result<WikiRegistration, CommandError> {
    with_registry(state, |registry| {
        registry.create_wiki(
            &request.display_name,
            PathBuf::from(request.root),
            &request.note_language,
        )
    })
}

#[tauri::command]
fn register_wiki(
    request: WikiInput,
    state: State<'_, AppState>,
) -> Result<WikiRegistration, CommandError> {
    with_registry(state, |registry| {
        registry.register_wiki(
            &request.display_name,
            PathBuf::from(request.root),
            &request.note_language,
        )
    })
}

#[tauri::command]
fn open_wiki(
    wiki_id: String,
    state: State<'_, AppState>,
) -> Result<WikiRegistration, CommandError> {
    with_registry(state, |registry| registry.open_wiki(&wiki_id))
}

#[tauri::command]
fn rename_wiki(
    wiki_id: String,
    display_name: String,
    state: State<'_, AppState>,
) -> Result<WikiRegistration, CommandError> {
    with_registry(state, |registry| {
        registry.rename_wiki(&wiki_id, &display_name)
    })
}

#[tauri::command]
fn remove_wiki_registration(
    wiki_id: String,
    state: State<'_, AppState>,
) -> Result<RegistrySnapshot, CommandError> {
    with_registry(state, |registry| registry.remove_registration(&wiki_id))
}

#[tauri::command]
fn get_wiki_settings(
    wiki_id: String,
    state: State<'_, AppState>,
) -> Result<WikiSettings, CommandError> {
    with_registry(state, |registry| registry.read_settings(&wiki_id))
}

#[tauri::command]
fn get_performance_status() -> PerformanceStatus {
    performance_status()
}

#[tauri::command]
async fn install_nvidia_acceleration() -> Result<PerformanceStatus, CommandError> {
    let detected = performance_status();
    if !detected.nvidia_present {
        return Err(CommandError {
            code: "nvidia_unavailable",
            message: "No NVIDIA graphics card was detected".to_owned(),
        });
    }
    let script = repository_root().join("scripts/enable-nvidia-acceleration.ps1");
    if !script.is_file() {
        return Err(CommandError {
            code: "installer_unavailable",
            message: "The NVIDIA acceleration installer is not available".to_owned(),
        });
    }
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script)
        .current_dir(repository_root())
        .output()
        .await
        .map_err(|error| CommandError {
            code: "acceleration_install_failed",
            message: error.to_string(),
        })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(CommandError {
            code: "acceleration_install_failed",
            message: if detail.is_empty() {
                "NVIDIA acceleration could not be installed".to_owned()
            } else {
                detail
            },
        });
    }
    let status = performance_status();
    if !status.cuda_enabled {
        return Err(CommandError {
            code: "acceleration_install_failed",
            message: "CUDA was installed but could not be activated".to_owned(),
        });
    }
    Ok(status)
}

#[tauri::command]
fn list_jobs(wiki_id: String, state: State<'_, AppState>) -> Result<Vec<JobSummary>, CommandError> {
    let wiki = with_registry(state, |registry| registry.open_wiki(&wiki_id))?;
    WikiCatalog::open(&wiki.canonical_root)?
        .list_jobs(&wiki_id)
        .map_err(CommandError::from)
}

#[tauri::command]
async fn start_import(
    wiki_id: String,
    source_paths: Vec<String>,
    on_event: Channel<JobEvent>,
    state: State<'_, AppState>,
) -> Result<JobSummary, CommandError> {
    validate_sources(&source_paths)?;
    let wiki = with_registry(state.clone(), |registry| registry.open_wiki(&wiki_id))?;
    let settings = with_registry(state.clone(), |registry| registry.read_settings(&wiki_id))?;
    let catalog = WikiCatalog::open(&wiki.canonical_root)?;
    let source_count = u32::try_from(source_paths.len()).map_err(|_| CommandError {
        code: "too_many_sources",
        message: "Too many documents were selected".to_owned(),
    })?;
    let job = catalog.create_job(&wiki_id, source_count)?;
    let (cancel_sender, cancel_receiver) = watch::channel(false);
    state
        .active_jobs
        .lock()
        .map_err(|_| unavailable_error())?
        .insert(job.job_id.clone(), cancel_sender);

    let active_jobs = Arc::clone(&state.active_jobs);
    let task_job = job.clone();
    let wiki_root = PathBuf::from(wiki.canonical_root);
    tauri::async_runtime::spawn(async move {
        run_worker_job(
            task_job.clone(),
            wiki_root,
            source_paths,
            settings.ocr_language,
            on_event,
            cancel_receiver,
        )
        .await;
        if let Ok(mut jobs) = active_jobs.lock() {
            jobs.remove(&task_job.job_id);
        }
    });
    Ok(job)
}

#[tauri::command]
fn read_job_log(
    wiki_id: String,
    job_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<JobLogEntry>, CommandError> {
    Uuid::parse_str(&job_id).map_err(|_| CommandError {
        code: "invalid_job",
        message: "The job identifier is invalid".to_owned(),
    })?;
    let wiki = with_registry(state, |registry| registry.open_wiki(&wiki_id))?;
    let log_path = Path::new(&wiki.canonical_root)
        .join(".llm-wiki")
        .join("logs")
        .join(format!("{job_id}.jsonl"));
    if !log_path.is_file() {
        return Ok(Vec::new());
    }
    let metadata = std::fs::metadata(&log_path).map_err(|error| CommandError {
        code: "log_unavailable",
        message: error.to_string(),
    })?;
    if metadata.len() > 5 * 1024 * 1024 {
        return Err(CommandError {
            code: "log_too_large",
            message: "The job log exceeds the 5 MB display limit".to_owned(),
        });
    }
    let content = std::fs::read_to_string(log_path).map_err(|error| CommandError {
        code: "log_unavailable",
        message: error.to_string(),
    })?;
    Ok(content
        .lines()
        .filter_map(|line| serde_json::from_str::<JobLogEntry>(line).ok())
        .collect())
}

#[tauri::command]
fn cancel_job(job_id: String, state: State<'_, AppState>) -> Result<(), CommandError> {
    let jobs = state.active_jobs.lock().map_err(|_| unavailable_error())?;
    let sender = jobs.get(&job_id).ok_or_else(|| CommandError {
        code: "unknown_job",
        message: "This import is no longer active".to_owned(),
    })?;
    sender.send(true).map_err(|_| unavailable_error())
}

fn validate_sources(source_paths: &[String]) -> Result<(), CommandError> {
    if source_paths.is_empty() {
        return Err(CommandError {
            code: "no_sources",
            message: "Select at least one document".to_owned(),
        });
    }
    if source_paths.len() > 500 {
        return Err(CommandError {
            code: "too_many_sources",
            message: "Select no more than 500 documents at once".to_owned(),
        });
    }
    for source in source_paths {
        let path = Path::new(source);
        let supported = path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| {
                matches!(
                    value.to_ascii_lowercase().as_str(),
                    "pdf" | "docx" | "txt" | "md"
                )
            });
        if !path.is_file() || !supported {
            return Err(CommandError {
                code: "unsupported_source",
                message: "One of the selected documents is missing or unsupported".to_owned(),
            });
        }
    }
    Ok(())
}

async fn run_worker_job(
    job: JobSummary,
    wiki_root: PathBuf,
    source_paths: Vec<String>,
    ocr_language: String,
    on_event: Channel<JobEvent>,
    mut cancel_receiver: watch::Receiver<bool>,
) {
    let catalog = match WikiCatalog::open(&wiki_root) {
        Ok(value) => value,
        Err(_) => return,
    };
    let mut command = worker_command();
    let mut child = match command.spawn() {
        Ok(value) => value,
        Err(_) => {
            finish_with_error(&catalog, &job, &on_event, "stage.worker_unavailable");
            return;
        }
    };
    let mut stdin = match child.stdin.take() {
        Some(value) => value,
        None => {
            finish_with_error(&catalog, &job, &on_event, "stage.worker_unavailable");
            return;
        }
    };
    let stdout = match child.stdout.take() {
        Some(value) => value,
        None => {
            finish_with_error(&catalog, &job, &on_event, "stage.worker_unavailable");
            return;
        }
    };
    if let Some(stderr) = child.stderr.take() {
        tauri::async_runtime::spawn(async move {
            let mut stderr_lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = stderr_lines.next_line().await {
                eprintln!("[LLM Wiki worker] {line}");
            }
        });
    }
    let mut lines = BufReader::new(stdout).lines();
    let handshake_id = Uuid::new_v4().to_string();
    let handshake = json!({
        "protocol_version": "1.0",
        "message_type": "request",
        "request_id": handshake_id,
        "payload": {"action": "handshake"}
    });
    if write_worker_message(&mut stdin, &handshake).await.is_err()
        || !worker_handshake_ready(&mut lines, &handshake_id).await
    {
        finish_with_error(&catalog, &job, &on_event, "stage.worker_unavailable");
        let _ = child.kill().await;
        return;
    }
    let request_id = Uuid::new_v4().to_string();
    let request = json!({
        "protocol_version": "1.0",
        "message_type": "request",
        "request_id": request_id,
        "wiki_id": job.wiki_id,
        "job_id": job.job_id,
        "payload": {
            "action": "start_job",
            "source_count": job.source_count,
            "source_paths": source_paths,
            "wiki_root": wiki_root,
            "ocr_language": ocr_language
        }
    });
    if write_worker_message(&mut stdin, &request).await.is_err() {
        finish_with_error(&catalog, &job, &on_event, "stage.worker_unavailable");
        return;
    }

    let mut cancellation_sent = false;
    loop {
        tokio::select! {
            changed = cancel_receiver.changed(), if !cancellation_sent => {
                if changed.is_ok() && *cancel_receiver.borrow() {
                    cancellation_sent = true;
                    let cancel = json!({
                        "protocol_version": "1.0",
                        "message_type": "request",
                        "request_id": Uuid::new_v4().to_string(),
                        "wiki_id": job.wiki_id,
                        "payload": {"action": "cancel_job", "job_id": job.job_id}
                    });
                    let _ = write_worker_message(&mut stdin, &cancel).await;
                    let _ = catalog.update_job(
                        &job.job_id,
                        JobState::Cancelled,
                        0.0,
                        Some("stage.cancelled"),
                    );
                    let _ = on_event.send(JobEvent {
                        job_id: job.job_id.clone(),
                        state: JobState::Cancelled,
                        progress: 0.0,
                        message: "stage.cancelled".to_owned(),
                        log_level: Some("warning".to_owned()),
                        source: None,
                        detail: None,
                    });
                }
            }
            next_line = lines.next_line() => {
                let Ok(Some(line)) = next_line else {
                    if !cancellation_sent {
                        finish_with_error(&catalog, &job, &on_event, "stage.worker_stopped");
                    }
                    break;
                };
                let Ok(envelope) = serde_json::from_str::<IpcEnvelope>(&line) else { continue; };
                if envelope.request_id != request_id { continue; }
                if cancellation_sent {
                    if matches!(envelope.message_type, MessageType::Response | MessageType::Error) {
                        break;
                    }
                    continue;
                }
                if handle_worker_event(&catalog, &job, &on_event, envelope) { break; }
            }
        }
    }
    let _ = child.kill().await;
}

async fn worker_handshake_ready(
    lines: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    request_id: &str,
) -> bool {
    let Ok(Ok(Some(line))) =
        tokio::time::timeout(std::time::Duration::from_secs(5), lines.next_line()).await
    else {
        return false;
    };
    let Ok(envelope) = serde_json::from_str::<IpcEnvelope>(&line) else {
        return false;
    };
    envelope.request_id == request_id
        && envelope.message_type == MessageType::Response
        && envelope
            .payload
            .get("status")
            .and_then(|value| value.as_str())
            == Some("ready")
}

async fn write_worker_message(
    stdin: &mut tokio::process::ChildStdin,
    value: &serde_json::Value,
) -> Result<(), std::io::Error> {
    stdin.write_all(value.to_string().as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await
}

fn handle_worker_event(
    catalog: &WikiCatalog,
    job: &JobSummary,
    channel: &Channel<JobEvent>,
    envelope: IpcEnvelope,
) -> bool {
    match envelope.message_type {
        MessageType::Progress => {
            let state = envelope
                .payload
                .get("state")
                .and_then(|value| value.as_str())
                .map(parse_job_state)
                .unwrap_or(JobState::NeedsReview);
            let progress = envelope
                .payload
                .get("progress")
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0);
            let message = envelope
                .payload
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or("stage.working");
            let log_level = envelope
                .payload
                .get("log_level")
                .and_then(|value| value.as_str())
                .map(str::to_owned);
            let source = envelope
                .payload
                .get("source")
                .and_then(|value| value.as_str())
                .map(str::to_owned);
            let detail = envelope
                .payload
                .get("detail")
                .and_then(|value| value.as_str())
                .map(str::to_owned);
            if let Some(level) = &log_level {
                println!(
                    "[LLM Wiki][{level}][{}] {message}{}{}",
                    job.job_id,
                    source
                        .as_deref()
                        .map(|value| format!(" ({value})"))
                        .unwrap_or_default(),
                    detail
                        .as_deref()
                        .map(|value| format!(": {value}"))
                        .unwrap_or_default()
                );
            }
            let _ = catalog.update_job(&job.job_id, state, progress, Some(message));
            let _ = channel.send(JobEvent {
                job_id: job.job_id.clone(),
                state,
                progress,
                message: message.to_owned(),
                log_level,
                source,
                detail,
            });
            false
        }
        MessageType::Response => {
            let _ = catalog.update_job(
                &job.job_id,
                JobState::Completed,
                1.0,
                Some("stage.completed"),
            );
            let _ = channel.send(JobEvent {
                job_id: job.job_id.clone(),
                state: JobState::Completed,
                progress: 1.0,
                message: "stage.completed".to_owned(),
                log_level: Some("info".to_owned()),
                source: None,
                detail: None,
            });
            true
        }
        MessageType::Error => {
            let cancelled = envelope
                .payload
                .get("category")
                .and_then(|value| value.as_str())
                == Some("cancelled");
            let state = if cancelled {
                JobState::Cancelled
            } else {
                JobState::Failed
            };
            let message = if cancelled {
                "stage.cancelled"
            } else {
                "stage.failed"
            };
            let detail = envelope
                .payload
                .get("detail")
                .and_then(|value| value.as_str())
                .map(str::to_owned);
            if let Some(value) = &detail {
                eprintln!("[LLM Wiki][ERROR][{}] {message}: {value}", job.job_id);
            }
            let _ = catalog.update_job(&job.job_id, state, 0.0, Some(message));
            let _ = channel.send(JobEvent {
                job_id: job.job_id.clone(),
                state,
                progress: 0.0,
                message: message.to_owned(),
                log_level: Some(if cancelled { "warning" } else { "error" }.to_owned()),
                source: None,
                detail,
            });
            true
        }
        MessageType::Request => false,
    }
}

fn finish_with_error(
    catalog: &WikiCatalog,
    job: &JobSummary,
    channel: &Channel<JobEvent>,
    message: &str,
) {
    let _ = catalog.update_job(&job.job_id, JobState::Failed, 0.0, Some(message));
    let _ = channel.send(JobEvent {
        job_id: job.job_id.clone(),
        state: JobState::Failed,
        progress: 0.0,
        message: message.to_owned(),
        log_level: Some("error".to_owned()),
        source: None,
        detail: None,
    });
}

fn worker_command() -> Command {
    let root = repository_root();
    let python = worker_python();
    let mut command = Command::new(python);
    command
        .arg("-m")
        .arg("llm_wiki_engine.cli")
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn worker_python() -> PathBuf {
    std::env::var_os("LLM_WIKI_PYTHON")
        .map(PathBuf::from)
        .unwrap_or_else(|| repository_root().join(".venv/Scripts/python.exe"))
}

fn performance_status() -> PerformanceStatus {
    let gpu_output = StdCommand::new("nvidia-smi")
        .args(["--query-gpu=name", "--format=csv,noheader"])
        .stdin(Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success());
    let nvidia_name = gpu_output.and_then(|output| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
    });
    if nvidia_name.is_none() {
        return PerformanceStatus {
            nvidia_present: false,
            cuda_enabled: false,
            device_name: None,
        };
    }
    let cuda_output = StdCommand::new(worker_python())
        .args([
            "-c",
            "import torch; print(torch.cuda.get_device_name(0) if torch.cuda.is_available() else '')",
        ])
        .stdin(Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success());
    let cuda_name = cuda_output.and_then(|output| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
    });
    PerformanceStatus {
        nvidia_present: true,
        cuda_enabled: cuda_name.is_some(),
        device_name: cuda_name.or(nvidia_name),
    }
}

fn parse_job_state(value: &str) -> JobState {
    match value {
        "acquiring" => JobState::Acquiring,
        "extracting" => JobState::Extracting,
        "ingesting" => JobState::Ingesting,
        "validating" => JobState::Validating,
        "staging" => JobState::Staging,
        "publishing" => JobState::Publishing,
        _ => JobState::NeedsReview,
    }
}

fn unavailable_error() -> CommandError {
    CommandError {
        code: "unavailable",
        message: "The operation is temporarily unavailable".to_owned(),
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let application_data = app.path().app_local_data_dir()?;
            let user_profile = std::env::var_os("USERPROFILE").map(PathBuf::from);
            let installation_directory = std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(PathBuf::from));
            let registry =
                RegistryStore::new(application_data, user_profile, installation_directory);
            if let Ok(snapshot) = registry.snapshot() {
                for wiki in snapshot.wikis {
                    if let Ok(catalog) = WikiCatalog::open(wiki.canonical_root) {
                        let _ = catalog.recover_interrupted_jobs();
                    }
                }
            }
            app.manage(AppState {
                registry: Mutex::new(registry),
                active_jobs: Arc::new(Mutex::new(HashMap::new())),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_registry,
            set_interface_language,
            create_wiki,
            register_wiki,
            open_wiki,
            rename_wiki,
            remove_wiki_registration,
            get_wiki_settings,
            get_performance_status,
            install_nvidia_acceleration,
            list_jobs,
            start_import,
            cancel_job,
            read_job_log
        ])
        .run(tauri::generate_context!())
        .expect("error while running LLM Wiki Desktop");
}
