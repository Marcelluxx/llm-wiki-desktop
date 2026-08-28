import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MarkdownContent } from "./MarkdownContent";

describe("MarkdownContent", () => {
  it("renders headings, emphasis, lists, tables, and code as formatted Markdown", () => {
    render(
      <MarkdownContent
        content={`# Sintesi\n\nTesto **importante**.\n\n- Primo\n- Secondo\n\n| Campo | Valore |\n| --- | --- |\n| Fonte | PDF |\n\n\`codice\``}
      />,
    );

    expect(screen.getByRole("heading", { name: "Sintesi" })).toBeInTheDocument();
    expect(screen.getByText("importante").tagName).toBe("STRONG");
    expect(screen.getByText("Primo").closest("li")).not.toBeNull();
    expect(screen.getByRole("table")).toBeInTheDocument();
    expect(screen.getByText("codice").tagName).toBe("CODE");
  });

  it("does not expose local or unsafe Markdown links as clickable navigation", () => {
    render(
      <MarkdownContent
        content={"[Nota locale](file:///C:/private/wiki.md) e [azione](javascript:alert(1))"}
      />,
    );

    expect(screen.getByText("Nota locale")).toHaveClass("markdown-local-reference");
    expect(screen.getByText("azione")).toHaveClass("markdown-local-reference");
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
  });
});
