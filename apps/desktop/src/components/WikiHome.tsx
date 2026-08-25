import { useEffect, useState } from "react";
import type { JobEvent, JobLogEntry, JobSummary, WikiRegistration } from "../contracts";
import type { Messages } from "../i18n";
import { formatAppError } from "../i18n";
import type { RegistryClient } from "../services/registry";

interface WikiHomeProps {
  wiki: WikiRegistration;
  messages: Messages;
  client: RegistryClient;
  onBack(): void;
  onSettings(): void;
}

export function WikiHome({ wiki, messages, client, onBack, onSettings }: WikiHomeProps) {
  const [jobs, setJobs] = useState<JobSummary[]>([]);
  const [selectedNames, setSelectedNames] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [logs, setLogs] = useState<JobLogEntry[]>([]);
  const [logsOpen, setLogsOpen] = useState(false);
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null);
  const activeJob = jobs.find((job) => !["completed", "failed", "cancelled"].includes(job.state));

  useEffect(() => {
    void client
      .listJobs(wiki.wiki_id)
      .then((loadedJobs) => {
        setJobs(loadedJobs);
        const latestJob = loadedJobs[0];
        if (latestJob) {
          setSelectedJobId(latestJob.job_id);
          return client.readJobLog(wiki.wiki_id, latestJob.job_id).then(setLogs);
        }
        return undefined;
      })
      .catch(() => undefined);
  }, [client, wiki.wiki_id]);

  function applyEvent(event: JobEvent) {
    if (event.log_level) {
      const entry: JobLogEntry = {
        timestamp: new Date().toISOString(),
        level: event.log_level,
        job_id: event.job_id,
        state: event.state,
        message: event.message,
        source: event.source ?? null,
        detail: event.detail ?? null,
      };
      setLogs((current) => [...current, entry].slice(-300));
      const consoleMessage = `[LLM Wiki][${event.job_id}] ${event.message}${event.source ? ` (${event.source})` : ""}`;
      if (event.log_level === "error") console.error(consoleMessage, event.detail ?? "");
      else if (event.log_level === "warning") console.warn(consoleMessage, event.detail ?? "");
      else console.info(consoleMessage, event.detail ?? "");
    }
    setJobs((current) =>
      current.map((job) =>
        job.job_id === event.job_id
          ? {
              ...job,
              state: event.state,
              stage_progress: event.progress,
              last_message: event.message,
              updated_at: new Date().toISOString(),
            }
          : job,
      ),
    );
  }

  async function addDocuments() {
    setError(null);
    try {
      const paths = await client.pickDocuments();
      if (paths.length === 0) return;
      setSelectedNames(paths.map(fileName));
      setLogs([]);
      setLogsOpen(true);
      const job = await client.startImport(wiki.wiki_id, paths, applyEvent);
      setSelectedJobId(job.job_id);
      setJobs((current) => [job, ...current.filter((item) => item.job_id !== job.job_id)]);
    } catch (reason) {
      setError(formatAppError(reason, messages));
    }
  }

  async function cancelActiveJob() {
    if (!activeJob) return;
    try {
      await client.cancelJob(activeJob.job_id);
    } catch (reason) {
      setError(formatAppError(reason, messages));
    }
  }

  return (
    <main className="app-main" id="main-content">
      <nav className="wiki-nav" aria-label={messages.yourWikis}>
        <button type="button" className="text-button" onClick={onBack}>
          <span aria-hidden="true">←</span> {messages.back}
        </button>
        <button type="button" className="icon-button" onClick={onSettings}>
          <span aria-hidden="true">⚙</span>
          <span className="sr-only">{messages.settings}</span>
        </button>
      </nav>
      <header className="wiki-heading">
        <div className="wiki-heading__mark" aria-hidden="true">
          {wiki.display_name.slice(0, 2).toUpperCase()}
        </div>
        <div>
          <p className="eyebrow">{messages.appName}</p>
          <h1>{wiki.display_name}</h1>
          <p className="path-copy">{wiki.canonical_root}</p>
        </div>
      </header>
      <section className="empty-state wiki-ready" aria-labelledby="wiki-ready-title">
        <div className="ready-check" aria-hidden="true">
          ✓
        </div>
        <h2 id="wiki-ready-title">{messages.emptyWiki}</h2>
        <p>{activeJob ? messages.importRunning : messages.emptyWikiHint}</p>
        {error && (
          <div className="error-banner" role="alert">
            {error}
          </div>
        )}
        {selectedNames.length > 0 && (
          <p className="selected-summary">
            {messages.selectedDocuments.replace("{count}", String(selectedNames.length))}
            <small>{selectedNames.slice(0, 3).join(" · ")}</small>
          </p>
        )}
        {activeJob && (
          <div className="job-progress" role="status" aria-live="polite">
            <div className="job-progress__labels">
              <strong>{stageLabel(activeJob.state, messages)}</strong>
              <span>{Math.round(activeJob.stage_progress * 100)}%</span>
            </div>
            <progress max="1" value={activeJob.stage_progress} />
            <button type="button" className="secondary-button" onClick={cancelActiveJob}>
              {messages.cancelImport}
            </button>
          </div>
        )}
        <button
          type="button"
          className="primary-button"
          onClick={addDocuments}
          disabled={Boolean(activeJob)}
        >
          {messages.addDocuments}
        </button>
        <small>{messages.supportedDocuments}</small>
        {selectedJobId && (
          <div className="log-area">
            <button
              type="button"
              className="text-button"
              onClick={() => setLogsOpen((value) => !value)}
              aria-expanded={logsOpen}
            >
              {logsOpen ? messages.hideLogs : messages.showLogs}
            </button>
            {logsOpen && (
              <section className="log-console" aria-label={messages.processingLogs}>
                <div className="log-console__header">
                  <strong>{messages.processingLogs}</strong>
                  <span>{logs.length}</span>
                </div>
                {logs.length === 0 ? (
                  <p>{messages.noLogs}</p>
                ) : (
                  <ol>
                    {logs.map((entry) => (
                      <li
                        key={`${entry.timestamp}-${entry.level}-${entry.message}-${entry.source ?? ""}-${entry.detail ?? ""}`}
                        data-level={entry.level}
                      >
                        <time>{formatLogTime(entry.timestamp)}</time>
                        <span>{entry.level.toUpperCase()}</span>
                        <code>
                          {entry.message}
                          {entry.source ? ` · ${entry.source}` : ""}
                          {entry.detail ? ` · ${entry.detail}` : ""}
                        </code>
                      </li>
                    ))}
                  </ol>
                )}
              </section>
            )}
          </div>
        )}
      </section>
    </main>
  );
}

function formatLogTime(timestamp: string): string {
  const date = new Date(timestamp);
  return Number.isNaN(date.getTime()) ? "--:--:--" : date.toLocaleTimeString();
}

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function stageLabel(state: JobSummary["state"], messages: Messages): string {
  if (state === "queued") return messages.stageQueued;
  if (state === "completed") return messages.stageCompleted;
  if (state === "cancelled") return messages.stageCancelled;
  if (state === "failed") return messages.stageFailed;
  return messages.stagePreparing;
}
