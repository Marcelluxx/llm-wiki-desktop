export const CONTRACT_VERSION = "1.0" as const;

export const MESSAGE_TYPES = ["request", "response", "progress", "error"] as const;
export const JOB_STATES = [
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
] as const;
export const SOURCE_FORMATS = ["pdf", "docx", "txt", "md"] as const;
export const ERROR_CATEGORIES = [
  "validation",
  "invalid_request",
  "unavailable",
  "timeout",
  "cancelled",
  "internal",
] as const;
export const REVIEW_SEVERITIES = ["info", "warning", "error"] as const;
export const PROVIDER_IDS = ["codex", "claude", "antigravity", "fake"] as const;
export const INGEST_STRATEGIES = ["forced_layout_ocr", "direct_text", "structured_docx"] as const;

export type MessageType = (typeof MESSAGE_TYPES)[number];
export type JobState = (typeof JOB_STATES)[number];
export type SourceFormat = (typeof SOURCE_FORMATS)[number];
export type ErrorCategory = (typeof ERROR_CATEGORIES)[number];
export type ReviewSeverity = (typeof REVIEW_SEVERITIES)[number];
export type ProviderId = (typeof PROVIDER_IDS)[number];
export type IngestStrategy = (typeof INGEST_STRATEGIES)[number];

export interface IpcEnvelope<TPayload extends Record<string, unknown> = Record<string, unknown>> {
  protocol_version: typeof CONTRACT_VERSION;
  message_type: MessageType;
  request_id: string;
  wiki_id?: string;
  job_id?: string;
  payload: TPayload;
}

export interface WikiRegistration {
  schema_version: typeof CONTRACT_VERSION;
  wiki_id: string;
  display_name: string;
  canonical_root: string;
  note_language: string;
  created_at: string;
  last_opened_at: string;
}

export interface WikiSettings {
  schema_version: typeof CONTRACT_VERSION;
  wiki_id: string;
  output_root: string;
  note_language: string;
  provider_id: ProviderId;
  ocr_language: string;
  open_in_obsidian_after_publish?: boolean;
}

export interface JobCheckpoint {
  state: JobState;
  completed_at: string;
  artifact_path?: string;
}

export interface JobRecord {
  schema_version: typeof CONTRACT_VERSION;
  job_id: string;
  wiki_id: string;
  state: JobState;
  stage_progress: number;
  created_at: string;
  updated_at: string;
  checkpoints: JobCheckpoint[];
}

export interface SourceManifestEntry {
  schema_version: typeof CONTRACT_VERSION;
  source_id: string;
  original_name: string;
  source_format: SourceFormat;
  content_sha256: string;
  byte_size: number;
  ingest_strategy: IngestStrategy;
}

export interface ExtractedPage {
  page_number: number;
  markdown: string;
  layout_confidence?: number;
}

export interface ExtractionArtifact {
  schema_version: typeof CONTRACT_VERSION;
  source_id: string;
  extractor: string;
  pages: ExtractedPage[];
  plain_text: string;
}

export interface WikiWrite {
  relative_path: string;
  content_sha256: string;
  markdown: string;
}

export interface WikiTransaction {
  schema_version: typeof CONTRACT_VERSION;
  transaction_id: string;
  wiki_id: string;
  writes: WikiWrite[];
  deletes: string[];
}

export interface ReviewItem {
  schema_version: typeof CONTRACT_VERSION;
  review_id: string;
  job_id: string;
  severity: ReviewSeverity;
  code: string;
  message: string;
  status: "open" | "accepted" | "rejected" | "resolved";
  relative_path?: string;
}

export interface PublicationResult {
  schema_version: typeof CONTRACT_VERSION;
  transaction_id: string;
  published_at: string;
  written_paths: string[];
  backup_created: boolean;
}
