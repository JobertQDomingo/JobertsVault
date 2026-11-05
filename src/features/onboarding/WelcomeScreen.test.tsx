import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { Account } from "@/bindings/Account";
import type { NewAccount } from "@/bindings/NewAccount";
import type { Settings } from "@/bindings/Settings";
import { ACCOUNTS, CATEGORIES, DASHBOARD, PERIOD } from "@/test/fixtures";
import { renderRoute, SETTINGS, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const FIRST = { ...SETTINGS, onboardingCompleted: false };
const { decimalsLocked, ...STORED } = FIRST;

function answers(): Answers {
  let settings = FIRST;
  let accounts: Account[] = [ACCOUNTS[0]];
  return {
    get_settings: () => settings,
    update_settings: ({ settings: next }: { settings: Settings }) =>
      (settings = { ...next, decimalsLocked: false }),
    list_accounts: () => ({ accounts, netBalance: 0 }),
    create_account: ({ account }: { account: NewAccount }) => {
      const created = { ...ACCOUNTS[1], id: 9, ...account };
      accounts = [...accounts, created];
      return created;
    },
    get_dashboard: DASHBOARD,
    get_period: PERIOD,
    list_categories: CATEGORIES,
  };
}

const saves = () =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === "update_settings")
    .map(([, args]) => (args as { settings: Settings }).settings);

test("the first launch goes to the welcome form, and the dashboard never shows", async () => {
  // Watches from the first render, with nothing cached, as the app starts.
  let dashboard = false;
  const watch = new MutationObserver(() => {
    if (document.body.textContent?.includes("Net cashflow")) dashboard = true;
  });
  watch.observe(document.body, { childList: true, subtree: true });
  const { router } = renderRoute("/", answers(), { seedSettings: false });
  await waitFor(() => expect(router.state.location.pathname).toBe("/welcome"));
  expect(
    await screen.findByRole("heading", { name: "Welcome to Jobert's Vault" }),
  ).toBeTruthy();
  expect(screen.queryByText("Net cashflow")).toBeNull();
  expect(
    (
      screen.getByRole("textbox", {
        name: "Currency symbol",
      }) as HTMLInputElement
    ).value,
  ).toBe("₱");
  expect(await screen.findByText("already added")).toBeTruthy();
  expect(screen.getByText("Cash")).toBeTruthy();
  watch.disconnect();
  expect(dashboard).toBe(false);
});

test("the finish buttons wait for the settings, so a save never goes out without them", async () => {
  renderRoute(
    "/welcome",
    {
      ...answers(),
      get_settings: () => {
        throw { code: "database", message: "disk I/O error" };
      },
    },
    { seedSettings: false },
  );
  expect(await screen.findByRole("button", { name: "Retry" })).toBeTruthy();
  const start = screen.getByRole("button", { name: "Start using the app" });
  expect((start as HTMLButtonElement).disabled).toBe(true);
  expect(
    (screen.getByRole("button", { name: "Set up later" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
});

test("+ Bank adds a bank account, listed straight away", async () => {
  renderRoute("/welcome", answers());
  fireEvent.click(await screen.findByRole("button", { name: "Bank" }));
  const dialog = within(
    await screen.findByRole("dialog", { name: "New account" }),
  );
  expect(
    (dialog.getByRole("combobox", { name: "Type" }) as HTMLInputElement).value,
  ).toBe("Bank");
  fireEvent.change(dialog.getByRole("textbox", { name: "Name" }), {
    target: { value: "BPI" },
  });
  fireEvent.click(dialog.getByRole("button", { name: "Create account" }));
  expect(await screen.findByText("BPI")).toBeTruthy();
  // Only what was there before the form opened is "already added".
  expect(screen.getAllByText("already added")).toHaveLength(1);
  expect(
    vi.mocked(invoke).mock.calls.find(([c]) => c === "create_account")?.[1],
  ).toEqual({
    account: {
      kind: "bank",
      name: "BPI",
      openingBalance: 0,
      creditLimit: null,
      statementDay: null,
      dueDay: null,
    },
  });
});

test("Start using the app ends it for good and opens the dashboard", async () => {
  const { router } = renderRoute("/welcome", answers());
  fireEvent.click(
    await screen.findByRole("button", { name: "Start using the app" }),
  );
  await waitFor(() => expect(router.state.location.pathname).toBe("/"));
  expect(saves()).toEqual([{ ...STORED, onboardingCompleted: true }]);
  // Back doesn't reopen it.
  await router.navigate(-1);
  await waitFor(() => expect(router.state.location.pathname).toBe("/"));
});

test("Set up later ends it too, keeping a currency already changed", async () => {
  const { router } = renderRoute("/welcome", answers());
  fireEvent.click(await screen.findByRole("radio", { name: /^After/ }));
  await waitFor(() => expect(saves()).toHaveLength(1));
  fireEvent.click(screen.getByRole("button", { name: "Set up later" }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/"));
  expect(saves()[1]).toEqual({
    ...STORED,
    currencyPosition: "after",
    onboardingCompleted: true,
  });
});

test("once completed, /welcome goes to the dashboard", async () => {
  const { router } = renderRoute("/welcome", {
    ...answers(),
    get_settings: SETTINGS,
  });
  await waitFor(() => expect(router.state.location.pathname).toBe("/"));
});
