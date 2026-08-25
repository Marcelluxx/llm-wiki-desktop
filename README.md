# LLM Wiki Desktop

LLM Wiki Desktop is a local-first Windows application that converts PDF, DOCX, TXT,
and Markdown sources into multiple isolated, Obsidian-compatible AI knowledge
bases.

The MVP is under active implementation. The approved product design and executable
implementation plan are available in `docs/superpowers/`.

## Architecture

- Tauri 2 + React/TypeScript desktop interface
- Rust orchestration core
- Bundled Python extraction and ingest worker
- Versioned JSON Schema contracts
- Provider adapters for Codex, Anthropic, and Antigravity

## Development prerequisites

- Windows 10/11 x64
- Node.js 24 or another version supported by the locked frontend toolchain
- Rust 1.98.0 MSVC toolchain
- Python 3.12 through 3.14
- `uv`
- Microsoft C++ Build Tools and WebView2 for Tauri development

End users will not need development tools, system Python, or system Java. Those
runtime dependencies will be packaged by the release milestone.

## Isolated developer setup

From PowerShell, run:

```powershell
.\scripts\bootstrap-dev.ps1
.\scripts\quality.ps1
```

The bootstrap keeps Rust and its cache inside the ignored `.tools/` directory. It
does not edit the global `PATH` or install Python/Java system-wide. npm and `uv` use
the committed lockfiles.

## Quality gates

```powershell
.\scripts\quality.ps1
```

The real reference vault is not a development or test target. All automated tests
use synthetic fixtures and disposable temporary directories.
