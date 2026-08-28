import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";
import type { JobSummary, RegistrySnapshot, WikiRegistration } from "./contracts";
import type { RegistryClient } from "./services/registry";

const wiki: WikiRegistration = {
  schema_version: "1.0",
  wiki_id: "wiki-test",
  display_name: "Ricerca personale",
  canonical_root: "C:\\Synthetic\\Ricerca",
  note_language: "it",
  created_at: "2026-08-25T08:00:00Z",
  last_opened_at: "2026-08-25T08:00:00Z",
};

const queuedJob: JobSummary = {
  schema_version: "1.0",
  job_id: "job-test",
  wiki_id: wiki.wiki_id,
  state: "queued",
  stage_progress: 0,
  source_count: 2,
  created_at: "2026-08-25T08:10:00Z",
  updated_at: "2026-08-25T08:10:00Z",
  last_message: "stage.queued",
};

function snapshot(
  wikis: WikiRegistration[] = [],
  language: "it" | "en" | null = "it",
): RegistrySnapshot {
  return { schema_version: "1.0", interface_language: language, wikis };
}

function fakeClient(initial: RegistrySnapshot, createError?: unknown): RegistryClient {
  return {
    getRegistry: vi.fn().mockResolvedValue(initial),
    setInterfaceLanguage: vi
      .fn()
      .mockImplementation(async (language) => ({ ...initial, interface_language: language })),
    setSelectedProvider: vi
      .fn()
      .mockImplementation(async (providerId) => ({ ...initial, selected_provider_id: providerId })),
    createWiki:
      createError !== undefined
        ? vi.fn().mockRejectedValue(createError)
        : vi.fn().mockResolvedValue(wiki),
    registerWiki: vi.fn().mockResolvedValue(wiki),
    openWiki: vi.fn().mockResolvedValue(wiki),
    renameWiki: vi.fn().mockResolvedValue(wiki),
    removeRegistration: vi.fn().mockResolvedValue(snapshot()),
    getWikiSettings: vi.fn(),
    getPerformanceStatus: vi.fn().mockResolvedValue({
      nvidia_present: false,
      cuda_enabled: false,
      device_name: null,
    }),
    listProviderStatuses: vi.fn().mockResolvedValue([]),
    runProviderAction: vi.fn().mockResolvedValue(undefined),
    listProviderModels: vi.fn().mockResolvedValue([]),
    configureOpenRouter: vi.fn().mockResolvedValue(undefined),
    configureOllama: vi.fn().mockResolvedValue(undefined),
    pullOllamaModel: vi.fn().mockResolvedValue(undefined),
    installNvidiaAcceleration: vi.fn(),
    pickFolder: vi.fn().mockResolvedValue(null),
    pickDocuments: vi.fn().mockResolvedValue([]),
    listJobs: vi.fn().mockResolvedValue([]),
    startImport: vi.fn(),
    cancelJob: vi.fn(),
    readJobLog: vi.fn().mockResolvedValue([]),
    listChatMessages: vi.fn().mockResolvedValue([]),
    sendChatMessage: vi.fn(),
    startWikiIngest: vi.fn(),
  };
}

describe("App", () => {
  it("shows first-run language choice", async () => {
    render(<App client={fakeClient(snapshot([], null))} />);

    expect(await screen.findByRole("heading", { name: "Scegli la lingua" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /English/ })).toBeInTheDocument();
  });

  it("shows a helpful empty wiki dashboard", async () => {
    render(<App client={fakeClient(snapshot())} />);

    expect(await screen.findByRole("heading", { name: "Le tue wiki" })).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Crea il tuo primo spazio di conoscenza" }),
    ).toBeInTheDocument();
  });

  it("opens a registered wiki", async () => {
    render(<App client={fakeClient(snapshot([wiki]))} />);

    fireEvent.click(await screen.findByRole("button", { name: /Apri/ }));

    expect(await screen.findByText("Questa wiki è pronta")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Ricerca personale" })).toBeInTheDocument();
  });

  it("inherits the global provider and exposes wiki chat and ingest", async () => {
    const client = fakeClient({ ...snapshot([wiki]), selected_provider_id: "codex" });
    vi.mocked(client.listProviderStatuses).mockResolvedValue([
      {
        provider_id: "codex",
        display_name: "Codex",
        transport: "cli",
        status: "connected",
        capabilities: ["structured_output"],
      },
    ]);
    vi.mocked(client.listJobs).mockResolvedValue([{ ...queuedJob, state: "completed" }]);
    vi.mocked(client.sendChatMessage).mockResolvedValue({
      message_id: "assistant-1",
      provider_id: "codex",
      role: "assistant",
      content: "Risposta basata sulla wiki",
      created_at: "2026-08-27T10:00:00Z",
    });
    vi.mocked(client.startWikiIngest).mockResolvedValue({
      message_id: "ingest-1",
      provider_id: "codex",
      role: "assistant",
      content: "Ingestione completata",
      created_at: "2026-08-27T10:01:00Z",
    });
    render(<App client={client} />);

    fireEvent.click(await screen.findByRole("button", { name: /Apri/ }));
    expect(
      await screen.findByRole("heading", { name: "Chat con la tua conoscenza" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Codex/ })).toBeInTheDocument();
    const expandChat = screen.getByRole("button", { name: /Espandi chat/ });
    fireEvent.click(expandChat);
    expect(expandChat.closest("aside")).toHaveClass("wiki-chat--expanded");
    expect(screen.getByRole("button", { name: /Riduci chat/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    fireEvent.click(screen.getByRole("button", { name: /Riduci chat/ }));

    fireEvent.change(screen.getByPlaceholderText("Chiedi qualcosa sui documenti della wiki…"), {
      target: { value: "Riassumi i concetti principali" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Invia messaggio" }));
    await waitFor(() =>
      expect(client.sendChatMessage).toHaveBeenCalledWith(
        wiki.wiki_id,
        "Riassumi i concetti principali",
        expect.any(Function),
      ),
    );
    expect(await screen.findByText("Risposta basata sulla wiki")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Ingest" }));
    await waitFor(() =>
      expect(client.startWikiIngest).toHaveBeenCalledWith(wiki.wiki_id, expect.any(Function)),
    );
  });

  it("shows the provider failure reason and automatically opens diagnostics", async () => {
    const client = fakeClient({ ...snapshot([wiki]), selected_provider_id: "antigravity" });
    vi.mocked(client.listProviderStatuses).mockResolvedValue([
      {
        provider_id: "antigravity",
        display_name: "Antigravity",
        transport: "cli",
        status: "connected",
        capabilities: ["structured_output"],
      },
    ]);
    vi.mocked(client.sendChatMessage).mockImplementation(async (_wikiId, _message, onEvent) => {
      onEvent({
        provider_id: "antigravity",
        kind: "error",
        message: "[provider_cli_incompatible] Il protocollo della CLI non è compatibile.",
      });
      throw {
        code: "provider_cli_incompatible",
        message: "Antigravity non ha completato la richiesta: protocollo CLI incompatibile.",
      };
    });
    render(<App client={client} />);

    fireEvent.click(await screen.findByRole("button", { name: /Apri/ }));
    fireEvent.change(
      await screen.findByPlaceholderText("Chiedi qualcosa sui documenti della wiki…"),
      {
        target: { value: "Test collegamento" },
      },
    );
    fireEvent.click(screen.getByRole("button", { name: "Invia messaggio" }));

    expect(
      await screen.findByText(
        "Antigravity non ha completato la richiesta: protocollo CLI incompatibile.",
      ),
    ).toBeInTheDocument();
    expect(await screen.findByText(/\[provider_cli_incompatible\]/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Nascondi flusso CLI/ })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  });

  it("shows the provider badge and opens the provider command center", async () => {
    const client = fakeClient(snapshot([wiki]));
    vi.mocked(client.listProviderStatuses).mockResolvedValue([
      {
        provider_id: "codex",
        display_name: "Codex",
        transport: "cli",
        status: "connected",
        version: "codex 1.0",
        capabilities: ["login", "models"],
      },
      {
        provider_id: "claude",
        display_name: "Claude",
        transport: "cli",
        status: "not_installed",
        capabilities: ["install", "login"],
      },
    ]);
    render(<App client={client} />);

    fireEvent.click(await screen.findByRole("button", { name: /Provider AI/ }));

    expect(await screen.findByRole("dialog", { name: "Provider AI" })).toBeInTheDocument();
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    expect(screen.getByText("Da installare")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Installa" })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: "Usa questo provider" }));
    await waitFor(() => expect(client.setSelectedProvider).toHaveBeenCalledWith("codex"));

    fireEvent.click(screen.getByRole("button", { name: "Aggiorna stato" }));
    await waitFor(() => expect(client.listProviderStatuses).toHaveBeenCalledWith(true));
  });

  it("selects supported documents and starts a visible import", async () => {
    const client = fakeClient(snapshot([wiki]));
    vi.mocked(client.pickDocuments).mockResolvedValue([
      "C:\\Synthetic\\manuale.pdf",
      "C:\\Synthetic\\note.md",
    ]);
    vi.mocked(client.startImport).mockImplementation(async (_wikiId, _paths, onEvent) => {
      setTimeout(() => {
        onEvent({
          job_id: queuedJob.job_id,
          state: "extracting",
          progress: 0.72,
          message: "ocr.working",
          log_level: "info",
          source: "manuale.pdf",
          detail: "document=1/2 page=3/15 elapsed=00:20 cpu=31.2% memory=2048MB",
        });
      }, 0);
      return queuedJob;
    });
    render(<App client={client} />);

    fireEvent.click(await screen.findByRole("button", { name: /Apri/ }));
    fireEvent.click(await screen.findByRole("button", { name: "Aggiungi documenti" }));

    expect(await screen.findByText("2 documenti selezionati")).toBeInTheDocument();
    expect(screen.getAllByText("manuale.pdf").length).toBeGreaterThan(0);
    expect(screen.getByText("note.md")).toBeInTheDocument();
    expect(screen.getByText("PDF")).toBeInTheDocument();
    expect(client.startImport).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Avvia importazione (2)" }));
    await waitFor(() => expect(screen.getByRole("progressbar")).toHaveValue(0.72));
    expect((await screen.findAllByText("OCR attivo")).length).toBeGreaterThan(0);
    expect(await screen.findByText(/pagina 3\/15/)).toBeInTheDocument();
    expect(client.startImport).toHaveBeenCalledWith(
      wiki.wiki_id,
      ["C:\\Synthetic\\manuale.pdf", "C:\\Synthetic\\note.md"],
      expect.any(Function),
    );
  });

  it("adds multiple selections to one queue, deduplicates, and removes individual files", async () => {
    const client = fakeClient(snapshot([wiki]));
    vi.mocked(client.pickDocuments)
      .mockResolvedValueOnce(["C:\\Synthetic\\uno.pdf"])
      .mockResolvedValueOnce(["C:\\Synthetic\\uno.pdf", "C:\\Synthetic\\due.pdf"]);
    render(<App client={client} />);

    fireEvent.click(await screen.findByRole("button", { name: /Apri/ }));
    fireEvent.click(await screen.findByRole("button", { name: "Aggiungi documenti" }));
    expect(await screen.findByText("1 documenti selezionati")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Aggiungi documenti" }));

    expect(await screen.findByText("2 documenti selezionati")).toBeInTheDocument();
    expect(screen.getByText("uno.pdf")).toBeInTheDocument();
    expect(screen.getByText("due.pdf")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Rimuovi documento: uno.pdf" }));
    expect(screen.queryByText("uno.pdf")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Avvia importazione (1)" })).toBeEnabled();
  });

  it("stops an import immediately and allows another selection", async () => {
    const client = fakeClient(snapshot([wiki]));
    vi.mocked(client.listJobs).mockResolvedValue([queuedJob]);
    vi.mocked(client.pickDocuments).mockResolvedValue(["C:\\Synthetic\\nuovo.pdf"]);
    vi.mocked(client.startImport).mockResolvedValue({ ...queuedJob, job_id: "job-new" });
    render(<App client={client} />);

    fireEvent.click(await screen.findByRole("button", { name: /Apri/ }));
    fireEvent.click(await screen.findByRole("button", { name: "Interrompi importazione" }));

    await waitFor(() => expect(client.cancelJob).toHaveBeenCalledWith(queuedJob.job_id));
    expect(screen.getByRole("button", { name: "Aggiungi documenti" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Aggiungi documenti" }));
    fireEvent.click(await screen.findByRole("button", { name: "Avvia importazione (1)" }));
    await waitFor(() => expect(client.startImport).toHaveBeenCalled());
  });

  it("shows optional NVIDIA acceleration and enables it from settings", async () => {
    const client = fakeClient(snapshot());
    vi.mocked(client.getPerformanceStatus).mockResolvedValue({
      nvidia_present: true,
      cuda_enabled: false,
      device_name: "NVIDIA Test GPU",
    });
    vi.mocked(client.installNvidiaAcceleration).mockResolvedValue({
      nvidia_present: true,
      cuda_enabled: true,
      device_name: "NVIDIA Test GPU",
    });
    render(<App client={client} />);

    fireEvent.click(await screen.findByRole("button", { name: "Impostazioni" }));
    expect(await screen.findByText("NVIDIA Test GPU")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Scarica e abilita CUDA" }));

    await waitFor(() => expect(client.installNvidiaAcceleration).toHaveBeenCalledOnce());
    expect(await screen.findByText("GPU attiva")).toBeInTheDocument();
  });

  it("moves keyboard focus into dialogs and closes them with Escape", async () => {
    render(<App client={fakeClient(snapshot())} />);

    fireEvent.click((await screen.findAllByRole("button", { name: "Crea una wiki" }))[0]);

    expect(screen.getByLabelText("Nome della wiki")).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it.each([
    ["forbidden_root", "Seleziona una cartella dedicata"],
    ["duplicate_or_nested_path", "Questa cartella appartiene già a un’altra wiki"],
    ["missing_path", "La cartella selezionata non esiste"],
    ["io", "Windows non consente di usare questa cartella"],
  ])("localizes registry error %s without losing the form", async (code, message) => {
    render(<App client={fakeClient(snapshot(), { code, message: "backend detail" })} />);

    fireEvent.click((await screen.findAllByRole("button", { name: "Crea una wiki" }))[0]);
    fireEvent.change(screen.getByLabelText("Nome della wiki"), { target: { value: "Test" } });
    fireEvent.change(screen.getByLabelText("Cartella"), {
      target: { value: "C:\\Synthetic\\Test" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Crea wiki" }));

    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent(message));
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });
});
