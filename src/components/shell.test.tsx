import { invoke } from "@tauri-apps/api/core";
import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { beforeEach, expect, test, vi } from "vitest";
import { createQueryClient } from "@/lib/queryClient";
import { queryKeys } from "@/lib/queryKeys";
import { routes } from "@/routes";
import { SETTINGS } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

let router: ReturnType<typeof createMemoryRouter>;

function renderAt(url: string) {
  vi.mocked(invoke).mockResolvedValue(null);
  const client = createQueryClient();
  client.setQueryData(queryKeys.settings, SETTINGS);
  router = createMemoryRouter(routes, { initialEntries: [url] });
  render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
}

const link = (name: string) => screen.getByRole("link", { name });
const title = () => screen.getByRole("heading", { level: 1 }).textContent;

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

test("the sidebar lists the Phase 1 screens and marks the current one", () => {
  renderAt("/");
  const nav = screen.getByRole("navigation");
  expect([...nav.querySelectorAll("a")].map((a) => a.textContent)).toEqual([
    "Dashboard",
    "Transactions",
    "Accounts",
    "Settings",
  ]);
  expect(link("Dashboard").getAttribute("aria-current")).toBe("page");
  expect(link("Accounts").getAttribute("aria-current")).toBeNull();
  expect(screen.getByText("Jobert's Vault")).toBeTruthy();
});

test("an account's details page highlights Accounts, not the dashboard", () => {
  renderAt("/accounts/5");
  expect(link("Accounts").getAttribute("aria-current")).toBe("page");
  expect(link("Dashboard").getAttribute("aria-current")).toBeNull();
});

test("the top bar title follows the route", async () => {
  renderAt("/transactions");
  expect(title()).toBe("Transactions");
  fireEvent.click(link("Settings"));
  await waitFor(() => expect(title()).toBe("Settings"));
  expect(link("Settings").getAttribute("aria-current")).toBe("page");
});

test("+ Add opens the Add transaction dialog, and Esc closes it", async () => {
  renderAt("/accounts");
  fireEvent.click(screen.getByRole("button", { name: /^Add/ }));
  const dialog = await screen.findByRole("dialog", { name: "Add transaction" });
  fireEvent.keyDown(dialog, { key: "Escape" });
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
});

test("an unknown path goes to the dashboard", async () => {
  renderAt("/nowhere");
  await waitFor(() => expect(title()).toBe("Dashboard"));
  expect(router.state.location.pathname).toBe("/");
});

test("screens with a period show the period bar; the others don't", () => {
  renderAt("/");
  expect(screen.getByRole("radiogroup", { name: "Period" })).toBeTruthy();
  fireEvent.click(link("Accounts"));
  expect(screen.queryByRole("radiogroup", { name: "Period" })).toBeNull();
});

test("screens not built yet render without router warnings", () => {
  const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
  renderAt("/settings");
  expect(warn).not.toHaveBeenCalled();
  warn.mockRestore();
});
