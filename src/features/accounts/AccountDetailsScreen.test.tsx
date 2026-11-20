import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { Account } from "@/bindings/Account";
import {
  ACCOUNT_LIST,
  ACCOUNTS,
  CATEGORIES,
  LIST,
  PERIOD,
} from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const notFound = (id: number) => ({ code: "not_found", entity: "account", id });
const answers = (accounts: Account[] = ACCOUNTS): Answers => ({
  list_accounts: { ...ACCOUNT_LIST, accounts },
  get_account: ({ id }: { id: number }) => {
    const found = accounts.find((a) => a.id === id);
    if (!found) throw notFound(id);
    return found;
  },
  list_categories: CATEGORIES,
  get_period: PERIOD,
  list_transactions: LIST,
  archive_account: null,
  unarchive_account: null,
  delete_account: null,
});

const calls = (command: string) =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === command)
    .map(([, args]) => args);
const openMenu = async () =>
  fireEvent.keyDown(
    await screen.findByRole("button", { name: "More actions" }),
    {
      key: "Enter",
    },
  );
const confirm = (name: string) =>
  within(screen.getByRole("dialog", { name })).getAllByRole("button");

test("a card shows what's owed, the limit, the cycle and the due date (CC-6)", async () => {
  renderRoute("/accounts/4", answers());
  expect(
    await screen.findByRole("heading", { name: "Credit card" }),
  ).toBeTruthy();
  expect(screen.getByText("6% of ₱10,000.00 limit")).toBeTruthy();
  const figures = screen.getByRole("table");
  expect(figures.textContent).toBe(
    "Available credit₱9,360.00" +
      "This cycle (Oct 21 – Nov 20)₱412.30 spent" +
      "Statement closesNov 20" +
      "Payment dueNov 5",
  );
  expect(screen.getByText("owed")).toBeTruthy();
  await waitFor(() =>
    expect(calls("list_transactions")).toContainEqual({
      filter: {
        period: { kind: "month", anchor: null },
        kind: null,
        accountId: 4,
        categoryId: null,
        paymentMethodId: null,
        search: null,
      },
      page: 0,
    }),
  );
});

test("a bank account shows its balance and no card figures", async () => {
  renderRoute("/accounts/2", answers());
  expect(await screen.findByRole("heading", { name: "Bank" })).toBeTruthy();
  expect(screen.getByText("₱8,410.75")).toBeTruthy();
  expect(screen.queryByRole("table")).toBeNull();
});

test("archiving an account with money asks Archive anyway and says so to the backend (F6)", async () => {
  renderRoute("/accounts/2", answers());
  await openMenu();
  fireEvent.click(await screen.findByRole("menuitem", { name: "Archive" }));
  const dialog = await screen.findByRole("dialog", { name: "Archive Bank?" });
  expect(dialog.textContent).toContain(
    "Bank still has ₱8,410.75. Archived accounts still count toward your net balance. Archive anyway?",
  );
  fireEvent.click(
    within(dialog).getByRole("button", { name: "Archive anyway" }),
  );
  await waitFor(() =>
    expect(calls("archive_account")).toEqual([
      { id: 2, acknowledgeBalance: true },
    ]),
  );
});

test("archiving an empty account asks plainly, and the last active one shows the backend's reason", async () => {
  const empty = ACCOUNTS.map((a) => (a.id === 3 ? { ...a, balance: 0 } : a));
  renderRoute("/accounts/3", {
    ...answers(empty),
    archive_account: () => {
      throw { code: "conflict", message: "Keep at least one active account" };
    },
  });
  await openMenu();
  fireEvent.click(await screen.findByRole("menuitem", { name: "Archive" }));
  const dialog = await screen.findByRole("dialog", {
    name: "Archive E-wallet?",
  });
  expect(dialog.textContent).toContain(
    "It will be hidden from lists but its history stays.",
  );
  fireEvent.click(within(dialog).getByRole("button", { name: "Archive" }));
  expect(
    await screen.findByText("Keep at least one active account"),
  ).toBeTruthy();
  expect(calls("archive_account")).toEqual([
    { id: 3, acknowledgeBalance: false },
  ]);
});

test("an archived account offers Unarchive instead", async () => {
  renderRoute("/accounts/5", answers());
  await openMenu();
  fireEvent.click(await screen.findByRole("menuitem", { name: "Unarchive" }));
  await waitFor(() => expect(calls("unarchive_account")).toEqual([{ id: 5 }]));
});

test("Delete only shows for an unused account", async () => {
  renderRoute("/accounts/2", answers());
  await openMenu();
  await screen.findByRole("menuitem", { name: "Edit" });
  expect(screen.queryByRole("menuitem", { name: "Delete" })).toBeNull();
});

test("after Delete it goes back to the list and never asks for the deleted account again", async () => {
  let accounts = ACCOUNTS.map((a) => (a.id === 3 ? { ...a, inUse: false } : a));
  const { router } = renderRoute("/accounts/3", {
    ...answers(),
    // The refresh takes a moment, as in the app, so the screen has gone before it ends.
    list_accounts: () =>
      new Promise((resolve) =>
        setTimeout(() => resolve({ ...ACCOUNT_LIST, accounts }), 50),
      ),
    get_account: ({ id }: { id: number }) => {
      const found = accounts.find((a) => a.id === id);
      if (!found) throw notFound(id);
      return found;
    },
    delete_account: () => {
      accounts = accounts.filter((a) => a.id !== 3);
      return null;
    },
  });
  await openMenu();
  fireEvent.click(await screen.findByRole("menuitem", { name: "Delete" }));
  const [, ok] = confirm("Delete E-wallet?");
  fireEvent.click(ok);
  await waitFor(() => expect(router.state.location.pathname).toBe("/accounts"));
  // Replaced, so Back doesn't return to the deleted account.
  expect(router.state.historyAction).toBe("REPLACE");
  expect(await screen.findByText("Deleted")).toBeTruthy();
  await screen.findByText("Net balance");
  expect(screen.queryByRole("button", { name: "Retry" })).toBeNull();
  expect(
    calls("get_account").filter((a) => (a as { id: number }).id === 3),
  ).toHaveLength(1);
});

test("an account that doesn't exist says so, with a way back instead of Retry", async () => {
  const { router } = renderRoute("/accounts/999", answers());
  expect(await screen.findByText("This account doesn't exist.")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Retry" })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "All accounts" }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/accounts"));
});

test("an id that isn't a number never reaches the backend", async () => {
  renderRoute("/accounts/abc", answers());
  expect(await screen.findByText("This account doesn't exist.")).toBeTruthy();
  expect(calls("get_account")).toEqual([]);
});

test("any other error offers Retry", async () => {
  renderRoute("/accounts/2", {
    ...answers(),
    get_account: () => {
      throw { code: "database", message: "disk I/O error" };
    },
  });
  expect(await screen.findByRole("button", { name: "Retry" })).toBeTruthy();
});

test("Edit opens the account's dialog with the type locked", async () => {
  // UI_FLOWS §5.2: a card's edit form holds the opening amount owed, with today's under it.
  renderRoute("/accounts/4", answers());
  await openMenu();
  fireEvent.click(await screen.findByRole("menuitem", { name: "Edit" }));
  const dialog = within(
    await screen.findByRole("dialog", { name: "Edit account" }),
  );
  expect(
    (
      dialog.getByRole("textbox", {
        name: "Opening amount owed",
        description: "Owed today: ₱640.00",
      }) as HTMLInputElement
    ).value,
  ).toBe("0.00");
  expect(
    (dialog.getByRole("combobox", { name: "Type" }) as HTMLInputElement)
      .disabled,
  ).toBe(true);
});

const CARD = ACCOUNTS[3];
const withLevel = (spendLevel: "soft" | "mid" | "hard", over = {}) =>
  ACCOUNTS.map((a) =>
    a === CARD
      ? {
          ...a,
          ...over,
          card: { ...a.card!, cycleSpending: 324000, spendLevel },
        }
      : a,
  );

test("the level reached shows under This cycle (CC-10, F16 step 5)", async () => {
  renderRoute("/accounts/4", answers(withLevel("soft")));
  const figures = await screen.findByRole("table");
  expect(figures.textContent).toContain(
    "This cycle (Oct 21 – Nov 20)₱3,240.00 spent" +
      "Spending levelPast your ₱3,000.00 soft level",
  );
});

test("spending exactly at the level says Reached (CC-10)", async () => {
  renderRoute("/accounts/4", answers(withLevel("soft", { spendSoft: 324000 })));
  const figures = await screen.findByRole("table");
  expect(figures.textContent).toContain(
    "Spending levelReached your ₱3,240.00 soft level",
  );
});

test("the edit form shows the card's own levels", async () => {
  renderRoute(
    "/accounts/4",
    answers(
      withLevel("hard", {
        spendSoft: 100000,
        spendMid: 200000,
        spendHard: 300000,
      }),
    ),
  );
  expect(
    await screen.findByText("Past your ₱3,000.00 hard level"),
  ).toBeTruthy();
  await openMenu();
  fireEvent.click(await screen.findByRole("menuitem", { name: "Edit" }));
  const dialog = within(
    await screen.findByRole("dialog", { name: "Edit account" }),
  );
  expect(
    ["Soft", "Mid", "Hard"].map(
      (name) =>
        (dialog.getByRole("textbox", { name }) as HTMLInputElement).value,
    ),
  ).toEqual(["1,000.00", "2,000.00", "3,000.00"]);
});
