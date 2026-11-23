import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { LoanDue } from "@/bindings/LoanDue";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import {
  ACCOUNT_LIST,
  activeFocus,
  CATEGORIES,
  DASHBOARD,
  DEFAULTS,
  LOAN_CATEGORIES,
  LOANS,
  PAY_BILL_DEFAULTS,
  PAYMENT_METHODS,
  PERIOD,
  RECENT,
  UPCOMING,
} from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const ANSWERS: Answers = {
  get_dashboard: DASHBOARD,
  get_period: PERIOD,
  list_categories: CATEGORIES,
};

// A stat card's label: the chart's legend and table repeat the names.
const CARD = { selector: "p" };

/** The glass panel holding a heading or a stat card's label. */
const panel = (name: string) =>
  within(
    screen
      .getByText(name, { selector: "p, h2" })
      .closest<HTMLElement>(".glass")!,
  );

test("the stat cards show the period's income, expense and net cashflow", async () => {
  renderRoute("/", ANSWERS);
  await screen.findByText("Net cashflow", CARD);
  expect(panel("Income").getByText("+₱4,200.00")).toBeTruthy();
  expect(panel("Expense").getByText("−₱2,950.40")).toBeTruthy();
  expect(panel("Net cashflow").getByText("+₱1,249.60")).toBeTruthy();
});

test("each stat card shows its % change from the comparison (DSH-3)", async () => {
  renderRoute("/", ANSWERS);
  await screen.findByText("Net cashflow", CARD);
  for (const [card, change] of [
    ["Income", "▲ Up 3.1%"],
    ["Expense", "▼ Down 8.0%"],
    ["Net cashflow", "▲ Up 44.1%"],
  ]) {
    const caption = panel(card).getByText("vs Oct 1–5");
    expect(caption.previousElementSibling!.textContent).toBe(change);
  }
});

test("the 5 newest transactions, grouped by day, never without their names", async () => {
  renderRoute("/", ANSWERS);
  await screen.findByText("Recent transactions");
  const recent = panel("Recent transactions");
  for (const text of [
    "Food & Groceries · Market",
    "Today",
    "Yesterday",
    "Mon, Oct 26",
    "Bank → Credit card",
    "Dining Out · Corner Cafe",
  ])
    expect(recent.getByText(text)).toBeTruthy();
  expect(recent.getAllByRole("button")).toHaveLength(5);
});

test("the income-vs-expense chart has the period's bars, in a table for screen readers", async () => {
  renderRoute("/", ANSWERS);
  const table = await screen.findByRole("table", {
    name: "Income, expense and net cashflow by day",
  });
  expect(within(table).getAllByRole("row")).toHaveLength(31);
  const day = within(table).getByRole("row", { name: /Wed, Nov 4, 2026/ });
  for (const amount of ["+₱4,200.00", "−₱2,852.70", "+₱1,347.30"])
    expect(within(day).getByText(amount)).toBeTruthy();
});

test("clicking a recent transaction opens it for editing", async () => {
  renderRoute("/", ANSWERS);
  fireEvent.click(await screen.findByText("Food & Groceries · Market"));
  expect(
    await screen.findByRole("dialog", { name: "Edit transaction" }),
  ).toBeTruthy();
});

test("View all goes to Transactions, Manage to Accounts", async () => {
  const { router } = renderRoute("/", ANSWERS);
  await screen.findByText("Recent transactions", { selector: "h2" });
  fireEvent.click(
    panel("Recent transactions").getByRole("link", { name: "View all" }),
  );
  await waitFor(() =>
    expect(router.state.location.pathname).toBe("/transactions"),
  );
  await router.navigate("/");
  await screen.findByText("Accounts", { selector: "h2" });
  fireEvent.click(panel("Accounts").getByRole("link", { name: "Manage" }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/accounts"));
});

test("accounts: the active ones, the card's details and the backend's net balance", async () => {
  renderRoute("/", ANSWERS);
  await screen.findByText("Net balance");
  const accounts = panel("Accounts");
  const names = accounts
    .getAllByRole("link")
    .filter((a) => a.getAttribute("href")?.startsWith("/accounts/"))
    .map((a) => a.querySelector("p")?.textContent);
  expect(names).toEqual(["Cash", "Bank", "E-wallet", "Credit card"]);
  expect(accounts.getByText("₱640.00 owed")).toBeTruthy();
  expect(accounts.getByText(/Due Nov 5/)).toBeTruthy();
  // The net balance includes the archived Old Wallet, so it isn't the rows' sum.
  expect(accounts.getByText("₱8,285.75")).toBeTruthy();
});

test("clicking an account opens its details", async () => {
  const { router } = renderRoute("/", ANSWERS);
  fireEvent.click(await screen.findByRole("link", { name: /^Bank/ }));
  await waitFor(() =>
    expect(router.state.location.pathname).toBe("/accounts/2"),
  );
});

test("with no transactions yet, it says how to add one and the totals are 0", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: { ...DASHBOARD, income: 0, expense: 0, net: 0, recent: [] },
  });
  expect(
    await screen.findByText(
      "No transactions yet. Press Ctrl+N to add your first one.",
    ),
  ).toBeTruthy();
  expect(
    screen.getAllByText("₱0.00", { selector: ".text-display" }),
  ).toHaveLength(3);
  fireEvent.click(
    panel("Recent transactions").getByRole("button", {
      name: /Add transaction/,
    }),
  );
  expect(
    await screen.findByRole("dialog", { name: "Add transaction" }),
  ).toBeTruthy();
});

test("if the dashboard can't load, Retry asks again", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: () => {
      throw { code: "database", message: "disk I/O error" };
    },
  });
  fireEvent.click(await screen.findByRole("button", { name: "Retry" }));
  await waitFor(() =>
    expect(
      vi.mocked(invoke).mock.calls.filter(([c]) => c === "get_dashboard"),
    ).toHaveLength(2),
  );
});

test("switching period keeps the totals until the new ones arrive", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: ({ query }: { query: PeriodQuery }) =>
      query.kind === "month" && query.anchor
        ? new Promise(() => {})
        : DASHBOARD,
  });
  await screen.findByText("Net cashflow", CARD);
  fireEvent.click(screen.getByRole("button", { name: "Previous period" }));
  await waitFor(() =>
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("get_dashboard", {
      query: { kind: "month", anchor: "2026-10-01" },
    }),
  );
  expect(panel("Income").getByText("+₱4,200.00")).toBeTruthy();
  // …marked as loading, so they can't pass for the new period's.
  expect(
    screen
      .getByText("Net cashflow", CARD)
      .closest("[aria-busy]")
      ?.getAttribute("aria-busy"),
  ).toBe("true");
});

test("if a refresh fails, the error replaces the old numbers", async () => {
  let fail = false;
  const { client } = renderRoute("/", {
    ...ANSWERS,
    get_dashboard: () => {
      if (fail) throw { code: "database", message: "disk I/O error" };
      return DASHBOARD;
    },
  });
  await screen.findByText("Net cashflow", CARD);
  fail = true;
  await client.invalidateQueries({ queryKey: ["ledger"] });
  expect(await screen.findByRole("button", { name: "Retry" })).toBeTruthy();
  expect(screen.queryByText("Net cashflow", CARD)).toBeNull();
});

test("if the categories can't load, the dashboard says so and Retry asks again", async () => {
  let fail = true;
  renderRoute("/", {
    ...ANSWERS,
    list_categories: () => {
      if (fail) throw { code: "database", message: "disk I/O error" };
      return CATEGORIES;
    },
  });
  const retry = await screen.findByRole("button", { name: "Retry" });
  fail = false;
  fireEvent.click(retry);
  expect(await screen.findByText("Food & Groceries · Market")).toBeTruthy();
});

test("a backup that's due shows a warning with Back up now (BAK-7)", async () => {
  let backedUp = false;
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: () => ({ ...DASHBOARD, backupDue: !backedUp }),
    backup_now: () => {
      backedUp = true;
      return { fileName: "jobertsvault-2026-11-05_0930.db" };
    },
  });
  expect(await screen.findByText("No backup in the last 7 days.")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Back up now" }));
  expect(
    await screen.findByText("Backup saved as jobertsvault-2026-11-05_0930.db"),
  ).toBeTruthy();
  await waitFor(() =>
    expect(screen.queryByText("No backup in the last 7 days.")).toBeNull(),
  );
});

test("no warning while backups are recent and no card is due", async () => {
  renderRoute("/", ANSWERS);
  await screen.findByText("Recent transactions");
  expect(screen.queryByText("No backup in the last 7 days.")).toBeNull();
  expect(screen.queryByText(/payment due/)).toBeNull();
});

test("budgets show each one's progress in its own current period (DSH-6)", async () => {
  renderRoute("/", ANSWERS);
  await screen.findByText("Budgets", { selector: "h2" });
  const budgets = panel("Budgets");
  const rows = budgets.getAllByRole("listitem");
  expect(rows.map((row) => row.textContent)).toEqual([
    "All spendingMonthly84%",
    "Food & GroceriesMonthly86%",
    "Dining OutWeekly120%",
  ]);
  expect(
    within(rows[2]).getByRole("progressbar", {
      name: "Dining Out, weekly used",
    }),
  ).toBeTruthy();
  expect(
    budgets.getByRole("link", { name: "Manage" }).getAttribute("href"),
  ).toBe("/budgets");
});

test("with no budgets, the panel says so and still links to Budgets", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: { ...DASHBOARD, budgets: [] },
  });
  await screen.findByText("Budgets", { selector: "h2" });
  const budgets = panel("Budgets");
  expect(budgets.getByText("No budgets yet.")).toBeTruthy();
  expect(budgets.getByRole("link", { name: "Manage" })).toBeTruthy();
});

test("Coming up lists each recurring date in the next 7 days, soonest first (DSH-7)", async () => {
  renderRoute("/", ANSWERS);
  await screen.findByText("Coming up", { selector: "h2" });
  const coming = panel("Coming up");
  const rows = coming.getAllByRole("listitem");
  expect(rows).toHaveLength(2);
  within(rows[0]).getByText("Dining Out · Netflix");
  within(rows[0]).getByText("Fri, Nov 6 · Credit card · auto");
  within(rows[0]).getByText("−₱15.99");
  within(rows[1]).getByText("Bank → E-wallet");
  within(rows[1]).getByText("Sun, Nov 8 · needs confirming");
  expect(
    coming.getByRole("link", { name: "View all" }).getAttribute("href"),
  ).toBe("/recurring");
});

test("with nothing coming up, the panel says so", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: { ...DASHBOARD, upcoming: [] },
  });
  await screen.findByText("Coming up", { selector: "h2" });
  panel("Coming up").getByText("Nothing in the next 7 days.");
});

test("past 8 dates, Coming up says how many more", async () => {
  const many = Array.from({ length: 10 }, (_, i) => ({
    ...UPCOMING[0],
    transaction: { ...UPCOMING[0].transaction, date: "2026-11-06" },
    ruleId: i + 1,
  }));
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: { ...DASHBOARD, upcoming: many },
  });
  await screen.findByText("Coming up", { selector: "h2" });
  const coming = panel("Coming up");
  expect(coming.getAllByRole("listitem")).toHaveLength(8);
  expect(
    coming.getByRole("link", { name: "+ 2 more" }).getAttribute("href"),
  ).toBe("/recurring");
});

const cardDue = (days: number, accountId = 4) => ({
  accountId,
  amountOwed: 64000,
  days,
});

test("a card due within 5 days shows a banner with the amount owed and Pay bill (CC-7)", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: { ...DASHBOARD, cardsDue: [cardDue(3)] },
    list_accounts: ACCOUNT_LIST,
    list_payment_methods: PAYMENT_METHODS,
    get_pay_bill_defaults: PAY_BILL_DEFAULTS,
  });
  const banner = (
    await screen.findByText(/payment due in 3 days/)
  ).closest<HTMLElement>("[role=status]")!;
  expect(banner.textContent).toBe(
    "Credit card payment due in 3 days: ₱640.00 owedPay bill",
  );
  fireEvent.click(within(banner).getByRole("button", { name: "Pay bill" }));
  const dialog = await screen.findByRole("dialog", { name: "Pay credit card" });
  await waitFor(() =>
    expect(
      (
        within(dialog).getByRole("textbox", {
          name: "Amount",
        }) as HTMLInputElement
      ).value,
    ).toBe("640.00"),
  );
});

test("the card banner says today and tomorrow (CC-7)", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: { ...DASHBOARD, cardsDue: [cardDue(0), cardDue(1, 2)] },
  });
  expect(await screen.findByText(/payment due today/)).toBeTruthy();
  expect(screen.getByText(/payment due tomorrow/)).toBeTruthy();
});

// Today is Thu, Nov 5, 2026. The E-wallet (account 3) holds ₱150.00.
const loanDue = (over: Partial<LoanDue> = {}): LoanDue => ({
  loanId: 1,
  name: "E-wallet loan",
  accountId: 3,
  paymentId: 14,
  amount: 100000,
  dueDate: "2026-11-07",
  days: 2,
  moreOverdue: 0,
  moveFirst: null,
  ...over,
});

const overdue = loanDue({
  loanId: 2,
  name: "Bank loan",
  accountId: 2,
  paymentId: 21,
  dueDate: "2026-10-10",
  days: -26,
  moreOverdue: 1,
});

const banners = () => screen.getAllByRole("status");

test("a loan payment due soon has a warning banner with Pay, before every other banner (LOAN-4)", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: {
      ...DASHBOARD,
      backupDue: true,
      cardsDue: [cardDue(3)],
      loansDue: [loanDue()],
    },
    list_loans: LOANS,
    list_accounts: ACCOUNT_LIST,
    list_categories: LOAN_CATEGORIES,
    list_payment_methods: PAYMENT_METHODS,
    get_transaction_defaults: DEFAULTS,
    suggest_payees: [],
  });
  await screen.findByText(/E-wallet loan:/);
  const [first, ...rest] = banners();
  expect(first.textContent).toBe("E-wallet loan: ₱1,000.00 due Sat, Nov 7Pay");
  expect(first.className).toContain("glass-warning");
  expect(rest.map((b) => b.textContent)).toEqual([
    "No backup in the last 7 days.Back up now",
    "Credit card payment due in 3 days: ₱640.00 owedPay bill",
  ]);
  fireEvent.click(within(first).getByRole("button", { name: "Pay" }));
  // The title names the payment it pays, from the loan list (UI_FLOWS F13).
  const form = within(
    await screen.findByRole("dialog", {
      name: "Mark paid: E-wallet loan, due Sat, Nov 28",
    }),
  );
  await waitFor(() =>
    expect(
      (form.getByRole("textbox", { name: "Amount" }) as HTMLInputElement).value,
    ).toBe("1,000.00"),
  );
});

test("the banners share one blurred panel as flat rows, so every banner blurs once (PERF-01)", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: {
      ...DASHBOARD,
      backupDue: true,
      cardsDue: [cardDue(3)],
      loansDue: [loanDue()],
      recurringDue: 2,
    },
    list_loans: LOANS,
  });
  await screen.findByText(/E-wallet loan:/);
  const all = banners();
  expect(all).toHaveLength(4);
  const panel = all[0].parentElement!;
  expect(panel.className).toContain("glass");
  for (const b of all) {
    expect(b.parentElement).toBe(panel);
    // A .glass inside a .glass is flat (globals.css, DESIGN_SYSTEM §2.3).
    expect(b.className).toContain("glass");
  }
});

test("with no banner, their panel is empty and hidden (PERF-01)", async () => {
  renderRoute("/", ANSWERS);
  await screen.findByText("Safe to spend");
  expect(screen.queryAllByRole("status")).toHaveLength(0);
  const empty = document.querySelector('[class~="empty:hidden"]');
  expect(empty?.childElementCount).toBe(0);
});

test("the loan banner says due today and due tomorrow (LOAN-4)", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: {
      ...DASHBOARD,
      loansDue: [
        loanDue({ dueDate: "2026-11-05", days: 0 }),
        loanDue({
          loanId: 2,
          name: "Bank loan",
          dueDate: "2026-11-06",
          days: 1,
        }),
      ],
    },
  });
  await screen.findAllByRole("button", { name: "Pay" });
  expect(banners().map((b) => b.textContent)).toEqual([
    "E-wallet loan: ₱1,000.00 due todayPay",
    "Bank loan: ₱1,000.00 due tomorrowPay",
  ]);
});

test("an overdue loan payment has a danger banner saying how many more are overdue (LOAN-4)", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: {
      ...DASHBOARD,
      loansDue: [overdue, { ...overdue, loanId: 3, moreOverdue: 0 }],
    },
  });
  await screen.findAllByText(/Bank loan:/);
  const [many, one] = banners();
  expect(many.textContent).toBe(
    "Bank loan: ₱1,000.00 overdue since Sat, Oct 10, and 1 more overduePay",
  );
  expect(many.className).toContain("glass-danger");
  expect(one.textContent).toBe(
    "Bank loan: ₱1,000.00 overdue since Sat, Oct 10Pay",
  );
});

test("a loan banner says what to move first when its account is short (LOAN-4)", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: {
      ...DASHBOARD,
      loansDue: [loanDue({ moveFirst: 85000 }), loanDue({ loanId: 2 })],
    },
  });
  await screen.findAllByRole("button", { name: "Pay" });
  const [short, covered] = banners();
  expect(short.textContent).toBe(
    "E-wallet loan: ₱1,000.00 due Sat, Nov 7E-wallet has ₱150.00. Move ₱850.00 there first.Pay",
  );
  expect(covered.textContent).not.toContain("there first");
});

test("Loans left shows under the net balance and links to Debt-free (LOAN-11)", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: {
      ...DASHBOARD,
      loansLeft: {
        totalLeft: 1400000,
        debtFreeOn: "2027-08-10",
        overdue: false,
      },
    },
  });
  const line = await screen.findByRole("link", { name: /Loans left/ });
  expect(line.textContent).toBe(
    "Loans left: ₱14,000.00 · debt-free Aug 10, 2027",
  );
  expect(line.getAttribute("href")).toBe("/debt-free");
  // Right under the net balance, in the Accounts panel.
  expect(line.previousElementSibling!.textContent).toContain("Net balance");
});

test("Loans left says when everything left is overdue, not a date already passed (LOAN-11)", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: {
      ...DASHBOARD,
      loansLeft: { totalLeft: 100000, debtFreeOn: "2026-11-05", overdue: true },
    },
  });
  const line = await screen.findByRole("link", { name: /Loans left/ });
  expect(line.textContent).toBe("Loans left: ₱1,000.00 · all overdue");
});

test("Loans left is hidden when nothing is left (LOAN-11)", async () => {
  renderRoute("/", ANSWERS);
  await screen.findByText("Net balance");
  expect(screen.queryByText(/Loans left/)).toBeNull();
});

/** Ten recent rows, each 52 px tall, beside a column that leaves `height` px for them. */
async function recentFitting(
  height: number,
  recent = Array.from({ length: 10 }, (_, i) => ({
    ...RECENT[0],
    id: 100 + i,
  })),
) {
  const rect = vi
    .spyOn(HTMLElement.prototype, "getBoundingClientRect")
    .mockImplementation(function (this: HTMLElement) {
      const rows = [...document.querySelectorAll("[data-recent-row]")];
      return { top: 0, bottom: (rows.indexOf(this) + 1) * 52 } as DOMRect;
    });
  const client = vi
    .spyOn(HTMLElement.prototype, "clientHeight", "get")
    .mockReturnValue(height);
  try {
    renderRoute("/", { ...ANSWERS, get_dashboard: { ...DASHBOARD, recent } });
    await screen.findByText("Recent transactions");
    const rows = [...document.querySelectorAll("[data-recent-row]")];
    return rows.filter((r) => !r.classList.contains("invisible")).length;
  } finally {
    rect.mockRestore();
    client.mockRestore();
  }
}

test("Recent transactions shows as many whole rows as fit beside the right column (DSH-8)", async () => {
  expect(await recentFitting(400)).toBe(7);
});

test("Recent transactions shows up to 20 beside a tall column (DSH-8)", async () => {
  const twenty = Array.from({ length: 20 }, (_, i) => ({
    ...RECENT[0],
    id: 100 + i,
  }));
  expect(await recentFitting(1100, twenty)).toBe(20);
});

test("Recent transactions always shows at least 5 rows (DSH-8)", async () => {
  expect(await recentFitting(100)).toBe(5);
});

test("a day whose rows don't fit hides its header too (DSH-8)", async () => {
  // 5 rows today, then 5 yesterday; 300 px fits the first 5.
  const recent = Array.from({ length: 10 }, (_, i) => ({
    ...RECENT[0],
    id: 100 + i,
    date: i < 5 ? RECENT[0].date : "2026-11-04",
  }));
  expect(await recentFitting(300, recent)).toBe(5);
  expect(
    screen
      .getByText("Yesterday", { selector: "h3" })
      .classList.contains("invisible"),
  ).toBe(true);
  expect(
    screen
      .getByText("Today", { selector: "h3" })
      .classList.contains("invisible"),
  ).toBe(false);
});

test("the week's and the month's focus sit flat at the top of Safe to spend, the month's first, and open /level (CHK-8)", async () => {
  renderRoute("/", {
    ...ANSWERS,
    list_loans: LOANS,
    get_dashboard: {
      ...DASHBOARD,
      focuses: [
        activeFocus({
          kind: "category_cap",
          categoryId: 2,
          target: 150000,
          actual: 124000,
          percent: 82,
          state: "near",
        }),
        activeFocus({
          kind: "logging",
          target: 20,
          actual: 9,
          start: "2026-11-01",
          end: "2026-11-30",
        }),
      ],
    },
  });
  const lines = (await screen.findAllByText(/'s focus: /)).map(
    (l) => l.textContent,
  );
  expect(lines).toEqual([
    "This month's focus: Log on at least 20 days this month · 9 days so far",
    "This week's focus: Keep Dining Out under ₱1,500.00 · ₱1,240.00 so far",
  ]);
  const card = panel("Safe to spend");
  // Flat in the card's own panel: no blurred panel of its own (DESIGN_SYSTEM §11).
  expect(card.getAllByText(/'s focus: /)).toHaveLength(2);
  expect(
    card
      .getByRole("progressbar", { hidden: true })
      .getAttribute("aria-valuenow"),
  ).toBe("82");
  // The link is named by its sentences: the bar's "82" and "82%" aren't read into it (A11Y-10).
  const link = screen.getByRole("link", {
    name: /^This month's focus: .*so far$/,
  });
  expect(link.getAttribute("href")).toBe("/level");
  expect(link).toBe(screen.getByText(/^This week's focus/).closest("a"));
});

test("with no active focus there are no focus rows (CHK-8)", async () => {
  renderRoute("/", { ...ANSWERS, list_loans: LOANS });
  await screen.findByText("Safe to spend");
  expect(screen.queryByText(/'s focus: /)).toBeNull();
});

test("a cap's row doesn't wait for the loans, which only a loan payment's sentence needs", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: {
      ...DASHBOARD,
      focuses: [
        activeFocus({
          kind: "spending_cap",
          target: 150000,
          actual: 1000,
          percent: 0,
          state: "ok",
        }),
      ],
    },
  });
  expect(
    await screen.findByText(/^This week's focus: Spend under ₱1,500.00/),
  ).toBeTruthy();
  expect(
    vi.mocked(invoke).mock.calls.some(([cmd]) => cmd === "list_loans"),
  ).toBe(false);
});
