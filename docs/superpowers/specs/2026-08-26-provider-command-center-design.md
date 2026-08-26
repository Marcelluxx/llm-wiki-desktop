# Provider Command Center design

Date: 2026-08-26  
Status: approved design  
Scope: provider selection, installation, authentication, model discovery, progress,
diagnostics, and preferences

This specification supersedes the provider installation and authentication details
in Sections 10, 12, 13, and 14 of the original desktop design where they conflict.
The provider-neutral ingest pipeline and its safety boundaries remain unchanged.

## 1. Outcome

The home screen always communicates whether an AI provider is ready. A compact
provider badge opens a premium command center where the user can select, install,
connect, update, and configure:

- Codex;
- Claude;
- Antigravity (`agy`);
- OpenRouter;
- Ollama as the first local-model backend.

The selected provider is the global default. Each wiki may override the provider
and model without changing other wikis. A later LM Studio adapter must fit the same
contract without changing the command center or preference schema.

No download, install, authentication attempt, model pull, or provider call happens
silently. Every operation has visible state, progress, cancellation, sanitized logs,
and a recoverable terminal result.

## 2. Product boundaries

This slice delivers provider readiness and the adapter foundations required by AI
ingest. It does not complete concept/entity/synthesis generation, billing
aggregation, or provider account management.

The app may install provider tools only after explicit confirmation. It never
changes the user's existing global Node.js installation. Provider credentials never
enter the application database, wiki, diagnostic log, or analytics.

Ollama is the only local backend in the first release. The provider interface keeps
`local_http` as a transport capability so LM Studio can be added later.

## 3. Approved interface

### 3.1 Global badge

The dashboard header shows the default provider logo, name, and one concise state:

- **Collegato** — installed, authenticated, and verified;
- **Da installare** — required executable is absent;
- **Accesso richiesto** — executable is ready but no usable authenticated session
  exists;
- **Chiave richiesta** — an API provider needs a credential;
- **Installato · offline** — a local provider is installed but its service is not
  reachable;
- **Aggiornamento richiesto** — the detected version is unsupported;
- **Azione richiesta** — a recoverable operation needs user intervention;
- **Non disponibile** — the provider cannot run on the current system.

The badge opens the Provider Command Center. It never implies that a provider is
connected based only on finding an executable.

### 3.2 Provider Command Center

Each provider card contains:

- official logo and provider name;
- transport type: official CLI, cloud API, or local model;
- version and selected model when known;
- status badge;
- one primary contextual action such as **Installa**, **Accedi**, **Configura**,
  **Scegli modello**, **Aggiorna**, or **Riprova**;
- a secondary **Gestisci** action when connected.

Logos are packaged as local optimized SVG/PNG assets sourced from official brand
materials or, where unavailable, a reviewed trademark-compatible provider mark.
Runtime UI never depends on a logo CDN. Each image has an accessible provider-name
alternative.

### 3.3 Preferences

The global default is selected in the command center. Wiki settings expose
**Usa predefinito globale** or a provider/model override. If an override becomes
unavailable, that wiki enters **Azione richiesta**; it does not silently switch to a
different online provider and send data elsewhere.

Before the first online ingest for a wiki/provider pair, the existing privacy
consent is still required. Ollama is labeled local only when the configured endpoint
is loopback and the selected model is local.

## 4. Architecture

### 4.1 Provider Manager

Rust owns a `ProviderManager` behind Tauri commands. The frontend never invokes a
shell or provider executable directly. The manager provides:

- `list_provider_statuses()`;
- `detect_provider(provider_id)`;
- `start_provider_install(provider_id)`;
- `cancel_provider_operation(operation_id)`;
- `start_provider_login(provider_id)`;
- `disconnect_provider(provider_id)`;
- `list_provider_models(provider_id)`;
- `set_default_provider(provider_id, model_id)`;
- `set_wiki_provider(wiki_id, selection | inherit)`;
- `read_provider_operation_log(operation_id)`.

Long-running commands return an operation identifier immediately and stream typed
events. Detection is bounded and runs concurrently per provider without blocking
dashboard rendering.

### 4.2 Adapter contract

Every adapter declares transport and capabilities, then implements the supported
subset of:

- `detect` and `version`;
- `install_plan` and `install`;
- `auth_status`, `connect`, and `disconnect`;
- `list_models`;
- `health_check`;
- later, `run_structured` for provider-neutral AI ingest.

Unsupported capabilities are explicit. The UI disables unavailable actions instead
of attempting guessed commands. Executables are launched with argument arrays,
controlled environment variables, hidden child windows, timeouts, process-tree
cancellation, and narrowly scoped working directories. Unrestricted permission
flags are prohibited.

### 4.3 Transport split

- Codex, Claude, and Antigravity use official CLI adapters.
- OpenRouter uses its HTTPS API directly.
- Ollama uses its loopback HTTP API directly.

This split preserves official subscription/login flows where supported while
avoiding an unnecessary CLI layer for API and local providers.

## 5. Installation sources and dependencies

### 5.1 Private Node.js runtime

Codex and Claude use a private Node.js LTS runtime managed under the app data
directory. It is downloaded on demand after confirmation and is not included in the
base installer. The app:

1. obtains the pinned Windows x64 archive and checksum manifest from `nodejs.org`;
2. checks available disk space before downloading;
3. streams to a temporary operation directory;
4. verifies the exact SHA-256 from the official manifest;
5. extracts with traversal and archive-size limits;
6. atomically activates the verified runtime;
7. never changes global `PATH`, npm configuration, or an existing Node install.

Provider npm packages are installed into separate app-managed prefixes with exact
versions and package integrity verification. A failed upgrade leaves the previous
working version active.

### 5.2 Codex

The app installs the official `@openai/codex` package with the private runtime when
Codex is absent. It delegates connection to `codex login`, verifies readiness with
`codex login status`, and delegates disconnection to `codex logout`. Browser login
and API-key login remain provider-owned flows; the app does not inspect
`auth.json` or read cached credentials.

Reference: <https://developers.openai.com/codex/cli/> and
<https://developers.openai.com/codex/auth/>.

### 5.3 Claude

The app installs the official `@anthropic-ai/claude-code` package with the private
runtime. It starts the official Claude Code authentication choice, which may use
Anthropic Console, an eligible Claude App subscription, or a supported enterprise
platform. The app observes process outcome and performs a non-secret readiness
probe; it never presents the provider login page as its own or reads Claude's
credential store.

Native Windows requires Git for Windows according to the current Claude Code
requirements. Detection reports that dependency separately and the confirmation
screen includes it when missing. Installation uses an official Git for Windows
release and verified release metadata; it does not silently substitute WSL.

Reference: <https://docs.anthropic.com/en/docs/claude-code/getting-started>.

### 5.4 Antigravity

Antigravity uses the official Windows `agy` distribution, not npm. The app downloads
the official installer artifact to a temporary directory, displays its origin, and
uses the documented flags that avoid modifying shell aliases or global `PATH` when
available. Authentication remains the official browser and Windows Credential
Manager flow. The app never reads the keyring.

Reference: <https://antigravity.google/docs/cli-install?hl=en>.

### 5.5 OpenRouter

OpenRouter requires no CLI or Node runtime. The setup opens the official key page,
accepts a pasted API key into a masked field, stores it as a generic credential in
Windows Credential Manager, and validates it with OpenRouter's authenticated key
endpoint without logging it. Models come from the official model endpoint and are
cached with a short expiry. Invalid, exhausted, restricted, and rate-limited keys
remain distinct states.

Reference: <https://openrouter.ai/docs/quickstart>.

### 5.6 Ollama

The app first probes `http://127.0.0.1:11434`. If Ollama is missing, setup uses the
official Windows installer after confirmation. The adapter reads version and model
inventory from the local API. Model pulls use Ollama's streaming API so downloaded
bytes, total bytes, digest, current layer, and percentage remain visible. Model size
and required free space are shown before pulling when metadata permits.

The endpoint defaults to loopback. A future custom endpoint is treated as online
unless it resolves exclusively to loopback. Model storage location is displayed and
can later expose Ollama's supported `OLLAMA_MODELS` configuration.

Reference: <https://docs.ollama.com/windows>.

## 6. Operation state and progress

Provider operations use this durable state machine:

`queued → detecting → awaiting_confirmation → downloading → verifying → installing
→ authenticating → validating → completed`

Terminal alternatives are `cancelled`, `failed`, and `action_required`. Retryable
failures retain the verified artifacts needed for a safe retry. Partial or
unverified files are never activated.

Every event includes, when applicable:

- operation, provider, component, phase, and human-readable message;
- bytes downloaded and total bytes;
- percentage only when a reliable total exists;
- transfer speed, elapsed time, estimated remaining time, and retry count;
- source hostname, target category, component/version, and expected disk use;
- checksum phase/result and child-process phase;
- normalized error category and stable diagnostic code.

Unknown totals use an indeterminate progress bar instead of fabricated percentages.
Download speed and ETA are smoothed to prevent distracting jumps. The visible log
retains the latest entries; the complete JSONL log is stored in application data,
not inside a wiki.

Cancellation terminates the child process tree, stops timers immediately, removes
unverified temporary files, keeps the last verified active version, and permits a
new operation. Closing the app records an interrupted operation; startup reconciles
temporary and active directories before offering retry.

## 7. Errors and recovery

Normalized categories include:

- offline, DNS, timeout, proxy authentication, TLS/certificate, HTTP status, and
  interrupted transfer;
- insufficient disk, permission denied, path too long, file lock, antivirus
  quarantine, and unsupported Windows version/architecture;
- missing or mismatched checksum, corrupted archive, unsafe archive path, package
  integrity failure, and unsupported provider version;
- dependency absent, executable not found after install, child crash, non-zero exit,
  malformed output, inactivity timeout, and cancellation timeout;
- browser launch failure, login declined, login timeout, session expired, workspace
  restriction, and unavailable authentication method;
- invalid/revoked API key, insufficient credit, forbidden model, rate limit, provider
  outage, and malformed provider response;
- Ollama service stopped, local endpoint unavailable, model missing, model pull
  failure, and insufficient RAM/VRAM when detectable.

Transient network/provider failures use bounded exponential backoff with visible
attempt count and next action. Integrity, permission, credential, and disk failures
never retry blindly. Unknown failures receive a stable operation ID, a safe summary,
expandable technical detail, **Copia log**, and **Riprova** where safe.

No design can enumerate every future provider failure. The normalized unknown-error
fallback guarantees that unrecognized failures remain visible, cancellable,
diagnosable, and non-destructive.

## 8. Secrets and log redaction

The application treats the following as secrets and removes them from UI, console,
JSONL, panic reports, and exported diagnostics:

- API keys and bearer/basic authorization headers;
- OAuth access, refresh, device, callback, and authorization codes;
- cookies and session identifiers;
- credential-file contents and keyring responses;
- URLs containing secret query parameters;
- stdin sent to provider login commands.

Redaction happens before an event crosses the backend/frontend boundary. Log fields
use allowlisted structured values where possible. Raw provider output is retained
only after line-by-line sanitization. **Copia log** copies the sanitized view.

## 9. Persistence

The global application catalog stores:

- detected provider installation path and version;
- supported capabilities and last health-check time;
- default provider/model;
- provider operation state and sanitized log path;
- non-secret model cache metadata.

Each wiki stores either `inherit` or an explicit provider/model selection plus the
existing privacy-consent record. Windows Credential Manager stores OpenRouter and
other future app-managed secrets. CLI-owned sessions remain exclusively in the
provider's credential storage.

Schema migrations are additive and preserve existing wikis whose provider is
currently `fake`. The fake adapter remains available in development and tests but
is hidden from production UI.

## 10. Testing and acceptance

Automated tests use fake executables, a local HTTP fixture server, disposable
directories, and a fake credential-store adapter. Normal CI never performs a live
login, downloads a real provider package, contacts a paid model, or consumes credit.

Coverage includes:

- detection for absent, valid, outdated, and malformed installations;
- private Node install, exact version pinning, checksum success/failure, atomic
  activation, and rollback;
- progress with known and unknown totals, speed/ETA, cancellation, retry, restart,
  and concurrent-operation rejection;
- network, proxy, TLS, disk, permission, locked-file, unsafe-archive, corrupted
  package, child crash, timeout, and unsupported-platform failures;
- every authentication state without exposing fake secrets;
- OpenRouter credential validation and model-cache expiry;
- Ollama health, model inventory, model pull streaming, service stopped, and missing
  model;
- global default, per-wiki override, unavailable override, and no silent provider
  fallback;
- English/Italian UI, keyboard navigation, screen-reader names, responsive layout,
  status color contrast, and reduced motion;
- secret-canary scanning across UI events, console output, JSONL, diagnostics,
  SQLite, and copied logs.

Acceptance requires:

1. The dashboard badge reports the truthful default-provider state.
2. All five provider cards show local logos, names, statuses, and contextual actions.
3. Missing CLI dependencies install only after confirmation from official sources.
4. The private Node runtime never modifies global Node or `PATH`.
5. Every long operation has visible progress/logs and immediate cancellation.
6. A failed install cannot replace a previously working provider.
7. Codex, Claude, and Antigravity delegate credentials to official flows.
8. OpenRouter credentials exist only in Windows Credential Manager.
9. Ollama can list and pull local models with truthful streamed progress.
10. Global defaults and per-wiki overrides survive restart.
11. No secret canary appears in any persistent or visible diagnostic surface.
12. Frontend, Rust, Python, schema, accessibility, and clean-Windows smoke gates pass.

## 11. Delivery sequence

Implementation proceeds in bounded slices:

1. contracts, persistence, fake adapters, and operation event model;
2. premium dashboard badge and Provider Command Center;
3. private Node manager and resumable verified downloader;
4. Codex and Claude installation/authentication adapters;
5. Antigravity adapter;
6. Windows Credential Manager and OpenRouter adapter;
7. Ollama detection, models, and pull progress;
8. per-wiki overrides, recovery, diagnostics, accessibility, and release validation.

Each slice must pass its focused tests and the existing unified quality command
before the next slice begins.
