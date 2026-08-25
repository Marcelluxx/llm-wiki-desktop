import { Channel, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  JobEvent,
  JobLogEntry,
  JobSummary,
  RegistrySnapshot,
  WikiRegistration,
  WikiSettings,
} from "../contracts";
import type { Language } from "../i18n";

export interface WikiInput {
  displayName: string;
  root: string;
  noteLanguage: Language;
}

export interface RegistryClient {
  getRegistry(): Promise<RegistrySnapshot>;
  setInterfaceLanguage(language: Language): Promise<RegistrySnapshot>;
  createWiki(request: WikiInput): Promise<WikiRegistration>;
  registerWiki(request: WikiInput): Promise<WikiRegistration>;
  openWiki(wikiId: string): Promise<WikiRegistration>;
  renameWiki(wikiId: string, displayName: string): Promise<WikiRegistration>;
  removeRegistration(wikiId: string): Promise<RegistrySnapshot>;
  getWikiSettings(wikiId: string): Promise<WikiSettings>;
  pickFolder(): Promise<string | null>;
  pickDocuments(): Promise<string[]>;
  listJobs(wikiId: string): Promise<JobSummary[]>;
  startImport(
    wikiId: string,
    sourcePaths: string[],
    onEvent: (event: JobEvent) => void,
  ): Promise<JobSummary>;
  cancelJob(jobId: string): Promise<void>;
  readJobLog(wikiId: string, jobId: string): Promise<JobLogEntry[]>;
}

const browserStorageKey = "llm-wiki.preview.registry.v1";

function isTauri(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

function emptySnapshot(): RegistrySnapshot {
  return { schema_version: "1.0", interface_language: null, wikis: [] };
}

function readBrowserSnapshot(): RegistrySnapshot {
  const stored = window.localStorage.getItem(browserStorageKey);
  return stored ? (JSON.parse(stored) as RegistrySnapshot) : emptySnapshot();
}

function saveBrowserSnapshot(snapshot: RegistrySnapshot): RegistrySnapshot {
  window.localStorage.setItem(browserStorageKey, JSON.stringify(snapshot));
  return snapshot;
}

function browserRegistration(request: WikiInput): WikiRegistration {
  const now = new Date().toISOString();
  return {
    schema_version: "1.0",
    wiki_id: crypto.randomUUID(),
    display_name: request.displayName.trim(),
    canonical_root: request.root,
    note_language: request.noteLanguage,
    created_at: now,
    last_opened_at: now,
  };
}

export const registryClient: RegistryClient = {
  async getRegistry() {
    return isTauri() ? invoke<RegistrySnapshot>("get_registry") : readBrowserSnapshot();
  },
  async setInterfaceLanguage(language) {
    if (isTauri()) return invoke("set_interface_language", { language });
    return saveBrowserSnapshot({ ...readBrowserSnapshot(), interface_language: language });
  },
  async createWiki(request) {
    if (isTauri()) return invoke("create_wiki", { request });
    const registration = browserRegistration(request);
    const snapshot = readBrowserSnapshot();
    saveBrowserSnapshot({ ...snapshot, wikis: [...snapshot.wikis, registration] });
    return registration;
  },
  async registerWiki(request) {
    if (isTauri()) return invoke("register_wiki", { request });
    return this.createWiki(request);
  },
  async openWiki(wikiId) {
    if (isTauri()) return invoke("open_wiki", { wikiId });
    const snapshot = readBrowserSnapshot();
    const wiki = snapshot.wikis.find((item) => item.wiki_id === wikiId);
    if (!wiki) throw new Error("Wiki not found");
    const updated = { ...wiki, last_opened_at: new Date().toISOString() };
    saveBrowserSnapshot({
      ...snapshot,
      wikis: snapshot.wikis.map((item) => (item.wiki_id === wikiId ? updated : item)),
    });
    return updated;
  },
  async renameWiki(wikiId, displayName) {
    if (isTauri()) return invoke("rename_wiki", { wikiId, displayName });
    const snapshot = readBrowserSnapshot();
    const wiki = snapshot.wikis.find((item) => item.wiki_id === wikiId);
    if (!wiki) throw new Error("Wiki not found");
    const updated = { ...wiki, display_name: displayName.trim() };
    saveBrowserSnapshot({
      ...snapshot,
      wikis: snapshot.wikis.map((item) => (item.wiki_id === wikiId ? updated : item)),
    });
    return updated;
  },
  async removeRegistration(wikiId) {
    if (isTauri()) return invoke("remove_wiki_registration", { wikiId });
    const snapshot = readBrowserSnapshot();
    return saveBrowserSnapshot({
      ...snapshot,
      wikis: snapshot.wikis.filter((item) => item.wiki_id !== wikiId),
    });
  },
  async getWikiSettings(wikiId) {
    if (isTauri()) return invoke("get_wiki_settings", { wikiId });
    const wiki = readBrowserSnapshot().wikis.find((item) => item.wiki_id === wikiId);
    if (!wiki) throw new Error("Wiki not found");
    return {
      schema_version: "1.0",
      wiki_id: wiki.wiki_id,
      output_root: wiki.canonical_root,
      note_language: wiki.note_language,
      provider_id: "fake",
      ocr_language: "ita+eng",
      open_in_obsidian_after_publish: false,
    };
  },
  async pickFolder() {
    if (!isTauri()) return null;
    const selected = await open({ directory: true, multiple: false });
    return typeof selected === "string" ? selected : null;
  },
  async pickDocuments() {
    if (!isTauri()) return [];
    const selected = await open({
      directory: false,
      multiple: true,
      filters: [{ name: "Documenti", extensions: ["pdf", "docx", "txt", "md"] }],
    });
    if (Array.isArray(selected)) return selected;
    return typeof selected === "string" ? [selected] : [];
  },
  async listJobs(wikiId) {
    if (isTauri()) return invoke("list_jobs", { wikiId });
    return [];
  },
  async startImport(wikiId, sourcePaths, onEvent) {
    if (isTauri()) {
      const channel = new Channel<JobEvent>();
      channel.onmessage = onEvent;
      return invoke("start_import", { wikiId, sourcePaths, onEvent: channel });
    }
    const now = new Date().toISOString();
    const job: JobSummary = {
      schema_version: "1.0",
      job_id: crypto.randomUUID(),
      wiki_id: wikiId,
      state: "queued",
      stage_progress: 0,
      source_count: sourcePaths.length,
      created_at: now,
      updated_at: now,
      last_message: "stage.queued",
    };
    window.setTimeout(
      () =>
        onEvent({
          job_id: job.job_id,
          state: "completed",
          progress: 1,
          message: "stage.completed",
        }),
      700,
    );
    return job;
  },
  async cancelJob(jobId) {
    if (isTauri()) await invoke("cancel_job", { jobId });
  },
  async readJobLog(wikiId, jobId) {
    if (isTauri()) return invoke("read_job_log", { wikiId, jobId });
    return [];
  },
};
