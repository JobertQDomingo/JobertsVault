import { invoke } from "@tauri-apps/api/core";
import {
  QueryClient,
  QueryClientProvider,
  useQuery,
} from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, expect, test, vi } from "vitest";
import type { AccountFields } from "@/bindings/AccountFields";
import type { BudgetFields } from "@/bindings/BudgetFields";
import type { CategoryFields } from "@/bindings/CategoryFields";
import type { GoalFields } from "@/bindings/GoalFields";
import type { NewAccount } from "@/bindings/NewAccount";
import type { NewCategory } from "@/bindings/NewCategory";
import type { RecurringRuleFields } from "@/bindings/RecurringRuleFields";
import type { Settings } from "@/bindings/Settings";
import type { TransactionFields } from "@/bindings/TransactionFields";
import {
  useArchiveAccount,
  useCreateAccount,
  useDeleteAccount,
  useReorderAccounts,
  useUnarchiveAccount,
  useUpdateAccount,
} from "./useAccounts";
import {
  useArchiveCategory,
  useCreateCategory,
  useDeleteCategory,
  useMergeCategories,
  useReorderCategories,
  useUnarchiveCategory,
  useUpdateCategory,
} from "./useCategories";
import {
  useArchivePaymentMethod,
  useCreatePaymentMethod,
  useDeletePaymentMethod,
  useReorderPaymentMethods,
  useUnarchivePaymentMethod,
  useUpdatePaymentMethod,
} from "./usePaymentMethods";
import * as settingsHooks from "./useSettings";
import { useUpdateSettings } from "./useSettings";
import * as accountHooks from "./useAccounts";
import * as categoryHooks from "./useCategories";
import * as paymentMethodHooks from "./usePaymentMethods";
import * as recurringHooks from "./useRecurring";
import {
  useCreateRecurringRule,
  useDeleteRecurringRule,
  usePauseRecurringRule,
  usePostOccurrence,
  useSkipOccurrence,
  useUpdateRecurringRule,
} from "./useRecurring";
import * as budgetHooks from "./useBudgets";
import {
  useCreateBudget,
  useDeleteBudget,
  useUpdateBudget,
} from "./useBudgets";
import * as goalHooks from "./useGoals";
import { useCreateGoal, useDeleteGoal, useUpdateGoal } from "./useGoals";
import * as transactionHooks from "./useTransactions";
import {
  useCreateTransaction,
  useDeleteTransaction,
  useUpdateTransaction,
} from "./useTransactions";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const account = {
  name: "Bank",
  openingBalance: 0,
  creditLimit: null,
  statementDay: null,
  dueDay: null,
} satisfies AccountFields;
const newAccount = { kind: "bank", ...account } satisfies NewAccount;
const category = {
  name: "Snacks",
  color: "cat-2",
  icon: "cookie",
} satisfies CategoryFields;
const newCategory = { kind: "expense", ...category } satisfies NewCategory;
const transaction: TransactionFields = {
  kind: "expense",
  amount: 25000,
  date: "2026-11-05",
  accountId: 1,
  toAccountId: null,
  categoryId: 2,
  payee: null,
  paymentMethodId: null,
  note: null,
};
const rule: RecurringRuleFields = {
  transaction,
  frequency: "monthly",
  interval: 1,
  endDate: null,
  maxOccurrences: null,
  mode: "auto",
};
const budget: BudgetFields = {
  categoryId: 8,
  period: "monthly",
  amount: 40000,
};
const goal: GoalFields = {
  name: "Trip",
  targetAmount: 500000,
  targetDate: "2027-05-01",
  accountId: 1,
};
const settings: Settings = {
  currencySymbol: "₱",
  currencyPosition: "before",
  currencyDecimals: 2,
  weekStart: "monday",
  monthStartDay: 1,
  showPaymentMethod: true,
  onboardingCompleted: true,
  autoUpdateEnabled: true,
  comparisonMode: "to_date",
};

// Every write the app can make (ARCHITECTURE §4.5), with the arguments it sends to Rust.
const WRITES = [
  [
    "useCreateAccount",
    useCreateAccount,
    newAccount,
    "create_account",
    { account: newAccount },
  ],
  [
    "useUpdateAccount",
    useUpdateAccount,
    { id: 2, account },
    "update_account",
    { id: 2, account },
  ],
  [
    "useReorderAccounts",
    useReorderAccounts,
    [2, 1],
    "reorder_accounts",
    { ids: [2, 1] },
  ],
  [
    "useArchiveAccount",
    useArchiveAccount,
    { id: 3, acknowledgeBalance: true },
    "archive_account",
    { id: 3, acknowledgeBalance: true },
  ],
  [
    "useUnarchiveAccount",
    useUnarchiveAccount,
    3,
    "unarchive_account",
    { id: 3 },
  ],
  ["useDeleteAccount", useDeleteAccount, 3, "delete_account", { id: 3 }],
  [
    "useCreateCategory",
    useCreateCategory,
    newCategory,
    "create_category",
    { category: newCategory },
  ],
  [
    "useUpdateCategory",
    useUpdateCategory,
    { id: 4, category },
    "update_category",
    { id: 4, category },
  ],
  [
    "useReorderCategories",
    useReorderCategories,
    [5, 4],
    "reorder_categories",
    { ids: [5, 4] },
  ],
  ["useArchiveCategory", useArchiveCategory, 4, "archive_category", { id: 4 }],
  [
    "useUnarchiveCategory",
    useUnarchiveCategory,
    4,
    "unarchive_category",
    { id: 4 },
  ],
  ["useDeleteCategory", useDeleteCategory, 4, "delete_category", { id: 4 }],
  [
    "useMergeCategories",
    useMergeCategories,
    { fromId: 4, intoId: 5 },
    "merge_categories",
    { fromId: 4, intoId: 5 },
  ],
  [
    "useCreatePaymentMethod",
    useCreatePaymentMethod,
    "Card",
    "create_payment_method",
    { name: "Card" },
  ],
  [
    "useUpdatePaymentMethod",
    useUpdatePaymentMethod,
    { id: 6, name: "Debit card" },
    "update_payment_method",
    { id: 6, name: "Debit card" },
  ],
  [
    "useReorderPaymentMethods",
    useReorderPaymentMethods,
    [7, 6],
    "reorder_payment_methods",
    { ids: [7, 6] },
  ],
  [
    "useArchivePaymentMethod",
    useArchivePaymentMethod,
    6,
    "archive_payment_method",
    { id: 6 },
  ],
  [
    "useUnarchivePaymentMethod",
    useUnarchivePaymentMethod,
    6,
    "unarchive_payment_method",
    { id: 6 },
  ],
  [
    "useDeletePaymentMethod",
    useDeletePaymentMethod,
    6,
    "delete_payment_method",
    { id: 6 },
  ],
  [
    "useCreateTransaction",
    useCreateTransaction,
    transaction,
    "create_transaction",
    { transaction },
  ],
  [
    "useUpdateTransaction",
    useUpdateTransaction,
    { id: 8, transaction },
    "update_transaction",
    { id: 8, transaction },
  ],
  [
    "useDeleteTransaction",
    useDeleteTransaction,
    8,
    "delete_transaction",
    { id: 8 },
  ],
  [
    "useCreateRecurringRule",
    useCreateRecurringRule,
    rule,
    "create_recurring_rule",
    { rule },
  ],
  [
    "useUpdateRecurringRule",
    useUpdateRecurringRule,
    { id: 9, rule },
    "update_recurring_rule",
    { id: 9, rule },
  ],
  [
    "usePauseRecurringRule",
    usePauseRecurringRule,
    { id: 9, paused: true },
    "pause_recurring_rule",
    { id: 9, paused: true },
  ],
  [
    "useDeleteRecurringRule",
    useDeleteRecurringRule,
    9,
    "delete_recurring_rule",
    { id: 9 },
  ],
  [
    "usePostOccurrence",
    usePostOccurrence,
    { id: 4, transaction: null },
    "post_occurrence",
    { id: 4, transaction: null },
  ],
  ["useSkipOccurrence", useSkipOccurrence, 4, "skip_occurrence", { id: 4 }],
  ["useCreateBudget", useCreateBudget, budget, "create_budget", { budget }],
  [
    "useUpdateBudget",
    useUpdateBudget,
    { id: 3, budget },
    "update_budget",
    { id: 3, budget },
  ],
  ["useDeleteBudget", useDeleteBudget, 3, "delete_budget", { id: 3 }],
  ["useCreateGoal", useCreateGoal, goal, "create_goal", { goal }],
  [
    "useUpdateGoal",
    useUpdateGoal,
    { id: 3, goal },
    "update_goal",
    { id: 3, goal },
  ],
  ["useDeleteGoal", useDeleteGoal, 3, "delete_goal", { id: 3 }],
  [
    "useUpdateSettings",
    useUpdateSettings,
    settings,
    "update_settings",
    { settings },
  ],
] as const;

test.each(WRITES)(
  "%s sends its command and refreshes every ledger query",
  async (_name, hook, args, command, invokeArgs) => {
    vi.mocked(invoke).mockImplementation((sent: string) =>
      Promise.resolve(
        sent === "update_settings"
          ? { ...settings, decimalsLocked: false }
          : null,
      ),
    );
    const client = new QueryClient();
    // Loaded, as in the app: a settings change is laid over them.
    client.setQueryData(["settings"], { ...settings, decimalsLocked: false });
    // A ledger query a screen is showing, under a key no write names: only refreshing all of
    // ['ledger'] reaches it.
    let loads = 0;
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    );
    const { result } = renderHook(
      () => {
        useQuery({
          queryKey: ["ledger", "onScreen"],
          // Takes a moment, like a real load, so only an awaited refresh is done in time.
          queryFn: () =>
            new Promise((done) => setTimeout(() => done(++loads), 10)),
        });
        return hook();
      },
      { wrapper },
    );
    await waitFor(() => expect(loads).toBe(1));

    // @ts-expect-error: each row pairs a hook with its own argument type.
    await result.current.mutateAsync(args);

    expect(invoke).toHaveBeenCalledWith(command, invokeArgs);
    // Already reloaded when the write resolves, so a dialog that closes then shows fresh data.
    expect(client.getQueryData(["ledger", "onScreen"])).toBe(2);
  },
);

const READS = [
  "useAccounts",
  "useAccount",
  "useCategories",
  "usePaymentMethods",
  "useTransactions",
  "useTransaction",
  "useTransactionDefaults",
  "usePayBillDefaults",
  "usePayeeSuggestions",
  "useRecurringRules",
  "useDueOccurrences",
  "useBudgets",
  "useGoals",
  "useSettings",
];

test("the table lists every write hook", () => {
  const hooks = [
    accountHooks,
    categoryHooks,
    paymentMethodHooks,
    transactionHooks,
    recurringHooks,
    budgetHooks,
    goalHooks,
    settingsHooks,
  ].flatMap((module) => Object.keys(module));
  expect(hooks.filter((name) => !READS.includes(name)).sort()).toEqual(
    WRITES.map(([name]) => name).sort(),
  );
});

test("a ledger write also refreshes the settings, whose decimals lock follows the amounts", async () => {
  vi.mocked(invoke).mockResolvedValue(null);
  const client = new QueryClient();
  client.setQueryData(["settings"], { ...settings, decimalsLocked: false });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  const { result } = renderHook(() => useCreateTransaction(), { wrapper });
  await result.current.mutateAsync(transaction);
  expect(client.getQueryState(["settings"])?.isInvalidated).toBe(true);
});
