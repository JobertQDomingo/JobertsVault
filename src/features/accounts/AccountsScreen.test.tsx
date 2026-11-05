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
      },
    },
  ]);
});
