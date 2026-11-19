// Sample data matching UI_FLOWS' examples, for screen tests.
import type { Account } from "@/bindings/Account";
import type { Breakdown } from "@/bindings/Breakdown";
import type { BudgetProgress } from "@/bindings/BudgetProgress";
import type { Category } from "@/bindings/Category";
import type { CategoryList } from "@/bindings/CategoryList";
import type { Comparison } from "@/bindings/Comparison";
import type { Dashboard } from "@/bindings/Dashboard";
import type { GoalProgress } from "@/bindings/GoalProgress";
import type { Loan } from "@/bindings/Loan";
import type { LoanList } from "@/bindings/LoanList";
import type { LoanPreview } from "@/bindings/LoanPreview";
import type { LoanPayment } from "@/bindings/LoanPayment";
import type { UpcomingDate } from "@/bindings/UpcomingDate";
import type { PaymentMethod } from "@/bindings/PaymentMethod";
import type { PeriodView } from "@/bindings/PeriodView";
import type { Timeseries } from "@/bindings/Timeseries";
import type { Transaction } from "@/bindings/Transaction";
import type { TransactionList } from "@/bindings/TransactionList";

export const TODAY = "2026-11-05";

const account = (over: Partial<Account> & Pick<Account, "id" | "name">) =>
  ({
    kind: "cash",
    openingBalance: 0,
    creditLimit: null,
    statementDay: null,
    dueDay: null,
    spendSoft: null,
    spendMid: null,
    spendHard: null,
    sortOrder: over.id,
    archived: false,
    balance: 0,
    card: null,
    inUse: true,
    ...over,
  }) satisfies Account;

export const ACCOUNTS: Account[] = [
  account({ id: 1, name: "Cash", balance: 32000 }),
  account({ id: 2, name: "Bank", kind: "bank", balance: 841075 }),
  account({ id: 3, name: "E-wallet", kind: "ewallet", balance: 15000 }),
  account({
    id: 4,
    name: "Credit card",
    kind: "credit_card",
    creditLimit: 1000000,
    statementDay: 20,
    dueDay: 5,
    spendSoft: 300000,
    spendMid: 400000,
    spendHard: 500000,
    balance: -64000,
    card: {
      amountOwed: 64000,
      availableCredit: 936000,
      utilizationPercent: 6,
      cycleStart: "2026-10-21",
      cycleEnd: "2026-11-20",
      cycleSpending: 41230,
      spendLevel: "none",
      dueDate: "2026-11-05",
    },
  }),
  account({ id: 5, name: "Old Wallet", archived: true, balance: 4500 }),
];

const category = (over: Partial<Category> & Pick<Category, "id" | "name">) =>
  ({
    kind: "expense",
    color: "cat-1",
    icon: "ellipsis",
    sortOrder: over.id,
    archived: false,
    transactionCount: 1,
    inUse: true,
    ...over,
  }) satisfies Category;

export const CATEGORIES: CategoryList = {
  categories: [
    category({
      id: 1,
      name: "Food & Groceries",
      color: "cat-2",
      icon: "shopping-cart",
    }),
    category({ id: 2, name: "Dining Out", color: "cat-6", icon: "utensils" }),
    category({
      id: 3,
      name: "Salary",
      kind: "income",
      color: "cat-12",
      icon: "briefcase",
    }),
    category({ id: 4, name: "Transportation", color: "cat-1", icon: "car" }),
    category({ id: 5, name: "Snacks", archived: true }),
  ],
  defaultColors: { income: "cat-1", expense: "cat-3" },
};

const transaction = (over: Partial<Transaction> & Pick<Transaction, "id">) =>
  ({
    kind: "expense",
    amount: 0,
    date: TODAY,
    accountId: 1,
    toAccountId: null,
    categoryId: null,
    payee: null,
    paymentMethodId: null,
    note: null,
    loanPaymentId: null,
    payslipId: null,
    ...over,
  }) satisfies Transaction;

/** The 5 newest, newest first: two today, two yesterday, one on Oct 26. */
export const RECENT: Transaction[] = [
  transaction({
    id: 7,
    amount: 8520,
    categoryId: 1,
    payee: "Market",
    note: "weekly shop",
  }),
  transaction({
    id: 6,
    kind: "transfer",
    amount: 120000,
    accountId: 2,
    toAccountId: 4,
    note: "October bill",
  }),
  transaction({
    id: 5,
    amount: 1250,
    date: "2026-11-04",
    accountId: 4,
    categoryId: 2,
    payee: "Corner Cafe",
  }),
  transaction({
    id: 4,
    kind: "income",
    amount: 420000,
    date: "2026-11-04",
    accountId: 2,
    categoryId: 3,
    payee: "Employer",
  }),
  transaction({ id: 3, amount: 2000, date: "2026-10-26", categoryId: 4 }),
];

export const PERIOD: PeriodView = {
  start: "2026-11-01",
  end: "2026-11-30",
  previous: { kind: "month", anchor: "2026-10-01" },
  next: null,
  todayQuery: { kind: "month", anchor: "2026-11-01" },
  isCurrent: true,
  today: TODAY,
};

/** November by day, with the dashboard's totals on the 4th and 5th. */
export const SERIES: Timeseries = {
  grain: "day",
  bars: Array.from({ length: 30 }, (_, i) => {
    const date = `2026-11-${String(i + 1).padStart(2, "0")}`;
    const [income, expense] =
      i === 3 ? [420000, 285270] : i === 4 ? [0, 9770] : [0, 0];
    return { start: date, end: date, income, expense, net: income - expense };
  }),
  today: TODAY,
};

/** November's expenses by category: Food, Dining Out, then the archived Snacks. */
export const BREAKDOWN: Breakdown = {
  kind: "expense",
  total: 295040,
  categories: [
    { categoryId: 1, amount: 200000, share: 678 },
    { categoryId: 2, amount: 85040, share: 288 },
    { categoryId: 5, amount: 10000, share: 34 },
  ],
  slices: [
    { categoryId: 1, amount: 200000, share: 678 },
    { categoryId: 2, amount: 85040, share: 288 },
    { categoryId: 5, amount: 10000, share: 34 },
  ],
};

/** November 1–5 next to October 1–5: more income, less spent, and Dining Out new this month. */
export const COMPARISON: Comparison = {
  inProgress: true,
  mode: "to_date",
  currentStart: "2026-11-01",
  currentEnd: TODAY,
  compareStart: "2026-10-01",
  compareEnd: "2026-10-05",
  income: { current: 420000, compare: 407400, difference: 12600, change: 31 },
  expense: {
    current: 295040,
    compare: 320700,
    difference: -25660,
    change: -80,
  },
  net: { current: 124960, compare: 86700, difference: 38260, change: 441 },
  incomeCategories: [
    {
      categoryId: 3,
      current: 420000,
      compare: 407400,
      difference: 12600,
      change: 31,
    },
  ],
  expenseCategories: [
    {
      categoryId: 1,
      current: 200000,
      compare: 320700,
      difference: -120700,
      change: -376,
    },
    {
      categoryId: 2,
      current: 95040,
      compare: 0,
      difference: 95040,
      change: null,
    },
  ],
  today: TODAY,
};

/** `list_budgets` on Nov 5: the mockup's budgets, one of them past its limit (UI_FLOWS §8.3). */
export const BUDGETS: BudgetProgress[] = [
  {
    id: 3,
    categoryId: null,
    period: "monthly",
    amount: 350000,
    start: "2026-11-01",
    end: "2026-11-30",
    spent: 295040,
    remaining: 54960,
    percent: 84,
  },
  {
    id: 1,
    categoryId: 1,
    period: "monthly",
    amount: 60000,
    start: "2026-11-01",
    end: "2026-11-30",
    spent: 52040,
    remaining: 7960,
    percent: 86,
  },
  {
    id: 2,
    categoryId: 2,
    period: "weekly",
    amount: 15000,
    start: "2026-11-02",
    end: "2026-11-08",
    spent: 18000,
    remaining: -3000,
    percent: 120,
  },
];

const goal = (
  over: Partial<GoalProgress> &
    Pick<GoalProgress, "id" | "name" | "accountId" | "targetAmount">,
) =>
  ({
    targetDate: null,
    achievedAt: null,
    archived: false,
    progress: 0,
    remaining: over.targetAmount,
    percent: 0,
    monthsLeft: null,
    monthlyNeeded: null,
    behind: null,
    ...over,
  }) satisfies GoalProgress;

/**
 * `list_goals` on Nov 5, from ACCOUNTS' balances: one on its way, one achieved, one past its
 * date, and an archived one (UI_FLOWS §8.4).
 */
export const GOALS: GoalProgress[] = [
  goal({
    id: 1,
    name: "Japan trip",
    accountId: 2,
    targetAmount: 1000000,
    targetDate: "2027-05-05",
    progress: 841075,
    remaining: 158925,
    percent: 84,
    monthsLeft: 6,
    monthlyNeeded: 26488,
  }),
  goal({
    id: 2,
    name: "New laptop",
    accountId: 1,
    targetAmount: 30000,
    targetDate: "2026-12-01",
    achievedAt: "2026-11-01",
    progress: 32000,
    remaining: 0,
    percent: 100,
    monthsLeft: 1,
  }),
  goal({
    id: 3,
    name: "Car",
    accountId: 3,
    targetAmount: 2000000,
    targetDate: "2026-10-01",
    progress: 15000,
    remaining: 1985000,
    monthsLeft: 0,
  }),
  goal({
    id: 4,
    name: "Old plan",
    accountId: 5,
    targetAmount: 10000,
    archived: true,
    progress: 4500,
    remaining: 5500,
    percent: 45,
  }),
];

/** Coming up on Nov 5 (DSH-7): a card subscription that auto-posts, and a top-up to confirm. */
export const UPCOMING: UpcomingDate[] = [
  {
    ruleId: 1,
    mode: "auto",
    transaction: {
      kind: "expense",
      amount: 1599,
      date: "2026-11-06",
      accountId: 4,
      toAccountId: null,
      categoryId: 2,
      payee: "Netflix",
      paymentMethodId: null,
      note: null,
    },
  },
  {
    ruleId: 2,
    mode: "confirm",
    transaction: {
      kind: "transfer",
      amount: 50000,
      date: "2026-11-08",
      accountId: 2,
      toAccountId: 3,
      categoryId: null,
      payee: null,
      paymentMethodId: null,
      note: null,
    },
  },
];

export const DASHBOARD: Dashboard = {
  period: PERIOD,
  income: 420000,
  expense: 295040,
  net: 124960,
  series: SERIES,
  comparison: COMPARISON,
  backupDue: false,
  recurringDue: 0,
  accounts: ACCOUNTS,
  // Archived accounts count too (ACC-5): 8,240.75 + Old Wallet's 45.00.
  netBalance: 828575,
  recent: RECENT,
  budgets: BUDGETS,
  upcoming: UPCOMING,
  cardsDue: [],
  loansDue: [],
  loansLeft: null,
  // No payday rule yet: the card asks for one.
  safeToSpend: {
    today: TODAY,
    moneyNow: 856075,
    safeToSpend: null,
    perDay: null,
    windowEnd: null,
    daysLeft: null,
    heldBack: null,
    heldFor: null,
    short: null,
    nextPayday: null,
    earlyPayday: null,
    rows: [],
  },
  focuses: [],
  today: TODAY,
};

export const ACCOUNT_LIST = { accounts: ACCOUNTS, netBalance: 828575 };

export const PAYMENT_METHODS: PaymentMethod[] = [
  { id: 1, name: "Cash", sortOrder: 1, archived: false, inUse: true },
  { id: 2, name: "Credit Card", sortOrder: 2, archived: false, inUse: true },
  { id: 3, name: "Old card", sortOrder: 3, archived: true, inUse: true },
];

/** `list_transactions` for this month: one page, with figures over every match. */
export const LIST: TransactionList = {
  items: RECENT.slice(0, 4),
  hasMore: false,
  summary: { count: 4, income: 420000, expense: 9770 },
  dayNets: [
    { date: TODAY, net: -8520 },
    { date: "2026-11-04", net: 418750 },
  ],
};

/** Quick-add starts on the card, the account used last (DATA_MODEL §5.8). */
export const DEFAULTS = { accountId: 4, today: TODAY };

/** Pay bill on the card (id 4): from Bank, the full ₱640.00 owed (DATA_MODEL §5.8). */
export const PAY_BILL_DEFAULTS = {
  fromAccountId: 2,
  amount: 64000,
  today: TODAY,
};

/** The seeded Loan category (DATA_MODEL §7), next to the other expense categories. */
export const LOAN_CATEGORIES: CategoryList = {
  ...CATEGORIES,
  categories: [
    ...CATEGORIES.categories,
    category({ id: 6, name: "Loan", color: "cat-2", icon: "coins" }),
  ],
};

const payment = (
  id: number,
  dueDate: string,
  over: Partial<LoanPayment> = {},
): LoanPayment => ({
  id,
  dueDate,
  amount: 100000,
  status: "unpaid",
  overdue: false,
  transactionId: null,
  paidAmount: null,
  paidOn: null,
  ...over,
});
const before = { status: "paidBefore", paidAmount: 100000 } as const;

const ewallet: Loan = {
  id: 1,
  name: "E-wallet loan",
  accountId: 3,
  categoryId: 6,
  isNew: false,
  amountBorrowed: 700000,
  borrowedOn: "2026-07-01",
  addedOn: "2026-10-01",
  archived: false,
  // 3 of 8 paid: two from before the app, one marked paid in it.
  payments: [
    payment(11, "2026-08-28", before),
    payment(12, "2026-09-28", before),
    payment(13, "2026-10-28", {
      status: "paid",
      transactionId: 50,
      paidAmount: 100000,
      paidOn: "2026-10-27",
    }),
    ...[
      "2026-11-28",
      "2026-12-28",
      "2027-01-28",
      "2027-02-28",
      "2027-03-28",
    ].map((date, i) => payment(14 + i, date)),
  ],
  left: 500000,
  paid: 300000,
  percentPaid: 37,
  next: payment(14, "2026-11-28"),
  interest: 100000,
  paidOff: false,
};

const bank: Loan = {
  id: 2,
  name: "Bank loan",
  accountId: 2,
  categoryId: 6,
  isNew: true,
  amountBorrowed: null,
  borrowedOn: "2026-09-15",
  addedOn: "2026-09-15",
  archived: false,
  payments: [
    payment(21, "2026-10-10", { overdue: true }),
    payment(22, "2026-11-10"),
    payment(23, "2026-12-10"),
  ],
  left: 300000,
  paid: 0,
  percentPaid: 0,
  next: payment(21, "2026-10-10", { overdue: true }),
  interest: null,
  paidOff: false,
};

const phone: Loan = {
  id: 3,
  name: "Phone loan",
  accountId: 3,
  categoryId: 6,
  isNew: false,
  amountBorrowed: null,
  borrowedOn: null,
  addedOn: "2026-10-01",
  archived: true,
  payments: [
    payment(31, "2026-06-01", before),
    payment(32, "2026-07-01", before),
  ],
  left: 0,
  paid: 200000,
  percentPaid: 100,
  next: null,
  interest: null,
  paidOff: true,
};

/** Two active loans (one with an overdue payment) and a paid-off one archived (UI_FLOWS §8.5). */
export const LOANS: LoanList = {
  loans: [ewallet, bank, phone],
  summary: {
    totalLeft: 800000,
    totalPaid: 500000,
    percentPaid: 38,
    debtFreeOn: "2027-03-28",
    daysToGo: 143,
    interest: 100000,
    streakDays: 51,
    freedUp: [
      { start: "2026-11-01", end: "2026-11-30", due: 300000, freed: 0 },
      { start: "2026-12-01", end: "2026-12-31", due: 200000, freed: 100000 },
      { start: "2027-01-01", end: "2027-01-31", due: 400000, freed: -100000 },
      { start: "2027-02-01", end: "2027-02-28", due: 300000, freed: 0 },
      { start: "2027-04-01", end: "2027-04-30", due: 0, freed: 300000 },
    ],
  },
};

/** preview_loan (LOAN-9, LOAN-10): one window unchanged, one lower, one made short. */
export const LOAN_PREVIEW: LoanPreview = {
  debtFreeBefore: "2027-03-28",
  debtFreeAfter: "2027-05-05",
  daysLater: 38,
  safeBefore: 840000,
  safeAfter: 740000,
  windows: [
    {
      start: "2026-11-05",
      end: "2026-12-14",
      leftBefore: 840000,
      leftAfter: 840000,
      shortBy: null,
    },
    {
      start: "2026-12-15",
      end: "2026-12-29",
      leftBefore: 1830000,
      leftAfter: 1730000,
      shortBy: null,
    },
    {
      start: "2026-12-30",
      end: "2027-01-14",
      leftBefore: 9000,
      leftAfter: -1000,
      shortBy: 18000,
    },
  ],
};
