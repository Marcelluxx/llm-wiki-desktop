# LLM Wiki Desktop

LLM Wiki Desktop is a local-first Windows application that converts PDF, DOCX, TXT,
and Markdown sources into multiple isolated, Obsidian-compatible AI knowledge
bases.

The MVP is under active implementation. The approved product design and executable
implementation plan are available in `docs/superpowers/`.

The current desktop build can create multiple isolated wikis, select PDF, DOCX,
TXT, and Markdown files through the native Windows picker, and process them in a
cancellable background job. Originals are copied into immutable content-addressed
storage, DOCX/TXT/MD files are converted locally, and every PDF is sent through the
OpenDataLoader full hybrid force-OCR route. Obsidian-ready source notes, extraction
artifacts, durable progress, and per-job logs are stored inside the selected wiki.

Processing logs are visible from the wiki screen and are also streamed to the
application console. The complete JSONL history is kept under
`.llm-wiki/logs/<job-id>.jsonl`; OpenDataLoader backend output is kept beside it for
PDF diagnostics.

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
- Java 11 or newer for local OpenDataLoader hybrid OCR
- `uv`
- Microsoft C++ Build Tools and WebView2 for Tauri development

End users will not need development tools, system Python, or system Java once the
release packaging milestone bundles those runtimes. The current developer build
uses the isolated `.venv` plus the locally available Java runtime.

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
