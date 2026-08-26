# Implementation status

Updated: 2026-08-25

## Current milestone

Milestones 4–5 — immutable extraction and forced PDF OCR: in progress.

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
- Actual source paths now cross the Rust/Python boundary; the worker hashes and
  atomically copies originals into per-wiki content-addressed storage.
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

## Validation evidence

- `scripts/quality.ps1` passes on Windows with Rust 1.98.0 and Python 3.12.13.
- Frontend: format, lint, strict type check, 13 tests, and production build pass.
- Rust: format, strict Clippy, 8 contract/registry/catalog tests, and workspace tests pass.
- Python/schema: Ruff, strict mypy, 14 tests, JSON Schema validation, and unsafe-path
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

Add extraction caching/resume semantics and synthetic PDF OCR fixtures, then build
the provider-neutral AI ingest that creates concepts, entities, syntheses, and
indexes from the extracted source notes.
