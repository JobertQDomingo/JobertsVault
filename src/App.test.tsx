import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import App from "./App";

test("shows the app name on a glass panel, never on the aurora", () => {
  render(<App />);
  const title = screen.getByRole("heading", { name: "Jobert's Vault" });
  expect(title.closest(".glass")).toBeTruthy();
});
