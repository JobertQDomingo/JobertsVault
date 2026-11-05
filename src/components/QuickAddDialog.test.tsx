import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { TransactionFields } from "@/bindings/TransactionFields";
import {
  ACCOUNT_LIST,
  CATEGORIES,
  DASHBOARD,
  DEFAULTS,
  LIST,
  PAYMENT_METHODS,
  PERIOD,
  RECENT,
  TODAY,
} from "@/test/fixtures";
import { queryKeys } from "@/lib/queryKeys";
import { renderRoute, SETTINGS, type Answers } from "@/test/render";
import { notify } from "./ui/toast";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const saved = (fields: TransactionFields) => ({ id: 99, ...fields });
const ANSWERS: Answers = {
  get_transaction_defaults: DEFAULTS,
  list_accounts: ACCOUNT_LIST,
  list_categories: CATEGORIES,
  list_payment_methods: PAYMENT_METHODS,
  create_transaction: ({ transaction }: { transaction: TransactionFields }) =>
    saved(transaction),
  update_transaction: ({ transaction }: { transaction: TransactionFields }) =>
    saved(transaction),
  delete_transaction: null,
  get_transaction: ({ id }: { id: number }) => RECENT.find((t) => t.id === id),
  // For opening from the dashboard and the transactions list.
  get_dashboard: DASHBOARD,
  get_period: PERIOD,
  list_transactions: LIST,
};

const calls = (command: string) =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === command)
    .map(([, args]) => args);
const dialog = () => within(screen.getByRole("dialog"));
const field = (name: string) =>
  dialog().getByRole(
    name === "Payee" || name === "Note" ? "textbox" : "combobox",
    { name },
  );
const amount = () =>
  dialog().getByRole("textbox", { name: "Amount" }) as HTMLInputElement;
const key = (k: string, mods: Partial<KeyboardEventInit> = {}) =>
  fireEvent.keyDown(document.activeElement!, { key: k, ...mods });
const type = (el: HTMLElement, text: string) => {
  el.focus();
  fireEvent.change(el, { target: { value: text } });
};

async function openQuickAdd(answers: Answers = ANSWERS) {
  renderRoute("/settings", answers);
  fireEvent.keyDown(window, { key: "n", ctrlKey: true });
  await screen.findByRole("dialog", { name: "Add transaction" });
  await waitFor(() => expect(document.activeElement).toBe(amount()));
}

test("it opens on Expense, the last account and today, with the cursor in Amount", async () => {
  await openQuickAdd();
  expect(
    dialog()
      .getByRole("radio", { name: "Expense" })
      .getAttribute("aria-checked"),
  ).toBe("true");
  expect((field("Account") as HTMLInputElement).value).toBe("Credit card");
  expect(
    (dialog().getByRole("textbox", { name: "Date" }) as HTMLInputElement).value,
  ).toBe("11/05/2026");
  expect(
    dialog().getByRole("button", { name: "Save & add another" }),
  ).toBeTruthy();
});

test("the keyboard path: 250, Tab, din, Enter, Enter saves the expense", async () => {
  await openQuickAdd();
  type(amount(), "250");
  type(field("Category"), "din");
  key("Enter"); // picks Dining Out
  key("Enter"); // saves
  await waitFor(() =>
    expect(calls("create_transaction")).toEqual([
      {
        transaction: {
          kind: "expense",
          amount: 25000,
          date: TODAY,
          accountId: 4,
          toAccountId: null,
          categoryId: 2,
          payee: null,
          paymentMethodId: null,
          note: null,
        },
      },
    ]),
  );
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(await screen.findByText("Saved")).toBeTruthy();
});

test("a second Enter while saving doesn't save twice", async () => {
  await openQuickAdd({
    ...ANSWERS,
    create_transaction: () => new Promise(() => {}),
  });
  type(amount(), "250");
  type(field("Category"), "din");
  key("Enter");
  key("Enter");
  key("Enter");
  await waitFor(() => expect(calls("create_transaction")).toHaveLength(1));
});

test("Shift+Enter saves and starts another, keeping the type, account and date", async () => {
  await openQuickAdd();
  fireEvent.click(dialog().getByRole("radio", { name: "Income" }));
  type(amount(), "1000");
  type(field("Category"), "sal");
  key("Enter");
  type(field("Payee"), "Employer");
  key("Enter", { shiftKey: true });
  await waitFor(() => expect(calls("create_transaction")).toHaveLength(1));
  await waitFor(() => expect(amount().value).toBe(""));
  expect(
    dialog()
      .getByRole("radio", { name: "Income" })
      .getAttribute("aria-checked"),
  ).toBe("true");
  expect((field("Account") as HTMLInputElement).value).toBe("Credit card");
  expect((field("Category") as HTMLInputElement).value).toBe("");
  expect((field("Payee") as HTMLInputElement).value).toBe("");
  expect(document.activeElement).toBe(amount());
});

test("a transfer has From and To, and sends no category, payee or method", async () => {
  await openQuickAdd();
  type(field("Payee"), "Shop");
  key("t", { altKey: true });
  expect(dialog().queryByRole("combobox", { name: "Category" })).toBeNull();
  expect(dialog().queryByRole("textbox", { name: "Payee" })).toBeNull();
  expect(dialog().queryByRole("combobox", { name: "Method" })).toBeNull();
  expect((field("From") as HTMLInputElement).value).toBe("Credit card");
  type(amount(), "500");
  fireEvent.click(field("To"));
  fireEvent.click(screen.getByRole("option", { name: "Bank" }));
  type(field("Note"), "move");
  key("Enter");
  await waitFor(() =>
    expect(calls("create_transaction")).toEqual([
      {
        transaction: {
          kind: "transfer",
          amount: 50000,
          date: TODAY,
          accountId: 4,
          toAccountId: 2,
          categoryId: null,
          payee: null,
          paymentMethodId: null,
          note: "move",
        },
      },
    ]),
  );
});

test("Alt+I switches to Income and drops the expense category", async () => {
  await openQuickAdd();
  type(field("Category"), "din");
  key("Enter");
  key("i", { altKey: true });
  expect(
    dialog()
      .getByRole("radio", { name: "Income" })
      .getAttribute("aria-checked"),
  ).toBe("true");
  expect((field("Category") as HTMLInputElement).value).toBe("");
  fireEvent.click(field("Category"));
  expect(screen.queryByRole("option", { name: "Dining Out" })).toBeNull();
  expect(screen.getByRole("option", { name: "Salary" })).toBeTruthy();
});

test("the backend's message shows under its field, and the dialog stays open", async () => {
  await openQuickAdd({
    ...ANSWERS,
    create_transaction: () => {
      throw {
        code: "validation",
        field: "categoryId",
        message: "Choose a category",
      };
    },
  });
  type(amount(), "250");
  key("Enter");
  expect(await dialog().findByText("Choose a category")).toBeTruthy();
  expect(field("Category").getAttribute("aria-invalid")).toBe("true");
  expect(amount().value).toBe("250");
});

test("a typed date that isn't committed yet is the one saved", async () => {
  await openQuickAdd();
  type(amount(), "250");
  type(field("Category"), "din");
  key("Enter");
  type(dialog().getByRole("textbox", { name: "Date" }), "11/04/2026");
  key("Enter");
  await waitFor(() =>
    expect(
      (calls("create_transaction")[0] as { transaction: TransactionFields })
        .transaction.date,
    ).toBe("2026-11-04"),
  );
});

test("any other error is a toast, and the dialog stays open", async () => {
  await openQuickAdd({
    ...ANSWERS,
    create_transaction: () => {
      throw { code: "database", message: "disk I/O error" };
    },
  });
  type(amount(), "250");
  type(field("Category"), "din");
  key("Enter");
  key("Enter");
  expect(
    await screen.findByText(/Something went wrong with the database/),
  ).toBeTruthy();
  expect(screen.getByRole("dialog", { name: "Add transaction" })).toBeTruthy();
});

test("closing asks first only when something was typed", async () => {
  await openQuickAdd();
  key("Escape");
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  fireEvent.keyDown(window, { key: "n", ctrlKey: true });
  await screen.findByRole("dialog");
  await waitFor(() => expect(document.activeElement).toBe(amount()));
  type(amount(), "12");
  fireEvent.click(dialog().getByRole("button", { name: "Close" }));
  const confirm = await screen.findByRole("dialog", {
    name: "Discard changes?",
  });
  fireEvent.click(within(confirm).getByRole("button", { name: "Cancel" }));
  await waitFor(() =>
    expect(
      screen.queryByRole("dialog", { name: "Discard changes?" }),
    ).toBeNull(),
  );
  // It left the field for the confirm dialog, so it shows as formatted.
  expect(amount().value).toBe("12.00");
  fireEvent.click(dialog().getByRole("button", { name: "Cancel" }));
  fireEvent.click(
    within(
      await screen.findByRole("dialog", { name: "Discard changes?" }),
    ).getByRole("button", { name: "Discard" }),
  );
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
});

test("editing loads the row, saves it as an update, and keeps no Save & add another", async () => {
  renderRoute("/", ANSWERS);
  fireEvent.click(await screen.findByText("Food & Groceries · Market"));
  await screen.findByRole("dialog", { name: "Edit transaction" });
  await waitFor(() => expect(amount().value).toBe("85.20"));
  expect((field("Category") as HTMLInputElement).value).toBe(
    "Food & Groceries",
  );
  expect((field("Account") as HTMLInputElement).value).toBe("Cash");
  expect((field("Payee") as HTMLInputElement).value).toBe("Market");
  expect((field("Note") as HTMLInputElement).value).toBe("weekly shop");
  expect(
    dialog().queryByRole("button", { name: "Save & add another" }),
  ).toBeNull();
  type(amount(), "90");
  fireEvent.click(dialog().getByRole("button", { name: "Save" }));
  await waitFor(() =>
    expect(calls("update_transaction")).toEqual([
      {
        id: 7,
        transaction: {
          kind: "expense",
          amount: 9000,
          date: TODAY,
          accountId: 1,
          toAccountId: null,
          categoryId: 1,
          payee: "Market",
          paymentMethodId: null,
          note: "weekly shop",
        },
      },
    ]),
  );
});

test("editing a transfer into an expense drops its To account", async () => {
  renderRoute("/", ANSWERS);
  fireEvent.click(await screen.findByText("Bank → Credit card"));
  await waitFor(() =>
    expect((field("To") as HTMLInputElement).value).toBe("Credit card"),
  );
  fireEvent.click(dialog().getByRole("radio", { name: "Expense" }));
  type(field("Category"), "food");
  key("Enter");
  key("Enter");
  await waitFor(() =>
    expect(
      (calls("update_transaction")[0] as { transaction: TransactionFields })
        .transaction,
    ).toMatchObject({
      kind: "expense",
      accountId: 2,
      toAccountId: null,
      categoryId: 1,
    }),
  );
});

test("Delete asks first, then deletes and closes", async () => {
  // Like the backend: once deleted, the row is gone.
  let deleted = false;
  renderRoute("/", {
    ...ANSWERS,
    delete_transaction: () => {
      deleted = true;
      return null;
    },
    get_transaction: ({ id }: { id: number }) => {
      if (deleted) throw { code: "not_found", entity: "transaction", id };
      return RECENT.find((t) => t.id === id);
    },
    // The balances change, so everything showing them refreshes.
    list_accounts: () =>
      deleted ? { ...ACCOUNT_LIST, netBalance: 820055 } : ACCOUNT_LIST,
    // …and the refresh takes a moment, as real calls do.
    list_categories: () =>
      deleted
        ? new Promise((done) => setTimeout(() => done(CATEGORIES), 50))
        : CATEGORIES,
  });
  fireEvent.click(await screen.findByText("Food & Groceries · Market"));
  await waitFor(() => expect(amount().value).toBe("85.20"));
  fireEvent.click(dialog().getByRole("button", { name: "Delete" }));
  const confirm = await screen.findByRole("dialog", {
    name: "Delete this transaction?",
  });
  expect(within(confirm).getByText("This can't be undone.")).toBeTruthy();
  fireEvent.click(within(confirm).getByRole("button", { name: "Delete" }));
  await waitFor(() => expect(calls("delete_transaction")).toEqual([{ id: 7 }]));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(await screen.findByText("Deleted")).toBeTruthy();
});

test("Duplicate opens a copy dated today and saves it as a new row", async () => {
  renderRoute("/transactions", ANSWERS);
  await screen.findByText("Corner Cafe");
  const row = screen.getByText("Corner Cafe").closest<HTMLElement>(".group")!;
  fireEvent.click(within(row).getByRole("button", { name: "Duplicate" }));
  await screen.findByRole("dialog", { name: "Add transaction" });
  await waitFor(() => expect(amount().value).toBe("12.50"));
  expect(
    (dialog().getByRole("textbox", { name: "Date" }) as HTMLInputElement).value,
  ).toBe("11/05/2026");
  fireEvent.click(dialog().getByRole("button", { name: "Save" }));
  await waitFor(() =>
    expect(calls("create_transaction")).toEqual([
      {
        transaction: {
          kind: "expense",
          amount: 1250,
          date: TODAY,
          accountId: 4,
          toAccountId: null,
          categoryId: 2,
          payee: "Corner Cafe",
          paymentMethodId: null,
          note: null,
        },
      },
    ]),
  );
});

test("the method field follows the setting", async () => {
  await openQuickAdd({
    ...ANSWERS,
    get_settings: { ...SETTINGS, showPaymentMethod: false },
  });
  await waitFor(() =>
    expect(dialog().queryByRole("combobox", { name: "Method" })).toBeNull(),
  );
});

test("Alt+T switches to Transfer even while a toast is showing", async () => {
  await openQuickAdd();
  notify.success("A toast is showing");
  await screen.findByText("A toast is showing");
  amount().focus();
  // Sonner's own Alt+T focuses its list; jsdom's focus trap would hide that, so watch the request.
  const focus = vi.spyOn(HTMLElement.prototype, "focus");
  key("t", { altKey: true, code: "KeyT" });
  const outside = focus.mock.contexts.filter(
    (el) => !(el as HTMLElement).closest("[role=dialog]"),
  );
  focus.mockRestore();
  expect(outside).toEqual([]);
  expect(
    dialog()
      .getByRole("radio", { name: "Transfer" })
      .getAttribute("aria-checked"),
  ).toBe("true");
});

test("the next quick-add starts on the account just used, not a cached one", async () => {
  let last = 4;
  await openQuickAdd({
    ...ANSWERS,
    get_transaction_defaults: () => ({ accountId: last, today: TODAY }),
    create_transaction: ({
      transaction,
    }: {
      transaction: TransactionFields;
    }) => {
      last = transaction.accountId;
      return saved(transaction);
    },
  });
  type(amount(), "250");
  type(field("Category"), "din");
  key("Enter");
  type(field("Account"), "bank");
  key("Enter");
  key("Enter");
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  fireEvent.keyDown(window, { key: "n", ctrlKey: true });
  await screen.findByRole("dialog");
  await waitFor(() => expect(document.activeElement).toBe(amount()));
  expect((field("Account") as HTMLInputElement).value).toBe("Bank");
});

const reopen = async () => {
  fireEvent.keyDown(window, { key: "n", ctrlKey: true });
  await screen.findByRole("dialog");
  await waitFor(() => expect(document.activeElement).toBe(amount()));
};
/** Opens ⧉ on the list row whose payee is `payee`. */
async function duplicateRow(payee: string) {
  const row = (await screen.findByText(payee)).closest<HTMLElement>(".group")!;
  fireEvent.click(within(row).getByRole("button", { name: "Duplicate" }));
  await screen.findByRole("dialog", { name: "Add transaction" });
}

test("reopening a row just edited shows the saved values, not cached ones", async () => {
  const rows = new Map(RECENT.map((t) => [t.id, t]));
  renderRoute("/", {
    ...ANSWERS,
    get_transaction: ({ id }: { id: number }) => rows.get(id),
    update_transaction: ({
      id,
      transaction,
    }: {
      id: number;
      transaction: TransactionFields;
    }) => {
      rows.set(id, { id, ...transaction });
      return { id, ...transaction };
    },
  });
  fireEvent.click(await screen.findByText("Food & Groceries · Market"));
  await waitFor(() => expect(amount().value).toBe("85.20"));
  type(amount(), "90");
  key("Enter");
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  fireEvent.click(await screen.findByText("Food & Groceries · Market"));
  await screen.findByRole("dialog", { name: "Edit transaction" });
  await waitFor(() => expect(amount().value).toBe("90.00"));
});

test("each opening asks for today again (the app can stay open past midnight)", async () => {
  let today = TODAY;
  await openQuickAdd({
    ...ANSWERS,
    get_transaction_defaults: () => ({ accountId: 4, today }),
  });
  key("Escape");
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  today = "2026-11-06";
  await reopen();
  expect(
    (dialog().getByRole("textbox", { name: "Date" }) as HTMLInputElement).value,
  ).toBe("11/06/2026");
});

test("an error for a field that isn't shown is a toast, and Save still works after", async () => {
  let fail = true;
  renderRoute("/transactions", {
    ...ANSWERS,
    get_settings: { ...SETTINGS, showPaymentMethod: false },
    get_transaction: ({ id }: { id: number }) => ({
      ...RECENT.find((t) => t.id === id),
      paymentMethodId: 3,
    }),
    create_transaction: ({
      transaction,
    }: {
      transaction: TransactionFields;
    }) => {
      if (fail)
        throw {
          code: "validation",
          field: "paymentMethodId",
          message: "This payment method is archived",
        };
      return saved(transaction);
    },
  });
  await duplicateRow("Corner Cafe");
  await waitFor(() => expect(amount().value).toBe("12.50"));
  fireEvent.click(dialog().getByRole("button", { name: "Save" }));
  expect(
    await screen.findByText("This payment method is archived"),
  ).toBeTruthy();
  fail = false;
  fireEvent.click(dialog().getByRole("button", { name: "Save" }));
  await waitFor(() => expect(calls("create_transaction")).toHaveLength(2));
});

test("if the settings can't load, the dialog says so instead of loading forever", async () => {
  const { client } = renderRoute("/settings", {
    ...ANSWERS,
    get_settings: () => {
      throw { code: "database", message: "disk I/O error" };
    },
  });
  client.removeQueries({ queryKey: queryKeys.settings });
  fireEvent.keyDown(window, { key: "n", ctrlKey: true });
  expect(await dialog().findByRole("button", { name: "Retry" })).toBeTruthy();
});

test("keys pressed in the delete confirm stay there", async () => {
  renderRoute("/", ANSWERS);
  fireEvent.click(await screen.findByText("Food & Groceries · Market"));
  await waitFor(() => expect(amount().value).toBe("85.20"));
  fireEvent.click(dialog().getByRole("button", { name: "Delete" }));
  const confirm = await screen.findByRole("dialog", {
    name: "Delete this transaction?",
  });
  fireEvent.keyDown(within(confirm).getByRole("button", { name: "Cancel" }), {
    key: "t",
    altKey: true,
  });
  fireEvent.click(within(confirm).getByRole("button", { name: "Cancel" }));
  await waitFor(() =>
    expect(
      screen.queryByRole("dialog", { name: "Delete this transaction?" }),
    ).toBeNull(),
  );
  expect(
    dialog()
      .getByRole("radio", { name: "Expense" })
      .getAttribute("aria-checked"),
  ).toBe("true");
});

test("a duplicate is dated today whatever its id", async () => {
  renderRoute("/transactions", {
    ...ANSWERS,
    list_transactions: { ...LIST, items: [{ ...RECENT[2], id: 0 }] },
    get_transaction: () => ({ ...RECENT[2], id: 0 }),
  });
  await duplicateRow("Corner Cafe");
  await waitFor(() =>
    expect(
      (dialog().getByRole("textbox", { name: "Date" }) as HTMLInputElement)
        .value,
    ).toBe("11/05/2026"),
  );
});

test("a field's error goes once the field is changed", async () => {
  await openQuickAdd({
    ...ANSWERS,
    create_transaction: () => {
      throw {
        code: "validation",
        field: "amount",
        message: "Enter an amount above 0",
      };
    },
  });
  key("Enter");
  expect(await dialog().findByText("Enter an amount above 0")).toBeTruthy();
  type(amount(), "250");
  await waitFor(() =>
    expect(dialog().queryByText("Enter an amount above 0")).toBeNull(),
  );
});

test("after Save & add another, an archived category isn't offered any more", async () => {
  renderRoute("/transactions", {
    ...ANSWERS,
    get_transaction: ({ id }: { id: number }) => ({
      ...RECENT.find((t) => t.id === id),
      categoryId: 5,
    }),
  });
  await duplicateRow("Corner Cafe");
  await waitFor(() =>
    expect((field("Category") as HTMLInputElement).value).toBe(
      "Snacks (archived)",
    ),
  );
  type(field("Category"), "din");
  key("Enter");
  key("Enter", { shiftKey: true });
  await waitFor(() => expect(amount().value).toBe(""));
  fireEvent.click(field("Category"));
  expect(
    screen.queryByRole("option", { name: "Snacks (archived)" }),
  ).toBeNull();
});
