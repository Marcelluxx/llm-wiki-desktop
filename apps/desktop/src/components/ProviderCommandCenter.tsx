import type { ProviderSummary } from "../contracts";
import type { Messages } from "../i18n";
import { useDialogFocus } from "../hooks/useDialogFocus";
import { ProviderLogo } from "./ProviderLogo";
import { providerStatus } from "./ProviderBadge";

interface ProviderCommandCenterProps {
  providers: ProviderSummary[];
  messages: Messages;
  onRefresh(): Promise<void>;
  onClose(): void;
}

export function ProviderCommandCenter({
  providers,
  messages,
  onRefresh,
  onClose,
}: ProviderCommandCenterProps) {
  const { dialogRef, onKeyDown } = useDialogFocus(onClose);
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
        <div className="modal-header">
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
        <div className="provider-list">
          {providers.map((provider) => (
            <article className="provider-card" key={provider.provider_id}>
              <ProviderLogo provider={provider.provider_id} />
              <div className="provider-card__copy">
                <h3>{provider.display_name}</h3>
                <p>{transportLabel(provider.transport, messages)}</p>
                {provider.version && <small>{provider.version}</small>}
              </div>
              <div className="provider-card__actions">
                <span className={`provider-state provider-state--${provider.status}`}>
                  {providerStatus(provider.status, messages)}
                </span>
                <button type="button" className="secondary-button">
                  {providerAction(provider.status, messages)}
                </button>
              </div>
            </article>
          ))}
        </div>
        <footer className="provider-center-footer">
          <p>{messages.providerSecurityHint}</p>
          <button type="button" className="text-button" onClick={() => void onRefresh()}>
            {messages.providerRefresh}
          </button>
        </footer>
      </section>
    </div>
  );
}

function providerAction(status: ProviderSummary["status"], messages: Messages): string {
  if (status === "not_installed") return messages.providerInstall;
  if (status === "auth_required") return messages.providerLogin;
  if (status === "key_required") return messages.providerConfigure;
  if (status === "installed_offline") return messages.providerStart;
  if (status === "update_required") return messages.providerUpdate;
  if (status === "connected") return messages.providerManage;
  return messages.retry;
}

function transportLabel(transport: ProviderSummary["transport"], messages: Messages): string {
  if (transport === "cloud_api") return messages.providerCloudApi;
  if (transport === "local_http") return messages.providerLocalModel;
  return messages.providerOfficialCli;
}
