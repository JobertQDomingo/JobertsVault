import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { NewAccount } from "@/bindings/NewAccount";
import { ACCOUNT_LIST, ACCOUNTS } from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const ANSWERS: Answers = {
  list_accounts: ACCOUNT_LIST,
  reorder_accounts: null,
  unarchive_account: null,
  create_account: ({ account }: { account: NewAccount }) => ({
    ...ACCOUNTS[0],
    id: 9,
    ...account,
  }),
  update_account: ({ id }: { id: number }) => ACCOUNTS.find((a) => a.id === id),
};

const calls = (command: string) =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === command)
    .map(([, args]) => args);
const dialog = () => within(screen.getByRole("dialog"));
const type = (el: HTMLElement, text: string) => {
  el.focus();
  fireEvent.change(el, { target: { value: text } });
};
const pick = (field: string, option: string) => {
  fireEvent.click(dialog().getByRole("combobox", { name: field }));
  fireEvent.click(screen.getByRole("option", { name: option }));
};
const rows = () =>
  screen
    .getAllByRole("link")
    .filter((a) => a.getAttribute("href")?.startsWith("/accounts/"))
    .map((a) => a.textContent);

test("lists the active accounts in order, with the net balance", async () => {
  renderRoute("/accounts", ANSWERS);
  await screen.findByText("Net balance");
  expect(screen.queryByText(/level/)).toBeNull();
  expect(rows().map((r) => r?.split("₱")[0])).toEqual([
    "CashCash",
    "BankBank",
    "E-walletE-wallet",
    "Credit cardCredit card · 6% of limit",
  ]);
  expect(screen.getByText("₱8,285.75")).toBeTruthy();
  expect(screen.queryByText("Old Wallet")).toBeNull();
});

test("archived accounts wait behind Show archived, and Unarchive brings one back", async () => {
  renderRoute("/accounts", ANSWERS);
  fireEvent.click(
    await screen.findByRole("button", { name: "Show archived (1)" }),
  );
  expect(screen.getByText("Old Wallet")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Unarchive Old Wallet" }));
  await waitFor(() => expect(calls("unarchive_account")).toEqual([{ id: 5 }]));
});

test("↓ on Cash's handle saves the new order", async () => {
  renderRoute("/accounts", ANSWERS);
  fireEvent.keyDown(await screen.findByRole("button", { name: "Move Cash" }), {
    key: "ArrowDown",
  });
  await waitFor(() =>
    expect(calls("reorder_accounts")).toEqual([{ ids: [2, 1, 3, 4] }]),
  );
});

test("the list says so when it can't load, with Retry", async () => {
  renderRoute("/accounts", {
    ...ANSWERS,
    list_accounts: () => {
      throw { code: "database", message: "disk I/O error" };
    },
  });
  expect(await screen.findByRole("button", { name: "Retry" })).toBeTruthy();
});

test("New account creates a credit card with its limit and days", async () => {
  renderRoute("/accounts", ANSWERS);
  fireEvent.click(await screen.findByRole("button", { name: "New account" }));
  type(dialog().getByRole("textbox", { name: "Name" }), "Visa");
  pick("Type", "Credit card");
  expect(
    dialog().getByRole("textbox", { name: "Amount owed now" }),
  ).toBeTruthy();
  type(dialog().getByRole("textbox", { name: "Credit limit" }), "10000");
  // CC-10: a new card starts at 3,000.00, 4,000.00 and 5,000.00.
  expect(
    ["Soft", "Mid", "Hard"].map(
      (name) =>
        (dialog().getByRole("textbox", { name }) as HTMLInputElement).value,
    ),
  ).toEqual(["3,000.00", "4,000.00", "5,000.00"]);
  expect(
    dialog().getByText(
      "Warnings for each statement cycle's spending. They never block a purchase.",
    ),
  ).toBeTruthy();
  type(dialog().getByRole("textbox", { name: "Hard" }), "6000");
  pick("Statement closes", "Day 20");
  pick("Payment due", "Day 5");
  fireEvent.click(dialog().getByRole("button", { name: "Create account" }));
  await waitFor(() =>
    expect(calls("create_account")).toEqual([
      {
        account: {
          kind: "credit_card",
          name: "Visa",
          openingBalance: 0,
          creditLimit: 1000000,
          statementDay: 20,
          dueDay: 5,
          spendSoft: 300000,
          spendMid: 400000,
          spendHard: 600000,
        },
      },
    ]),
  );
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(await screen.findByText("Saved")).toBeTruthy();
});

test("a cash account sends no card fields, and a backend error shows under its field", async () => {
  renderRoute("/accounts", {
    ...ANSWERS,
    create_account: () => {
      throw {
        code: "validation",
        field: "name",
        message: "An account with this name already exists",
      };
    },
  });
  fireEvent.click(await screen.findByRole("button", { name: "New account" }));
  type(dialog().getByRole("textbox", { name: "Name" }), "Bank");
  type(dialog().getByRole("textbox", { name: "Opening balance" }), "50");
  fireEvent.click(dialog().getByRole("button", { name: "Create account" }));
  expect(
    await dialog().findByText("An account with this name already exists"),
  ).toBeTruthy();
  expect(calls("create_account")).toEqual([
    {
      account: {
        kind: "cash",
        name: "Bank",
        openingBalance: 5000,
        creditLimit: null,
        statementDay: null,
        dueDay: null,
        spendSoft: null,
        spendMid: null,
        spendHard: null,
      },
    },
  ]);
});

test("a card's row shows the spending level it has reached (CC-10)", async () => {
  const card = ACCOUNTS[3];
  const accounts = ACCOUNTS.map((a) =>
    a === card ? { ...a, card: { ...a.card!, spendLevel: "mid" as const } } : a,
  );
  renderRoute("/accounts", {
    ...ANSWERS,
    list_accounts: { ...ACCOUNT_LIST, accounts },
  });
  await screen.findByText("Net balance");
  expect(rows()[3]).toContain("Credit card · 6% of limit · Mid level");
});

test("a card's levels that don't rise get the backend's message under them", async () => {
  renderRoute("/accounts", {
    ...ANSWERS,
    create_account: () => {
      throw {
        code: "validation",
        field: "spendMid",
        message: "Make it more than the soft level",
      };
    },
  });
  fireEvent.click(await screen.findByRole("button", { name: "New account" }));
  type(dialog().getByRole("textbox", { name: "Name" }), "Visa");
  pick("Type", "Credit card");
  type(dialog().getByRole("textbox", { name: "Credit limit" }), "10000");
  pick("Statement closes", "Day 20");
  pick("Payment due", "Day 5");
  type(dialog().getByRole("textbox", { name: "Mid" }), "");
  fireEvent.click(dialog().getByRole("button", { name: "Create account" }));
  expect(
    await dialog().findByText("Make it more than the soft level"),
  ).toBeTruthy();
  // An empty level goes as 0, for the backend to name.
  expect(calls("create_account")).toMatchObject([
    { account: { spendSoft: 300000, spendMid: 0, spendHard: 500000 } },
  ]);
});
