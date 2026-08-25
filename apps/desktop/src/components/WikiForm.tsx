import { type FormEvent, useState } from "react";
import { formatAppError, type Language, type Messages } from "../i18n";
import { useDialogFocus } from "../hooks/useDialogFocus";
import type { WikiInput } from "../services/registry";

interface WikiFormProps {
  mode: "create" | "register";
  language: Language;
  messages: Messages;
  onPickFolder(): Promise<string | null>;
  onSubmit(request: WikiInput): Promise<void>;
  onClose(): void;
}

export function WikiForm({
  mode,
  language,
  messages,
  onPickFolder,
  onSubmit,
  onClose,
}: WikiFormProps) {
  const [displayName, setDisplayName] = useState("");
  const [root, setRoot] = useState("");
  const [noteLanguage, setNoteLanguage] = useState<Language>(language);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const { dialogRef, onKeyDown } = useDialogFocus(onClose);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setError(null);
    setBusy(true);
    try {
      await onSubmit({ displayName, root, noteLanguage });
    } catch (reason) {
      setError(formatAppError(reason, messages));
    } finally {
      setBusy(false);
    }
  }

  async function browse() {
    const selected = await onPickFolder();
    if (selected) setRoot(selected);
  }

  const title = mode === "create" ? messages.createWiki : messages.registerWiki;

  return (
    <div className="modal-backdrop">
      <section
        ref={dialogRef}
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="wiki-form-title"
        onKeyDown={onKeyDown}
      >
        <div className="modal-header">
          <div>
            <p className="eyebrow">{messages.appName}</p>
            <h2 id="wiki-form-title">{title}</h2>
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
        <form onSubmit={submit}>
          <label htmlFor="wiki-name">
            <span>{messages.wikiName}</span>
            <input
              id="wiki-name"
              data-initial-focus
              required
              maxLength={80}
              value={displayName}
              onChange={(event) => setDisplayName(event.target.value)}
            />
          </label>
          <div className="field-group">
            <label htmlFor="wiki-root">{messages.folder}</label>
            <div className="path-field">
              <input
                id="wiki-root"
                required
                value={root}
                onChange={(event) => setRoot(event.target.value)}
                placeholder="C:\\Users\\…"
              />
              <button type="button" className="secondary-button" onClick={browse}>
                {messages.browse}
              </button>
            </div>
            <small>{messages.pathHint}</small>
          </div>
          <label htmlFor="note-language">
            <span>{messages.noteLanguage}</span>
            <select
              id="note-language"
              value={noteLanguage}
              onChange={(event) => setNoteLanguage(event.target.value as Language)}
            >
              <option value="it">Italiano</option>
              <option value="en">English</option>
            </select>
          </label>
          {error && (
            <div className="error-banner" role="alert">
              {error}
            </div>
          )}
          <div className="modal-actions">
            <button type="button" className="text-button" onClick={onClose}>
              {messages.cancel}
            </button>
            <button type="submit" className="primary-button" disabled={busy}>
              {busy ? messages.creating : mode === "create" ? messages.create : messages.register}
            </button>
          </div>
        </form>
      </section>
    </div>
  );
}
