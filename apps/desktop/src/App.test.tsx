import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("App", () => {
  it("renders the foundation status", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "LLM Wiki Desktop" })).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Foundation workspace ready");
  });
});
