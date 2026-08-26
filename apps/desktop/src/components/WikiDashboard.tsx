import type { ProviderSummary, WikiRegistration } from "../contracts";
import type { Messages } from "../i18n";
import { ProviderBadge } from "./ProviderBadge";

interface WikiDashboardProps {
  messages: Messages;
  wikis: WikiRegistration[];
  onCreate(): void;
  onRegister(): void;
  onOpen(wiki: WikiRegistration): void;
  onSettings(): void;
  provider: ProviderSummary | null;
  onProvider(): void;
}

export function WikiDashboard({
  messages,
  wikis,
  onCreate,
  onRegister,
  onOpen,
  onSettings,
  provider,
  onProvider,
}: WikiDashboardProps) {
  return (
    <main className="app-main" id="main-content">
      <header className="page-header">
        <div>
          <p className="eyebrow">{messages.appName}</p>
          <h1>{messages.yourWikis}</h1>
          <p className="muted">{messages.dashboardHint}</p>
        </div>
        <div className="dashboard-header-actions">
          <ProviderBadge provider={provider} messages={messages} onClick={onProvider} />
          <button type="button" className="icon-button" onClick={onSettings}>
            <span aria-hidden="true">⚙</span>
            <span className="sr-only">{messages.settings}</span>
          </button>
        </div>
      </header>

      <div className="primary-actions">
        <button type="button" className="primary-button" onClick={onCreate}>
          <span aria-hidden="true">＋</span> {messages.createWiki}
        </button>
        <button type="button" className="secondary-button" onClick={onRegister}>
          {messages.registerWiki}
        </button>
      </div>

      {wikis.length === 0 ? (
        <section className="empty-state" aria-labelledby="empty-title">
          <div className="empty-illustration" aria-hidden="true">
            <span>◇</span>
            <span>◇</span>
            <span>◇</span>
          </div>
          <h2 id="empty-title">{messages.noWikis}</h2>
          <p>{messages.noWikisHint}</p>
          <button type="button" className="primary-button" onClick={onCreate}>
            {messages.createWiki}
          </button>
        </section>
      ) : (
        <section className="wiki-grid" aria-label={messages.yourWikis}>
          {wikis.map((wiki) => (
            <article className="wiki-card" key={wiki.wiki_id}>
              <div className="wiki-card__icon" aria-hidden="true">
                {wiki.display_name.slice(0, 2).toUpperCase()}
              </div>
              <div className="wiki-card__body">
                <h2>{wiki.display_name}</h2>
                <p title={wiki.canonical_root}>{wiki.canonical_root}</p>
              </div>
              <button type="button" className="card-action" onClick={() => onOpen(wiki)}>
                {messages.open} <span aria-hidden="true">→</span>
              </button>
            </article>
          ))}
        </section>
      )}
    </main>
  );
}
