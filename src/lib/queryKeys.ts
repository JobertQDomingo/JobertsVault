// Every key that involves money starts with "ledger", so one refresh covers them all (ARCHITECTURE §5.2).
export const queryKeys = {
  ledger: ["ledger"],
  accounts: ["ledger", "accounts"],
  account: (id: number) => ["ledger", "accounts", id] as const,
  // Transaction rows show these names, and a merge changes the totals by category.
  categories: ["ledger", "categories"],
  paymentMethods: ["ledger", "paymentMethods"],
  settings: ["settings"],
} as const;
