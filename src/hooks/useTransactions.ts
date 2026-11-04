import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import type { TransactionFilter } from "@/bindings/TransactionFilter";
import {
  createTransaction,
  deleteTransaction,
  getPayBillDefaults,
  getTransaction,
  getTransactionDefaults,
  listTransactions,
  updateTransaction,
} from "@/api/transactions";
import { queryKeys } from "@/lib/queryKeys";
import { useLedgerMutation } from "./useLedgerMutation";

/** Loads pages of 100 as the list scrolls (UI_FLOWS §3). */
export const useTransactions = (filter: TransactionFilter) =>
  useInfiniteQuery({
    queryKey: queryKeys.transactions(filter),
    queryFn: ({ pageParam }) => listTransactions({ filter, page: pageParam }),
    initialPageParam: 0,
    getNextPageParam: (last, _all, page) =>
      last.hasMore ? page + 1 : undefined,
  });

export const useTransaction = (id: number) =>
  useQuery({
    queryKey: queryKeys.transaction(id),
    queryFn: () => getTransaction(id),
  });

export const useTransactionDefaults = () =>
  useQuery({
    queryKey: queryKeys.transactionDefaults,
    queryFn: getTransactionDefaults,
  });

export const usePayBillDefaults = (cardId: number) =>
  useQuery({
    queryKey: queryKeys.payBillDefaults(cardId),
    queryFn: () => getPayBillDefaults(cardId),
  });

export const useCreateTransaction = () => useLedgerMutation(createTransaction);
export const useUpdateTransaction = () => useLedgerMutation(updateTransaction);
export const useDeleteTransaction = () => useLedgerMutation(deleteTransaction);
