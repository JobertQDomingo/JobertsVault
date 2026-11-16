import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { LoanList } from "@/bindings/LoanList";
import type { TransactionFields } from "@/bindings/TransactionFields";
import {
  ACCOUNT_LIST,
  DEFAULTS,
  LOAN_CATEGORIES,
  LOAN_PREVIEW,
  LOANS,
  PAYMENT_METHODS,
} from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const ANSWERS: Answers = {
  // Quick-add's card spending level check (CC-11) asks nothing here.
  preview_card_spend: null,
  list_loans: LOANS,
  list_accounts: ACCOUNT_LIST,
  list_categories: LOAN_CATEGORIES,
  get_transaction_defaults: DEFAULTS,
  list_payment_methods: PAYMENT_METHODS,
  suggest_payees: [],
  archive_loan: null,
  unarchive_loan: null,
  delete_loan: null,
  mark_loan_payment_paid: ({
    transaction,
  }: {
    transaction: TransactionFields;
  }) => ({
    id: 99,
    loanPaymentId: 14,
    ...transaction,
  }),
  mark_loan_payment_unpaid: null,
  create_transaction: ({
    transaction,
  }: {
    transaction: TransactionFields;
  }) => ({
    id: 98,
    loanPaymentId: null,
    ...transaction,
  }),
};

const calls = (command: string) =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === command)
    .map(([, args]) => args);
const card = async (name: string) =>
  within(
    (
      await screen.findByRole("button", { name: `Open ${name}` })
    ).closest<HTMLElement>("li")!,
  );
const openMenu = async (name: string) =>
  fireEvent.keyDown(
    await screen.findByRole("button", { name: `More actions for ${name}` }),
    { key: "Enter" },
  );
const menuItems = () =>
  within(screen.getByRole("menu"))
    .getAllByRole("menuitem")
    .map((i) => i.textContent);
const withSummary = (
  over: Partial<LoanList["summary"]>,
  loans = LOANS.loans,
) => ({
  loans,
  summary: { ...LOANS.summary, ...over },
});

test("the top panel shows the debt-free date, the totals, the streak and the interest (LOAN-5 to LOAN-8)", async () => {
  renderRoute("/debt-free", ANSWERS);
  await screen.findByText("Debt-free on Sun, Mar 28, 2027");
  screen.getByText("143 days to go");
  screen.getByText("₱8,000.00");
  screen.getByText(/left · ₱5,000.00 paid/);
  expect(
    screen
      .getByRole("progressbar", { name: "Loans paid" })
      .getAttribute("aria-valuenow"),
  ).toBe("38");
  screen.getByText("38% paid");
  screen.getByText(
    "51 days since your last new loan · ₱1,000.00 in interest on 1 loan",
  );
});

test("the streak and interest hide when there's none, and one day is singular", async () => {
  renderRoute("/debt-free", {
    ...ANSWERS,
    list_loans: withSummary({ streakDays: null, interest: null, daysToGo: 1 }),
  });
  await screen.findByText("1 day to go");
  expect(screen.queryByText(/since your last new loan|in interest/)).toBeNull();
});

test("the last payment due today counts down to today, not to an overdue warning", async () => {
  renderRoute("/debt-free", {
    ...ANSWERS,
    list_loans: withSummary({ debtFreeOn: "2026-11-05", daysToGo: 0 }),
  });
  await screen.findByText("Last payment today");
  expect(screen.queryByText(/overdue\. Pay it/)).toBeNull();
});

test("with nothing left it says You're debt-free.", async () => {
  const paidOff = LOANS.loans.filter((l) => l.paidOff);
  renderRoute("/debt-free", {
    ...ANSWERS,
    list_loans: withSummary(
      {
        debtFreeOn: null,
        daysToGo: null,
        totalLeft: 0,
        percentPaid: 100,
        freedUp: [],
      },
      paidOff,
    ),
  });
  await screen.findByText("You're debt-free.");
  expect(screen.queryByText(/days to go/)).toBeNull();
  screen.getByText("No active loans.");
  expect(screen.queryByText("Money freed up")).toBeNull();
});

test("when everything left is overdue it says what's owed instead of the date (DATA_MODEL §5.11)", async () => {
  const bank = LOANS.loans[1];
  const overdue = {
    ...bank,
    payments: bank.payments.map((p) => ({ ...p, overdue: true })),
  };
  renderRoute("/debt-free", {
    ...ANSWERS,
    list_loans: withSummary(
      { totalLeft: 300000, debtFreeOn: "2026-11-05", daysToGo: 0 },
      [overdue],
    ),
  });
  await screen.findByText("₱3,000.00 overdue. Pay it to be debt-free.");
  expect(screen.queryByText(/Debt-free on|to go/)).toBeNull();
});

test("each loan is a card: account, count paid, % paid, what's left, interest and the next payment", async () => {
  renderRoute("/debt-free", ANSWERS);
  const ewallet = await card("E-wallet loan");
  ewallet.getByText("E-wallet · 3 of 8 paid");
  expect(ewallet.getByRole("progressbar").getAttribute("aria-valuenow")).toBe(
    "37",
  );
  ewallet.getByText("₱5,000.00 left · ₱1,000.00 interest");
  expect(ewallet.getByText(/^Next/).textContent).toBe(
    "Next ₱1,000.00 due Sat, Nov 28",
  );
  const bank = await card("Bank loan");
  bank.getByText("Bank · 0 of 3 paid");
  bank.getByText("₱3,000.00 left");
  const overdue = bank.getByText("Next ₱1,000.00 overdue since Sat, Oct 10");
  expect(overdue.className).toContain("text-danger");
  // Archived loans are folded away; the rest keep the backend's order.
  expect(
    screen
      .getAllByRole("button", { name: /^Open / })
      .map((b) => b.getAttribute("aria-label")),
  ).toEqual(["Open E-wallet loan", "Open Bank loan"]);
  fireEvent.click(screen.getByRole("button", { name: "Show archived (1)" }));
  const phone = await card("Phone loan");
  phone.getByText("Paid off");
  expect(phone.queryByText(/^Next/)).toBeNull();
});

test("the ⋯ menu offers Archive only when paid off, Delete only when none was marked paid, and Unarchive when archived", async () => {
  renderRoute("/debt-free", ANSWERS);
  await openMenu("E-wallet loan");
  expect(menuItems()).toEqual(["Edit"]);
  fireEvent.keyDown(screen.getByRole("menu"), { key: "Escape" });
  await openMenu("Bank loan");
  expect(menuItems()).toEqual(["Edit", "Delete"]);
  fireEvent.click(screen.getByRole("menuitem", { name: "Delete" }));
  const confirm = await screen.findByRole("dialog", {
    name: "Delete this loan?",
  });
  within(confirm).getByText("Its payments are deleted. This can't be undone.");
  fireEvent.click(within(confirm).getByRole("button", { name: "Delete" }));
  await waitFor(() => expect(calls("delete_loan")).toEqual([{ id: 2 }]));
  await screen.findByText("Deleted");

  fireEvent.click(screen.getByRole("button", { name: "Show archived (1)" }));
  await openMenu("Phone loan");
  expect(menuItems()).toEqual(["Unarchive"]);
  fireEvent.click(screen.getByRole("menuitem", { name: "Unarchive" }));
  await waitFor(() => expect(calls("unarchive_loan")).toEqual([{ id: 3 }]));
});

test("a paid-off active loan can be archived", async () => {
  const done = { ...LOANS.loans[2], archived: false };
  renderRoute("/debt-free", {
    ...ANSWERS,
    list_loans: withSummary({}, [done]),
  });
  await openMenu("Phone loan");
  expect(menuItems()).toEqual(["Edit", "Archive", "Delete"]);
  fireEvent.click(screen.getByRole("menuitem", { name: "Archive" }));
  await waitFor(() => expect(calls("archive_loan")).toEqual([{ id: 3 }]));
  await screen.findByText("Archived");
});

test("with no loans the screen is one empty state with Add loan", async () => {
  renderRoute("/debt-free", {
    ...ANSWERS,
    list_loans: withSummary({ debtFreeOn: null, freedUp: [] }, []),
  });
  await screen.findByText("No loans. Keep it that way.");
  expect(screen.queryByText("Loans")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Add loan" }));
  await screen.findByRole("dialog", { name: "Add loan" });
});

test("Money freed up has a row per month, compared with this month (LOAN-7)", async () => {
  renderRoute("/debt-free", ANSWERS);
  const panel = within(
    (await screen.findByText("Money freed up")).closest<HTMLElement>(
      "section, div",
    )!,
  );
  const rows = panel.getAllByRole("listitem").map((li) => li.textContent);
  expect(rows).toEqual([
    "November 2026This month₱3,000.00 to loans",
    "December 2026₱1,000.00 less than this month₱2,000.00 to loans",
    "January 2027₱1,000.00 more than this month₱4,000.00 to loans",
    "February 2027Same as this month₱3,000.00 to loans",
    "April 2027₱3,000.00 less than this month₱0.00",
  ]);
});

test("a card opens its payments: paid before, paid with its date, overdue and unpaid", async () => {
  renderRoute("/debt-free", ANSWERS);
  fireEvent.click(
    await screen.findByRole("button", { name: "Open E-wallet loan" }),
  );
  const dialog = within(
    await screen.findByRole("dialog", { name: "E-wallet loan" }),
  );
  const rows = dialog.getAllByRole("listitem");
  expect(rows).toHaveLength(8);
  within(rows[0]).getByText("Fri, Aug 28, 2026");
  within(rows[0]).getByText("Paid before");
  within(rows[2]).getByText("Paid Oct 27");
  within(rows[3]).getByText("Unpaid");
  // The next payment's Mark paid is the main one.
  expect(
    within(rows[3]).getByRole("button", { name: "Mark paid" }).className,
  ).toContain("bg-primary");
  expect(
    within(rows[4]).getByRole("button", { name: "Mark paid" }).className,
  ).not.toContain("bg-primary");
  expect(within(rows[0]).queryByRole("button")).toBeNull();
});

test("an overdue payment says so in red, with Mark paid", async () => {
  renderRoute("/debt-free", ANSWERS);
  fireEvent.click(
    await screen.findByRole("button", { name: "Open Bank loan" }),
  );
  const dialog = within(
    await screen.findByRole("dialog", { name: "Bank loan" }),
  );
  const first = within(dialog.getAllByRole("listitem")[0]);
  expect(first.getByText("Overdue").className).toContain("text-danger");
  first.getByRole("button", { name: "Mark paid" });
});

test("Mark paid opens the transaction form filled in, fixed to Expense, and records it (F13)", async () => {
  renderRoute("/debt-free", ANSWERS);
  fireEvent.click(
    await screen.findByRole("button", { name: "Open E-wallet loan" }),
  );
  const payments = await screen.findByRole("dialog", { name: "E-wallet loan" });
  fireEvent.click(
    within(within(payments).getAllByRole("listitem")[3]).getByRole("button", {
      name: "Mark paid",
    }),
  );
  const form = within(
    await screen.findByRole("dialog", { name: /^Mark paid/ }),
  );
  await waitFor(() =>
    expect(
      (form.getByRole("textbox", { name: "Amount" }) as HTMLInputElement).value,
    ).toBe("1,000.00"),
  );
  expect(
    (form.getByRole("combobox", { name: "Category" }) as HTMLInputElement)
      .value,
  ).toBe("Loan");
  expect(
    (form.getByRole("combobox", { name: "Account" }) as HTMLInputElement).value,
  ).toBe("E-wallet");
  expect(form.queryByRole("radiogroup", { name: "Type" })).toBeNull();
  expect(form.queryByRole("button", { name: "Save & add another" })).toBeNull();
  // A late fee.
  const amount = form.getByRole("textbox", { name: "Amount" });
  fireEvent.change(amount, { target: { value: "1050" } });
  fireEvent.click(form.getByRole("button", { name: "Save" }));
  await waitFor(() =>
    expect(calls("mark_loan_payment_paid")).toEqual([
      {
        paymentId: 14,
        transaction: {
          kind: "expense",
          amount: 105000,
          date: DEFAULTS.today,
          accountId: 3,
          toAccountId: null,
          categoryId: 6,
          payee: "E-wallet loan",
          paymentMethodId: null,
          note: null,
        },
      },
    ]),
  );
  await screen.findByText("Marked paid");
  await waitFor(() =>
    expect(screen.queryByRole("dialog", { name: /^Mark paid/ })).toBeNull(),
  );
});

test("Mark paid keeps the loan's archived account and category", async () => {
  const ewallet = { ...LOANS.loans[0], accountId: 5, categoryId: 5 };
  renderRoute("/debt-free", {
    ...ANSWERS,
    list_loans: withSummary({}, [ewallet]),
  });
  fireEvent.click(
    await screen.findByRole("button", { name: "Open E-wallet loan" }),
  );
  const payments = await screen.findByRole("dialog", { name: "E-wallet loan" });
  fireEvent.click(
    within(within(payments).getAllByRole("listitem")[3]).getByRole("button", {
      name: "Mark paid",
    }),
  );
  const form = within(
    await screen.findByRole("dialog", { name: /^Mark paid/ }),
  );
  await waitFor(() =>
    expect(
      (form.getByRole("combobox", { name: "Account" }) as HTMLInputElement)
        .value,
    ).toBe("Old Wallet (archived)"),
  );
  expect(
    (form.getByRole("combobox", { name: "Category" }) as HTMLInputElement)
      .value,
  ).toBe("Snacks (archived)");
});

test("Mark unpaid asks first, then deletes the expense", async () => {
  renderRoute("/debt-free", ANSWERS);
  fireEvent.click(
    await screen.findByRole("button", { name: "Open E-wallet loan" }),
  );
  const dialog = within(
    await screen.findByRole("dialog", { name: "E-wallet loan" }),
  );
  fireEvent.keyDown(
    dialog.getByRole("button", { name: "More actions for Wed, Oct 28, 2026" }),
    { key: "Enter" },
  );
  fireEvent.click(await screen.findByRole("menuitem", { name: "Mark unpaid" }));
  const confirm = await screen.findByRole("dialog", {
    name: "Mark this payment unpaid?",
  });
  within(confirm).getByText("The expense recorded for it is deleted.");
  fireEvent.click(within(confirm).getByRole("button", { name: "Mark unpaid" }));
  await waitFor(() =>
    expect(calls("mark_loan_payment_unpaid")).toEqual([{ paymentId: 13 }]),
  );
  await screen.findByText("Marked unpaid");
});

test("Edit in the payments dialog opens the loan's form", async () => {
  renderRoute("/debt-free", ANSWERS);
  fireEvent.click(
    await screen.findByRole("button", { name: "Open E-wallet loan" }),
  );
  const dialog = within(
    await screen.findByRole("dialog", { name: "E-wallet loan" }),
  );
  fireEvent.click(dialog.getByRole("button", { name: "Edit" }));
  await screen.findByRole("dialog", { name: "Edit loan" });
  expect(screen.queryByRole("dialog", { name: "E-wallet loan" })).toBeNull();
});

test("saving any quick-add on this screen refreshes the loans rather than breaking them", async () => {
  renderRoute("/debt-free", ANSWERS);
  await screen.findByText("Debt-free on Sun, Mar 28, 2027");
  fireEvent.keyDown(window, { key: "n", ctrlKey: true });
  const form = within(
    await screen.findByRole("dialog", { name: "Add transaction" }),
  );
  const amount = await form.findByRole("textbox", { name: "Amount" });
  fireEvent.change(amount, { target: { value: "5" } });
  fireEvent.click(form.getByRole("button", { name: "Save" }));
  await waitFor(() => expect(calls("list_loans").length).toBeGreaterThan(1));
  await screen.findByText("Saved");
  expect(screen.queryByRole("button", { name: "Retry" })).toBeNull();
  screen.getByText("Debt-free on Sun, Mar 28, 2027");
});

test("an archived loan's payments have no Edit, as its card offers only Unarchive", async () => {
  renderRoute("/debt-free", ANSWERS);
  fireEvent.click(
    await screen.findByRole("button", { name: "Show archived (1)" }),
  );
  fireEvent.click(
    await screen.findByRole("button", { name: "Open Phone loan" }),
  );
  const dialog = within(
    await screen.findByRole("dialog", { name: "Phone loan" }),
  );
  expect(dialog.queryByRole("button", { name: "Edit" })).toBeNull();
});

test("Mark paid on a payment that's no longer unpaid says so and closes, instead of loading forever", async () => {
  let paid = false;
  const paidLoans = () => {
    const [ewallet, ...rest] = LOANS.loans;
    return {
      ...LOANS,
      loans: [
        {
          ...ewallet,
          payments: ewallet.payments.map((p) =>
            p.id === 14 && paid
              ? { ...p, status: "paid", transactionId: 60 }
              : p,
          ),
        },
        ...rest,
      ],
    };
  };
  renderRoute("/debt-free", { ...ANSWERS, list_loans: paidLoans });
  fireEvent.click(
    await screen.findByRole("button", { name: "Open E-wallet loan" }),
  );
  const payments = await screen.findByRole("dialog", { name: "E-wallet loan" });
  // Paid elsewhere meanwhile (e.g. from the dashboard banner), and the list has caught up.
  paid = true;
  fireEvent.click(
    within(within(payments).getAllByRole("listitem")[3]).getByRole("button", {
      name: "Mark paid",
    }),
  );
  await screen.findByText("This payment is no longer unpaid.");
  await waitFor(() =>
    expect(screen.queryByRole("dialog", { name: /^Mark paid/ })).toBeNull(),
  );
});

// Before you borrow (F15, LOAN-9).

const panel = async () =>
  within(
    (await screen.findByRole("heading", { name: "Before you borrow" }))
      .parentElement!,
  );

test("Before you borrow tries a monthly loan and shows what it would do, saving nothing (F15)", async () => {
  renderRoute("/debt-free", { ...ANSWERS, preview_loan: LOAN_PREVIEW });
  const p = await panel();
  fireEvent.change(p.getByRole("textbox", { name: "Payment" }), {
    target: { value: "1000" },
  });
  fireEvent.change(p.getByRole("textbox", { name: "How many" }), {
    target: { value: "6" },
  });
  // The first payment starts a month from today.
  expect(
    (p.getByRole("textbox", { name: "First payment" }) as HTMLInputElement)
      .value,
  ).toBe("12/05/2026");
  fireEvent.click(p.getByRole("button", { name: "Try it" }));
  await p.findByText("Mar 28, 2027 → May 5, 2027");
  expect(calls("preview_loan")).toEqual([
    { draft: { amount: 100000, count: 6, firstDate: "2026-12-05" } },
  ]);
  p.getByText("(38 days later)", { exact: false });
  p.getByText("₱8,400.00 → ₱7,400.00");
  const windows = within(
    p.getByRole("list", { name: "Pay windows it changes" }),
  )
    .getAllByRole("listitem")
    .map((li) => li.textContent);
  // A window it doesn't change is left out.
  expect(windows).toHaveLength(2);
  expect(windows[0]).toBe("Dec 15–29: ₱18,300.00 → ₱17,300.00");
  const short = p.getByText(/would run short by ₱180\.00/);
  expect(short.closest("li")?.className).toContain("text-danger");
  expect(calls("create_loan")).toEqual([]);
});

test("Before you borrow sends blank fields for the backend to name, and its error shows under the field", async () => {
  renderRoute("/debt-free", {
    ...ANSWERS,
    preview_loan: () => {
      throw {
        code: "validation",
        field: "count",
        message: "Enter 1 to 600 payments",
      };
    },
  });
  const p = await panel();
  fireEvent.click(p.getByRole("button", { name: "Try it" }));
  await p.findByText("Enter 1 to 600 payments");
  expect(calls("preview_loan")).toEqual([
    { draft: { amount: 0, count: 0, firstDate: "2026-12-05" } },
  ]);
  expect(
    p.getByRole("textbox", { name: "How many" }).getAttribute("aria-invalid"),
  ).toBe("true");
});

test("Before you borrow leaves out Safe to spend with no payday rule, and says when the date doesn't move", async () => {
  renderRoute("/debt-free", {
    ...ANSWERS,
    preview_loan: {
      ...LOAN_PREVIEW,
      debtFreeAfter: "2027-03-28",
      daysLater: 0,
      safeBefore: null,
      safeAfter: null,
      windows: [],
    },
  });
  const p = await panel();
  fireEvent.click(p.getByRole("button", { name: "Try it" }));
  await p.findByText("(same day)", { exact: false });
  expect(p.queryByText(/Safe to spend/)).toBeNull();
  expect(p.queryByRole("list", { name: "Pay windows it changes" })).toBeNull();
});

test("with no loans the screen is one empty state, without Before you borrow (§8.5)", async () => {
  renderRoute("/debt-free", {
    ...ANSWERS,
    list_loans: withSummary({}, []),
  });
  await screen.findByText("No loans. Keep it that way.");
  expect(screen.queryByText("Before you borrow")).toBeNull();
});

test("changing a field after Try it clears the result and its error, so nothing shown is for other values", async () => {
  renderRoute("/debt-free", {
    ...ANSWERS,
    preview_loan: () => {
      throw {
        code: "validation",
        field: "count",
        message: "Enter 1 to 600 payments",
      };
    },
  });
  const p = await panel();
  fireEvent.click(p.getByRole("button", { name: "Try it" }));
  await p.findByText("Enter 1 to 600 payments");
  fireEvent.change(p.getByRole("textbox", { name: "How many" }), {
    target: { value: "6" },
  });
  expect(p.queryByText("Enter 1 to 600 payments")).toBeNull();
});

test("Try it again with the same values asks again, so a failure can be retried", async () => {
  let fail = true;
  renderRoute("/debt-free", {
    ...ANSWERS,
    preview_loan: () => {
      if (fail) throw { code: "database", message: "The database is busy" };
      return LOAN_PREVIEW;
    },
  });
  const p = await panel();
  fireEvent.click(p.getByRole("button", { name: "Try it" }));
  await p.findByText(/Something went wrong with the database/);
  fail = false;
  fireEvent.click(p.getByRole("button", { name: "Try it" }));
  await p.findByText("Mar 28, 2027 → May 5, 2027");
  expect(calls("preview_loan")).toHaveLength(2);
});
