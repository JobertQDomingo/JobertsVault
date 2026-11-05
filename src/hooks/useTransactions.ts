import {
  keepPreviousData,
  useInfiniteQuery,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
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
    // A new filter keeps the old rows until its own arrive, instead of blanking.
    placeholderData: keepPreviousData,
  });

/** Loads only with an id: quick-add asks for one only to edit or duplicate. */
export const useTransaction = (id?: number) =>
  useQuery({
    queryKey: queryKeys.transaction(id ?? 0),
    queryFn: () => getTransaction(id as number),
    enabled: id !== undefined,
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
/** Drops the deleted row's query before the ledger refresh, so it isn't fetched again. */
export const useDeleteTransaction = () => {
  const client = useQueryClient();
  return useLedgerMutation(async (id: number) => {
    await deleteTransaction(id);
    client.removeQueries({ queryKey: queryKeys.transaction(id) });
  });
};
