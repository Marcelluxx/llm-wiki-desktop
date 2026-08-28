# Implementation status

Updated: 2026-08-28

## Current milestone

Milestone 6 — provider-neutral chat and agentic ingest frontend: in progress.

Release candidate version: 0.8.2.

## Completed outcomes

- Product design approved and committed in `5dd6d5a`.
- Implementation plan and master build prompt committed in `d83339d`.
- Design metadata normalization committed in `d8387e9`.
- Reproducible npm, Cargo, and `uv` workspaces with committed lockfiles.
- Minimal Tauri 2 Windows shell, React frontend, Rust orchestration crate, and typed
  Python worker.
- Versioned JSON Schemas and matching TypeScript, Rust, and Python contract types.
- Synthetic contract fixtures covering IPC, multi-wiki configuration, jobs,
  extraction, review, transactions, and publication.
- Isolated PowerShell bootstrap, unified quality command, and Windows GitHub Actions
  quality gates including secret scanning.
- Bilingual first-run experience, responsive **Your wikis** dashboard, create/connect
  flow, empty wiki home, and global/wiki settings panel.
- Rust-backed local registry with stable wiki identifiers, canonical Windows paths,
  native folder picker, and isolated visible/internal wiki skeletons.
- Safe registration removal that never deletes the wiki folder or its notes.
- Keyboard focus containment, Escape handling, visible focus states, and responsive
  layouts for Italian and English text.
- Native multi-file selection for PDF, DOCX, TXT, and Markdown with validation and
  a 500-document batch limit.
- Per-wiki SQLite job catalog, stage checkpoints, recoverable-state migration,
  background progress events, and user cancellation.
- Versioned Python NDJSON dispatcher with capability handshake, concurrent fake-job
  progress, cancellation, shutdown, and deterministic crash support.
- Enabled document intake interface with selected-file summary, progress bar, and
  localized status/error states.
- Actual source paths now cross the Rust/Python boundary; the worker hashes each
  original and stores its drive root plus relative path in the per-wiki SQLite
  catalog without copying the source file.
- Local DOCX extraction preserves headings, paragraphs, lists, and tables; TXT and
  Markdown bypass OCR while retaining their source structure.
- OpenDataLoader PDF 2.5.5 is pinned. PDFs exposing selectable text use its fast
  structural Markdown/JSON parser; only PDFs without embedded text start the
  loopback-only full hybrid force-OCR backend.
- Each processed source produces a validated Markdown artifact and an
  Obsidian-ready note under `sources/`.
- Structured JSONL job logs are persisted inside each wiki, exposed in the wiki
  interface, and mirrored to the browser, Rust, and worker consoles. OCR backend
  output is retained as a separate diagnostic log.
- PDF batches use one OpenDataLoader invocation and one warmed hybrid backend, as
  recommended upstream, avoiding repeated JVM startup. The UI retains elapsed time,
  process-tree CPU/RAM metrics, live model/backend events, inactivity warnings,
  cancellation, and a page-count-aware watchdog.
- Startup detects NVIDIA hardware without downloading CUDA. Settings reports CPU/GPU
  status and exposes the optional CUDA installer only when NVIDIA hardware exists.
- Import cancellation is reflected immediately, stops the visible timer, re-enables
  document selection, and still signals the worker to terminate active child tools.
- The selected-file summary is a horizontally scrollable card grid with a clear
  file-type icon for every queued document.
- Document selection is cumulative, ignores repeated paths, supports per-file
  removal before processing, and keeps completed documents visible while new ones
  are appended for the next import.
- Extraction artifacts now use a SHA-256/configuration cache identity. Re-importing
  unchanged bytes with the same extractor configuration restores the source note
  without rerunning DOCX parsing, PDF extraction, or OCR.
- Every wiki has a durable SQLite chat transcript and automatically inherits the
  selected global provider when it has no explicit provider override.
- The wiki workspace includes a premium chat surface with provider readiness,
  streamed CLI/API details, persistent messages, and a guided provider-selection
  empty state.
- The wiki workspace now prioritizes the assistant with a wider two-column layout,
  a taller conversation viewport, more readable Markdown messages, and a dedicated
  focus-mode control that expands the chat over the workspace and collapses with
  either its control or Escape.
- A provider-neutral Tauri chat command supports Codex, Claude Code, Antigravity,
  OpenRouter, Ollama, and the deterministic fake provider. CLI processes run hidden
  and stream structured output into the app.
- The Antigravity adapter now follows the official streaming-input contract exactly:
  it sends `event: user` NDJSON over standard input without combining that mode with
  the incompatible `-p` flag, parses both incremental and terminal response shapes,
  and preserves the effective `plan`/`accept-edits` mode.
- Antigravity ingest uses a two-turn streaming session: the first turn prepares and
  starts the operation, while the second carries the explicit approval already given
  through the Ingest button and continues interrupted/planning-only runs. Both
  terminal results are required, sandboxing remains enabled, and unrestricted
  permission bypass is never used.
- Provider execution now reports process start, prompt delivery, stream activity,
  terminal status, structured provider failures, timeouts, malformed/empty responses,
  model/authentication/version errors, and local I/O failures. Error details open
  automatically in the chat and a path/secret-redacted JSONL diagnostic is appended
  to `.llm-wiki/logs/provider-events.jsonl`.
- Antigravity stream initialization is inspected at run time. If the user's global
  CLI profile exposes `always-proceed`, the chat opens a visible security warning;
  the application still enforces sandboxing and `plan` mode for read-only chat.
- Assistant replies are rendered as safe GitHub-flavored Markdown, including
  headings, emphasis, lists, tables, blockquotes, and code. Raw HTML, remote images,
  unsafe protocols, and direct local-file navigation remain disabled.
- Chat context recursively includes generated Markdown notes and validated artifact
  `document.md` files, so both pre-ingest extractions and the published wiki can be
  used as evidence.
- Completed extraction jobs enable an Ingest action. The CLI agent receives the
  operation request and is restricted to the active wiki workspace. Each wiki owns
  an editable root-level `AGENTS.md`, created from the architecture blueprint when
  missing and never overwritten when customized. This file is the sole ingest rule
  set sent to the provider; the app no longer duplicates those rules in its prompt.
  Before execution, the app validates every SHA-256 artifact identity and its
  manifest; Antigravity receives the exact wiki through `--add-dir`, reports its
  effective working directory, and completion is rejected unless the active wiki's
  operation log was actually updated.
- Non-SHA extraction workspaces are ignored by the artifact inventory and future PDF
  parser output is staged under `.llm-wiki/staging/jobs/`, preventing job UUIDs from
  being misreported as corrupt content-addressed artifacts.
- Published Markdown no longer exposes internal `source_id` or `source_ids`
  properties. Opening a wiki removes those legacy frontmatter properties from
  existing managed notes, and the ingest blueprint enforces the same rule while
  preserving readable provenance.
- Opening or creating a wiki merges an Obsidian exclusion for `.llm-wiki/` into the
  vault configuration and graph filter. Internal extraction evidence, source files,
  cache data, and images remain available to the application without becoming
  visible graph nodes.
- Application, frontend, Rust crates, Python package, and worker protocol now share
  release version 0.8.2, enforced by an automated synchronization check.
- A Windows release workflow builds a private locked Python runtime and private Java
  runtime, embeds them in one NSIS installer, generates a SHA-256 manifest, uploads
  both artifacts, and creates a draft GitHub Release for clean-machine validation.
- The release runtime keeps third-party license files in a compact inventory so deep
  wheel paths cannot break NSIS packaging while license information remains bundled.
- The public README now separates end-user installation from source-code setup and
  documents GitHub Assets, checksum verification, SmartScreen, first run, providers,
  OCR/GPU behavior, privacy, troubleshooting, updates, uninstall, and release steps.

## Validation evidence

- `scripts/quality.ps1` passes on Windows with Rust 1.98.0 and Python 3.12.13.
- Frontend: format, lint, strict type check, 19 tests, and production build pass.
- Rust: format, strict Clippy, 13 core/contract/registry tests plus 12 desktop adapter
  tests pass. The adapter suite includes an end-to-end hidden-process NDJSON pipe.
- Python/schema: Ruff, strict mypy, 16 tests, JSON Schema validation, and unsafe-path
  rejection pass.
- `tauri build --debug --no-bundle` produces
  `target/debug/llm-wiki-desktop.exe`; a hidden launch smoke test confirmed startup.
- A local forced-OCR benchmark on an RTX 2070 processed the four 37-page test PDFs
  in one 78.9-second batch. The first 15-page document absorbed model warm-up;
  subsequent 9-, 7-, and 6-page documents reused the loaded pipeline.
- A real 22-page digital PDF completed structural Markdown and JSON extraction in
  2.28 seconds without starting OCR.
- Browser inspection confirmed unclipped first-run, empty-dashboard, create-dialog,
  settings, Italian/English expansion, and initial keyboard focus behavior.

## Active assumptions

- The development package manager is npm with a committed lockfile.
- Rust is pinned to 1.98.0 with the MSVC target.
- Python supports 3.12 through 3.14 and is locked with `uv`.
- The reference Obsidian vault remains read-only reference material and is excluded
  from all tests.

## Blockers

- None.

## Next action

Add deterministic provider contract fixtures, cancellation and session-resume for
chat/ingest, then move generated knowledge writes behind validated staging and
transactional publication.
