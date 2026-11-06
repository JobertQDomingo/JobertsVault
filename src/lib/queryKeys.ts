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
  // The last account used and the amount owed change with every write.
  transactionDefaults: ["ledger", "transactionDefaults"],
  payBillDefaults: (cardId: number) =>
    ["ledger", "payBillDefaults", cardId] as const,
  // Period dates follow the week and month start settings, which refresh the ledger.
  period: (query: PeriodQuery) => ["ledger", "period", query] as const,
  dashboard: (query: PeriodQuery) => ["ledger", "dashboard", query] as const,
  settings: ["settings"],
  startup: ["startup"],
  // The startup update check and install, each run once per start (UPD-1).
  startupUpdateCheck: ["startupUpdate", "check"],
  startupUpdateInstall: ["startupUpdate", "install"],
} as const;
