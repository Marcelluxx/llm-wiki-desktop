use std::{path::PathBuf, sync::Mutex};

use llm_wiki_app_core::{RegistrySnapshot, RegistryStore, WikiRegistration, WikiSettings};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

struct AppState {
    registry: Mutex<RegistryStore>,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WikiInput {
    display_name: String,
    root: String,
    note_language: String,
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let application_data = app.path().app_local_data_dir()?;
            let user_profile = std::env::var_os("USERPROFILE").map(PathBuf::from);
            let installation_directory = std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(PathBuf::from));
            app.manage(AppState {
                registry: Mutex::new(RegistryStore::new(
                    application_data,
                    user_profile,
                    installation_directory,
                )),
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
            get_wiki_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running LLM Wiki Desktop");
}
