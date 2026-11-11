import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { RecurringRule } from "@/bindings/RecurringRule";
import type { RecurringRuleFields } from "@/bindings/RecurringRuleFields";
import {
  ACCOUNT_LIST,
  CATEGORIES,
  DEFAULTS,
  PAYMENT_METHODS,
  TODAY,
} from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const rule = (
  over: Partial<RecurringRule> & Pick<RecurringRule, "id">,
): RecurringRule => ({
  transaction: {
    kind: "expense",
    amount: 12000,
    date: "2026-01-31",
    accountId: 2,
    toAccountId: null,
    categoryId: 4,
    payee: "Bus pass",
    paymentMethodId: null,
    note: null,
  },
  frequency: "monthly",
  interval: 1,
  endDate: null,
  maxOccurrences: null,
  mode: "confirm",
  paused: false,
  nextDate: "2026-11-30",
  ...over,
});

const BUS = rule({ id: 1 });
const RULES: RecurringRule[] = [
  rule({
    id: 2,
    transaction: {
      ...BUS.transaction,
      kind: "income",
      amount: 420000,
      date: "2026-10-25",
      categoryId: 3,
      payee: null,
    },
    mode: "auto",
    nextDate: "2026-11-25",
  }),
  BUS,
  rule({
    id: 3,
    transaction: { ...BUS.transaction, date: "2026-11-02", payee: null },
    frequency: "weekly",
    paused: true,
    nextDate: "2026-11-09",
    mode: "auto",
  }),
  rule({
    id: 4,
    transaction: {
      ...BUS.transaction,
      kind: "transfer",
      amount: 50000,
      date: "2026-03-15",
      toAccountId: 3,
      categoryId: null,
      payee: null,
    },
    frequency: "yearly",
    interval: 2,
    nextDate: "2028-03-15",
  }),
  rule({
    id: 5,
    transaction: { ...BUS.transaction, payee: "Old gym" },
    nextDate: null,
    maxOccurrences: 3,
  }),
];

const ANSWERS: Answers = {
  list_recurring_rules: RULES,
  list_accounts: ACCOUNT_LIST,
  list_categories: CATEGORIES,
  list_payment_methods: PAYMENT_METHODS,
  get_transaction_defaults: DEFAULTS,
  suggest_payees: [],
  create_recurring_rule: ({ rule }: { rule: RecurringRuleFields }) => ({
    id: 9,
    ...rule,
    paused: false,
    nextDate: rule.transaction.date,
  }),
  update_recurring_rule: ({ id }: { id: number }) =>
    RULES.find((r) => r.id === id),
  pause_recurring_rule: ({ id }: { id: number }) =>
    RULES.find((r) => r.id === id),
  delete_recurring_rule: null,
};

const calls = (command: string) =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === command)
    .map(([, args]) => args);
const form = () => within(screen.getByRole("dialog"));
const text = (name: string) =>
  form().getByRole("textbox", { name }) as HTMLInputElement;
const type = (el: HTMLElement, value: string) => {
  el.focus();
  fireEvent.change(el, { target: { value } });
};

async function open(answers: Answers = ANSWERS) {
  renderRoute("/recurring", answers);
  await screen.findByText("Transportation · Bus pass");
}

/** The ⋯ menu of the rule whose row reads `title`. */
async function menu(title: string, item: string) {
  fireEvent.keyDown(
    screen.getAllByRole("button", { name: `More actions for ${title}` })[0],
    { key: "Enter" },
  );
  fireEvent.click(await screen.findByRole("menuitem", { name: item }));
}

test("each rule shows its schedule, next date, amount and mode, or Paused (§8.2)", async () => {
  await open();
  const rows = screen
    .getAllByText(/ · next | · ended|^Weekly/)
    .map((caption) => caption.closest("button")!.textContent);
  expect(rows).toEqual([
    "SalaryMonthly on the 25th · next Nov 25+₱4,200.00Auto",
    "Transportation · Bus passMonthly on the 31st · next Nov 30−₱120.00Confirm",
    "TransportationWeekly on Mondays−₱120.00Paused",
    "Bank → E-walletEvery 2 years on Mar 15 · next Mar 15, 2028₱500.00Confirm",
    "Transportation · Old gymMonthly on the 31st · ended−₱120.00Confirm",
  ]);
});

test("with no rules, an empty state offers New rule", async () => {
  renderRoute("/recurring", { ...ANSWERS, list_recurring_rules: [] });
  expect(
    await screen.findByText("No recurring transactions yet."),
  ).toBeTruthy();
  expect(screen.getAllByRole("button", { name: "New rule" })).toHaveLength(2);
});

test("⋯ pauses and resumes a rule, and deletes one after asking (REC-5)", async () => {
  await open();
  await menu("Salary", "Pause");
  expect(await screen.findByText("Paused", { selector: "li *" })).toBeTruthy();
  await menu("Transportation", "Resume");
  await waitFor(() =>
    expect(calls("pause_recurring_rule")).toEqual([
      { id: 2, paused: true },
      { id: 3, paused: false },
    ]),
  );

  await menu("Bank → E-wallet", "Delete");
  const confirm = within(
    await screen.findByRole("dialog", { name: "Delete this rule?" }),
  );
  expect(
    confirm.getByText("The transactions it already created stay."),
  ).toBeTruthy();
  fireEvent.click(confirm.getByRole("button", { name: "Delete" }));
  await waitFor(() =>
    expect(calls("delete_recurring_rule")).toEqual([{ id: 4 }]),
  );
});

test("New rule is the transaction form plus a schedule, and saves the rule (REC-1)", async () => {
  await open();
  fireEvent.click(screen.getByRole("button", { name: "New rule" }));
  await screen.findByRole("dialog", { name: "New rule" });
  await waitFor(() => expect(text("Starts").value).toBe("11/05/2026"));
  expect(
    form().getByRole("radio", { name: "Monthly" }).getAttribute("aria-checked"),
  ).toBe("true");
  expect(form().getByText("month")).toBeTruthy();
  expect(
    form().queryByRole("button", { name: "Save & add another" }),
  ).toBeNull();

  type(text("Amount"), "150");
  type(form().getByRole("combobox", { name: "Category" }), "din");
  fireEvent.keyDown(document.activeElement!, { key: "Enter" });
  fireEvent.click(form().getByRole("radio", { name: "Weekly" }));
  type(text("Every"), "2");
  expect(form().getByText("weeks")).toBeTruthy();
  fireEvent.click(form().getByRole("radio", { name: "Number of times" }));
  type(text("Ends after"), "6");
  fireEvent.click(form().getByRole("radio", { name: "Confirm" }));
  fireEvent.click(form().getByRole("button", { name: "Save" }));

  await waitFor(() =>
    expect(calls("create_recurring_rule")).toEqual([
      {
        rule: {
          transaction: {
            kind: "expense",
            amount: 15000,
            date: TODAY,
            accountId: 4,
            toAccountId: null,
            categoryId: 2,
            payee: null,
            paymentMethodId: null,
            note: null,
          },
          frequency: "weekly",
          interval: 2,
          endDate: null,
          maxOccurrences: 6,
          mode: "confirm",
        },
      },
    ]),
  );
  expect(await screen.findByText("Saved")).toBeTruthy();
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
});

test("editing the template keeps the start date, so the rule keeps its place (REC-5)", async () => {
  await open();
  fireEvent.click(screen.getByText("Transportation · Bus pass"));
  await screen.findByRole("dialog", { name: "Edit rule" });
  await waitFor(() => expect(text("Amount").value).toBe("120.00"));
  type(text("Amount"), "130");
  fireEvent.click(form().getByRole("button", { name: "Save" }));
  await waitFor(() =>
    expect(calls("update_recurring_rule")).toEqual([
      {
        id: 1,
        rule: {
          transaction: { ...BUS.transaction, amount: 13000 },
          frequency: "monthly",
          interval: 1,
          endDate: null,
          maxOccurrences: null,
          mode: "confirm",
        },
      },
    ]),
  );
});

test("a new interval starts the rule again from its next date (DATA_MODEL §4.1)", async () => {
  await open();
  fireEvent.click(screen.getByText("Transportation · Bus pass"));
  await screen.findByRole("dialog", { name: "Edit rule" });
  await waitFor(() => expect(text("Starts").value).toBe("01/31/2026"));
  type(text("Every"), "2");
  expect(text("Starts").value).toBe("11/30/2026");
  fireEvent.click(form().getByRole("button", { name: "Save" }));
  await waitFor(() =>
    expect(calls("update_recurring_rule")).toMatchObject([
      {
        id: 1,
        rule: { interval: 2, transaction: { date: "2026-11-30" } },
      },
    ]),
  );
});

test("a schedule's error shows under its field, and the form stays open", async () => {
  await open({
    ...ANSWERS,
    create_recurring_rule: () => {
      throw {
        code: "validation",
        field: "interval",
        message: "Enter a number from 1 to 999",
      };
    },
  });
  fireEvent.click(screen.getByRole("button", { name: "New rule" }));
  await screen.findByRole("dialog", { name: "New rule" });
  await waitFor(() => expect(text("Every").value).toBe("1"));
  type(text("Every"), "0");
  fireEvent.click(form().getByRole("button", { name: "Save" }));
  expect(await form().findByText("Enter a number from 1 to 999")).toBeTruthy();
  expect(text("Every").getAttribute("aria-invalid")).toBe("true");
});
