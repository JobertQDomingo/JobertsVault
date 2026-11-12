import { invoke } from "@tauri-apps/api/core";
import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { beforeEach, expect, test, vi } from "vitest";
import { createQueryClient } from "@/lib/queryClient";
import { queryKeys } from "@/lib/queryKeys";
import { routes } from "@/routes";
import { AppShell } from "./AppShell";
import { QuickAddProvider, useQuickAdd } from "./QuickAddProvider";
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

test("the sidebar lists the screens built so far and marks the current one", () => {
  renderAt("/");
  const nav = screen.getByRole("navigation");
  expect([...nav.querySelectorAll("a")].map((a) => a.textContent)).toEqual([
    "Dashboard",
    "Transactions",
    "Accounts",
    "Reports",
    "Recurring",
    "Budgets",
    "Goals",
    "Settings",
  ]);
  expect(link("Dashboard").getAttribute("aria-current")).toBe("page");
  expect(link("Accounts").getAttribute("aria-current")).toBeNull();
  expect(screen.getByText("Jobert's Vault")).toBeTruthy();
});

test.each(["Recurring", "Budgets", "Goals"])(
  "the %s screen is marked current and has no shortcut (UI_FLOWS §10)",
  (name) => {
    renderAt(`/${name.toLowerCase()}`);
    expect(link(name).getAttribute("aria-current")).toBe("page");
    expect(link("Dashboard").getAttribute("aria-current")).toBeNull();
    expect(link(name).getAttribute("aria-keyshortcuts")).toBeNull();
    expect(link(name).hasAttribute("title")).toBe(false);
  },
);

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

const press = (key: string, mods: Partial<KeyboardEventInit> = {}) =>
  fireEvent.keyDown(window, { key, ...mods });

test("Ctrl+N opens quick-add on any screen, instead of a new window", async () => {
  renderAt("/settings");
  expect(press("n", { ctrlKey: true })).toBe(false); // the browser's default was prevented
  expect(
    await screen.findByRole("dialog", { name: "Add transaction" }),
  ).toBeTruthy();
});

test("Ctrl+N works with Caps Lock on, but Ctrl+Shift+N isn't it", async () => {
  renderAt("/");
  press("N", { ctrlKey: true, shiftKey: true });
  expect(screen.queryByRole("dialog")).toBeNull();
  press("N", { ctrlKey: true });
  expect(await screen.findByRole("dialog")).toBeTruthy();
});

test("Ctrl+1–4 and Ctrl+, go to the screens", async () => {
  renderAt("/");
  for (const [key, name] of [
    ["2", "Transactions"],
    ["3", "Accounts"],
    ["4", "Reports"],
    [",", "Settings"],
    ["1", "Dashboard"],
  ]) {
    press(key, { ctrlKey: true });
    await waitFor(() => expect(title()).toBe(name));
  }
});

test("with a dialog open, shortcuts don't act behind it", async () => {
  renderAt("/");
  press("n", { ctrlKey: true });
  await screen.findByRole("dialog");
  expect(press("3", { ctrlKey: true })).toBe(false);
  expect(router.state.location.pathname).toBe("/");
});

test("Ctrl+F focuses the screen's search box", () => {
  vi.mocked(invoke).mockResolvedValue(null);
  router = createMemoryRouter(
    [
      {
        element: <AppShell />,
        children: [
          {
            index: true,
            element: <input type="search" aria-label="Search" />,
          },
        ],
      },
    ],
    { initialEntries: ["/"] },
  );
  const client = createQueryClient();
  client.setQueryData(queryKeys.settings, SETTINGS);
  render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  expect(press("f", { ctrlKey: true })).toBe(false);
  expect(document.activeElement).toBe(screen.getByRole("searchbox"));
});

test("each screen's link names its shortcut", () => {
  renderAt("/");
  expect(link("Transactions").getAttribute("aria-keyshortcuts")).toBe(
    "Control+2",
  );
  expect(link("Settings").title).toBe("Settings (Ctrl+,)");
  expect(link("Reports").getAttribute("aria-keyshortcuts")).toBe("Control+4");
});

test("shortcuts also wait while a menu or an alert dialog is open", () => {
  renderAt("/");
  for (const role of ["menu", "alertdialog"]) {
    const overlay = document.body.appendChild(document.createElement("div"));
    overlay.setAttribute("role", role);
    press("3", { ctrlKey: true });
    overlay.remove();
  }
  expect(router.state.location.pathname).toBe("/");
});

test("Ctrl+1–4 follow the key's place, so other keyboard layouts work", async () => {
  renderAt("/settings");
  // On AZERTY the 1 key types "&", and the 4 key "'".
  press("&", { ctrlKey: true, code: "Digit1" });
  await waitFor(() => expect(title()).toBe("Dashboard"));
  press("'", { ctrlKey: true, code: "Digit4" });
  await waitFor(() => expect(title()).toBe("Reports"));
});

test("Alt+← and Alt+→ never go Back or Forward, even on screens without a period", () => {
  renderAt("/settings");
  for (const key of ["ArrowLeft", "ArrowRight", "Home"])
    expect(press(key, { altKey: true })).toBe(false);
});

test("editing opens the dialog as Edit transaction, whatever the id", async () => {
  function EditZero() {
    const { edit } = useQuickAdd();
    return <button onClick={() => edit(0)}>Edit</button>;
  }
  vi.mocked(invoke).mockReturnValue(new Promise(() => {}));
  render(
    <QueryClientProvider client={createQueryClient()}>
      <QuickAddProvider>
        <EditZero />
      </QuickAddProvider>
    </QueryClientProvider>,
  );
  fireEvent.click(screen.getByRole("button", { name: "Edit" }));
  expect(
    await screen.findByRole("dialog", { name: "Edit transaction" }),
  ).toBeTruthy();
});
