import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { TransactionFields } from "@/bindings/TransactionFields";
import {
  ACCOUNT_LIST,
  ACCOUNTS,
  CATEGORIES,
  DEFAULTS,
  LIST,
  PAY_BILL_DEFAULTS,
  PAYMENT_METHODS,
  PERIOD,
  TODAY,
} from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const ANSWERS: Answers = {
  list_accounts: ACCOUNT_LIST,
  get_account: ({ id }: { id: number }) => ACCOUNTS.find((a) => a.id === id),
  list_categories: CATEGORIES,
  list_payment_methods: PAYMENT_METHODS,
  get_period: PERIOD,
  list_transactions: LIST,
  get_transaction_defaults: DEFAULTS,
  get_pay_bill_defaults: PAY_BILL_DEFAULTS,
  create_transaction: ({
    transaction,
  }: {
    transaction: TransactionFields;
  }) => ({
    id: 99,
    ...transaction,
  }),
};

const calls = (command: string) =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === command)
    .map(([, args]) => args);
const value = (dialog: HTMLElement, name: string) =>
  (
    within(dialog).getByRole(name === "Note" ? "textbox" : "combobox", {
      name,
    }) as HTMLInputElement
  ).value;
const amount = (dialog: HTMLElement) =>
  (within(dialog).getByRole("textbox", { name: "Amount" }) as HTMLInputElement)
    .value;

async function payBill(answers: Answers = ANSWERS) {
  renderRoute("/accounts/4", answers);
  fireEvent.click(await screen.findByRole("button", { name: "Pay bill" }));
  const dialog = await screen.findByRole("dialog", { name: "Pay credit card" });
  await waitFor(() => expect(within(dialog).queryByRole("status")).toBeNull());
  return dialog;
}

test("Pay bill opens a transfer to the card for the full amount owed (CC-9)", async () => {
  const dialog = await payBill();
  expect(amount(dialog)).toBe("640.00");
  expect(value(dialog, "From")).toBe("Bank");
  expect(value(dialog, "To")).toBe("Credit card");
  expect(value(dialog, "Note")).toBe("Credit card payment");
  expect(
    (within(dialog).getByRole("textbox", { name: "Date" }) as HTMLInputElement)
      .value,
  ).toBe("11/05/2026");
  // One payment: no type switch and no Save & add another (the mockup).
  expect(within(dialog).queryByRole("radiogroup")).toBeNull();
  expect(
    within(dialog).queryByRole("button", { name: "Save & add another" }),
  ).toBeNull();
  await waitFor(() =>
    expect(calls("get_pay_bill_defaults")).toEqual([{ cardId: 4 }]),
  );
  fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));
  await waitFor(() =>
    expect(calls("create_transaction")).toEqual([
      {
        transaction: {
          kind: "transfer",
          amount: 64000,
          date: TODAY,
          accountId: 2,
          toAccountId: 4,
          categoryId: null,
          payee: null,
          paymentMethodId: null,
          note: "Credit card payment",
        },
      },
    ]),
  );
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
});

test("with nothing owed and no account to pay from, those fields start empty", async () => {
  const dialog = await payBill({
    ...ANSWERS,
    get_pay_bill_defaults: { fromAccountId: null, amount: null, today: TODAY },
  });
  expect(amount(dialog)).toBe("");
  expect(value(dialog, "From")).toBe("");
  expect(value(dialog, "To")).toBe("Credit card");
});

test("Alt+E doesn't turn a payment into an expense", async () => {
  const dialog = await payBill();
  fireEvent.keyDown(within(dialog).getByRole("textbox", { name: "Amount" }), {
    key: "e",
    altKey: true,
  });
  expect(within(dialog).getByRole("combobox", { name: "To" })).toBeTruthy();
});

test("a bank account has no Pay bill", async () => {
  renderRoute("/accounts/2", ANSWERS);
  await screen.findByRole("heading", { name: "Bank" });
  expect(screen.queryByRole("button", { name: "Pay bill" })).toBeNull();
});

test("if the defaults can't load, the dialog offers Retry", async () => {
  renderRoute("/accounts/4", {
    ...ANSWERS,
    get_pay_bill_defaults: () => {
      throw { code: "database", message: "disk I/O error" };
    },
  });
  fireEvent.click(await screen.findByRole("button", { name: "Pay bill" }));
  const dialog = await screen.findByRole("dialog", { name: "Pay credit card" });
  expect(
    await within(dialog).findByRole("button", { name: "Retry" }),
  ).toBeTruthy();
});

test("a failed reload never starts the form from the old cached amount", async () => {
  let fail = false;
  renderRoute("/accounts/4", {
    ...ANSWERS,
    get_pay_bill_defaults: () => {
      if (fail) throw { code: "database", message: "disk I/O error" };
      return PAY_BILL_DEFAULTS;
    },
  });
  fireEvent.click(await screen.findByRole("button", { name: "Pay bill" }));
  let dialog = await screen.findByRole("dialog", { name: "Pay credit card" });
  await waitFor(() => expect(amount(dialog)).toBe("640.00"));
  fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  fail = true;
  fireEvent.click(screen.getByRole("button", { name: "Pay bill" }));
  dialog = await screen.findByRole("dialog", { name: "Pay credit card" });
  expect(
    await within(dialog).findByRole("button", { name: "Retry" }),
  ).toBeTruthy();
  expect(within(dialog).queryByRole("textbox", { name: "Amount" })).toBeNull();
});
