import type { ProviderId } from "../contracts";

export function ProviderLogo({ provider }: { provider: ProviderId }) {
  if (provider === "claude") {
    return <span className="provider-logo provider-logo--claude">✳</span>;
  }
  if (provider === "antigravity") {
    return (
      <span className="provider-logo provider-logo--antigravity" aria-hidden="true">
        A
      </span>
    );
  }
  if (provider === "openrouter") {
    return (
      <span className="provider-logo provider-logo--openrouter" aria-hidden="true">
        ⇄
      </span>
    );
  }
  if (provider === "ollama") {
    return (
      <span className="provider-logo provider-logo--ollama" aria-hidden="true">
        ♙
      </span>
    );
  }
  return (
    <span className="provider-logo provider-logo--codex" aria-hidden="true">
      ◉
    </span>
  );
}
