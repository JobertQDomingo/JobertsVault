// Sample data matching UI_FLOWS' examples, for screen tests.
import type { Account } from "@/bindings/Account";
import type { Category } from "@/bindings/Category";
import type { CategoryList } from "@/bindings/CategoryList";
import type { Dashboard } from "@/bindings/Dashboard";
import type { PaymentMethod } from "@/bindings/PaymentMethod";
import type { PeriodView } from "@/bindings/PeriodView";
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

export const DASHBOARD: Dashboard = {
  period: PERIOD,
  income: 420000,
  expense: 295040,
  net: 124960,
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
