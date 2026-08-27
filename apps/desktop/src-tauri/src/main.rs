#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::{Command as StdCommand, Stdio},
    sync::{Arc, Mutex},
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use llm_wiki_app_core::{
    CatalogError, IpcEnvelope, JobState, JobSummary, MessageType, ProviderId, ProviderModel,
    RegistrySnapshot, RegistryStore, WikiCatalog, WikiRegistration, WikiSettings,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{Manager, State, ipc::Channel};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::Command,
    sync::watch,
};
use uuid::Uuid;

mod providers;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
fn hide_std_command_window(command: &mut StdCommand) {
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(windows)]
fn hide_async_command_window(command: &mut Command) {
    command.creation_flags(CREATE_NO_WINDOW);
}

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

#[derive(Debug, Clone, Serialize)]
struct ProviderActionLogEvent {
    provider_id: ProviderId,
    level: String,
    message: String,
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
fn set_selected_provider(
    provider_id: ProviderId,
    state: State<'_, AppState>,
) -> Result<RegistrySnapshot, CommandError> {
    with_registry(state, |registry| {
        registry.set_selected_provider(provider_id)
    })
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
async fn list_provider_statuses(detailed: Option<bool>) -> Vec<llm_wiki_app_core::ProviderSummary> {
    let _ = detailed;
    providers::detect_all_fast()
}

#[tauri::command]
async fn run_provider_action(
    provider_id: ProviderId,
    action: String,
    on_event: Channel<ProviderActionLogEvent>,
) -> Result<(), CommandError> {
    let script = provider_script(provider_id, &action)?;
    let title = format!("LLM Wiki - {:?}", provider_id);
    let failure_tail = if cfg!(debug_assertions) {
        "Read-Host 'Premi Invio per chiudere'; exit 1"
    } else {
        "exit 1"
    };
    let wrapped = format!(
        "$Host.UI.RawUI.WindowTitle='{title}'; $ErrorActionPreference='Stop'; try {{ {script}; Write-Host ''; Write-Host 'Operazione completata. Ritorno a LLM Wiki...' -ForegroundColor Green; Start-Sleep -Milliseconds 900 }} catch {{ Write-Host ''; Write-Host 'Operazione non riuscita:' -ForegroundColor Red; Write-Host $_.Exception.Message -ForegroundColor Red; {failure_tail} }}"
    );
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
        ])
        .arg(wrapped);
    #[cfg(windows)]
    {
        if cfg!(debug_assertions) {
            command.creation_flags(0x0000_0010);
        } else {
            command.creation_flags(0x0800_0000);
        }
    }
    let status = run_provider_process(command, provider_id, &on_event).await?;
    if !status.success() {
        return Err(CommandError {
            code: "provider_action_failed",
            message: format!(
                "The {:?} operation exited with status {status}",
                provider_id
            ),
        });
    }
    Ok(())
}

async fn run_provider_process(
    mut command: Command,
    provider_id: ProviderId,
    on_event: &Channel<ProviderActionLogEvent>,
) -> Result<std::process::ExitStatus, CommandError> {
    let _ = on_event.send(ProviderActionLogEvent {
        provider_id,
        level: "info".to_owned(),
        message: "Operazione avviata".to_owned(),
    });
    if cfg!(debug_assertions) {
        return command.status().await.map_err(|error| CommandError {
            code: "provider_action_failed",
            message: error.to_string(),
        });
    }

    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| CommandError {
        code: "provider_action_failed",
        message: error.to_string(),
    })?;
    let stdout_task = child.stdout.take().map(|stdout| {
        let channel = on_event.clone();
        tauri::async_runtime::spawn(stream_provider_output(stdout, channel, provider_id, "info"))
    });
    let stderr_task = child.stderr.take().map(|stderr| {
        let channel = on_event.clone();
        tauri::async_runtime::spawn(stream_provider_output(
            stderr,
            channel,
            provider_id,
            "detail",
        ))
    });
    let status = child.wait().await.map_err(|error| CommandError {
        code: "provider_action_failed",
        message: error.to_string(),
    })?;
    if let Some(task) = stdout_task {
        let _ = task.await;
    }
    if let Some(task) = stderr_task {
        let _ = task.await;
    }
    Ok(status)
}

async fn stream_provider_output(
    reader: impl AsyncRead + Unpin,
    channel: Channel<ProviderActionLogEvent>,
    provider_id: ProviderId,
    level: &'static str,
) {
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let message = line.trim();
        if message.is_empty() {
            continue;
        }
        let _ = channel.send(ProviderActionLogEvent {
            provider_id,
            level: level.to_owned(),
            message: message.chars().take(500).collect(),
        });
    }
}

fn provider_script(provider_id: ProviderId, action: &str) -> Result<&'static str, CommandError> {
    let script = match (provider_id, action) {
        (ProviderId::Codex, "install" | "update") => {
            "Write-Host 'Download e installazione di Codex (fonte ufficiale OpenAI)...'; irm https://chatgpt.com/codex/install.ps1 | iex"
        }
        (ProviderId::Claude, "install" | "update") => {
            r#"$runtime = Join-Path $env:LOCALAPPDATA 'LLMWiki\runtime'; $nodeRoot = Join-Path $runtime 'node'; $node = Join-Path $nodeRoot 'node.exe'; if (-not (Test-Path $node)) { Write-Host 'Preparazione del runtime Node.js privato di LLM Wiki...'; $release = (irm https://nodejs.org/dist/index.json | Where-Object { $_.lts } | Select-Object -First 1); $version = $release.version; $archiveName = "node-$version-win-x64.zip"; $archive = Join-Path $env:TEMP $archiveName; $url = "https://nodejs.org/dist/$version/$archiveName"; Write-Host "Download $url"; Invoke-WebRequest $url -OutFile $archive; Write-Host 'Verifica SHA-256...'; $checksums = (irm "https://nodejs.org/dist/$version/SHASUMS256.txt"); $expected = (($checksums -split "`n" | Where-Object { $_ -match [regex]::Escape($archiveName) }) -split '\s+')[0]; $actual = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant(); if ($actual -ne $expected) { throw 'Verifica SHA-256 di Node.js non riuscita' }; $extract = Join-Path $runtime 'node-extract'; New-Item $extract -ItemType Directory -Force | Out-Null; Expand-Archive $archive $extract -Force; $folder = Get-ChildItem $extract -Directory | Select-Object -First 1; New-Item $nodeRoot -ItemType Directory -Force | Out-Null; Copy-Item (Join-Path $folder.FullName '*') $nodeRoot -Recurse -Force; Remove-Item $archive -Force; Remove-Item $extract -Recurse -Force }; $prefix = Join-Path $runtime 'providers\claude'; Write-Host 'Installazione Claude Code dal pacchetto ufficiale Anthropic...'; & (Join-Path $nodeRoot 'npm.cmd') install --prefix $prefix @anthropic-ai/claude-code"#
        }
        (ProviderId::Antigravity, "install" | "update") => {
            "Write-Host 'Download e installazione di Antigravity (fonte ufficiale Google)...'; irm https://antigravity.google/cli/install.ps1 | iex"
        }
        (ProviderId::Ollama, "install" | "update") => {
            "Write-Host 'Download e installazione di Ollama (fonte ufficiale)...'; irm https://ollama.com/install.ps1 | iex"
        }
        (ProviderId::Codex, "login" | "manage") => "codex login",
        (ProviderId::Claude, "login" | "manage") => {
            r#"$privateClaude = Join-Path $env:LOCALAPPDATA 'LLMWiki\runtime\providers\claude\node_modules\.bin\claude.cmd'; $cli = if (Test-Path $privateClaude) { $privateClaude } else { 'claude' }; $process = Start-Process cmd.exe -ArgumentList @('/d', '/c', ('"' + $cli + '"')) -NoNewWindow -PassThru; $deadline = (Get-Date).AddMinutes(5); do { Start-Sleep -Milliseconds 700; $authenticated = if (Test-Path (Join-Path $env:USERPROFILE '.claude.json')) { try { $null -ne ((Get-Content (Join-Path $env:USERPROFILE '.claude.json') -Raw | ConvertFrom-Json).oauthAccount) } catch { $false } } else { $false }; if ($authenticated) { taskkill.exe /PID $process.Id /T /F 2>$null | Out-Null; break }; if ($process.HasExited) { throw 'Claude si è chiuso prima di completare l’accesso' } } while ((Get-Date) -lt $deadline); if (-not $authenticated) { taskkill.exe /PID $process.Id /T /F 2>$null | Out-Null; throw 'Tempo scaduto durante l’accesso a Claude' }"#
        }
        (ProviderId::Antigravity, "login" | "manage") => {
            r#"$cli = (Get-Command agy -ErrorAction Stop).Source; $process = Start-Process $cli -NoNewWindow -PassThru; $deadline = (Get-Date).AddMinutes(5); do { Start-Sleep -Milliseconds 700; $authenticated = (cmdkey.exe /list | Out-String) -match 'gemini:antigravity'; if ($authenticated) { taskkill.exe /PID $process.Id /T /F 2>$null | Out-Null; break }; if ($process.HasExited) { throw 'Antigravity si è chiuso prima di completare l’accesso' } } while ((Get-Date) -lt $deadline); if (-not $authenticated) { taskkill.exe /PID $process.Id /T /F 2>$null | Out-Null; throw 'Tempo scaduto durante l’accesso ad Antigravity' }"#
        }
        (ProviderId::Ollama, "start" | "manage") => {
            r#"$knownOllama = Join-Path $env:LOCALAPPDATA 'Programs\Ollama\ollama.exe'; $ollama = if (Test-Path $knownOllama) { $knownOllama } else { (Get-Command ollama -ErrorAction Stop).Source }; Start-Process $ollama -ArgumentList 'serve' -WindowStyle Hidden; $ready = $false; for ($attempt = 1; $attempt -le 30; $attempt++) { try { $null = Invoke-RestMethod http://127.0.0.1:11434/api/version -TimeoutSec 1; $ready = $true; break } catch { Start-Sleep -Milliseconds 350 } }; if (-not $ready) { throw 'Ollama non ha avviato il servizio locale entro il tempo previsto' }; Write-Host 'Ollama è pronto su 127.0.0.1:11434.'"#
        }
        _ => {
            return Err(CommandError {
                code: "provider_action_unavailable",
                message: "This provider action is not available".to_owned(),
            });
        }
    };
    Ok(script)
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModel {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct OllamaModelsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
    #[serde(default)]
    size: Option<u64>,
}

#[tauri::command]
async fn list_provider_models(provider_id: ProviderId) -> Result<Vec<ProviderModel>, CommandError> {
    if !matches!(provider_id, ProviderId::Openrouter | ProviderId::Ollama) {
        return Ok(Vec::new());
    }
    let url = if provider_id == ProviderId::Openrouter {
        "https://openrouter.ai/api/v1/models"
    } else {
        "http://127.0.0.1:11434/api/tags"
    };
    let mut command = Command::new("curl.exe");
    command.args([
        "--fail",
        "--silent",
        "--show-error",
        "--connect-timeout",
        "5",
        "--max-time",
        "12",
        url,
    ]);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    let output = command.output().await.map_err(|error| CommandError {
        code: "provider_models_unavailable",
        message: error.to_string(),
    })?;
    if !output.status.success() {
        return Err(CommandError {
            code: "provider_models_unavailable",
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    if provider_id == ProviderId::Openrouter {
        let response: OpenRouterModelsResponse =
            serde_json::from_slice(&output.stdout).map_err(|error| CommandError {
                code: "provider_models_invalid",
                message: error.to_string(),
            })?;
        return Ok(response
            .data
            .into_iter()
            .map(|model| ProviderModel {
                model_id: model.id,
                display_name: model.name,
                size_bytes: None,
                local: false,
            })
            .collect());
    }
    let response: OllamaModelsResponse =
        serde_json::from_slice(&output.stdout).map_err(|error| CommandError {
            code: "provider_models_invalid",
            message: error.to_string(),
        })?;
    Ok(response
        .models
        .into_iter()
        .map(|model| ProviderModel {
            model_id: model.name.clone(),
            display_name: model.name,
            size_bytes: model.size,
            local: true,
        })
        .collect())
}

#[tauri::command]
fn configure_openrouter(api_key: Option<String>, model_id: String) -> Result<(), CommandError> {
    if let Some(api_key) = api_key {
        let trimmed = api_key.trim();
        if !trimmed.starts_with("sk-or-") || trimmed.len() < 20 {
            return Err(CommandError {
                code: "invalid_openrouter_key",
                message: "The OpenRouter API key does not have the expected format".to_owned(),
            });
        }
        write_openrouter_credential(trimmed)?;
    } else if !openrouter_credential_exists() {
        return Err(CommandError {
            code: "openrouter_key_required",
            message: "Enter an OpenRouter API key".to_owned(),
        });
    }
    let model_id = model_id.trim();
    if model_id.is_empty() {
        return Err(CommandError {
            code: "openrouter_model_required",
            message: "Select an OpenRouter model".to_owned(),
        });
    }
    save_openrouter_model(model_id)
}

#[tauri::command]
fn configure_ollama(model_id: String) -> Result<(), CommandError> {
    save_provider_model("ollama", model_id.trim())
}

#[tauri::command]
async fn pull_ollama_model(
    model_id: String,
    on_event: Channel<ProviderActionLogEvent>,
) -> Result<(), CommandError> {
    let model_id = model_id.trim();
    if model_id.is_empty()
        || !model_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".:_-/".contains(character))
    {
        return Err(CommandError {
            code: "invalid_ollama_model",
            message: "Enter a valid Ollama model name".to_owned(),
        });
    }
    let known_ollama = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|root| root.join("Programs/Ollama/ollama.exe"));
    let program = known_ollama
        .as_ref()
        .filter(|path| path.is_file())
        .map(|path| path.as_os_str())
        .unwrap_or_else(|| std::ffi::OsStr::new("ollama.exe"));
    let mut command = Command::new(program);
    command.args(["pull", model_id]);
    #[cfg(windows)]
    {
        if cfg!(debug_assertions) {
            command.creation_flags(0x0000_0010);
        } else {
            command.creation_flags(0x0800_0000);
        }
    }
    let _ = on_event.send(ProviderActionLogEvent {
        provider_id: ProviderId::Ollama,
        level: "info".to_owned(),
        message: format!("Download del modello {model_id}"),
    });
    let status = run_provider_process(command, ProviderId::Ollama, &on_event).await?;
    if !status.success() {
        return Err(CommandError {
            code: "ollama_pull_failed",
            message: format!("Ollama exited with status {status}"),
        });
    }
    Ok(())
}

fn openrouter_config_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("com.llmwiki.desktop/providers/openrouter.json")
}

fn save_openrouter_model(model_id: &str) -> Result<(), CommandError> {
    save_provider_model("openrouter", model_id)
}

fn save_provider_model(provider: &str, model_id: &str) -> Result<(), CommandError> {
    if model_id.is_empty() {
        return Err(CommandError {
            code: "provider_model_required",
            message: "Select a provider model".to_owned(),
        });
    }
    let path = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(format!("com.llmwiki.desktop/providers/{provider}.json"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| CommandError {
            code: "provider_config_failed",
            message: error.to_string(),
        })?;
    }
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({ "model_id": model_id })).unwrap(),
    )
    .map_err(|error| CommandError {
        code: "provider_config_failed",
        message: error.to_string(),
    })
}

pub(crate) fn openrouter_selected_model() -> Option<String> {
    let bytes = std::fs::read(openrouter_config_path()).ok()?;
    serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()?
        .get("model_id")?
        .as_str()
        .map(str::to_owned)
}

pub(crate) fn ollama_selected_model() -> Option<String> {
    let path = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("com.llmwiki.desktop/providers/ollama.json");
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()?
        .get("model_id")?
        .as_str()
        .map(str::to_owned)
}

const OPENROUTER_CREDENTIAL_TARGET: &str = "LLMWiki.OpenRouter";

#[cfg(windows)]
#[repr(C)]
struct WindowsCredential {
    flags: u32,
    credential_type: u32,
    target_name: *mut u16,
    comment: *mut u16,
    last_written: [u32; 2],
    credential_blob_size: u32,
    credential_blob: *mut u8,
    persist: u32,
    attribute_count: u32,
    attributes: *mut std::ffi::c_void,
    target_alias: *mut u16,
    user_name: *mut u16,
}

#[cfg(windows)]
#[link(name = "Advapi32")]
unsafe extern "system" {
    fn CredWriteW(credential: *const WindowsCredential, flags: u32) -> i32;
    fn CredReadW(
        target: *const u16,
        credential_type: u32,
        flags: u32,
        credential: *mut *mut WindowsCredential,
    ) -> i32;
    fn CredFree(buffer: *mut std::ffi::c_void);
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(windows)]
fn write_openrouter_credential(api_key: &str) -> Result<(), CommandError> {
    let mut target = wide_null(OPENROUTER_CREDENTIAL_TARGET);
    let mut username = wide_null("OpenRouter API key");
    let mut blob = api_key.as_bytes().to_vec();
    let credential = WindowsCredential {
        flags: 0,
        credential_type: 1,
        target_name: target.as_mut_ptr(),
        comment: std::ptr::null_mut(),
        last_written: [0, 0],
        credential_blob_size: blob.len() as u32,
        credential_blob: blob.as_mut_ptr(),
        persist: 2,
        attribute_count: 0,
        attributes: std::ptr::null_mut(),
        target_alias: std::ptr::null_mut(),
        user_name: username.as_mut_ptr(),
    };
    let written = unsafe { CredWriteW(&credential, 0) };
    blob.fill(0);
    if written == 0 {
        return Err(CommandError {
            code: "credential_store_failed",
            message: std::io::Error::last_os_error().to_string(),
        });
    }
    Ok(())
}

#[cfg(not(windows))]
fn write_openrouter_credential(_api_key: &str) -> Result<(), CommandError> {
    Err(CommandError {
        code: "credential_store_unavailable",
        message: "Windows Credential Manager is unavailable".to_owned(),
    })
}

#[cfg(windows)]
pub(crate) fn openrouter_credential_exists() -> bool {
    windows_credential_exists(OPENROUTER_CREDENTIAL_TARGET)
}

#[cfg(windows)]
fn windows_credential_exists(target_name: &str) -> bool {
    let target = wide_null(target_name);
    let mut credential = std::ptr::null_mut();
    let found = unsafe { CredReadW(target.as_ptr(), 1, 0, &mut credential) } != 0;
    if !credential.is_null() {
        unsafe { CredFree(credential.cast()) };
    }
    found
}

#[cfg(not(windows))]
pub(crate) fn openrouter_credential_exists() -> bool {
    false
}

pub(crate) fn provider_auth_marker_exists(provider_id: ProviderId) -> bool {
    let profile = std::env::var_os("USERPROFILE").map(PathBuf::from);
    match provider_id {
        ProviderId::Codex => profile.is_some_and(|root| root.join(".codex/auth.json").is_file()),
        ProviderId::Claude => profile.is_some_and(|root| {
            std::fs::read_to_string(root.join(".claude.json"))
                .ok()
                .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
                .is_some_and(|value| value.get("oauthAccount").is_some())
        }),
        ProviderId::Antigravity => {
            #[cfg(windows)]
            {
                windows_credential_exists("gemini:antigravity")
            }
            #[cfg(not(windows))]
            {
                false
            }
        }
        _ => false,
    }
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
    let mut command = Command::new("powershell.exe");
    command
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script)
        .current_dir(repository_root());
    #[cfg(windows)]
    hide_async_command_window(&mut command);
    let output = command.output().await.map_err(|error| CommandError {
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
    #[cfg(windows)]
    hide_async_command_window(&mut command);
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
    let mut gpu_command = StdCommand::new("nvidia-smi");
    gpu_command
        .args(["--query-gpu=name", "--format=csv,noheader"])
        .stdin(Stdio::null());
    #[cfg(windows)]
    hide_std_command_window(&mut gpu_command);
    let gpu_output = gpu_command
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
    let mut cuda_command = StdCommand::new(worker_python());
    cuda_command
        .args([
            "-c",
            "import torch; print(torch.cuda.get_device_name(0) if torch.cuda.is_available() else '')",
        ])
        .stdin(Stdio::null());
    #[cfg(windows)]
    hide_std_command_window(&mut cuda_command);
    let cuda_output = cuda_command
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
    std::panic::set_hook(Box::new(|info| {
        eprintln!("[LLM Wiki] Panic occurred: {info}");
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let application_data = app.path().app_local_data_dir().map_err(|err| {
                eprintln!("[LLM Wiki] Failed to get app_local_data_dir: {err}");
                err
            })?;
            let user_profile = std::env::var_os("USERPROFILE").map(PathBuf::from);
            let installation_directory = std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(PathBuf::from));
            let registry =
                RegistryStore::new(application_data, user_profile, installation_directory);
            let recovery_roots = registry
                .snapshot()
                .map(|snapshot| {
                    snapshot
                        .wikis
                        .into_iter()
                        .map(|wiki| wiki.canonical_root)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            std::thread::spawn(move || {
                for root in recovery_roots {
                    if let Ok(catalog) = WikiCatalog::open(root) {
                        let _ = catalog.recover_interrupted_jobs();
                    }
                }
            });
            app.manage(AppState {
                registry: Mutex::new(registry),
                active_jobs: Arc::new(Mutex::new(HashMap::new())),
            });
            println!("[LLM Wiki] Setup completed successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_registry,
            set_interface_language,
            set_selected_provider,
            create_wiki,
            register_wiki,
            open_wiki,
            rename_wiki,
            remove_wiki_registration,
            get_wiki_settings,
            get_performance_status,
            list_provider_statuses,
            run_provider_action,
            list_provider_models,
            configure_openrouter,
            configure_ollama,
            pull_ollama_model,
            install_nvidia_acceleration,
            list_jobs,
            start_import,
            cancel_job,
            read_job_log
        ])
        .run(tauri::generate_context!())
        .expect("error while running LLM Wiki Desktop");
}
