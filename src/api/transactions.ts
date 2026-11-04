import type { PayBillDefaults } from "@/bindings/PayBillDefaults";
import type { Transaction } from "@/bindings/Transaction";
import type { TransactionDefaults } from "@/bindings/TransactionDefaults";
import type { TransactionFields } from "@/bindings/TransactionFields";
import type { TransactionFilter } from "@/bindings/TransactionFilter";
import type { TransactionList } from "@/bindings/TransactionList";
import { call } from "./client";

/** Page `page` (from 0) of 100 rows, newest first, with a summary and day nets over every match. */
export const listTransactions = ({
  filter,
  page,
}: {
  filter: TransactionFilter;
  page: number;
}) => call<TransactionList>("list_transactions", { filter, page });

export const getTransaction = (id: number) =>
  call<Transaction>("get_transaction", { id });

export const createTransaction = (transaction: TransactionFields) =>
  call<Transaction>("create_transaction", { transaction });

export const updateTransaction = ({
  id,
  transaction,
}: {
  id: number;
  transaction: TransactionFields;
}) => call<Transaction>("update_transaction", { id, transaction });

export const deleteTransaction = (id: number) =>
  call<null>("delete_transaction", { id });

/**
 * Quick-add's account and today's date. A duplicate is `getTransaction` with this `today`,
 * saved through `createTransaction` (ARCHITECTURE §4.5).
 */
export const getTransactionDefaults = () =>
  call<TransactionDefaults>("get_transaction_defaults");

/** Pay bill's From account and amount owed (CC-9). A non-card account is a `conflict`. */
export const getPayBillDefaults = (cardId: number) =>
  call<PayBillDefaults>("get_pay_bill_defaults", { cardId });
