use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::contracts::{CONTRACT_VERSION, ProviderId, WikiRegistration, WikiSettings};

const REGISTRY_FILE_NAME: &str = "wiki-registry.json";
const RESERVED_DIRECTORY: &str = ".llm-wiki";
const VISIBLE_DIRECTORIES: &[&str] = &[
    "sources",
    "concepts",
    "entities",
    "syntheses",
    "indexes",
    "attachments",
];
const INTERNAL_DIRECTORIES: &[&str] = &["raw", "artifacts", "staging", "backups"];

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("The wiki name cannot be empty")]
    EmptyName,
    #[error("The wiki path must be an absolute path")]
    RelativePath,
    #[error("The selected folder does not exist")]
    MissingPath,
    #[error("The selected path is not a folder")]
    NotDirectory,
    #[error("Select a folder inside the drive or user profile, not the broad root itself")]
    ForbiddenRoot,
    #[error("This folder overlaps another registered wiki")]
    DuplicateOrNestedPath,
    #[error("No wiki with id {0} is registered")]
    UnknownWiki(String),
    #[error("The interface language must be 'it' or 'en'")]
    UnsupportedLanguage,
    #[error("Cannot access the selected location: {0}")]
    Io(#[from] std::io::Error),
    #[error("The local registry is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Cannot create a timestamp: {0}")]
    Time(#[from] time::error::Format),
}

impl RegistryError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EmptyName => "empty_name",
            Self::RelativePath => "relative_path",
            Self::MissingPath => "missing_path",
            Self::NotDirectory => "not_directory",
            Self::ForbiddenRoot => "forbidden_root",
            Self::DuplicateOrNestedPath => "duplicate_or_nested_path",
            Self::UnknownWiki(_) => "unknown_wiki",
            Self::UnsupportedLanguage => "unsupported_language",
            Self::Io(_) => "io",
            Self::Json(_) => "invalid_registry",
            Self::Time(_) => "internal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrySnapshot {
    pub schema_version: String,
    pub interface_language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_provider_id: Option<ProviderId>,
    pub wikis: Vec<WikiRegistration>,
}

impl Default for RegistrySnapshot {
    fn default() -> Self {
        Self {
            schema_version: CONTRACT_VERSION.to_owned(),
            interface_language: None,
            selected_provider_id: None,
            wikis: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RegistryStore {
    registry_path: PathBuf,
    forbidden_roots: Vec<PathBuf>,
}

impl RegistryStore {
    pub fn new(
        application_data_directory: impl AsRef<Path>,
        user_profile: Option<PathBuf>,
        installation_directory: Option<PathBuf>,
    ) -> Self {
        let forbidden_roots = [user_profile, installation_directory]
            .into_iter()
            .flatten()
            .filter_map(|path| canonicalize_existing(&path).ok())
            .collect();

        Self {
            registry_path: application_data_directory.as_ref().join(REGISTRY_FILE_NAME),
            forbidden_roots,
        }
    }

    pub fn snapshot(&self) -> Result<RegistrySnapshot, RegistryError> {
        if !self.registry_path.exists() {
            return Ok(RegistrySnapshot::default());
        }
        let bytes = fs::read(&self.registry_path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn set_interface_language(
        &self,
        language: &str,
    ) -> Result<RegistrySnapshot, RegistryError> {
        if !matches!(language, "it" | "en") {
            return Err(RegistryError::UnsupportedLanguage);
        }
        let mut snapshot = self.snapshot()?;
        snapshot.interface_language = Some(language.to_owned());
        self.save(&snapshot)?;
        Ok(snapshot)
    }

    pub fn set_selected_provider(
        &self,
        provider_id: ProviderId,
    ) -> Result<RegistrySnapshot, RegistryError> {
        let mut snapshot = self.snapshot()?;
        snapshot.selected_provider_id = Some(provider_id);
        self.save(&snapshot)?;
        Ok(snapshot)
    }

    pub fn create_wiki(
        &self,
        display_name: &str,
        requested_root: impl AsRef<Path>,
        note_language: &str,
    ) -> Result<WikiRegistration, RegistryError> {
        self.add_wiki(display_name, requested_root.as_ref(), note_language, false)
    }

    pub fn register_wiki(
        &self,
        display_name: &str,
        requested_root: impl AsRef<Path>,
        note_language: &str,
    ) -> Result<WikiRegistration, RegistryError> {
        self.add_wiki(display_name, requested_root.as_ref(), note_language, true)
    }

    pub fn open_wiki(&self, wiki_id: &str) -> Result<WikiRegistration, RegistryError> {
        let mut snapshot = self.snapshot()?;
        let wiki = snapshot
            .wikis
            .iter_mut()
            .find(|wiki| wiki.wiki_id == wiki_id)
            .ok_or_else(|| RegistryError::UnknownWiki(wiki_id.to_owned()))?;
        wiki.last_opened_at = timestamp()?;
        let result = wiki.clone();
        self.save(&snapshot)?;
        Ok(result)
    }

    pub fn rename_wiki(
        &self,
        wiki_id: &str,
        display_name: &str,
    ) -> Result<WikiRegistration, RegistryError> {
        let display_name = validate_name(display_name)?;
        let mut snapshot = self.snapshot()?;
        let wiki = snapshot
            .wikis
            .iter_mut()
            .find(|wiki| wiki.wiki_id == wiki_id)
            .ok_or_else(|| RegistryError::UnknownWiki(wiki_id.to_owned()))?;
        wiki.display_name = display_name;
        let result = wiki.clone();
        self.save(&snapshot)?;
        Ok(result)
    }

    pub fn remove_registration(&self, wiki_id: &str) -> Result<RegistrySnapshot, RegistryError> {
        let mut snapshot = self.snapshot()?;
        let previous_length = snapshot.wikis.len();
        snapshot.wikis.retain(|wiki| wiki.wiki_id != wiki_id);
        if snapshot.wikis.len() == previous_length {
            return Err(RegistryError::UnknownWiki(wiki_id.to_owned()));
        }
        self.save(&snapshot)?;
        Ok(snapshot)
    }

    pub fn read_settings(&self, wiki_id: &str) -> Result<WikiSettings, RegistryError> {
        let snapshot = self.snapshot()?;
        let wiki = find_wiki(&snapshot, wiki_id)?;
        let bytes = fs::read(
            Path::new(&wiki.canonical_root)
                .join(RESERVED_DIRECTORY)
                .join("settings.json"),
        )?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn add_wiki(
        &self,
        display_name: &str,
        requested_root: &Path,
        note_language: &str,
        must_exist: bool,
    ) -> Result<WikiRegistration, RegistryError> {
        let display_name = validate_name(display_name)?;
        let mut snapshot = self.snapshot()?;
        let canonical_root = self.validate_root(requested_root, must_exist, &snapshot)?;

        let wiki_id = existing_wiki_id(&canonical_root)?
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        if snapshot.wikis.iter().any(|wiki| wiki.wiki_id == wiki_id) {
            return Err(RegistryError::DuplicateOrNestedPath);
        }
        fs::create_dir_all(&canonical_root)?;
        create_wiki_skeleton(&canonical_root, &wiki_id, note_language)?;

        let now = timestamp()?;
        let registration = WikiRegistration {
            schema_version: CONTRACT_VERSION.to_owned(),
            wiki_id,
            display_name,
            canonical_root: display_path(&canonical_root),
            note_language: note_language.to_owned(),
            created_at: now.clone(),
            last_opened_at: now,
        };
        snapshot.wikis.push(registration.clone());
        self.save(&snapshot)?;
        Ok(registration)
    }

    fn validate_root(
        &self,
        requested_root: &Path,
        must_exist: bool,
        snapshot: &RegistrySnapshot,
    ) -> Result<PathBuf, RegistryError> {
        if !requested_root.is_absolute() {
            return Err(RegistryError::RelativePath);
        }
        if must_exist && !requested_root.exists() {
            return Err(RegistryError::MissingPath);
        }
        if requested_root.exists() && !requested_root.is_dir() {
            return Err(RegistryError::NotDirectory);
        }

        let canonical_root = if requested_root.exists() {
            canonicalize_existing(requested_root)?
        } else {
            let parent = requested_root
                .parent()
                .ok_or(RegistryError::ForbiddenRoot)?;
            let leaf = requested_root
                .file_name()
                .ok_or(RegistryError::ForbiddenRoot)?;
            canonicalize_existing(parent)?.join(leaf)
        };

        if canonical_root.components().count() <= 2
            || canonical_root.parent().is_none()
            || self
                .forbidden_roots
                .iter()
                .any(|root| root == &canonical_root)
        {
            return Err(RegistryError::ForbiddenRoot);
        }

        for wiki in &snapshot.wikis {
            let registered = canonicalize_existing(Path::new(&wiki.canonical_root))?;
            if canonical_root == registered
                || canonical_root.starts_with(&registered)
                || registered.starts_with(&canonical_root)
            {
                return Err(RegistryError::DuplicateOrNestedPath);
            }
        }

        Ok(canonical_root)
    }

    fn save(&self, snapshot: &RegistrySnapshot) -> Result<(), RegistryError> {
        if let Some(parent) = self.registry_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(snapshot)?;
        fs::write(&self.registry_path, bytes)?;
        Ok(())
    }
}

fn find_wiki<'a>(
    snapshot: &'a RegistrySnapshot,
    wiki_id: &str,
) -> Result<&'a WikiRegistration, RegistryError> {
    snapshot
        .wikis
        .iter()
        .find(|wiki| wiki.wiki_id == wiki_id)
        .ok_or_else(|| RegistryError::UnknownWiki(wiki_id.to_owned()))
}

fn create_wiki_skeleton(
    root: &Path,
    wiki_id: &str,
    note_language: &str,
) -> Result<(), RegistryError> {
    for directory in VISIBLE_DIRECTORIES {
        fs::create_dir_all(root.join(directory))?;
    }
    let internal_root = root.join(RESERVED_DIRECTORY);
    for directory in INTERNAL_DIRECTORIES {
        fs::create_dir_all(internal_root.join(directory))?;
    }

    let index_path = root.join("index.md");
    if !index_path.exists() {
        let title = if note_language == "it" {
            "La mia wiki"
        } else {
            "My wiki"
        };
        fs::write(index_path, format!("# {title}\n\n"))?;
    }

    let settings_path = internal_root.join("settings.json");
    if !settings_path.exists() {
        let settings = WikiSettings {
            schema_version: CONTRACT_VERSION.to_owned(),
            wiki_id: wiki_id.to_owned(),
            output_root: display_path(root),
            note_language: note_language.to_owned(),
            provider_id: ProviderId::Fake,
            model_id: None,
            use_global_provider: false,
            ocr_language: "ita+eng".to_owned(),
            open_in_obsidian_after_publish: false,
        };
        fs::write(settings_path, serde_json::to_vec_pretty(&settings)?)?;
    }
    Ok(())
}

fn existing_wiki_id(root: &Path) -> Result<Option<String>, RegistryError> {
    let settings_path = root.join(RESERVED_DIRECTORY).join("settings.json");
    if !settings_path.exists() {
        return Ok(None);
    }
    let settings: WikiSettings = serde_json::from_slice(&fs::read(settings_path)?)?;
    Ok(Some(settings.wiki_id))
}

fn canonicalize_existing(path: &Path) -> Result<PathBuf, std::io::Error> {
    path.canonicalize()
}

fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    if let Some(value) = value.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{value}");
    }
    value.strip_prefix(r"\\?\").unwrap_or(&value).to_owned()
}

fn validate_name(display_name: &str) -> Result<String, RegistryError> {
    let value = display_name.trim();
    if value.is_empty() {
        return Err(RegistryError::EmptyName);
    }
    Ok(value.chars().take(80).collect())
}

fn timestamp() -> Result<String, time::error::Format> {
    OffsetDateTime::now_utc().format(&Rfc3339)
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    #[test]
    fn rejects_a_windows_drive_root_before_any_folder_is_created() {
        let system_drive = std::env::var("SystemDrive").expect("Windows system drive");
        let drive_root = PathBuf::from(format!(r"{system_drive}\"));
        let store = RegistryStore::new(std::env::temp_dir().join("llm-wiki-test"), None, None);

        assert!(matches!(
            store.validate_root(&drive_root, false, &RegistrySnapshot::default()),
            Err(RegistryError::ForbiddenRoot)
        ));
    }
}
