import type { WikiRegistration } from "../contracts";
import type { Messages } from "../i18n";

interface WikiHomeProps {
  wiki: WikiRegistration;
  messages: Messages;
  onBack(): void;
  onSettings(): void;
}

export function WikiHome({ wiki, messages, onBack, onSettings }: WikiHomeProps) {
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
        <p>{messages.emptyWikiHint}</p>
        <button type="button" className="primary-button" disabled title={messages.comingNext}>
          {messages.addDocuments}
        </button>
        <small>{messages.comingNext}</small>
      </section>
    </main>
  );
}
