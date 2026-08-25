# Implementation status

Updated: 2026-08-25

## Current milestone

Milestone 3 — supervised Python worker and durable jobs: in progress.

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

## Validation evidence

- `scripts/quality.ps1` passes on Windows with Rust 1.98.0 and Python 3.12.13.
- Frontend: format, lint, strict type check, 11 tests, and production build pass.
- Rust: format, strict Clippy, 8 contract/registry/catalog tests, and workspace tests pass.
- Python/schema: Ruff, strict mypy, 7 tests, JSON Schema validation, and unsafe-path
  rejection pass.
- `tauri build --debug --no-bundle` produces
  `target/debug/llm-wiki-desktop.exe`; a hidden launch smoke test confirmed startup.
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

Complete Milestone 3 worker crash/restart and resume semantics, then connect the
selected files to immutable acquisition and forced OpenDataLoader PDF OCR.
