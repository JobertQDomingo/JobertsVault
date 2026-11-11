// Sample data matching UI_FLOWS' examples, for screen tests.
import type { Account } from "@/bindings/Account";
import type { Breakdown } from "@/bindings/Breakdown";
import type { Category } from "@/bindings/Category";
import type { CategoryList } from "@/bindings/CategoryList";
import type { Comparison } from "@/bindings/Comparison";
import type { Dashboard } from "@/bindings/Dashboard";
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
    balance: -64000,
    card: {
      amountOwed: 64000,
      availableCredit: 936000,
      utilizationPercent: 6,
      cycleStart: "2026-10-21",
      cycleEnd: "2026-11-20",
      cycleSpending: 41230,
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
