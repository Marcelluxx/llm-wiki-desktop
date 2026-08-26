# Provider Command Center implementation plan

Date: 2026-08-26  
Design: `docs/superpowers/specs/2026-08-26-provider-command-center-design.md`

## Working rules

- Preserve the existing uncommitted `0.3.1` version changes in workspace manifests
  and lockfiles. Do not stage or rewrite them as part of provider commits.
- Use official provider distribution and authentication flows only.
- Never log, persist, inspect, or test with real credentials.
- Keep the fake provider available only for deterministic tests and browser preview.
- Finish each task with focused tests, then run `scripts/quality.ps1` before release.

## Task 1 — Provider contracts and fixtures

Files:

- Modify `apps/desktop/src/contracts.ts`
- Modify `crates/app-core/src/contracts.rs`
- Modify `worker/llm_wiki_engine/contracts.py`
- Modify `schemas/v1/wiki-settings.schema.json`
- Add `schemas/v1/provider-operation.schema.json`
- Modify `tests/fixtures/contracts/enums.json`
- Add `tests/fixtures/contracts/provider-operation.json`
- Modify contract tests in TypeScript, Rust, and Python

Steps:

1. Add `openrouter` and `ollama` provider identifiers while retaining `fake` for
   tests.
2. Define provider transport, readiness status, capabilities, model summary,
   operation phase/state, progress metrics, log entry, preference, and normalized
   error structures.
3. Add `provider_id`, optional `model_id`, and inheritance semantics without
   breaking existing wiki settings.
4. Validate identical enum values and representative payloads in all languages.
5. Run contract-focused frontend, Rust, and Python tests.

Commit: `feat: define provider management contracts`

## Task 2 — Durable preferences and operation catalog

Files:

- Add `crates/app-core/src/providers.rs`
- Modify `crates/app-core/src/lib.rs`
- Modify `crates/app-core/src/registry.rs`
- Modify `crates/app-core/src/catalog.rs`
- Add `crates/app-core/tests/providers.rs`

Steps:

1. Add an application-data provider catalog containing installations, default
   selection, operations, and sanitized operation logs.
2. Add additive migration from current registry/wiki settings, mapping existing
   `fake` selections to an unset production default while preserving test fixtures.
3. Add per-wiki `inherit` or explicit provider/model selection.
4. Implement atomic JSON/SQLite writes and interrupted-operation recovery.
5. Test restart persistence, migration, unavailable overrides, isolation between
   wikis, and no secret-shaped fields in storage.

Commit: `feat: persist provider preferences and operations`

## Task 3 — Provider manager and safe process boundary

Files:

- Add `apps/desktop/src-tauri/src/providers/mod.rs`
- Add `apps/desktop/src-tauri/src/providers/process.rs`
- Add `apps/desktop/src-tauri/src/providers/redaction.rs`
- Modify `apps/desktop/src-tauri/src/main.rs`
- Add Rust unit tests beside the new modules

Steps:

1. Introduce the `ProviderAdapter` trait and `ProviderManager` with concurrent,
   bounded detection.
2. Add Tauri commands for statuses, operations, cancellation, login, logout,
   models, global default, wiki override, and logs.
3. Launch executables with argument arrays, controlled environment, hidden windows,
   timeouts, process-tree cancellation, and no shell interpolation.
4. Normalize child output and redact credentials before emitting or persisting it.
5. Add a deterministic fake adapter covering all operation phases and failures.
6. Test cancellation, child crash, inactivity, malformed output, redaction canaries,
   and rejection of concurrent conflicting operations.

Commit: `feat: add provider manager and operation streaming`

## Task 4 — Frontend service and application state

Files:

- Modify `apps/desktop/src/services/registry.ts`
- Modify `apps/desktop/src/App.tsx`
- Modify `apps/desktop/src/App.test.tsx`
- Modify `apps/desktop/src/i18n.ts`

Steps:

1. Add typed client methods and Tauri channels for provider operations.
2. Load provider statuses independently of wiki/performance loading so one failed
   detector does not block the dashboard.
3. Keep the active operation and latest provider snapshot in top-level state.
4. Add browser-preview fake behavior for success, progress, failure, and cancel.
5. Add Italian/English strings for every status, phase, action, error category, and
   progress field.
6. Test loading, stale events, cancellation, retry, and truthful badge state.

Commit: `feat: connect provider state to the desktop UI`

## Task 5 — Premium badge and Provider Command Center

Files:

- Add `apps/desktop/src/components/ProviderBadge.tsx`
- Add `apps/desktop/src/components/ProviderCommandCenter.tsx`
- Add `apps/desktop/src/components/ProviderOperationPanel.tsx`
- Add `apps/desktop/src/assets/providers/*.svg`
- Modify `apps/desktop/src/components/WikiDashboard.tsx`
- Modify `apps/desktop/src/components/WikiHome.tsx`
- Modify `apps/desktop/src/styles.css`
- Modify `apps/desktop/src/App.test.tsx`

Steps:

1. Package reviewed local logos for Codex, Claude, Antigravity, OpenRouter, and
   Ollama; do not load remote assets at runtime.
2. Implement the header badge with logo, name, and truthful readiness state.
3. Implement the accessible modal command center with provider cards, contextual
   actions, version/model details, focus trap, Escape close, and responsive layout.
4. Implement the detailed progress panel: bytes, percentage/indeterminate bar,
   speed, ETA, elapsed time, retry count, source host, target, disk check, checksum,
   live sanitized logs, copy log, retry, and cancel.
5. Add reduced-motion and high-contrast behavior.
6. Test all badges/actions, keyboard navigation, focus restoration, progress forms,
   and secret-free copied logs.

Commit: `feat: add premium provider command center`

## Task 6 — Verified downloader and private Node runtime

Files:

- Add `apps/desktop/src-tauri/src/providers/download.rs`
- Add `apps/desktop/src-tauri/src/providers/node_runtime.rs`
- Modify `apps/desktop/src-tauri/Cargo.toml`
- Add synthetic archive/checksum fixtures under `tests/fixtures/providers/`
- Add Rust tests beside downloader/runtime modules

Steps:

1. Add HTTPS streaming with response-size limits, redirect policy, resumable
   temporary files where supported, speed/ETA calculation, and bounded retry.
2. Validate source host, content length, disk capacity, SHA-256, archive entries,
   extracted size, and executable version before activation.
3. Download pinned Node LTS Windows x64 plus its official checksum manifest on
   demand; activate under application data without touching global `PATH`.
4. Install exact provider npm packages into isolated prefixes with integrity and
   post-install version checks.
5. Preserve the previous active runtime/package until the replacement validates.
6. Test offline, timeout, proxy/TLS fixture errors, bad checksum, corrupted/traversal
   archive, low disk, locked destination, cancel, resume, rollback, and cleanup.

Commit: `feat: add verified private provider runtime installer`

## Task 7 — Codex and Claude adapters

Files:

- Add `apps/desktop/src-tauri/src/providers/codex.rs`
- Add `apps/desktop/src-tauri/src/providers/claude.rs`
- Add fake CLI fixtures under `tests/fixtures/providers/bin/`
- Add Rust tests beside adapters

Steps:

1. Detect system and app-managed installations with deterministic precedence and
   supported-version checks.
2. Install `@openai/codex` and `@anthropic-ai/claude-code` with the private runtime.
3. Delegate Codex login/status/logout to documented commands without reading its
   credential file.
4. Delegate Claude's official authentication choice and probe readiness without
   reading provider credentials; detect Git for Windows as a separate dependency.
5. Normalize versions, models, login outcomes, browser-launch failure, expiry,
   workspace restrictions, timeouts, and non-zero exits.
6. Test exclusively with fake executables and secret canaries.

Commit: `feat: integrate Codex and Claude provider setup`

## Task 8 — Antigravity adapter

Files:

- Add `apps/desktop/src-tauri/src/providers/antigravity.rs`
- Extend provider fixtures and Rust tests

Steps:

1. Detect `agy` in the documented user location and app-recorded installation.
2. Retrieve and run the official Windows installer artifact only after confirmation,
   avoiding alias and global `PATH` modifications where supported.
3. Delegate browser/keyring authentication and expose only non-secret status.
4. Normalize installation, browser, enterprise-login, keyring, version, and process
   errors.
5. Test all behavior with fake installer/CLI processes.

Commit: `feat: integrate Antigravity provider setup`

## Task 9 — Windows credentials and OpenRouter

Files:

- Add `apps/desktop/src-tauri/src/providers/credentials.rs`
- Add `apps/desktop/src-tauri/src/providers/openrouter.rs`
- Extend command center forms and tests

Steps:

1. Implement a Windows Credential Manager abstraction plus an in-memory test
   implementation.
2. Accept the key through a masked, non-persistent frontend field and pass it once to
   the backend.
3. Store, validate, replace, and remove the generic credential without echoing it.
4. Fetch/cache the official model catalog and distinguish invalid, revoked,
   restricted, exhausted, rate-limited, and unavailable states.
5. Add strict HTTP timeouts, response limits, and redacted diagnostics.
6. Test against a local fixture server, including a canary scan of all outputs.

Commit: `feat: integrate OpenRouter with protected credentials`

## Task 10 — Ollama and local models

Files:

- Add `apps/desktop/src-tauri/src/providers/ollama.rs`
- Extend command center model selection and tests

Steps:

1. Probe loopback version and model endpoints without requiring authentication.
2. Add explicit-confirmation launch of the official Windows installer when missing.
3. List local models and stream model-pull layer, digest, byte, and total progress.
4. Show model destination/space information when available and use indeterminate
   progress when totals are unknown.
5. Treat non-loopback custom endpoints as online for privacy consent.
6. Test service stopped, malformed API, model absent, cancel, pull failure, low disk,
   and local/remote endpoint classification.

Commit: `feat: integrate Ollama local models`

## Task 11 — Wiki overrides, recovery, and final validation

Files:

- Modify `apps/desktop/src/components/SettingsPanel.tsx`
- Modify provider and wiki settings services/components
- Modify `README.md`
- Modify `docs/implementation-status.md`
- Modify `.github/workflows/quality.yml` if new test commands are required

Steps:

1. Add global inheritance and explicit provider/model override controls to wiki
   settings.
2. Reconcile interrupted operations at startup and expose safe retry/remove-temp
   actions.
3. Add sanitized diagnostic export/copy behavior and final localized errors.
4. Verify that unavailable overrides never silently fall back to another online
   provider.
5. Run `scripts/quality.ps1`, frontend accessibility tests, Rust fixture integration
   tests, Tauri debug build, and a clean-user-profile smoke test with fake providers.
6. Scan UI events, console, JSONL, SQLite, copied logs, and diagnostics for secret
   canaries.
7. Update documentation with implemented capabilities and remaining limitations.

Commit: `feat: complete provider setup and model preferences`

## Release evidence

- All existing document-ingestion tests remain green.
- Provider contract, operation, UI, installer, authentication, and local-model tests
  pass without live credentials or paid calls.
- The base app remains usable for local OCR when no provider is configured.
- A failed/cancelled provider operation leaves no active partial installation.
- Git status contains only the user's pre-existing version changes and intentionally
  ignored local brainstorming artifacts after the provider commits.
