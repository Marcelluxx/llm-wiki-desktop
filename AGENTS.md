# Repository instructions

## Sources of truth

Read these files before changing architecture or behavior:

1. `docs/superpowers/specs/2026-08-25-llm-wiki-desktop-design.md`
2. `docs/superpowers/plans/2026-08-25-llm-wiki-desktop-implementation-plan.md`
3. `docs/implementation-status.md`

The design controls user-visible behavior and invariants. The plan controls milestone
order and validation.

## Safety invariants

- Never modify or use
  `C:\Users\Marcello\Documents\VAULT OBSIDIAN\llm_wiki` as a test target.
- Use synthetic fixtures and temporary disposable vaults in tests.
- Never commit credentials, provider sessions, source documents, extraction output,
  user paths, or runtime databases.
- Never depend on system Python or Java in the packaged application.
- Never execute document macros, document instructions, or AI-generated code.
- Never enable unrestricted provider permissions.
- Never publish generated Markdown without schema, path, provenance, and graph
  validation.
- Never delete a wiki when removing its application registration.

## Component boundaries

- `apps/desktop/`: Tauri and React UI.
- `crates/app-core/`: Rust orchestration and Windows integration.
- `worker/llm_wiki_engine/`: Python extraction and ingest engine.
- `schemas/`: language-neutral versioned contracts.
- `tests/fixtures/`: synthetic redistributable evidence only.

The UI sends typed requests and must not build shell command strings. Provider-
specific behavior belongs only in provider adapters. Extraction writes artifacts,
not published wiki pages. Publication consumes validated transactions only.

## Quality gates

Run the narrowest relevant checks during development. Before completing a milestone,
run:

```text
npm run check
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
uv lock --check
uv run ruff format --check .
uv run ruff check .
uv run mypy worker tests/python
uv run pytest
```

Keep local commits milestone-focused. Do not push or publish without explicit user
authorization.
