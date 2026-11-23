// Sample data in the DEV database only (jobertsvault-dev.db), like UI_FLOWS' examples.
export default async (app) => {
  const acct = (kind, name, openingBalance, card = {}) =>
    app.invoke("create_account", { account: { kind, name, openingBalance, creditLimit: card.limit ?? null, statementDay: card.statement ?? null, dueDay: card.due ?? null } });
  const bank = await acct("bank", "Bank", 841075);
  const wallet = await acct("ewallet", "E-wallet", 15000);
  const card = await acct("credit_card", "Credit card", 0, { limit: 1000000, statement: 20, due: 5 });
  const { categories } = await app.invoke("list_categories");
  const cat = (name) => categories.find((c) => c.name === name).id;
  const cash = 1;
  const t = (kind, amount, date, accountId, more = {}) =>
    app.invoke("create_transaction", { transaction: { kind, amount, date, accountId, toAccountId: null, categoryId: null, payee: null, paymentMethodId: null, note: null, ...more } });
  await t("income", 420000, "2026-11-01", bank.id, { categoryId: cat("Salary"), payee: "Employer" });
  await t("expense", 184000, "2026-10-28", card.id, { categoryId: cat("Shopping"), payee: "Department store" });
  await t("expense", 2000, "2026-11-04", cash, { categoryId: cat("Transportation") });
  await t("expense", 1250, "2026-11-04", card.id, { categoryId: cat("Dining Out"), payee: "Corner Cafe" });
  await t("transfer", 120000, "2026-11-05", bank.id, { toAccountId: card.id, note: "October bill" });
  await t("income", 50000, "2026-11-02", cash, { categoryId: cat("Bonus"), payee: "Office raffle" });
  await t("expense", 8520, "2026-11-05", cash, { categoryId: cat("Food & Groceries"), payee: "Market", note: "weekly shop" });
  const a = await app.invoke("list_accounts");
  app.log(JSON.stringify(a.accounts.map((x) => [x.name, x.balance, x.card && x.card.amountOwed])), "net", a.netBalance);
};
