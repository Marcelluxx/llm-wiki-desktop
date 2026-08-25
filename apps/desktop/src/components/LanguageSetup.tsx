import type { Language, Messages } from "../i18n";

interface LanguageSetupProps {
  messages: Messages;
  onChoose(language: Language): void;
}

export function LanguageSetup({ messages, onChoose }: LanguageSetupProps) {
  return (
    <main className="welcome-shell">
      <section className="welcome-card" aria-labelledby="language-title">
        <div className="brand-mark" aria-hidden="true">
          LW
        </div>
        <p className="eyebrow">LLM Wiki Desktop</p>
        <h1 id="language-title">{messages.chooseLanguage}</h1>
        <p className="muted">{messages.chooseLanguageHint}</p>
        <div className="language-grid">
          <button type="button" className="language-option" onClick={() => onChoose("it")}>
            <span className="language-code">IT</span>
            <span>{messages.italian}</span>
          </button>
          <button type="button" className="language-option" onClick={() => onChoose("en")}>
            <span className="language-code">EN</span>
            <span>{messages.english}</span>
          </button>
        </div>
      </section>
    </main>
  );
}
