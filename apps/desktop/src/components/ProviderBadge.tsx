import type { ProviderSummary } from "../contracts";
import type { Messages } from "../i18n";
import { ProviderLogo } from "./ProviderLogo";

interface ProviderBadgeProps {
  provider: ProviderSummary | null;
  messages: Messages;
  onClick(): void;
}

export function ProviderBadge({ provider, messages, onClick }: ProviderBadgeProps) {
  const status = provider
    ? `${messages.providerSelected} · ${providerStatus(provider.status, messages)}`
    : messages.providerChoose;
  return (
    <button
      type="button"
      className="provider-badge"
      onClick={onClick}
      aria-label={`${provider?.display_name ?? messages.aiProvider} · ${status}`}
    >
      {provider ? (
        <ProviderLogo provider={provider.provider_id} />
      ) : (
        <span className="status-dot" />
      )}
      <span>
        <strong>{provider?.display_name ?? messages.providerChoose}</strong>
        <small>{status}</small>
      </span>
      <span aria-hidden="true">⌄</span>
    </button>
  );
}

export function providerStatus(status: ProviderSummary["status"], messages: Messages): string {
  const values: Record<ProviderSummary["status"], string> = {
    checking: messages.providerChecking,
    connected: messages.providerConnected,
    not_installed: messages.providerNotInstalled,
    auth_required: messages.providerAuthRequired,
    key_required: messages.providerKeyRequired,
    installed_offline: messages.providerInstalledOffline,
    update_required: messages.providerUpdateRequired,
    action_required: messages.providerActionRequired,
    unavailable: messages.providerUnavailable,
  };
  return values[status];
}
