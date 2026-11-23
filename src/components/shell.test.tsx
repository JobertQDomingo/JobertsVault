import { invoke } from "@tauri-apps/api/core";
import { QueryClientProvider } from "@tanstack/react-query";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { beforeEach, describe, expect, test, vi } from "vitest";
import { createQueryClient } from "@/lib/queryClient";
import { queryKeys } from "@/lib/queryKeys";
import { SHORTCUTS } from "@/lib/shortcuts";
import { routes } from "@/routes";
import { AppShell } from "./AppShell";
import { QuickAddProvider, useQuickAdd } from "./QuickAddProvider";
import { CHECKIN_STATUS } from "@/test/fixtures";
import { renderRoute, SETTINGS } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

let router: ReturnType<typeof createMemoryRouter>;

function renderAt(url: string) {
  vi.mocked(invoke).mockResolvedValue(null);
  const client = createQueryClient();
  client.setQueryData(queryKeys.settings, SETTINGS);
  client.setQueryData(queryKeys.checkin, null);
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
    "Debt-free",
    "Forecast",
    "Settings",
  ]);
  // The level chip beside the app name is the only way to /level (UI_FLOWS item 101).
  expect(within(nav).queryByRole("link", { name: /level/i })).toBeNull();
  expect(link("Dashboard").getAttribute("aria-current")).toBe("page");
  expect(link("Accounts").getAttribute("aria-current")).toBeNull();
  expect(screen.getByText("Jobert's Vault")).toBeTruthy();
});

test.each(["Recurring", "Budgets", "Goals", "Debt-free", "Forecast"])(
  "the %s screen is marked current and has no shortcut (UI_FLOWS §10, §11 item 19)",
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

test("Skip to content comes first and moves focus past the sidebar to the screen (A11Y-09)", () => {
  renderAt("/transactions");
  const skip = screen.getByRole("link", { name: "Skip to content" });
  const first = document.querySelector<HTMLElement>("a[href], button");
  expect(first).toBe(skip);
  fireEvent.click(skip);
  expect(document.activeElement).toBe(screen.getByRole("main"));
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

test("? shows every shortcut of UI_FLOWS §10, on any screen and any layout", async () => {
  renderAt("/settings");
  expect(press("?", { shiftKey: true })).toBe(false);
  const dialog = await screen.findByRole("dialog", {
    name: "Keyboard shortcuts",
  });
  expect(dialog.querySelectorAll("tbody tr")).toHaveLength(SHORTCUTS.length);
  expect(within(dialog).getAllByText("Alt+←")).toHaveLength(2);
  expect(within(dialog).getByText(/^While a check-in is due/)).toBeTruthy();
  fireEvent.keyDown(dialog, { key: "Escape" });
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  // Some layouts type ? without Shift.
  press("?");
  expect(await screen.findByRole("dialog")).toBeTruthy();
});

test("? opens the shortcuts where it's on a number key (Canadian French Shift+6)", async () => {
  renderAt("/");
  expect(press("?", { shiftKey: true, code: "Digit6" })).toBe(false);
  expect(
    await screen.findByRole("dialog", { name: "Keyboard shortcuts" }),
  ).toBeTruthy();
});

test("? types into a field instead of opening the shortcuts", async () => {
  renderAt("/");
  const field = document.body.appendChild(document.createElement("input"));
  field.focus();
  expect(fireEvent.keyDown(field, { key: "?", shiftKey: true })).toBe(true);
  await new Promise((r) => setTimeout(r, 20));
  expect(screen.queryByRole("dialog")).toBeNull();
  field.remove();
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

describe("the level chip (LVL-5)", () => {
  const { levelChange: _, ...STATUS } = CHECKIN_STATUS;
  const chip = () =>
    screen.queryByRole("img", {
      name: /^Level \d+, \d+ of \d+ conditions met/,
    });

  // The Level screen must stay reachable whatever the status does (UI_FLOWS §1, item 101).
  test.each([
    ["loads", () => new Promise(() => {})],
    [
      "fails",
      () => {
        throw new Error("boom");
      },
    ],
  ])(
    "is a level 0 placeholder link to /level while the status %s",
    async (_, answer) => {
      const { router } = renderRoute("/settings", {
        get_vault_status: answer,
      });
      await screen.findByRole("navigation");
      await new Promise((r) => setTimeout(r, 50));
      expect(chip()).toBeNull();
      const placeholder = screen.getByRole("link", { name: "Vault level" });
      expect(placeholder.getAttribute("title")).toBe("Vault level");
      expect(placeholder.textContent).toBe("—");
      // An empty ring: the track only, no arc.
      expect(placeholder.querySelectorAll("circle")).toHaveLength(1);
      fireEvent.click(placeholder);
      await waitFor(() =>
        expect(router.state.location.pathname).toBe("/level"),
      );
    },
  );

  test("is marked the current page on /level and nowhere else", async () => {
    renderRoute("/level", {
      get_vault_status: STATUS,
      list_categories: [],
      list_accounts: [],
    });
    const link = await screen.findByRole("link", { name: /^Level 4,/ });
    expect(link.getAttribute("aria-current")).toBe("page");
    expect(link.getAttribute("title")).toBe("Vault level");
    fireEvent.click(screen.getByRole("link", { name: "Dashboard" }));
    await waitFor(() =>
      expect(
        screen.getByRole("link", { name: /^Level 4,/ }).getAttribute(
          "aria-current",
        ),
      ).toBeNull(),
    );
  });

  test("names the level and the ring, opens /level, and follows a save", async () => {
    let level = 4;
    const { router, client } = renderRoute("/transactions", {
      get_vault_status: () => ({
        ...STATUS,
        level,
        nextLevel: { ...STATUS.nextLevel, level: level + 1 },
      }),
    });
    await waitFor(() =>
      expect(chip()?.getAttribute("aria-label")).toBe(
        "Level 4, 1 of 2 conditions met for level 5",
      ),
    );
    // A save refreshes every ['ledger'] query, the status among them.
    level = 5;
    await client.invalidateQueries({ queryKey: queryKeys.ledger });
    await waitFor(() =>
      expect(chip()?.getAttribute("aria-label")).toBe(
        "Level 5, 1 of 2 conditions met for level 6",
      ),
    );
    fireEvent.click(screen.getByRole("link", { name: /^Level 5,/ }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/level"));
  });
});
