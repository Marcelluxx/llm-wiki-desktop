import type { ProviderId } from "../contracts";

const providerAssets: Partial<Record<ProviderId, { src: string; label: string }>> = {
  codex: { src: "https://openrouter.ai/images/icons/OpenAI.svg", label: "OpenAI Codex" },
  claude: { src: "https://openrouter.ai/images/icons/Anthropic.svg", label: "Anthropic Claude" },
  antigravity: {
    src: "https://antigravity.google/assets/image/antigravity-logo.png",
    label: "Google Antigravity",
  },
  openrouter: { src: "https://openrouter.ai/favicon/glyph.png", label: "OpenRouter" },
  ollama: { src: "https://ollama.com/public/icon-64x64.png", label: "Ollama" },
};

export function ProviderLogo({ provider }: { provider: ProviderId }) {
  const asset = providerAssets[provider] ?? providerAssets.codex;
  return (
    <span className={`provider-logo provider-logo--${provider}`} title={asset?.label}>
      <img src={asset?.src} alt={asset?.label ?? provider} draggable={false} />
    </span>
  );
}
