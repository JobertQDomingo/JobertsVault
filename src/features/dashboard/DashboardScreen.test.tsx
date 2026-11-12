import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import {
  ACCOUNT_LIST,
  CATEGORIES,
  DASHBOARD,
  PAY_BILL_DEFAULTS,
  PAYMENT_METHODS,
  PERIOD,
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

const cardDue = (days: number) => ({ accountId: 4, amountOwed: 64000, days });

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
    get_dashboard: { ...DASHBOARD, cardsDue: [cardDue(0), cardDue(1)] },
  });
  expect(await screen.findByText(/payment due today/)).toBeTruthy();
  expect(screen.getByText(/payment due tomorrow/)).toBeTruthy();
});
