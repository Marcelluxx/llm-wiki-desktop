import { type FormEvent, useEffect, useRef, useState } from "react";
import type { ChatMessageRecord, ChatStreamEvent, ProviderSummary } from "../contracts";
import type { Messages } from "../i18n";
import { formatAppError } from "../i18n";
import type { RegistryClient } from "../services/registry";
import { MarkdownContent } from "./MarkdownContent";
import { ProviderLogo } from "./ProviderLogo";

interface WikiChatProps {
  wikiId: string;
  provider: ProviderSummary | null;
  ingestAvailable: boolean;
  messages: Messages;
  client: RegistryClient;
  onProvider(): void;
}

type ChatEventEntry = ChatStreamEvent & { id: number };

export function WikiChat({
  wikiId,
  provider,
  ingestAvailable,
  messages,
  client,
  onProvider,
}: WikiChatProps) {
  const [history, setHistory] = useState<ChatMessageRecord[]>([]);
  const [input, setInput] = useState("");
  const [events, setEvents] = useState<ChatEventEntry[]>([]);
  const [liveAnswer, setLiveAnswer] = useState("");
  const [busy, setBusy] = useState<"chat" | "ingest" | null>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const scrollAnchor = useRef<HTMLDivElement>(null);
  const nextEventId = useRef(0);
  const providerReady = provider?.status === "connected";
  const ingestSupported = provider?.transport === "cli";

  useEffect(() => {
    void client
      .listChatMessages(wikiId)
      .then(setHistory)
      .catch(() => undefined);
  }, [client, wikiId]);

  useEffect(() => {
    scrollAnchor.current?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
  });

  useEffect(() => {
    if (!expanded) return undefined;
    document.body.classList.add("chat-expanded");
    const collapseOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setExpanded(false);
    };
    window.addEventListener("keydown", collapseOnEscape);
    return () => {
      document.body.classList.remove("chat-expanded");
      window.removeEventListener("keydown", collapseOnEscape);
    };
  }, [expanded]);

  function receiveEvent(event: ChatStreamEvent) {
    nextEventId.current += 1;
    setEvents((current) => [...current.slice(-199), { ...event, id: nextEventId.current }]);
    if (event.kind === "delta") setLiveAnswer((current) => current + event.message);
    if (event.kind === "message") setLiveAnswer(event.message);
    if (event.kind === "error" || event.kind === "warning") setDetailsOpen(true);
  }

  async function send(event: FormEvent) {
    event.preventDefault();
    const content = input.trim();
    if (!content || busy || !providerReady) return;
    const optimistic: ChatMessageRecord = {
      message_id: `pending-${crypto.randomUUID()}`,
      provider_id: provider.provider_id,
      role: "user",
      content,
      created_at: new Date().toISOString(),
    };
    setHistory((current) => [...current, optimistic]);
    setInput("");
    setBusy("chat");
    setError(null);
    setEvents([]);
    setLiveAnswer("");
    try {
      const response = await client.sendChatMessage(wikiId, content, receiveEvent);
      setHistory((current) => [...current, response]);
      setLiveAnswer("");
    } catch (reason) {
      setError(formatAppError(reason, messages));
      setDetailsOpen(true);
    } finally {
      setBusy(null);
    }
  }

  async function ingest() {
    if (!ingestAvailable || !ingestSupported || busy || !providerReady) return;
    setBusy("ingest");
    setError(null);
    setEvents([]);
    setLiveAnswer("");
    setHistory((current) => [
      ...current,
      {
        message_id: `ingest-${crypto.randomUUID()}`,
        provider_id: provider.provider_id,
        role: "system",
        content: messages.chatIngestStarted,
        created_at: new Date().toISOString(),
      },
    ]);
    try {
      const response = await client.startWikiIngest(wikiId, receiveEvent);
      setHistory((current) => [...current, response]);
      setLiveAnswer("");
    } catch (reason) {
      setError(formatAppError(reason, messages));
      setDetailsOpen(true);
    } finally {
      setBusy(null);
    }
  }

  return (
    <aside
      className={`wiki-chat${expanded ? " wiki-chat--expanded" : ""}`}
      aria-labelledby="wiki-chat-title"
    >
      <header className="wiki-chat__header">
        <div>
          <p className="eyebrow">{messages.chatEyebrow}</p>
          <h2 id="wiki-chat-title">{messages.chatTitle}</h2>
        </div>
        <div className="chat-header-actions">
          {provider && (
            <button type="button" className="chat-provider-chip" onClick={onProvider}>
              <ProviderLogo provider={provider.provider_id} />
              <span>
                <strong>
                  {provider.provider_id === "claude" ? "Claude Code" : provider.display_name}
                </strong>
                <small className="chat-provider-state">
                  {providerReady ? messages.providerConnected : messages.providerActionRequired}
                </small>
              </span>
            </button>
          )}
          <button
            type="button"
            className="chat-expand-button"
            onClick={() => setExpanded((current) => !current)}
            aria-pressed={expanded}
            title={expanded ? messages.chatCollapse : messages.chatExpand}
          >
            <span aria-hidden="true">{expanded ? "↙" : "↗"}</span>
            {expanded ? messages.chatCollapse : messages.chatExpand}
          </button>
        </div>
      </header>

      {!providerReady ? (
        <section className="chat-provider-empty">
          <div className="chat-orb" aria-hidden="true">
            ✦
          </div>
          <h3>{messages.chatProviderRequired}</h3>
          <p>{messages.chatProviderRequiredHint}</p>
          <button type="button" className="primary-button" onClick={onProvider}>
            {messages.providerChoose}
          </button>
        </section>
      ) : (
        <>
          <div className="chat-thread" aria-live="polite">
            {history.length === 0 && !busy && (
              <section className="chat-welcome">
                <div className="chat-orb" aria-hidden="true">
                  ✦
                </div>
                <h3>{messages.chatWelcome}</h3>
                <p>{messages.chatWelcomeHint}</p>
              </section>
            )}
            {history.map((entry) => (
              <article
                key={entry.message_id}
                className={`chat-message chat-message--${entry.role}`}
              >
                <small className="chat-message-role">
                  {entry.role === "user"
                    ? messages.chatYou
                    : entry.role === "system"
                      ? messages.chatActivity
                      : provider.display_name}
                </small>
                <MarkdownContent content={entry.content} />
              </article>
            ))}
            {busy && (
              <article className="chat-message chat-message--assistant chat-message--live">
                <small className="chat-message-role">{provider.display_name}</small>
                <MarkdownContent
                  content={
                    liveAnswer ||
                    (busy === "ingest" ? messages.chatIngestWorking : messages.chatThinking)
                  }
                />
                <span className="typing-indicator" aria-hidden="true">
                  <i />
                  <i />
                  <i />
                </span>
              </article>
            )}
            <div ref={scrollAnchor} />
          </div>

          {error && (
            <div className="error-banner chat-error" role="alert">
              {error}
            </div>
          )}

          {events.length > 0 && (
            <section className="chat-cli-flow">
              <button
                type="button"
                className="text-button chat-flow-toggle"
                aria-expanded={detailsOpen}
                onClick={() => setDetailsOpen((current) => !current)}
              >
                {detailsOpen ? messages.chatHideFlow : messages.chatShowFlow} · {events.length}
              </button>
              {detailsOpen && (
                <ol>
                  {events.map((event) => (
                    <li key={event.id} data-kind={event.kind}>
                      <span>{event.kind}</span>
                      <code>{event.message}</code>
                    </li>
                  ))}
                </ol>
              )}
            </section>
          )}

          <div className="chat-ingest-action">
            <div>
              <strong>{messages.chatIngestTitle}</strong>
              <small className="chat-ingest-hint">
                {!ingestSupported
                  ? messages.chatIngestProviderUnsupported
                  : ingestAvailable
                    ? messages.chatIngestReady
                    : messages.chatIngestDisabled}
              </small>
            </div>
            <button
              type="button"
              className="ingest-button"
              disabled={!ingestAvailable || !ingestSupported || busy !== null}
              onClick={() => void ingest()}
            >
              {busy === "ingest" ? messages.chatIngestWorking : messages.chatIngestButton}
            </button>
          </div>

          <form className="chat-composer" onSubmit={send}>
            <textarea
              value={input}
              rows={3}
              maxLength={20_000}
              placeholder={messages.chatPlaceholder}
              disabled={busy !== null}
              onChange={(event) => setInput(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey) {
                  event.preventDefault();
                  event.currentTarget.form?.requestSubmit();
                }
              }}
            />
            <button
              type="submit"
              disabled={!input.trim() || busy !== null}
              aria-label={messages.chatSend}
            >
              ↑
            </button>
            <small className="chat-composer-hint">{messages.chatComposerHint}</small>
          </form>
        </>
      )}
    </aside>
  );
}
