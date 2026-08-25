import { useEffect, useState } from "react";
import type { JobEvent, JobSummary, WikiRegistration } from "../contracts";
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
  const activeJob = jobs.find((job) => !["completed", "failed", "cancelled"].includes(job.state));

  useEffect(() => {
    void client
      .listJobs(wiki.wiki_id)
      .then(setJobs)
      .catch(() => undefined);
  }, [client, wiki.wiki_id]);

  function applyEvent(event: JobEvent) {
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
      const job = await client.startImport(wiki.wiki_id, paths, applyEvent);
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
      </section>
    </main>
  );
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
