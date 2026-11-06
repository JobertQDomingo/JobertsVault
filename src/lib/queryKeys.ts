import type { CategoryKind } from "@/bindings/CategoryKind";
import type { ComparisonMode } from "@/bindings/ComparisonMode";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import type { TransactionFilter } from "@/bindings/TransactionFilter";

// Every key that involves money starts with "ledger", so one refresh covers them all (ARCHITECTURE §5.2).
export const queryKeys = {
  ledger: ["ledger"],
  accounts: ["ledger", "accounts"],
  account: (id: number) => ["ledger", "accounts", id] as const,
  // Transaction rows show these names, and a merge changes the totals by category.
  categories: ["ledger", "categories"],
  paymentMethods: ["ledger", "paymentMethods"],
  transactions: (filter: TransactionFilter) =>
    ["ledger", "transactions", filter] as const,
  transaction: (id: number) => ["ledger", "transaction", id] as const,
  // A save can add a payee or change its last category.
  payees: (prefix: string, kind: CategoryKind) =>
    ["ledger", "payees", prefix, kind] as const,
  // The last account used and the amount owed change with every write.
  transactionDefaults: ["ledger", "transactionDefaults"],
  payBillDefaults: (cardId: number) =>
    ["ledger", "payBillDefaults", cardId] as const,
  // Period dates follow the week and month start settings, which refresh the ledger.
  period: (query: PeriodQuery) => ["ledger", "period", query] as const,
  dashboard: (query: PeriodQuery) => ["ledger", "dashboard", query] as const,
  timeseries: (query: PeriodQuery, accountId: number | null) =>
    ["ledger", "timeseries", query, accountId] as const,
  breakdown: (
    query: PeriodQuery,
    kind: CategoryKind,
    accountId: number | null,
  ) => ["ledger", "breakdown", query, kind, accountId] as const,
  // A null mode follows the setting, which refreshes the ledger when it changes.
  comparison: (
    query: PeriodQuery,
    mode: ComparisonMode | null,
    accountId: number | null,
  ) => ["ledger", "comparison", query, mode, accountId] as const,
  settings: ["settings"],
  startup: ["startup"],
  appVersion: ["appVersion"],
} as const;
