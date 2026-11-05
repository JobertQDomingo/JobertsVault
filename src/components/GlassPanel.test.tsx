import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { GlassPanel } from "./GlassPanel";

test("a panel is Level 1 glass", () => {
  render(<GlassPanel>Income</GlassPanel>);
  const classes = screen.getByText("Income").className.split(" ");
  expect(classes).toContain("glass");
  expect(classes).toContain("rounded-panel");
  expect(classes).not.toContain("glass-raised");
});

test("raised is Level 2 glass", () => {
  render(<GlassPanel raised>Menu</GlassPanel>);
  const classes = screen.getByText("Menu").className.split(" ");
  expect(classes).toContain("glass-raised");
  expect(classes).toContain("rounded-dialog");
  expect(classes).not.toContain("glass");
});

test("a caller's radius wins", () => {
  render(<GlassPanel className="rounded-toolbar">Period</GlassPanel>);
  const classes = screen.getByText("Period").className.split(" ");
  expect(classes).toContain("rounded-toolbar");
  expect(classes).not.toContain("rounded-panel");
});

test("passes props through", () => {
  render(<GlassPanel role="region" aria-label="Accounts" />);
  expect(screen.getByRole("region", { name: "Accounts" })).toBeTruthy();
});
