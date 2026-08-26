import { describe, expect, it } from "vitest";
import enumFixture from "../../../tests/fixtures/contracts/contract-enums.json";
import extractionFixture from "../../../tests/fixtures/contracts/extraction-artifact.json";
import jobFixture from "../../../tests/fixtures/contracts/job.json";
import publicationFixture from "../../../tests/fixtures/contracts/publication-result.json";
import reviewFixture from "../../../tests/fixtures/contracts/review-item.json";
import sourceFixture from "../../../tests/fixtures/contracts/source-manifest.json";
import transactionFixture from "../../../tests/fixtures/contracts/wiki-transaction.json";
import wikiFixture from "../../../tests/fixtures/contracts/wiki-registration.json";
import settingsFixture from "../../../tests/fixtures/contracts/wiki-settings.json";
import {
  ERROR_CATEGORIES,
  type ExtractionArtifact,
  INGEST_STRATEGIES,
  type JobRecord,
  JOB_STATES,
  MESSAGE_TYPES,
  type PublicationResult,
  PROVIDER_IDS,
  PROVIDER_OPERATION_STATES,
  PROVIDER_STATUSES,
  PROVIDER_TRANSPORTS,
  type ReviewItem,
  REVIEW_SEVERITIES,
  type SourceManifestEntry,
  SOURCE_FORMATS,
  type WikiRegistration,
  type WikiSettings,
  type WikiTransaction,
} from "./contracts";

describe("shared contracts", () => {
  it("keeps enum values aligned with the neutral fixture", () => {
    expect(MESSAGE_TYPES).toEqual(enumFixture.message_types);
    expect(JOB_STATES).toEqual(enumFixture.job_states);
    expect(SOURCE_FORMATS).toEqual(enumFixture.source_formats);
    expect(ERROR_CATEGORIES).toEqual(enumFixture.error_categories);
    expect(REVIEW_SEVERITIES).toEqual(enumFixture.review_severities);
    expect(PROVIDER_IDS).toEqual(enumFixture.provider_ids);
    expect(PROVIDER_TRANSPORTS).toEqual(enumFixture.provider_transports);
    expect(PROVIDER_STATUSES).toEqual(enumFixture.provider_statuses);
    expect(PROVIDER_OPERATION_STATES).toEqual(enumFixture.provider_operation_states);
    expect(INGEST_STRATEGIES).toEqual(enumFixture.ingest_strategies);
  });

  it("accepts every versioned domain fixture at compile time", () => {
    const values = [
      wikiFixture as WikiRegistration,
      settingsFixture as WikiSettings,
      jobFixture as JobRecord,
      sourceFixture as SourceManifestEntry,
      extractionFixture as ExtractionArtifact,
      transactionFixture as WikiTransaction,
      reviewFixture as ReviewItem,
      publicationFixture as PublicationResult,
    ];

    expect(values.every((value) => value.schema_version === "1.0")).toBe(true);
  });
});
