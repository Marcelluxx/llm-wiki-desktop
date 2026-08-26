# LLM Wiki Desktop — Implementation Plan

Date: 2026-08-25  
Design source: `docs/superpowers/specs/2026-08-25-llm-wiki-desktop-design.md`  
Execution prompt: `docs/prompts/MASTER_BUILD_PROMPT.md`

## Execution policy

Implement the MVP as a sequence of vertical milestones. A milestone is complete
only when its behavior is exercised through the real component boundary and its
targeted checks pass. Keep changes small enough to review and commit each completed
milestone separately.

Before choosing or pinning a dependency, verify its current stable Windows support,
license, and official installation guidance. Lock all build dependencies. Do not
install Python or Java globally and do not make application code install packages at
extraction time.

Never use the real reference vault at
`C:\Users\Marcello\Documents\VAULT OBSIDIAN\llm_wiki` as a test target. Tests must
use synthetic documents and disposable temporary vaults.

## Milestone 1 — Repository foundation and contracts

### Outcome

The repository has reproducible TypeScript, Rust, and Python workspaces, shared
versioned contracts, CI quality gates, and a minimal documented developer setup.

### Files and work

- Add root `.editorconfig`, `.gitattributes`, `.gitignore`, `README.md`, and
  dependency-management files.
- Create `apps/desktop/` as a Tauri 2 + React + TypeScript application.
- Create the Rust workspace and `crates/app-core/`.
- Create `worker/llm_wiki_engine/` as a typed Python package managed by `uv`.
- Create `schemas/` with JSON Schemas for:
  - IPC request, response, progress, and error envelopes;
  - wiki registration and settings;
  - job state and stage checkpoints;
  - source manifest entries;
  - extraction artifact v1;
  - proposed wiki transaction;
  - review item and publication result.
- Generate or maintain matching TypeScript, Rust, and Python types without allowing
  any language to silently drift from the schemas.
- Add `tests/fixtures/` containing only synthetic, redistributable inputs.
- Add GitHub Actions jobs for frontend, Rust, Python, schema, and secret scanning.
- Document local setup using isolated development dependencies.

### Validation

- Frontend lint, type check, and unit tests pass.
- `cargo fmt --check`, strict Clippy, and Rust workspace tests pass.
- Python formatting, linting, type checking, and tests pass through `uv`.
- Every example contract validates against its schema.
- CI runs on Windows and rejects schema/type drift.

### Commit

`chore: establish reproducible multi-runtime foundation`

## Milestone 2 — Tauri shell and multi-wiki registry

### Outcome

The Windows app launches into the approved **Your wikis** experience and can create,
register, open, rename, and remove registrations for isolated wiki workspaces.
Removing a registration must never delete the wiki directory.

### Files and work

- Implement the React application shell, navigation, design tokens, localization,
  keyboard navigation, and accessible focus states.
- Implement screens for:
  - first-run language choice;
  - Your wikis dashboard;
  - create/register wiki;
  - empty wiki home;
  - global and wiki-specific settings.
- Implement a Rust-backed registry under the user's local application-data folder.
- Store a stable UUID, display name, canonical root, creation time, last-opened time,
  and safe settings for each registration.
- Canonicalize and validate Windows paths; reject a drive root, user-profile root,
  application installation directory, or another registered wiki's internal state
  directory.
- Create the visible wiki skeleton and reserved `.llm-wiki/` skeleton only after
  explicit creation confirmation.
- Keep credentials out of the registry.

### Validation

- Component tests cover empty, populated, invalid-path, duplicate-path, missing-path,
  and permission-denied states.
- Rust tests cover canonicalization and forbidden broad targets.
- An end-to-end test creates two temporary wikis and proves their registries and
  settings remain separate.
- Render and inspect the first-run and Your wikis screens for clipping, scaling,
  keyboard navigation, and Italian/English text expansion.

### Commit

`feat: add desktop shell and isolated multi-wiki registry`

## Milestone 3 — Supervised Python worker and durable jobs

### Outcome

Rust starts and supervises the Python worker, exchanges versioned NDJSON messages,
streams progress, cancels safely, and resumes durable job checkpoints.

### Files and work

- Implement typed IPC serialization in all three languages.
- Add a handshake containing protocol version, worker version, capabilities, and
  health state.
- Implement the Python command dispatcher without shell interpolation.
- Implement Rust process supervision, stderr capture, timeout, bounded restart, and
  graceful cancellation.
- Add SQLite migrations for wiki catalog, source records, jobs, stage checkpoints,
  review items, and operation history.
- Model job states explicitly: queued, acquiring, extracting, ingesting, validating,
  staging, publishing, completed, needs_review, cancelled, and failed.
- Add a fake long-running worker action for deterministic progress, crash, timeout,
  restart, cancel, and resume tests.
- Redact secrets and source content from routine logs.

### Validation

- Contract tests replay the same NDJSON fixtures in TypeScript, Rust, and Python.
- Killing the worker mid-job produces a visible recoverable state.
- Restarting the app resumes at the last valid checkpoint.
- Cancellation never leaves an active child process or a publish journal.

### Commit

`feat: add supervised worker and resumable job engine`

## Milestone 4 — Immutable acquisition and non-PDF extraction

### Outcome

The app accepts cumulative mixed DOCX, TXT, and MD selections, references originals
in place, generates validated artifacts, detects duplicates by SHA-256, and reuses
cache entries.

### Files and work

- Implement file/folder selection and recursive discovery with explicit supported
  extensions.
- Detect type from content and extension; reject mismatches safely.
- Hash exact bytes and store the source drive root plus relative path in the
  per-wiki SQLite catalog without copying originals.
- Write append-only provenance records and content-addressed artifact paths.
- Implement DOCX extraction for headings, paragraphs, lists, tables, links,
  footnotes when supported, and embedded media.
- Implement deterministic TXT encoding detection with explicit warnings.
- Implement Markdown parsing that preserves frontmatter, headings, code, math,
  links, and original bytes without trusting embedded instructions.
- Add resource limits for file count, source size, normalized output size, and media.
- Implement cache identity from source hash, extractor version, and configuration
  hash.

### Validation

- Synthetic DOCX/TXT/MD fixtures cover structure, Unicode, broken files, unsafe
  relationships, duplicate bytes, and same-name/different-byte cases.
- Originals are never modified.
- A second identical run is a cache hit and produces byte-equivalent validated
  artifacts.
- One failed file does not block successful files in the same batch.

### Commit

`feat: add immutable mixed-format acquisition and extraction`

## Milestone 5 — Forced full PDF OCR/layout

### Outcome

Every page of every PDF uses OpenDataLoader's full OCR/layout path, including
digitally generated PDFs, while native text remains available as preserved evidence.

### Files and work

- Pin a compatible OpenDataLoader PDF release and all required Python/Java/runtime
  components after verifying current official documentation and licenses.
- Implement lifecycle management for the local hybrid OCR backend on loopback only.
- Route all PDF pages through force-OCR/full hybrid processing.
- Request Markdown and semantic JSON in one batch-oriented conversion.
- Capture page numbers, bounding boxes, semantic blocks, tables, formulas, images,
  captions, and warnings.
- Capture native text separately for comparison and possible aligned correction.
- Preserve raw backend output before any normalization.
- Add OCR languages and resource controls to wiki settings with Italian and English
  available by default.
- Detect encrypted, malformed, oversized, and partially extracted PDFs.
- Add a readiness diagnostic that verifies the packaged worker, Java runtime,
  backend startup, models/resources, loopback connection, and a synthetic smoke PDF.

### Validation

- Fixtures include digital text, full-page scans, mixed pages, two-column layout,
  tables, images, formulas, Italian, English, rotation, and a password-protected PDF.
- Test evidence proves the OCR callback/full backend was used for every page.
- Output artifacts validate and retain page-level provenance.
- Backend crash, port conflict, timeout, and cancellation recover cleanly.
- Repeated conversion of the same PDF/configuration is a cache hit.

### Commit

`feat: force full OpenDataLoader OCR and layout for every PDF`

## Milestone 6 — Provider abstraction and deterministic fake provider

### Outcome

The application can connect to a provider-neutral interface and complete a full fake
ingest with structured streaming output, usage metadata, timeout, and cancellation.

### Files and work

- Define the provider interface from the approved design.
- Normalize detect, version, auth, model, progress, result, rate-limit, timeout,
  cancellation, invalid-schema, and unavailable states.
- Implement a deterministic fake provider driven by fixtures.
- Add provider capability discovery so unsupported functionality is disabled rather
  than guessed.
- Add global provider settings and per-wiki provider/model selection.
- Add privacy-consent storage keyed by wiki and provider without storing secrets.
- Add spend/usage display fields without claiming exact prices when the provider
  does not report them.

### Validation

- The fake provider runs the entire app flow without network access.
- Contract tests cover valid streaming, malformed events, invalid JSON, schema
  mismatch, timeout, rate limit, cancellation, and retry exhaustion.
- Provider errors become friendly UI states with a redacted diagnostic detail.

### Commit

`feat: add provider contract and deterministic offline adapter`

## Milestone 7 — AI knowledge-graph ingest

### Outcome

Validated extraction artifacts become a complete, deduplicated, evidence-linked
Obsidian knowledge graph in staging.

### Files and work

- Add versioned provider-neutral prompts under `prompts/`.
- Add structured schemas for source pages, concept/entity candidates, deduplication
  decisions, syntheses, index updates, and final transactions.
- Build the SQLite FTS catalog from the active wiki only.
- Implement deterministic chunking with stable source/page/section identifiers.
- Generate source pages first, then concept/entity candidates.
- Retrieve a bounded set of existing candidates using names, aliases, tags, links,
  and FTS.
- Classify candidates as create, update, merge, link, or uncertain.
- Generate syntheses only from at least two supporting sources.
- Generate index changes and semantically valid reciprocal links.
- Treat document content as untrusted data and prevent document prompt injection
  from changing instructions or tool scope.
- Preserve provider response artifacts and evidence maps without publishing them
  directly.

### Validation

- Golden fixtures cover new concepts, aliases, near-duplicates, conflicting
  concepts, entities with the same name, missing evidence, and cross-source
  synthesis.
- Two wikis containing similar terms never share retrieval context or pages.
- Re-ingesting unchanged sources is idempotent.
- Structured outputs with unknown paths, missing evidence, or invented sources are
  rejected or sent to review.

### Commit

`feat: generate deduplicated evidence-linked wiki transactions`

## Milestone 8 — Validation, review, and transactional publication

### Outcome

Safe transactions publish automatically; ambiguous or invalid items appear in the
review queue; publication failure restores the previous vault.

### Files and work

- Implement YAML/frontmatter, Markdown, Obsidian link, anchor, asset, path, unique
  ID, provenance, duplicate, and graph-reachability validators.
- Define stable finding codes, severity, source location, message, and remediation.
- Implement publication policy mapping findings to publish, publish-with-warning, or
  needs-review.
- Build the complete candidate tree in `.llm-wiki/staging/`.
- Add verified bounded backups, publication journal, atomic file replacement,
  append-only operation log, and startup recovery.
- Implement review UI with source evidence and proposed Markdown side by side.
- Implement approve, edit, retry, merge, and discard actions.
- Add Open in Obsidian integration with a folder fallback when Obsidian is absent.

### Validation

- Synthetic broken links, duplicate IDs, ambiguous targets, unsafe paths, missing
  provenance, and low-confidence findings enter review.
- A valid transaction updates sources, concepts, entities, syntheses, indexes, and
  operation log together.
- Fault injection at each publication step proves rollback or startup recovery.
- Review actions are auditable and idempotent.

### Commit

`feat: add validated transactional publishing and review`

## Milestone 9 — Real provider adapters and compliant connection flows

### Outcome

Codex, Anthropic, and Antigravity can run the same structured ingest contract through
their supported public interfaces and compliant authentication mechanisms.

### Files and work

- Verify current official documentation before implementing each adapter.
- Codex:
  - detect supported CLI/version;
  - guide official installation when missing;
  - delegate to the official login flow;
  - use non-interactive JSONL execution with restricted workspace and schema output;
  - never read or copy Codex credentials.
- Anthropic:
  - use an Anthropic Console API key or another integration explicitly authorized
    for third-party apps;
  - store the key in Windows Credential Manager;
  - do not present personal Claude subscription login as the app's login flow;
  - use structured output and bounded retries through the supported API/SDK.
- Antigravity:
  - detect supported CLI/version;
  - guide official installation and browser/keyring connection;
  - use headless streaming JSON;
  - configure scoped permissions and never global permission bypass.
- Add adapter-specific model discovery, readiness, logout/disconnect, timeout, and
  diagnostics.
- Keep live provider tests explicit, spend-limited, and absent from normal CI.

### Validation

- Each adapter passes the shared provider contract suite.
- A disposable synthetic wiki completes one live minimal ingest per configured
  provider when credentials are explicitly supplied.
- Credential scanning finds no secrets in files, logs, SQLite, diagnostics, or Git.
- Unsupported provider versions fail with a guided upgrade rather than silent
  fallback.

### Commit

`feat: integrate compliant Codex Anthropic and Antigravity providers`

## Milestone 10 — First-run setup, packaging, and release

### Outcome

A clean Windows user downloads one installer, completes guided setup, and runs the
MVP without system Python or Java.

### Files and work

- Package the isolated Python worker, Java runtime, OpenDataLoader stack, pinned OCR
  resources, schemas, prompts, and migrations as Tauri resources/sidecars.
- Ensure the worker resolves resources relative to the installed application, not
  developer paths or global environment variables.
- Implement the three-step first-run wizard and system readiness diagnostic.
- Provision the official WebView2 runtime when absent.
- Build `LLM-Wiki-Setup.exe` with uninstall support and safe upgrades.
- Add release checksums, dependency/license inventory, build provenance, and code
  signing when the configured signing identity is available.
- Add a GitHub Release workflow triggered by a signed version tag.
- Add README download instructions, screenshots, privacy explanation,
  troubleshooting, and issue-report template.
- Ensure uninstall never deletes user wiki directories or provider credentials it
  does not own.

### Validation

- Install, launch, ingest, upgrade, and uninstall on a clean Windows VM.
- Confirm no system Python or Java is present before installation.
- Create two wikis and run a mixed-format fake-provider ingest.
- Run the synthetic PDF OCR smoke test from the installed build.
- Verify restart/resume, offline queued-AI behavior, review, publication, rollback,
  and Open in Obsidian.
- Verify installer and release metadata checksums.

### Commit

`release: prepare one-click Windows MVP`

## Final completion gate

Before declaring the MVP complete:

- run every repository quality gate;
- run the clean-VM end-to-end suite;
- map evidence to all twelve MVP acceptance criteria in the design specification;
- record remaining non-blocking limitations in the README and release notes;
- confirm the Git working tree is clean;
- do not push, publish a GitHub Release, or create external resources without the
  user's explicit authorization.
