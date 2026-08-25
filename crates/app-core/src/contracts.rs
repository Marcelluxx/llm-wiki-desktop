use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CONTRACT_VERSION: &str = "1.0";

pub const MESSAGE_TYPES: &[&str] = &["request", "response", "progress", "error"];
pub const JOB_STATES: &[&str] = &[
    "queued",
    "acquiring",
    "extracting",
    "ingesting",
    "validating",
    "staging",
    "publishing",
    "completed",
    "needs_review",
    "cancelled",
    "failed",
];
pub const SOURCE_FORMATS: &[&str] = &["pdf", "docx", "txt", "md"];
pub const ERROR_CATEGORIES: &[&str] = &[
    "validation",
    "invalid_request",
    "unavailable",
    "timeout",
    "cancelled",
    "internal",
];
pub const REVIEW_SEVERITIES: &[&str] = &["info", "warning", "error"];
pub const PROVIDER_IDS: &[&str] = &["codex", "claude", "antigravity", "fake"];
pub const INGEST_STRATEGIES: &[&str] = &["forced_layout_ocr", "direct_text", "structured_docx"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Request,
    Response,
    Progress,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Acquiring,
    Extracting,
    Ingesting,
    Validating,
    Staging,
    Publishing,
    Completed,
    NeedsReview,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceFormat {
    Pdf,
    Docx,
    Txt,
    Md,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    Validation,
    InvalidRequest,
    Unavailable,
    Timeout,
    Cancelled,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderId {
    Codex,
    Claude,
    Antigravity,
    Fake,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestStrategy {
    ForcedLayoutOcr,
    DirectText,
    StructuredDocx,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IpcEnvelope {
    pub protocol_version: String,
    pub message_type: MessageType,
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wiki_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WikiRegistration {
    pub schema_version: String,
    pub wiki_id: String,
    pub display_name: String,
    pub canonical_root: String,
    pub note_language: String,
    pub created_at: String,
    pub last_opened_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WikiSettings {
    pub schema_version: String,
    pub wiki_id: String,
    pub output_root: String,
    pub note_language: String,
    pub provider_id: ProviderId,
    pub ocr_language: String,
    #[serde(default)]
    pub open_in_obsidian_after_publish: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobCheckpoint {
    pub state: JobState,
    pub completed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobRecord {
    pub schema_version: String,
    pub job_id: String,
    pub wiki_id: String,
    pub state: JobState,
    pub stage_progress: f64,
    pub created_at: String,
    pub updated_at: String,
    pub checkpoints: Vec<JobCheckpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceManifestEntry {
    pub schema_version: String,
    pub source_id: String,
    pub original_name: String,
    pub source_format: SourceFormat,
    pub content_sha256: String,
    pub byte_size: u64,
    pub ingest_strategy: IngestStrategy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractedPage {
    pub page_number: u32,
    pub markdown: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_confidence: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionArtifact {
    pub schema_version: String,
    pub source_id: String,
    pub extractor: String,
    pub pages: Vec<ExtractedPage>,
    pub plain_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WikiWrite {
    pub relative_path: String,
    pub content_sha256: String,
    pub markdown: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WikiTransaction {
    pub schema_version: String,
    pub transaction_id: String,
    pub wiki_id: String,
    pub writes: Vec<WikiWrite>,
    pub deletes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Open,
    Accepted,
    Rejected,
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewItem {
    pub schema_version: String,
    pub review_id: String,
    pub job_id: String,
    pub severity: ReviewSeverity,
    pub code: String,
    pub message: String,
    pub status: ReviewStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationResult {
    pub schema_version: String,
    pub transaction_id: String,
    pub published_at: String,
    pub written_paths: Vec<String>,
    pub backup_created: bool,
}
