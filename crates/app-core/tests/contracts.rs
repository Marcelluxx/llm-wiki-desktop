use std::{fs, path::PathBuf};

use llm_wiki_app_core::contracts::{
    ERROR_CATEGORIES, INGEST_STRATEGIES, JOB_STATES, MESSAGE_TYPES, PROVIDER_IDS,
    PROVIDER_OPERATION_STATES, PROVIDER_STATUSES, PROVIDER_TRANSPORTS, REVIEW_SEVERITIES,
    SOURCE_FORMATS,
};
use llm_wiki_app_core::{
    ExtractionArtifact, IpcEnvelope, JobRecord, PublicationResult, ReviewItem, SourceManifestEntry,
    WikiRegistration, WikiSettings, WikiTransaction,
};
use serde_json::Value;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/contracts")
        .join(name)
}

fn read_fixture(name: &str) -> String {
    fs::read_to_string(fixture_path(name)).expect("contract fixture must be readable")
}

#[test]
fn parses_shared_contract_fixtures() {
    let envelope: IpcEnvelope =
        serde_json::from_str(&read_fixture("ipc-request.json")).expect("valid IPC fixture");
    let wiki: WikiRegistration =
        serde_json::from_str(&read_fixture("wiki-registration.json")).expect("valid wiki fixture");
    let job: JobRecord =
        serde_json::from_str(&read_fixture("job.json")).expect("valid job fixture");
    let settings: WikiSettings =
        serde_json::from_str(&read_fixture("wiki-settings.json")).expect("valid settings fixture");
    let source: SourceManifestEntry =
        serde_json::from_str(&read_fixture("source-manifest.json")).expect("valid source fixture");
    let extraction: ExtractionArtifact =
        serde_json::from_str(&read_fixture("extraction-artifact.json"))
            .expect("valid extraction fixture");
    let transaction: WikiTransaction = serde_json::from_str(&read_fixture("wiki-transaction.json"))
        .expect("valid transaction fixture");
    let review: ReviewItem =
        serde_json::from_str(&read_fixture("review-item.json")).expect("valid review fixture");
    let publication: PublicationResult =
        serde_json::from_str(&read_fixture("publication-result.json"))
            .expect("valid publication fixture");

    assert_eq!(envelope.protocol_version, "1.0");
    assert_eq!(wiki.note_language, "it");
    assert_eq!(job.stage_progress, 0.0);
    assert_eq!(settings.ocr_language, "ita+eng");
    assert_eq!(source.byte_size, 2048);
    assert_eq!(extraction.pages.len(), 1);
    assert_eq!(transaction.writes.len(), 1);
    assert_eq!(review.code, "possible_duplicate");
    assert!(publication.backup_created);
}

#[test]
fn enum_values_match_the_language_neutral_fixture() {
    let values: Value =
        serde_json::from_str(&read_fixture("contract-enums.json")).expect("valid enum fixture");

    assert_eq!(values["message_types"], serde_json::json!(MESSAGE_TYPES));
    assert_eq!(values["job_states"], serde_json::json!(JOB_STATES));
    assert_eq!(values["source_formats"], serde_json::json!(SOURCE_FORMATS));
    assert_eq!(
        values["error_categories"],
        serde_json::json!(ERROR_CATEGORIES)
    );
    assert_eq!(
        values["review_severities"],
        serde_json::json!(REVIEW_SEVERITIES)
    );
    assert_eq!(values["provider_ids"], serde_json::json!(PROVIDER_IDS));
    assert_eq!(
        values["provider_transports"],
        serde_json::json!(PROVIDER_TRANSPORTS)
    );
    assert_eq!(
        values["provider_statuses"],
        serde_json::json!(PROVIDER_STATUSES)
    );
    assert_eq!(
        values["provider_operation_states"],
        serde_json::json!(PROVIDER_OPERATION_STATES)
    );
    assert_eq!(
        values["ingest_strategies"],
        serde_json::json!(INGEST_STRATEGIES)
    );
}
