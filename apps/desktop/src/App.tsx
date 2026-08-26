import { useCallback, useEffect, useMemo, useState } from "react";
import { LanguageSetup } from "./components/LanguageSetup";
import { SettingsPanel } from "./components/SettingsPanel";
import { WikiDashboard } from "./components/WikiDashboard";
import { WikiForm } from "./components/WikiForm";
import { WikiHome } from "./components/WikiHome";
import type { PerformanceStatus, RegistrySnapshot, WikiRegistration } from "./contracts";
import { formatAppError, getMessages, type Language } from "./i18n";
import { registryClient, type RegistryClient, type WikiInput } from "./services/registry";

interface AppProps {
  client?: RegistryClient;
}

export function App({ client = registryClient }: AppProps) {
  const [snapshot, setSnapshot] = useState<RegistrySnapshot | null>(null);
  const [currentWiki, setCurrentWiki] = useState<WikiRegistration | null>(null);
  const [formMode, setFormMode] = useState<"create" | "register" | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [performance, setPerformance] = useState<PerformanceStatus | null>(null);

  const load = useCallback(async () => {
    setLoadError(null);
    try {
      setSnapshot(await client.getRegistry());
    } catch (reason) {
      setLoadError(formatAppError(reason, getMessages("it")));
    }
  }, [client]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    void client
      .getPerformanceStatus()
      .then(setPerformance)
      .catch(() => undefined);
  }, [client]);

  const language: Language = snapshot?.interface_language ?? "it";
  const messages = useMemo(() => getMessages(language), [language]);

  async function chooseLanguage(nextLanguage: Language) {
    setSnapshot(await client.setInterfaceLanguage(nextLanguage));
  }

  async function submitWiki(request: WikiInput) {
    const wiki =
      formMode === "register"
        ? await client.registerWiki(request)
        : await client.createWiki(request);
    setSnapshot((value) => (value ? { ...value, wikis: [...value.wikis, wiki] } : value));
    setFormMode(null);
    setCurrentWiki(wiki);
  }

  async function openWiki(wiki: WikiRegistration) {
    const updated = await client.openWiki(wiki.wiki_id);
    setSnapshot((value) =>
      value
        ? {
            ...value,
            wikis: value.wikis.map((item) => (item.wiki_id === updated.wiki_id ? updated : item)),
          }
        : value,
    );
    setCurrentWiki(updated);
  }

  async function renameCurrent(displayName: string) {
    if (!currentWiki) return;
    const updated = await client.renameWiki(currentWiki.wiki_id, displayName);
    setCurrentWiki(updated);
    setSnapshot((value) =>
      value
        ? {
            ...value,
            wikis: value.wikis.map((item) => (item.wiki_id === updated.wiki_id ? updated : item)),
          }
        : value,
    );
  }

  async function removeCurrent() {
    if (!currentWiki) return;
    setSnapshot(await client.removeRegistration(currentWiki.wiki_id));
    setCurrentWiki(null);
    setSettingsOpen(false);
  }

  if (loadError) {
    return (
      <main className="welcome-shell">
        <section className="welcome-card" role="alert">
          <h1>LLM Wiki</h1>
          <p>{loadError}</p>
          <button type="button" className="primary-button" onClick={load}>
            {messages.retry}
          </button>
        </section>
      </main>
    );
  }

  if (!snapshot) {
    return (
      <main className="loading-shell" role="status">
        <div className="brand-mark" aria-hidden="true">
          LW
        </div>
        <p>{messages.loading}</p>
      </main>
    );
  }

  if (!snapshot.interface_language) {
    return <LanguageSetup messages={messages} onChoose={chooseLanguage} />;
  }

  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>
      {currentWiki ? (
        <WikiHome
          wiki={currentWiki}
          messages={messages}
          client={client}
          onBack={() => setCurrentWiki(null)}
          onSettings={() => setSettingsOpen(true)}
        />
      ) : (
        <WikiDashboard
          messages={messages}
          wikis={snapshot.wikis}
          onCreate={() => setFormMode("create")}
          onRegister={() => setFormMode("register")}
          onOpen={openWiki}
          onSettings={() => setSettingsOpen(true)}
        />
      )}

      {formMode && (
        <WikiForm
          key={formMode}
          mode={formMode}
          language={language}
          messages={messages}
          onPickFolder={() => client.pickFolder()}
          onSubmit={submitWiki}
          onClose={() => setFormMode(null)}
        />
      )}

      {settingsOpen && (
        <SettingsPanel
          language={language}
          messages={messages}
          wiki={currentWiki}
          performance={performance}
          onInstallAcceleration={async () => {
            const status = await client.installNvidiaAcceleration();
            setPerformance(status);
          }}
          onLanguage={chooseLanguage}
          onRename={currentWiki ? renameCurrent : undefined}
          onRemove={currentWiki ? removeCurrent : undefined}
          onClose={() => setSettingsOpen(false)}
        />
      )}
    </div>
  );
}
