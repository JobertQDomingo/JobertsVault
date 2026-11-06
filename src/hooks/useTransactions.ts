import {
  keepPreviousData,
  skipToken,
  useInfiniteQuery,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import type { TransactionFilter } from "@/bindings/TransactionFilter";
import type { TransactionKind } from "@/bindings/TransactionKind";
import {
  createTransaction,
  deleteTransaction,
  getPayBillDefaults,
  getTransaction,
  getTransactionDefaults,
  listTransactions,
  suggestPayees,
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
    queryFn: id === undefined ? skipToken : () => getTransaction(id),
  });

export const useTransactionDefaults = (enabled = true) =>
  useQuery({
    queryKey: queryKeys.transactionDefaults,
    queryFn: enabled ? getTransactionDefaults : skipToken,
  });

/**
 * Quick-add's payee suggestions (TXN-6): asked for only once something's typed, and never for a
 * transfer. The last list stays while the next keystroke's loads, so it doesn't flicker.
 */
export const usePayeeSuggestions = (prefix: string, kind: TransactionKind) => {
  const text = prefix.trim();
  const category = kind === "transfer" ? null : kind;
  return useQuery({
    queryKey: queryKeys.payees(text, category ?? "expense"),
    queryFn: text && category ? () => suggestPayees(text, category) : skipToken,
    placeholderData: keepPreviousData,
  });
};

/** Only loads when given a card. */
export const usePayBillDefaults = (cardId?: number) =>
  useQuery({
    queryKey: queryKeys.payBillDefaults(cardId ?? 0),
    queryFn:
      cardId === undefined ? skipToken : () => getPayBillDefaults(cardId),
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
