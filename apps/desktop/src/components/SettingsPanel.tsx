import { useState } from "react";
import type { PerformanceStatus, WikiRegistration } from "../contracts";
import { formatAppError, type Language, type Messages } from "../i18n";
import { useDialogFocus } from "../hooks/useDialogFocus";

interface SettingsPanelProps {
  language: Language;
  messages: Messages;
  wiki: WikiRegistration | null;
  performance: PerformanceStatus | null;
  onInstallAcceleration(): Promise<void>;
  onLanguage(language: Language): Promise<void>;
  onRename?(name: string): Promise<void>;
  onRemove?(): Promise<void>;
  onClose(): void;
}

export function SettingsPanel({
  language,
  messages,
  wiki,
  performance,
  onInstallAcceleration,
  onLanguage,
  onRename,
  onRemove,
  onClose,
}: SettingsPanelProps) {
  const [name, setName] = useState(wiki?.display_name ?? "");
  const [error, setError] = useState<string | null>(null);
  const [installingAcceleration, setInstallingAcceleration] = useState(false);
  const { dialogRef, onKeyDown } = useDialogFocus(onClose);

  async function run(action: () => Promise<void>) {
    setError(null);
    try {
      await action();
    } catch (reason) {
      setError(formatAppError(reason, messages));
    }
  }

  return (
    <div className="modal-backdrop modal-backdrop--right">
      <aside
        ref={dialogRef}
        className="settings-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
        onKeyDown={onKeyDown}
      >
        <div className="modal-header">
          <div>
            <p className="eyebrow">{messages.appName}</p>
            <h2 id="settings-title">{wiki ? messages.wikiSettings : messages.globalSettings}</h2>
          </div>
          <button
            type="button"
            className="icon-button"
            onClick={onClose}
            aria-label={messages.close}
            data-initial-focus
          >
            ×
          </button>
        </div>

        <div className="settings-section">
          <label htmlFor="interface-language">
            <span>{messages.interfaceLanguage}</span>
            <select
              id="interface-language"
              value={language}
              onChange={(event) => run(() => onLanguage(event.target.value as Language))}
            >
              <option value="it">Italiano</option>
              <option value="en">English</option>
            </select>
          </label>
        </div>

        <div className="settings-section performance-settings">
          <div className="settings-section__heading">
            <strong>{messages.performance}</strong>
            {performance?.cuda_enabled && (
              <span className="status-badge status-badge--enabled">{messages.gpuEnabled}</span>
            )}
          </div>
          {!performance ? (
            <small>{messages.gpuChecking}</small>
          ) : performance.cuda_enabled ? (
            <div className="read-only-setting">
              <span>{messages.gpuAcceleration}</span>
              <strong>{messages.active}</strong>
              {performance.device_name && <small>{performance.device_name}</small>}
            </div>
          ) : performance.nvidia_present ? (
            <div className="gpu-install-card">
              <div>
                <strong>{performance.device_name ?? "NVIDIA"}</strong>
                <small>{messages.gpuAvailableHint}</small>
              </div>
              <button
                type="button"
                className="secondary-button"
                disabled={installingAcceleration}
                onClick={() => {
                  setInstallingAcceleration(true);
                  void run(onInstallAcceleration).finally(() => setInstallingAcceleration(false));
                }}
              >
                {installingAcceleration ? messages.gpuInstalling : messages.enableGpu}
              </button>
            </div>
          ) : (
            <div className="read-only-setting">
              <span>{messages.gpuAcceleration}</span>
              <strong>{messages.cpuMode}</strong>
              <small>{messages.noNvidiaHint}</small>
            </div>
          )}
        </div>

        {wiki && onRename && onRemove && (
          <>
            <div className="settings-section">
              <div className="field-group">
                <label htmlFor="settings-wiki-name">{messages.wikiName}</label>
                <div className="path-field">
                  <input
                    id="settings-wiki-name"
                    value={name}
                    onChange={(event) => setName(event.target.value)}
                  />
                  <button
                    type="button"
                    className="secondary-button"
                    onClick={() => run(() => onRename(name))}
                  >
                    {messages.save}
                  </button>
                </div>
              </div>
              <div className="read-only-setting">
                <span>{messages.outputFolder}</span>
                <code>{wiki.canonical_root}</code>
              </div>
              <div className="read-only-setting">
                <span>{messages.aiProvider}</span>
                <strong>—</strong>
                <small>{messages.providerLater}</small>
              </div>
            </div>
            <div className="danger-zone">
              <p>{messages.removeWarning}</p>
              <button type="button" className="danger-button" onClick={() => run(onRemove)}>
                {messages.remove}
              </button>
            </div>
          </>
        )}

        {error && (
          <div className="error-banner" role="alert">
            {error}
          </div>
        )}
      </aside>
    </div>
  );
}
