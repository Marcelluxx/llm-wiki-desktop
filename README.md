# LLM Wiki Desktop

LLM Wiki Desktop is a local-first Windows application that converts PDF, DOCX, TXT,
and Markdown sources into multiple isolated, Obsidian-compatible AI knowledge
bases.

The MVP is under active implementation. The approved product design and executable
implementation plan are available in `docs/superpowers/`.

The current desktop build can create multiple isolated wikis, select PDF, DOCX,
TXT, and Markdown files through the native Windows picker, and process them in a
cancellable background job. The queue is cumulative: later selections are appended,
duplicates are ignored, and queued files can be removed individually. Originals stay
in their existing folders; each wiki records their SHA-256, drive root, and relative
path in its private catalog instead of creating another copy. DOCX/TXT/MD files are
converted locally. PDFs with selectable text use
OpenDataLoader's fast structural Markdown/JSON extraction; only image-based PDFs use
the full hybrid force-OCR route. Obsidian-ready source notes, extraction
artifacts, durable progress, and per-job logs are stored inside the selected wiki.

Processing logs are visible from the wiki screen and are also streamed to the
application console. The complete JSONL history is kept under
`.llm-wiki/logs/<job-id>.jsonl`; OpenDataLoader backend output is kept beside it for
PDF diagnostics.

All selected PDFs are submitted in one OpenDataLoader call so the Java process and
the warmed hybrid backend are reused across the complete batch. During a long OCR
batch the activity panel reports elapsed time, process-tree CPU/RAM use, model
downloads, and relevant backend messages. A low-activity watchdog warns after two
minutes and a page-count-aware safety timeout stops a genuinely stuck batch.

The app detects NVIDIA hardware without downloading CUDA. Settings > Performance
shows whether GPU acceleration is active and offers a one-click optional CUDA
download only on NVIDIA systems. Without NVIDIA, OCR stays in CPU mode and the
approximately 1.8 GB CUDA runtime is never requested.

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

On a Windows machine with a supported NVIDIA GPU, enable local OCR acceleration
after bootstrap:

```powershell
.\scripts\enable-nvidia-acceleration.ps1
```

## Quality gates

```powershell
.\scripts\quality.ps1
```

The real reference vault is not a development or test target. All automated tests
use synthetic fixtures and disposable temporary directories.
