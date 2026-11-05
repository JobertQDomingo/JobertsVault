import { invoke } from "@tauri-apps/api/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, expect, test, vi } from "vitest";
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
import { useUpdateSettings } from "./useSettings";
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
};
const category = { name: "Snacks", color: "cat-2", icon: "cookie" };
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
const settings: Settings = {
  currencySymbol: "₱",
  currencyPosition: "before",
  currencyDecimals: 2,
  weekStart: "monday",
  monthStartDay: 1,
  showPaymentMethod: true,
  onboardingCompleted: true,
  autoUpdateEnabled: true,
};

// Every write the app can make (ARCHITECTURE §4.5), with the arguments it sends to Rust.
const WRITES = [
  [
    "useCreateAccount",
    useCreateAccount,
    { kind: "bank", ...account },
    "create_account",
    { account: { kind: "bank", ...account } },
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
    { kind: "expense", ...category },
    "create_category",
    { category: { kind: "expense", ...category } },
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
    vi.mocked(invoke).mockResolvedValue({ ...settings, decimalsLocked: false });
    const client = new QueryClient();
    client.setQueryData(["ledger", "dashboard"], "old totals");
    client.setQueryData(["ledger", "accounts"], "old balances");
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    );
    const { result } = renderHook(() => hook(), { wrapper });

    // @ts-expect-error: each row pairs a hook with its own argument type.
    await result.current.mutateAsync(args);

    expect(invoke).toHaveBeenCalledWith(command, invokeArgs);
    for (const key of [
      ["ledger", "dashboard"],
      ["ledger", "accounts"],
    ])
      expect(client.getQueryState(key)?.isInvalidated).toBe(true);
  },
);
