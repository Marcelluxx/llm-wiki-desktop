# LLM Wiki Desktop — Product and Architecture Design

Date: 2026-08-25

Status: Approved by user

Target: Windows 10/11 x64

Working directory: `E:\PROGETTI\llm-wiki-desktop`

## 1. Purpose

LLM Wiki Desktop is a one-click Windows application that turns selected PDF, DOCX,
TXT, and Markdown files into one or more complete, Obsidian-compatible knowledge
bases. It combines local document extraction with an AI provider selected by the
user. The application must hide terminals, Python environments, Java setup, and
provider-specific command syntax from the end user.

The product is local-first:

- source acquisition, hashing, OCR, extraction, caching, validation, and publishing
  run locally;
- content is sent online only to the AI provider selected by the user and only after
  an explicit privacy disclosure;
- the resulting knowledge base is ordinary Markdown and remains usable without the
  application.

The product is not an Obsidian replacement, a cloud synchronization service, a PDF
editor, or a general-purpose autonomous coding agent.

## 2. Success criteria

An ordinary Windows user can:

1. download one `Setup.exe` asset from GitHub Releases;
2. install the application without installing Python or Java;
3. choose the interface and output language;
4. connect a supported AI provider through a simple, provider-compliant flow;
5. create or register multiple independent wiki workspaces;
6. select any mixture of PDF, DOCX, TXT, and MD files or folders;
7. press one primary action, **Create the wiki**;
8. follow progress without seeing a terminal;
9. open validated results directly in Obsidian;
10. review only uncertain or invalid results;
11. resume interrupted jobs without repeating completed OCR or AI work.

## 3. Multi-wiki model

The application starts on a **Your wikis** screen. A wiki is an isolated workspace
with its own:

- stable identifier and display name;
- output folder or Obsidian vault;
- source archive and provenance manifest;
- extraction artifacts and content-addressed cache;
- SQLite catalog and job history;
- source, concept, entity, synthesis, and index pages;
- language and AI instructions;
- preferred provider and model;
- review queue and publication log.

A wiki may point either to an independent Obsidian vault or to a dedicated folder
inside an existing vault. Cross-wiki retrieval, deduplication, and linking are
disabled. During ingest, an AI provider receives context only from the active wiki.
Moving or sharing pages between wikis is outside the MVP because it requires an
explicit provenance and link-migration workflow.

Global settings contain interface language, installed providers, authentication
status, update channel, diagnostics, and default resource limits. Wiki-specific
settings contain destination, note language, provider/model choice, ingest
instructions, and review policy.

## 4. User experience

### 4.1 First-run wizard

The first-run wizard contains three short steps:

1. **Language** — interface language and default note language.
2. **AI provider** — detect or install a provider, open its official connection
   flow, and verify readiness.
3. **Destination** — create the first wiki or register an existing vault/folder.

The final page runs an internal readiness check for the bundled OCR worker, writable
paths, available disk space, provider authentication, and a minimal synthetic
conversion. It reports a single user-facing state: Ready, Needs attention, or
Unavailable. Technical detail is available behind an expandable diagnostics link.

### 4.2 Daily workflow

Inside a wiki, the main screen contains:

- a large drop area for files and folders;
- a file list with detected type, duplicate status, and obvious errors;
- the current provider and destination;
- one primary button, **Create the wiki**.

Advanced OCR, model, concurrency, cache, and publishing controls are hidden in an
advanced settings panel. The safe defaults must be sufficient for normal use.

During processing, the application shows stage-level and file-level progress. The
job continues in the background and can be cancelled safely. Completion reports the
number of sources, concepts, entities, syntheses, indexes, published pages, and
items requiring review. The primary completion action is **Open in Obsidian**.

### 4.3 Review queue

The review screen displays only blocked items and warnings that require judgment,
such as incomplete OCR, ambiguous concept matches, conflicting metadata, missing
provenance, unsupported links, or low-confidence structure. It shows the source
evidence next to the proposed Markdown and offers approve, edit, retry, merge, or
discard actions.

## 5. Technical architecture

The system has four boundaries:

1. **Tauri 2 + React/TypeScript desktop UI** — windows, navigation, selection,
   progress, review, settings, and accessible localization.
2. **Rust orchestration core** — process lifecycle, IPC, job control, cancellation,
   resource limits, secure command construction, atomic publication, updates, and
   OS integration.
3. **Bundled Python worker** — format routing, OpenDataLoader execution, DOCX/TXT/MD
   extraction, artifact production, catalog access, AI prompts, structured response
   validation, graph linting, and candidate generation.
4. **Provider adapters** — Codex, Anthropic, and Antigravity integrations behind a
   common contract.

The UI never builds shell command strings. It sends typed requests to Rust. Rust
starts the Python worker with an argument array and communicates through newline-
delimited JSON over standard input/output. Diagnostics use standard error. Every
message includes a protocol version, request identifier, wiki identifier, and job
identifier.

The Python worker is a long-lived supervised process. A crash cannot crash the UI;
Rust records the failure, preserves the checkpoint, and restarts the worker within
a bounded retry policy.

## 6. Repository layout

```text
llm-wiki-desktop/
├── apps/desktop/              Tauri and React application
├── crates/app-core/           Rust orchestration and OS integration
├── worker/llm_wiki_engine/    Python extraction and ingest package
├── schemas/                   Versioned IPC, artifact, and AI-output schemas
├── prompts/                   Versioned provider-neutral ingest instructions
├── tests/                     Unit, integration, contract, and end-to-end tests
├── installer/                 Windows packaging and bundled-runtime manifests
├── docs/                      Architecture, decisions, privacy, and operations
└── .github/workflows/         CI, signed build, checksums, and Releases
```

Files should be small and responsibility-focused. Provider-specific behavior must
remain inside its adapter. Extraction must not write directly to published wiki
paths. Publishing must not know how source formats are parsed.

## 7. Document acquisition and extraction

### 7.1 Common acquisition

For every selected file, the worker:

1. resolves and validates the exact path;
2. records size, modification time, original name, and MIME/type result;
3. computes SHA-256 from the exact bytes;
4. records the source drive root, relative path, and SHA-256 in the active wiki's
   private catalog without copying the original;
5. treats identical SHA-256 values as one source record and refreshes its location;
6. reuses a validated extraction artifact when the source and configuration hashes
   match.

Documents are untrusted input. The application does not execute macros, embedded
programs, links, or document-provided commands.

### 7.2 PDF

Every PDF runs through OpenDataLoader. A digitally generated PDF with selectable
text uses its fast structural Markdown/JSON parser so layout and semantic structure
are retained without the cost of image OCR. PDFs without usable embedded text use
the full hybrid force-OCR backend.

The PDF artifact contains:

- OCR/layout Markdown;
- OpenDataLoader semantic JSON;
- page numbers and bounding boxes;
- detected headings, paragraphs, lists, tables, formulas, images, and captions;
- extracted assets where applicable;
- native text captured separately for comparison and recovery;
- per-page warnings and quality measurements;
- tool, model, language, and configuration versions.

Native text and parser structure remain in the extraction artifact for provenance.

Encrypted PDFs enter review unless the user supplies the password for the current
job. Passwords remain in memory and are never logged or persisted.

### 7.3 DOCX

DOCX files are parsed directly without OCR. The extractor preserves heading levels,
paragraphs, tables, lists, hyperlinks, footnotes where supported, and embedded
media. Macros and external relationships are not executed. Unsupported or lossy
elements generate explicit warnings instead of silent omission.

### 7.4 TXT and Markdown

TXT and MD files are decoded using a deterministic encoding strategy and are never
sent through OCR. Existing Markdown structure is parsed, not blindly trusted.
Frontmatter, links, code fences, math, and headings are represented in the artifact.
The original bytes and normalized text remain distinct so normalization cannot
erase provenance.

## 8. Artifact and wiki model

Each extraction produces a versioned artifact before AI processing. The artifact is
validated against JSON Schema and stored under a content-addressed path. AI output
cannot replace the extraction artifact.

The published Markdown graph uses these page types:

- `source` — detailed, evidence-linked representation of one source;
- `concept` — reusable idea deduplicated across sources in the active wiki;
- `entity` — person, organization, place, standard, or named body;
- `synthesis` — durable comparison or cross-source overview;
- `index` — navigation page for a category, course, project, or root catalog.

Every page contains YAML frontmatter with a stable page identifier, title, type,
tags, created/updated dates, source identifiers, language, and generation metadata.
Source pages include exact source provenance. Generated claims retain source/page or
section references wherever the input format permits them.

Suggested visible wiki layout:

```text
<wiki-root>/
├── index.md
├── sources/
├── concepts/
├── entities/
├── syntheses/
├── indexes/
└── attachments/
```

Application state is stored under a reserved `.llm-wiki/` directory excluded from
normal Obsidian navigation:

```text
.llm-wiki/
├── artifacts/         Validated extraction and AI artifacts
├── staging/           Candidate wiki transaction
├── backups/           Bounded publication snapshots
├── manifest.json      Source provenance
├── operations.jsonl   Append-only ingest and publish log
└── catalog.sqlite3    Jobs, graph catalog, FTS index, and review state
```

`catalog.sqlite3` stores each source SHA-256, drive root, and relative path. The
original is referenced in place and is not duplicated inside the wiki.

The application validates that `.llm-wiki/` is inside the exact configured wiki
root and never accepts a broad drive or user-profile path as a destructive target.

## 9. AI ingest pipeline

AI processing is automatic after successful extraction. The pipeline is multi-pass
and uses versioned, provider-neutral instructions plus strict structured outputs:

1. create or update the source page from the validated artifact;
2. extract candidate concepts and entities with evidence references;
3. query the active wiki catalog using exact names, aliases, tags, links, and SQLite
   full-text search;
4. ask the provider to classify each candidate as create, update, merge, link, or
   uncertain against a bounded set of existing candidates;
5. create or update concept and entity pages;
6. create a synthesis only when at least two sources support durable comparative or
   integrative value;
7. update category and root indexes;
8. add semantically valid reciprocal links;
9. return a complete proposed transaction and confidence/evidence report.

The provider never receives content from another wiki. Large sources are processed
in deterministic chunks with overlap and stable identifiers. Final synthesis uses
chunk summaries tied to source evidence rather than untraceable free text.

Document text is data, never instruction. Provider prompts explicitly ignore
commands found inside documents, and tool permissions are restricted to the active
staging area. The application does not enable unrestricted or
`dangerously-skip-permissions` execution modes.

## 10. Provider adapter contract and authentication

Every adapter implements:

- `detect()` and `version()`;
- `install_or_update()` using an official source and verified release metadata;
- `auth_status()` returning a machine-readable state;
- `connect()` and `disconnect()` without exposing credentials to application logs;
- `list_models()`;
- `run_structured()` with streaming progress, schema validation, usage metadata,
  timeout, and cancellation;
- normalized error categories.

### Codex

The Codex adapter uses the official non-interactive execution interface and JSONL
events. Connection is delegated to the official Codex login flow. The app reuses
the local authenticated session without reading its credentials.

### Antigravity

The Antigravity adapter uses headless/print mode with JSON or streaming JSON.
Connection is delegated to the official browser/keyring flow. The app never enables
global permission bypass; it uses the narrowest workspace and permission policy.

### Anthropic

The public application uses an Anthropic Console API key or another integration
explicitly authorized for third-party software. The key is stored through Windows
Credential Manager and is never written to the project, logs, or SQLite database.
The application does not present Claude.ai subscription login as its own login flow
and does not route public-app usage through personal subscription credentials.

Provider CLI binaries are not silently repackaged. The first-run wizard retrieves
the selected provider from its official distribution channel when licensing and
platform support allow it. Provider versions and capabilities are detected at run
time, while the application communicates only through tested adapter contracts.

## 11. Validation and publication

Validation runs before any generated page reaches the visible wiki. It checks:

- JSON and frontmatter schemas;
- required provenance and evidence fields;
- filename and globally unique page-ID rules;
- Markdown parsing and Obsidian-compatible links;
- missing, ambiguous, or case-mismatched link targets;
- graph reachability from an index;
- reciprocal relationships where semantically required;
- duplicate concepts and entities;
- unsupported assets and unsafe paths;
- confidence and extraction warnings.

Pages publish automatically only when all errors are absent and no review-blocking
warning remains. Warnings with safe, explicit policy may publish while remaining in
the operation report. Ambiguous merges, missing provenance, incomplete extraction,
and low-confidence structure always enter review.

Publishing is transaction-like on Windows: build the complete result in staging,
validate it, create a bounded backup of affected files, record a journal, replace
files atomically one at a time, update indexes and the append-only operation log,
and mark the journal complete. A failure triggers rollback from the verified
backup. Startup recovery resolves any incomplete journal before accepting a new
job.

## 12. Reliability and error handling

Jobs are state machines with durable checkpoints for acquisition, extraction, AI
passes, validation, and publication. Each source is isolated so one failure does not
discard successful work from other sources.

The application provides:

- bounded automatic retries for transient provider and process failures;
- exponential backoff with visible next action;
- cancellation at safe checkpoints;
- resume after application or machine restart;
- cache reuse without claiming incomplete artifacts as success;
- time, memory, file-size, page-count, and output-size limits;
- clear user messages with an optional redacted diagnostic bundle;
- no terminal windows for routine operations.

Diagnostic exports redact paths on request and must never include source content,
passwords, API keys, OAuth tokens, or credential files.

## 13. Privacy and security

Before the first online ingest for each provider, the application displays which
content leaves the computer and links to the provider's current privacy terms. The
user must confirm once per wiki/provider combination, and can revoke that consent.

The application:

- performs OCR and extraction locally;
- stores secrets only in approved OS/provider credential stores;
- avoids shell interpolation and passes argument arrays;
- validates every path against the active wiki root;
- blocks path traversal and unsafe archive entries;
- treats provider output as untrusted until schema and path validation pass;
- never executes generated code or document macros;
- never edits original selected files;
- maintains immutable source hashes and an append-only operation history.

## 14. Installation, updates, and GitHub delivery

GitHub Releases exposes one prominent asset, `LLM-Wiki-Setup.exe`. It installs the
Tauri application, isolated Python runtime, Java runtime, OpenDataLoader stack,
pinned OCR resources, schemas, and worker without relying on system Python or Java.
If WebView2 is unavailable, setup provisions the official runtime.

Provider installation occurs during first-run setup because providers update
independently and may impose redistribution restrictions. The UI handles official
download, launch, connection, and verification as one guided flow. Core local OCR
continues to work when provider services are temporarily unavailable, while AI
ingest remains queued until a provider is ready.

Releases are produced by GitHub Actions from locked dependencies. Each release
includes checksums, dependency and license inventory, build provenance, and a
signed executable when a code-signing identity is configured. The application
offers updates but never updates during an active ingest or without a recovery
checkpoint.

## 15. Test strategy

### Unit and contract tests

- format routing and MIME validation;
- deterministic hashing and cache keys;
- PDF/DOCX/TXT/MD normalizers;
- schema parsing and IPC compatibility;
- provider event normalization;
- frontmatter, link, path, and graph validators;
- job state transitions and retry policy;
- publication journal and rollback.

### Integration tests

- synthetic digital and image-only PDFs forced through full OCR/layout;
- headings, tables, lists, formulas, images, and multilingual OCR;
- DOCX structure and embedded media;
- TXT encoding and Markdown preservation;
- mixed-format batch ingest;
- duplicate source, concept, and entity handling;
- interrupted worker and provider processes;
- staged publication into a disposable Obsidian vault.

Provider contract tests use deterministic fake adapters by default. Live tests are
explicit, credential-gated, spend-limited, and excluded from pull-request CI.

### End-to-end and release tests

- installation and uninstall on a clean Windows VM with no Python or Java;
- first-run provider connection and readiness diagnostics;
- creation of two isolated wikis with no cross-contamination;
- complete ingest, restart/resume, review, publication, and Open in Obsidian flow;
- upgrade from the previous supported release;
- offline launch and queued-AI behavior;
- checksum and dependency/license verification.

## 16. MVP acceptance criteria

The MVP is complete when all of the following are demonstrated on a clean Windows
machine:

1. `Setup.exe` installs and launches without manual dependency installation.
2. The user can create at least two isolated wiki workspaces.
3. A mixed batch of PDF, DOCX, TXT, and MD files completes from one primary action.
4. Every PDF page is processed by OpenDataLoader: structural extraction for
   selectable text and the configured hybrid OCR/layout path for scanned pages.
5. The chosen provider creates source, concept, entity, synthesis, and index pages
   with valid structured metadata and provenance.
6. Existing concepts are linked or updated instead of duplicated in tested cases.
7. Valid output publishes automatically; blocked output appears in review.
8. Killing and restarting the app resumes without repeating completed OCR.
9. A simulated publication failure restores the previous vault state.
10. No test finds credentials in logs, project files, SQLite, or diagnostics.
11. The output opens in Obsidian with valid navigation and no unresolved required
    links.
12. GitHub Actions produces the release installer and its verification metadata.

## 17. Implementation sequence

Implementation should proceed in independently verifiable vertical slices:

1. repository foundation, typed contracts, and CI;
2. Tauri shell, multi-wiki registry, and settings;
3. supervised Python worker and durable job state;
4. immutable acquisition and mixed-format extraction;
5. forced full-PDF OCR/layout and artifact validation;
6. provider-neutral structured ingest with a fake adapter;
7. Codex, Anthropic, and Antigravity adapters;
8. graph validation, review, transactional publication, and recovery;
9. first-run setup, bundled runtimes, Windows installer, and clean-VM tests;
10. accessibility, localization, performance, security, and release hardening.

Each slice must preserve the component boundaries defined above and include its own
tests before the next slice begins.
