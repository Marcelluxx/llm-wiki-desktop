import { useRef, useState } from "react";
import type {
  ProviderActionLogEvent,
  ProviderId,
  ProviderModel,
  ProviderSummary,
} from "../contracts";
import type { Messages } from "../i18n";
import { useDialogFocus } from "../hooks/useDialogFocus";
import { ProviderLogo } from "./ProviderLogo";
import { providerStatus } from "./ProviderBadge";

type OperationLogEntry = ProviderActionLogEvent & { id: number };

interface ProviderCommandCenterProps {
  providers: ProviderSummary[];
  selectedProviderId?: ProviderId;
  messages: Messages;
  onRefresh(): Promise<void>;
  onSelect(providerId: ProviderId): Promise<void>;
  onAction(
    providerId: ProviderId,
    action: string,
    onEvent: (event: ProviderActionLogEvent) => void,
  ): Promise<void>;
  onListModels(providerId: ProviderId): Promise<ProviderModel[]>;
  onConfigureOpenRouter(apiKey: string | null, modelId: string): Promise<void>;
  onConfigureOllama(modelId: string): Promise<void>;
  onPullOllamaModel(
    modelId: string,
    onEvent: (event: ProviderActionLogEvent) => void,
  ): Promise<void>;
  onClose(): void;
}

export function ProviderCommandCenter({
  providers,
  selectedProviderId,
  messages,
  onRefresh,
  onSelect,
  onAction,
  onListModels,
  onConfigureOpenRouter,
  onConfigureOllama,
  onPullOllamaModel,
  onClose,
}: ProviderCommandCenterProps) {
  const { dialogRef, onKeyDown } = useDialogFocus(onClose);
  const [busyProvider, setBusyProvider] = useState<ProviderId | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [configuringProvider, setConfiguringProvider] = useState<"openrouter" | "ollama" | null>(
    null,
  );
  const [models, setModels] = useState<ProviderModel[]>([]);
  const [modelsLoading, setModelsLoading] = useState(false);
  const [modelQuery, setModelQuery] = useState("");
  const [openRouterKey, setOpenRouterKey] = useState("");
  const [ollamaModelToDownload, setOllamaModelToDownload] = useState("qwen3:4b");
  const [operationLogs, setOperationLogs] = useState<OperationLogEntry[]>([]);
  const [showOperationDetails, setShowOperationDetails] = useState(false);
  const nextOperationLogId = useRef(0);
  const configuredProvider = providers.find(
    (provider) => provider.provider_id === configuringProvider,
  );
  const [providerModel, setProviderModel] = useState("");

  function appendOperationLog(event: ProviderActionLogEvent) {
    nextOperationLogId.current += 1;
    setOperationLogs((current) => [
      ...current.slice(-99),
      { ...event, id: nextOperationLogId.current },
    ]);
  }

  async function refresh() {
    setRefreshing(true);
    setNotice(null);
    try {
      await onRefresh();
    } catch (reason) {
      setNotice(errorMessage(reason));
    } finally {
      setRefreshing(false);
    }
  }

  async function act(provider: ProviderSummary) {
    const action = providerActionId(provider.status);
    if (action === "install" && !window.confirm(messages.providerConfirmInstall)) return;
    setBusyProvider(provider.provider_id);
    setNotice(messages.providerWorking);
    setOperationLogs([]);
    try {
      await onAction(provider.provider_id, action, appendOperationLog);
      await onRefresh();
      setNotice(messages.providerActionDone);
    } catch (reason) {
      setNotice(errorMessage(reason));
    } finally {
      setBusyProvider(null);
    }
  }

  async function select(provider: ProviderSummary) {
    setBusyProvider(provider.provider_id);
    setNotice(null);
    try {
      await onSelect(provider.provider_id);
      setNotice(messages.providerSelectedNotice.replace("{provider}", provider.display_name));
    } catch (reason) {
      setNotice(errorMessage(reason));
    } finally {
      setBusyProvider(null);
    }
  }

  async function openProviderConfiguration(providerId: "openrouter" | "ollama") {
    const provider = providers.find((candidate) => candidate.provider_id === providerId);
    setConfiguringProvider(providerId);
    setModelsLoading(true);
    setNotice(null);
    setOpenRouterKey("");
    setModelQuery("");
    setProviderModel(provider?.selected_model ?? "");
    try {
      setModels(await onListModels(providerId));
    } catch (reason) {
      setNotice(errorMessage(reason));
    } finally {
      setModelsLoading(false);
    }
  }

  async function manageOllama(provider: ProviderSummary) {
    if (provider.status !== "connected") {
      setBusyProvider("ollama");
      setNotice(messages.providerWorking);
      setOperationLogs([]);
      try {
        await onAction("ollama", "start", appendOperationLog);
        await onRefresh();
      } catch (reason) {
        setNotice(errorMessage(reason));
        setBusyProvider(null);
        return;
      }
      setBusyProvider(null);
    }
    await openProviderConfiguration("ollama");
  }

  async function saveProviderConfiguration() {
    if (!configuringProvider) return;
    setBusyProvider(configuringProvider);
    setNotice(messages.providerWorking);
    try {
      if (configuringProvider === "openrouter") {
        await onConfigureOpenRouter(openRouterKey.trim() || null, providerModel);
      } else {
        await onConfigureOllama(providerModel);
      }
      await onRefresh();
      setConfiguringProvider(null);
      setOpenRouterKey("");
      setNotice(messages.providerActionDone);
    } catch (reason) {
      setNotice(errorMessage(reason));
    } finally {
      setBusyProvider(null);
    }
  }

  async function pullOllamaModel() {
    setBusyProvider("ollama");
    setNotice(messages.providerDownloadingModel.replace("{model}", ollamaModelToDownload.trim()));
    setOperationLogs([]);
    try {
      await onPullOllamaModel(ollamaModelToDownload.trim(), appendOperationLog);
      setModels(await onListModels("ollama"));
      setNotice(messages.providerModelDownloaded);
    } catch (reason) {
      setNotice(errorMessage(reason));
    } finally {
      setBusyProvider(null);
    }
  }

  const visibleModels = models
    .filter((model) => {
      const query = modelQuery.trim().toLocaleLowerCase();
      return (
        !query ||
        model.display_name.toLocaleLowerCase().includes(query) ||
        model.model_id.toLocaleLowerCase().includes(query)
      );
    })
    .slice(0, 80);
  const selectedModel = models.find((model) => model.model_id === providerModel);
  if (selectedModel && !visibleModels.includes(selectedModel)) visibleModels.unshift(selectedModel);

  return (
    <div className="modal-backdrop provider-modal-backdrop">
      <section
        ref={dialogRef}
        className="modal provider-command-center"
        role="dialog"
        aria-modal="true"
        aria-labelledby="provider-center-title"
        onKeyDown={onKeyDown}
      >
        <div className="modal-header provider-center-heading">
          <div>
            <p className="eyebrow">{messages.aiProvider}</p>
            <h2 id="provider-center-title">{messages.providerCenter}</h2>
            <p className="muted">{messages.providerCenterHint}</p>
          </div>
          <button
            type="button"
            className="icon-button"
            onClick={onClose}
            aria-label={messages.close}
          >
            ×
          </button>
        </div>

        <div className="provider-selection-summary">
          <span>{messages.providerActiveLabel}</span>
          <strong>
            {(() => {
              const activeProvider = providers.find(
                (provider) => provider.provider_id === selectedProviderId,
              );
              return activeProvider
                ? providerDisplayName(activeProvider.provider_id, activeProvider.display_name)
                : messages.providerNoneSelected;
            })()}
          </strong>
          <small>{messages.providerActiveHint}</small>
        </div>

        <div className="provider-list">
          {providers.map((provider) => {
            const selected = provider.provider_id === selectedProviderId;
            const ready = provider.status === "connected";
            return (
              <article
                className={`provider-card${selected ? " provider-card--selected" : ""}`}
                key={provider.provider_id}
              >
                <div className="provider-card__topline">
                  <ProviderLogo provider={provider.provider_id} />
                  <div className="provider-card__copy">
                    <h3>{providerDisplayName(provider.provider_id, provider.display_name)}</h3>
                    <p>{providerCompany(provider.provider_id)}</p>
                    <small>{transportLabel(provider.transport, messages)}</small>
                  </div>
                  <span className={`provider-state provider-state--${provider.status}`}>
                    {providerStatus(provider.status, messages)}
                  </span>
                </div>

                {provider.selected_model && (
                  <p className="provider-model-line">
                    <span>{messages.providerModel}</span>
                    <strong>{provider.selected_model}</strong>
                  </p>
                )}

                <div className="provider-card__footer">
                  {ready ? (
                    <button
                      type="button"
                      className={selected ? "provider-selected-button" : "provider-select-button"}
                      disabled={selected || busyProvider !== null}
                      onClick={() => void select(provider)}
                    >
                      {selected ? (
                        `✓ ${messages.providerSelected}`
                      ) : (
                        <>
                          {messages.providerUseThis} <span aria-hidden="true">→</span>
                        </>
                      )}
                    </button>
                  ) : (
                    <button
                      type="button"
                      className="secondary-button"
                      disabled={busyProvider !== null || refreshing}
                      onClick={() =>
                        void (provider.provider_id === "openrouter"
                          ? openProviderConfiguration("openrouter")
                          : act(provider))
                      }
                    >
                      {busyProvider === provider.provider_id
                        ? messages.providerWorking
                        : providerAction(provider.status, messages)}
                    </button>
                  )}
                  {((provider.provider_id === "openrouter" && ready) ||
                    (provider.provider_id === "ollama" && provider.status !== "not_installed")) && (
                    <button
                      type="button"
                      className="secondary-button provider-manage-button"
                      disabled={busyProvider !== null}
                      onClick={() =>
                        void (provider.provider_id === "ollama"
                          ? manageOllama(provider)
                          : openProviderConfiguration("openrouter"))
                      }
                    >
                      {messages.providerManage}
                    </button>
                  )}
                </div>
                {busyProvider === provider.provider_id && (
                  <div className="provider-operation-progress" role="progressbar">
                    <span />
                  </div>
                )}
              </article>
            );
          })}
        </div>

        {notice && (
          <p className="provider-notice" role="status">
            {notice}
          </p>
        )}
        {operationLogs.length > 0 && (
          <div className="provider-operation-details">
            <button
              type="button"
              className="text-button provider-log-toggle"
              aria-expanded={showOperationDetails}
              onClick={() => setShowOperationDetails((current) => !current)}
            >
              {showOperationDetails ? messages.providerHideDetails : messages.providerShowDetails}
            </button>
            {showOperationDetails && (
              <div className="provider-operation-log" role="log" aria-live="polite">
                <strong>{messages.providerOperationDetails}</strong>
                {operationLogs.map((event) => (
                  <p key={event.id} data-level={event.level}>
                    <span>{event.level.toUpperCase()}</span>
                    {event.message}
                  </p>
                ))}
              </div>
            )}
          </div>
        )}
        <footer className="provider-center-footer">
          <p>{messages.providerSecurityHint}</p>
          <button
            type="button"
            className="text-button"
            disabled={refreshing || busyProvider !== null}
            onClick={() => void refresh()}
          >
            {refreshing ? messages.providerRefreshing : messages.providerRefresh}
          </button>
        </footer>

        {configuringProvider && (
          <div className="provider-config-backdrop">
            <section className="provider-config-panel" aria-labelledby="provider-config-title">
              <div className="provider-config-header">
                <ProviderLogo provider={configuringProvider} />
                <div>
                  <p className="eyebrow">{providerCompany(configuringProvider)}</p>
                  <h3 id="provider-config-title">
                    {configuringProvider === "openrouter"
                      ? messages.providerConfigureOpenRouter
                      : messages.providerManageOllama}
                  </h3>
                </div>
                <button
                  type="button"
                  className="icon-button"
                  aria-label={messages.close}
                  onClick={() => setConfiguringProvider(null)}
                >
                  ×
                </button>
              </div>
              {configuringProvider === "openrouter" && (
                <label className="provider-config-field">
                  <span>{messages.providerApiKey}</span>
                  <input
                    type="password"
                    autoComplete="new-password"
                    value={openRouterKey}
                    placeholder={
                      configuredProvider?.status === "connected"
                        ? "••••••••••••••••••••"
                        : "sk-or-v1-…"
                    }
                    onChange={(event) => setOpenRouterKey(event.target.value)}
                  />
                  <small>{messages.providerApiKeyEditHint}</small>
                </label>
              )}
              <label className="provider-config-field">
                <span>{messages.providerModel}</span>
                <input
                  type="search"
                  value={modelQuery}
                  placeholder={messages.providerSearchModels}
                  onChange={(event) => setModelQuery(event.target.value)}
                />
                <select
                  value={providerModel}
                  disabled={modelsLoading}
                  onChange={(event) => setProviderModel(event.target.value)}
                >
                  <option value="">
                    {modelsLoading ? messages.providerModelsLoading : messages.providerChooseModel}
                  </option>
                  {visibleModels.map((model) => (
                    <option key={model.model_id} value={model.model_id}>
                      {model.display_name}
                    </option>
                  ))}
                </select>
              </label>
              {configuringProvider === "ollama" && (
                <div className="ollama-download-panel">
                  <div>
                    <strong>{messages.providerDownloadModel}</strong>
                    <small>{messages.providerDownloadModelHint}</small>
                  </div>
                  <div className="ollama-download-row">
                    <input
                      value={ollamaModelToDownload}
                      placeholder="qwen3:4b"
                      onChange={(event) => setOllamaModelToDownload(event.target.value)}
                    />
                    <button
                      type="button"
                      className="secondary-button"
                      disabled={!ollamaModelToDownload.trim() || busyProvider !== null}
                      onClick={() => void pullOllamaModel()}
                    >
                      {busyProvider === "ollama"
                        ? messages.providerDownloading
                        : messages.providerDownload}
                    </button>
                  </div>
                  {busyProvider === "ollama" && (
                    <div className="provider-operation-progress" role="progressbar">
                      <span />
                    </div>
                  )}
                </div>
              )}
              <div className="provider-config-actions">
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => setConfiguringProvider(null)}
                >
                  {messages.cancel}
                </button>
                <button
                  type="button"
                  className="primary-button"
                  disabled={!providerModel || busyProvider !== null}
                  onClick={() => void saveProviderConfiguration()}
                >
                  {messages.save}
                </button>
              </div>
            </section>
          </div>
        )}
      </section>
    </div>
  );
}

function providerAction(status: ProviderSummary["status"], messages: Messages): string {
  if (status === "not_installed") return messages.providerInstall;
  if (status === "auth_required") return messages.providerLogin;
  if (status === "key_required") return messages.providerConfigure;
  if (status === "action_required") return messages.providerConfigure;
  if (status === "installed_offline") return messages.providerStart;
  if (status === "update_required") return messages.providerUpdate;
  return messages.retry;
}

function providerActionId(status: ProviderSummary["status"]): string {
  if (status === "not_installed") return "install";
  if (status === "auth_required") return "login";
  if (status === "installed_offline") return "start";
  if (status === "update_required") return "update";
  return "manage";
}

function transportLabel(transport: ProviderSummary["transport"], messages: Messages): string {
  if (transport === "cloud_api") return messages.providerCloudApi;
  if (transport === "local_http") return messages.providerLocalModel;
  return messages.providerOfficialCli;
}

function providerDisplayName(providerId: ProviderId, fallback: string): string {
  return providerId === "claude" ? "Claude Code" : fallback;
}

function providerCompany(providerId: ProviderId): string {
  const companies: Partial<Record<ProviderId, string>> = {
    codex: "OpenAI",
    claude: "Anthropic",
    antigravity: "Google",
    openrouter: "OpenRouter",
    ollama: "Ollama",
  };
  return companies[providerId] ?? "LLM Wiki";
}

function errorMessage(reason: unknown): string {
  if (typeof reason === "string") return reason;
  if (reason && typeof reason === "object" && "message" in reason) return String(reason.message);
  return "Operazione non riuscita. Controlla la connessione e riprova.";
}
