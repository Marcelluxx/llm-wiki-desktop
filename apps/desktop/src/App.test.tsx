import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { App } from "./App";
import type { RegistrySnapshot, WikiRegistration } from "./contracts";
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
    createWiki:
      createError !== undefined
        ? vi.fn().mockRejectedValue(createError)
        : vi.fn().mockResolvedValue(wiki),
    registerWiki: vi.fn().mockResolvedValue(wiki),
    openWiki: vi.fn().mockResolvedValue(wiki),
    renameWiki: vi.fn().mockResolvedValue(wiki),
    removeRegistration: vi.fn().mockResolvedValue(snapshot()),
    getWikiSettings: vi.fn(),
    pickFolder: vi.fn().mockResolvedValue(null),
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
