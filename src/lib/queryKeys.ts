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
  settings: ["settings"],
} as const;
