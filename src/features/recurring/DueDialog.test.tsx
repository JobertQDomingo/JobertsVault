import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { DueOccurrence } from "@/bindings/DueOccurrence";
import type { TransactionFields } from "@/bindings/TransactionFields";
import {
  ACCOUNT_LIST,
  CATEGORIES,
  DASHBOARD,
  DEFAULTS,
  PAYMENT_METHODS,
  PERIOD,
  TODAY,
} from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const WATER: DueOccurrence = {
  id: 11,
  ruleId: 1,
  isPayday: false,
  early: false,
  transaction: {
    kind: "expense",
    amount: 3240,
    date: "2026-10-24",
    accountId: 2,
    toAccountId: null,
    categoryId: 1,
    payee: "Market",
    paymentMethodId: null,
    note: null,
  },
};
const SAVINGS: DueOccurrence = {
  id: 12,
  ruleId: 2,
  isPayday: false,
  early: false,
  transaction: {
    ...WATER.transaction,
    kind: "transfer",
    amount: 64000,
    date: TODAY,
    toAccountId: 4,
    categoryId: null,
    payee: null,
  },
};

const salary = {
  ...WATER.transaction,
  kind: "income",
  amount: 420000,
  categoryId: 3,
  payee: "Employer",
} as const;
/** A payday due yesterday, and one 2 days ahead that can be recorded early (PAY-4). */
const PAYDAY: DueOccurrence = {
  id: 13,
  ruleId: 3,
  isPayday: true,
  early: false,
  transaction: { ...salary, date: "2026-11-04" },
};
const EARLY: DueOccurrence = {
  id: null,
  ruleId: 4,
  isPayday: true,
  early: true,
  transaction: { ...salary, date: "2026-11-07" },
};

/** The app with `due` waiting; posting or skipping one takes it off the list. */
function renderDue(due: DueOccurrence[], answers: Answers = {}) {
  let left = [...due];
  const settle = ({ id }: { id: number }) => {
    left = left.filter((o) => o.id !== id);
    return null;
  };
  renderRoute("/", {
    // The banner counts only the dates due, never the early ones (F10 step 4).
    get_dashboard: () => ({
      ...DASHBOARD,
      recurringDue: left.filter((o) => !o.early).length,
    }),
    get_period: PERIOD,
    list_categories: CATEGORIES,
    list_accounts: ACCOUNT_LIST,
    list_payment_methods: PAYMENT_METHODS,
    get_transaction_defaults: DEFAULTS,
    suggest_payees: [],
    list_due_occurrences: () => left,
    post_occurrence: settle,
    skip_occurrence: settle,
    ...answers,
  });
}

const calls = (command: string) =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === command)
    .map(([, args]) => args);
const review = () =>
  within(screen.getByRole("dialog", { name: "Recurring transactions due" }));
const row = (text: string) =>
  within(review().getByText(text).closest<HTMLElement>("li")!);

async function openReview(due: DueOccurrence[], answers?: Answers) {
  renderDue(due, answers);
  fireEvent.click(await screen.findByRole("button", { name: "Review" }));
  // The dashboard's recent list has rows like these too, so wait inside the dialog.
  await review().findAllByRole("listitem");
}

test("the banner counts the dates due, and Review lists each one oldest first (F10)", async () => {
  await openReview([WATER, SAVINGS]);
  expect(screen.getByText("2 recurring transactions are due.")).toBeTruthy();
  const rows = review().getAllByRole("listitem");
  expect(rows.map((r) => r.textContent)).toEqual([
    "Food & Groceries · MarketDue Sat, Oct 24 · Bank−₱32.40PostEdit & postSkip",
    "Bank → Credit cardDue today₱640.00PostEdit & postSkip",
  ]);
  // Every row has the same buttons, so each says which date it settles.
  for (const button of within(rows[0]).getAllByRole("button")) {
    const id = button.getAttribute("aria-describedby")!;
    expect(document.getElementById(id)!.textContent).toBe(
      "Food & Groceries · MarketDue Sat, Oct 24 · Bank",
    );
  }
});

test("one date due reads in the singular", async () => {
  renderDue([WATER]);
  expect(
    await screen.findByText("1 recurring transaction is due."),
  ).toBeTruthy();
});

test("no banner while nothing is due", async () => {
  renderDue([]);
  await screen.findByText("Recent transactions");
  expect(screen.queryByRole("button", { name: "Review" })).toBeNull();
});

test("Post posts a date as it is, Skip settles one, and the review closes once none is left", async () => {
  await openReview([WATER, SAVINGS]);
  fireEvent.click(
    row("Food & Groceries · Market").getByRole("button", { name: "Post" }),
  );
  expect(await screen.findByText("Posted")).toBeTruthy();
  expect(calls("post_occurrence")).toEqual([{ id: 11, transaction: null }]);
  await waitFor(() =>
    expect(review().queryByText("Food & Groceries · Market")).toBeNull(),
  );
  expect(screen.getByText("1 recurring transaction is due.")).toBeTruthy();

  fireEvent.click(
    row("Bank → Credit card").getByRole("button", { name: "Skip" }),
  );
  expect(await screen.findByText("Skipped")).toBeTruthy();
  expect(calls("skip_occurrence")).toEqual([{ id: 12 }]);
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(screen.queryByRole("button", { name: "Review" })).toBeNull();
});

test("Edit & post opens the form on the date's fields and posts what's saved", async () => {
  await openReview([WATER, SAVINGS]);
  fireEvent.click(
    row("Food & Groceries · Market").getByRole("button", {
      name: "Edit & post",
    }),
  );
  const form = within(
    await screen.findByRole("dialog", { name: "Edit & post" }),
  );
  const amount = (await form.findByRole("textbox", {
    name: "Amount",
  })) as HTMLInputElement;
  await waitFor(() => expect(amount.value).toBe("32.40"));
  const combo = (name: string) =>
    (form.getByRole("combobox", { name }) as HTMLInputElement).value;
  expect([combo("Category"), combo("Account"), combo("Payee")]).toEqual([
    "Food & Groceries",
    "Bank",
    "Market",
  ]);
  expect(form.queryByRole("button", { name: "Save & add another" })).toBeNull();

  fireEvent.change(amount, { target: { value: "35" } });
  fireEvent.click(form.getByRole("button", { name: "Save" }));
  expect(await screen.findByText("Posted")).toBeTruthy();
  const edited: TransactionFields = { ...WATER.transaction, amount: 3500 };
  expect(calls("post_occurrence")).toEqual([{ id: 11, transaction: edited }]);
  await waitFor(() =>
    expect(screen.queryByRole("dialog", { name: "Edit & post" })).toBeNull(),
  );
  expect(review().getAllByRole("listitem")).toHaveLength(1);
});

test("a date that can't be posted says why in a toast and stays on the list", async () => {
  await openReview([WATER], {
    post_occurrence: () => {
      throw {
        code: "conflict",
        message: "This date was already posted or skipped",
      };
    },
  });
  fireEvent.click(review().getByRole("button", { name: "Post" }));
  expect(
    await screen.findByText("This date was already posted or skipped"),
  ).toBeTruthy();
  expect(review().getByText("Food & Groceries · Market")).toBeTruthy();
});

test("a payday has Record payslip and Skip, an early one only Record now, and the banner counts the due ones (F10 step 4)", async () => {
  await openReview([WATER, PAYDAY, EARLY]);
  expect(screen.getByText("2 recurring transactions are due.")).toBeTruthy();
  const rows = review().getAllByRole("listitem");
  expect(rows.map((r) => r.textContent)).toEqual([
    "Food & Groceries · MarketDue Sat, Oct 24 · Bank−₱32.40PostEdit & postSkip",
    "Salary · EmployerDue yesterday · Bank+₱4,200.00Record payslipSkip",
    "Salary · EmployerPayday Sat, Nov 7 · Bank+₱4,200.00Record now",
  ]);
  const early = within(rows[2]).getByRole("button", { name: "Record now" });
  expect(
    document.getElementById(early.getAttribute("aria-describedby")!)!
      .textContent,
  ).toBe("Salary · EmployerPayday Sat, Nov 7 · Bank");

  fireEvent.click(within(rows[1]).getByRole("button", { name: "Skip" }));
  expect(await screen.findByText("Skipped")).toBeTruthy();
  expect(calls("skip_occurrence")).toEqual([{ id: 13 }]);
});

test("the first payslip starts with no gross, no net and no error (F14)", async () => {
  await openReview([WATER, EARLY], {
    get_payslip_defaults: {
      accountId: 2,
      categoryId: 3,
      date: TODAY,
      gross: null,
      deductions: [],
    },
    preview_payslip: ({ payslip }: { payslip: { gross: number } }) => ({
      net: payslip.gross,
    }),
  });
  fireEvent.click(review().getByRole("button", { name: "Record now" }));
  const form = within(
    await screen.findByRole("dialog", { name: "Payslip · Salary" }),
  );
  const gross = (await form.findByRole("textbox", {
    name: "Gross pay",
  })) as HTMLInputElement;
  expect(gross.value).toBe("");
  expect(form.getByTestId("payslip-net").textContent).toBe("");
  expect(form.queryByRole("alert")).toBeNull();
  // An empty gross isn't asked about, so typing one never flashes a stale ₱0.00 and its error.
  expect(calls("preview_payslip")).toEqual([]);
  fireEvent.change(gross, { target: { value: "4" } });
  expect(form.queryByRole("alert")).toBeNull();
  expect(
    await form.findByText("₱4.00", { selector: "[data-testid=payslip-net]" }),
  ).toBeTruthy();
  expect(form.queryByRole("alert")).toBeNull();
});

test("Record now opens the payslip for that payday, and saving records it on its date (F14)", async () => {
  await openReview([WATER, EARLY], {
    list_categories: {
      ...CATEGORIES,
      categories: [
        ...CATEGORIES.categories,
        {
          ...CATEGORIES.categories[0],
          id: 7,
          name: "SSS",
          icon: "shield",
        },
      ],
    },
    get_payslip_defaults: {
      accountId: 2,
      categoryId: 3,
      date: TODAY,
      gross: 460000,
      deductions: [{ categoryId: 7, amount: 18000 }],
    },
    // The backend's answer, which the form only shows.
    preview_payslip: ({ payslip }: { payslip: { gross: number } }) => ({
      net: payslip.gross === 460000 ? 442000 : 0,
    }),
    record_payslip: () => null,
  });
  fireEvent.click(review().getByRole("button", { name: "Record now" }));
  const form = within(
    await screen.findByRole("dialog", { name: "Payslip · Salary" }),
  );
  expect(calls("get_payslip_defaults")).toEqual([
    { ruleId: 4, date: "2026-11-07" },
  ]);
  expect(await form.findByText("₱4,420.00")).toBeTruthy();
  expect(
    (
      form.getByRole("combobox", {
        name: "Deduction 1 category",
      }) as HTMLInputElement
    ).value,
  ).toBe("SSS");
  fireEvent.click(form.getByRole("button", { name: "Save" }));
  expect(await screen.findByText("Payslip saved")).toBeTruthy();
  expect(calls("record_payslip")).toEqual([
    {
      ruleId: 4,
      date: "2026-11-07",
      payslip: {
        date: TODAY,
        accountId: 2,
        gross: 460000,
        deductions: [{ categoryId: 7, amount: 18000 }],
        note: null,
      },
    },
  ]);
});

test("the review stays open while an early payday is left, though nothing is due (F10)", async () => {
  await openReview([WATER, EARLY]);
  fireEvent.click(review().getByRole("button", { name: "Post" }));
  await waitFor(() => expect(calls("post_occurrence")).toHaveLength(1));
  await waitFor(() =>
    expect(screen.queryByRole("button", { name: "Review" })).toBeNull(),
  );
  expect(review().getByRole("button", { name: "Record now" })).toBeTruthy();
});
