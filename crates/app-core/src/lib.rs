pub mod catalog;
pub mod contracts;
pub mod registry;

pub use catalog::{CatalogError, ChatMessageRecord, JobSummary, WikiCatalog};
pub use contracts::{
    CONTRACT_VERSION, ErrorCategory, ExtractionArtifact, IpcEnvelope, JobRecord, JobState,
    MessageType, ProviderId, ProviderModel, ProviderOperationEvent, ProviderOperationState,
    ProviderStatus, ProviderSummary, ProviderTransport, PublicationResult, ReviewItem,
    ReviewSeverity, SourceFormat, SourceManifestEntry, WikiRegistration, WikiSettings,
    WikiTransaction,
};
pub use registry::{
    RegistryError, RegistrySnapshot, RegistryStore, ensure_obsidian_visibility,
    ensure_wiki_agents_file, remove_legacy_source_properties,
};
