"""Typed language bindings for the versioned IPC contracts."""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from typing import Any, NotRequired, TypedDict

CONTRACT_VERSION = "1.0"


class MessageType(StrEnum):
    REQUEST = "request"
    RESPONSE = "response"
    PROGRESS = "progress"
    ERROR = "error"


class JobState(StrEnum):
    QUEUED = "queued"
    ACQUIRING = "acquiring"
    EXTRACTING = "extracting"
    INGESTING = "ingesting"
    VALIDATING = "validating"
    STAGING = "staging"
    PUBLISHING = "publishing"
    COMPLETED = "completed"
    NEEDS_REVIEW = "needs_review"
    CANCELLED = "cancelled"
    FAILED = "failed"


class SourceFormat(StrEnum):
    PDF = "pdf"
    DOCX = "docx"
    TXT = "txt"
    MD = "md"


class ErrorCategory(StrEnum):
    VALIDATION = "validation"
    INVALID_REQUEST = "invalid_request"
    UNAVAILABLE = "unavailable"
    TIMEOUT = "timeout"
    CANCELLED = "cancelled"
    INTERNAL = "internal"


class ReviewSeverity(StrEnum):
    INFO = "info"
    WARNING = "warning"
    ERROR = "error"


class ProviderId(StrEnum):
    CODEX = "codex"
    CLAUDE = "claude"
    ANTIGRAVITY = "antigravity"
    OPENROUTER = "openrouter"
    OLLAMA = "ollama"
    FAKE = "fake"


class ProviderTransport(StrEnum):
    CLI = "cli"
    CLOUD_API = "cloud_api"
    LOCAL_HTTP = "local_http"


class ProviderStatus(StrEnum):
    CHECKING = "checking"
    CONNECTED = "connected"
    NOT_INSTALLED = "not_installed"
    AUTH_REQUIRED = "auth_required"
    KEY_REQUIRED = "key_required"
    INSTALLED_OFFLINE = "installed_offline"
    UPDATE_REQUIRED = "update_required"
    ACTION_REQUIRED = "action_required"
    UNAVAILABLE = "unavailable"


class ProviderOperationState(StrEnum):
    QUEUED = "queued"
    DETECTING = "detecting"
    AWAITING_CONFIRMATION = "awaiting_confirmation"
    DOWNLOADING = "downloading"
    VERIFYING = "verifying"
    INSTALLING = "installing"
    AUTHENTICATING = "authenticating"
    VALIDATING = "validating"
    COMPLETED = "completed"
    CANCELLED = "cancelled"
    FAILED = "failed"
    ACTION_REQUIRED = "action_required"


class IngestStrategy(StrEnum):
    FORCED_LAYOUT_OCR = "forced_layout_ocr"
    DIRECT_TEXT = "direct_text"
    STRUCTURED_DOCX = "structured_docx"


@dataclass(frozen=True, slots=True)
class IpcEnvelope:
    protocol_version: str
    message_type: MessageType
    request_id: str
    payload: dict[str, Any]
    wiki_id: str | None = None
    job_id: str | None = None

    @classmethod
    def from_mapping(cls, value: dict[str, Any]) -> IpcEnvelope:
        allowed = {
            "protocol_version",
            "message_type",
            "request_id",
            "payload",
            "wiki_id",
            "job_id",
        }
        unknown = set(value) - allowed
        if unknown:
            raise ValueError(f"Unknown IPC fields: {sorted(unknown)}")
        return cls(
            protocol_version=str(value["protocol_version"]),
            message_type=MessageType(value["message_type"]),
            request_id=str(value["request_id"]),
            payload=dict(value["payload"]),
            wiki_id=str(value["wiki_id"]) if value.get("wiki_id") is not None else None,
            job_id=str(value["job_id"]) if value.get("job_id") is not None else None,
        )


class WikiRegistration(TypedDict):
    schema_version: str
    wiki_id: str
    display_name: str
    canonical_root: str
    note_language: str
    created_at: str
    last_opened_at: str


class WikiSettings(TypedDict):
    schema_version: str
    wiki_id: str
    output_root: str
    note_language: str
    provider_id: ProviderId
    model_id: NotRequired[str]
    use_global_provider: NotRequired[bool]
    ocr_language: str
    open_in_obsidian_after_publish: NotRequired[bool]


class ProviderModel(TypedDict):
    model_id: str
    display_name: str
    size_bytes: NotRequired[int]
    local: bool


class ProviderSummary(TypedDict):
    provider_id: ProviderId
    display_name: str
    transport: ProviderTransport
    status: ProviderStatus
    version: NotRequired[str]
    selected_model: NotRequired[str]
    detail: NotRequired[str]
    capabilities: list[str]


class ProviderOperationEvent(TypedDict):
    operation_id: str
    provider_id: ProviderId
    state: ProviderOperationState
    message: str
    progress: NotRequired[float]
    bytes_downloaded: NotRequired[int]
    bytes_total: NotRequired[int]
    bytes_per_second: NotRequired[int]
    eta_seconds: NotRequired[int]
    elapsed_seconds: NotRequired[int]
    attempt: NotRequired[int]
    component: NotRequired[str]
    source_host: NotRequired[str]
    detail: NotRequired[str]
    log_level: NotRequired[str]
    error_code: NotRequired[str]


class JobCheckpoint(TypedDict):
    state: JobState
    completed_at: str
    artifact_path: NotRequired[str]


class JobRecord(TypedDict):
    schema_version: str
    job_id: str
    wiki_id: str
    state: JobState
    stage_progress: float
    created_at: str
    updated_at: str
    checkpoints: list[JobCheckpoint]


class SourceManifestEntry(TypedDict):
    schema_version: str
    source_id: str
    original_name: str
    source_format: SourceFormat
    content_sha256: str
    byte_size: int
    ingest_strategy: IngestStrategy


class ExtractedPage(TypedDict):
    page_number: int
    markdown: str
    layout_confidence: NotRequired[float]


class ExtractionArtifact(TypedDict):
    schema_version: str
    source_id: str
    extractor: str
    pages: list[ExtractedPage]
    plain_text: str


class WikiWrite(TypedDict):
    relative_path: str
    content_sha256: str
    markdown: str


class WikiTransaction(TypedDict):
    schema_version: str
    transaction_id: str
    wiki_id: str
    writes: list[WikiWrite]
    deletes: list[str]


class ReviewItem(TypedDict):
    schema_version: str
    review_id: str
    job_id: str
    severity: ReviewSeverity
    code: str
    message: str
    status: str
    relative_path: NotRequired[str]


class PublicationResult(TypedDict):
    schema_version: str
    transaction_id: str
    published_at: str
    written_paths: list[str]
    backup_created: bool


MESSAGE_TYPES = tuple(item.value for item in MessageType)
JOB_STATES = tuple(item.value for item in JobState)
SOURCE_FORMATS = tuple(item.value for item in SourceFormat)
ERROR_CATEGORIES = tuple(item.value for item in ErrorCategory)
REVIEW_SEVERITIES = tuple(item.value for item in ReviewSeverity)
PROVIDER_IDS = tuple(item.value for item in ProviderId)
INGEST_STRATEGIES = tuple(item.value for item in IngestStrategy)
