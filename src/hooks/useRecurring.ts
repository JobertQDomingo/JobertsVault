import { useQuery } from "@tanstack/react-query";
import {
  createRecurringRule,
  deleteRecurringRule,
  listRecurringRules,
  pauseRecurringRule,
  updateRecurringRule,
} from "@/api/recurring";
import { queryKeys } from "@/lib/queryKeys";
import { useLedgerMutation } from "./useLedgerMutation";

export const useRecurringRules = () =>
  useQuery({ queryKey: queryKeys.recurringRules, queryFn: listRecurringRules });

// Ledger writes: a rule puts its account, category and payment method in use, and its amount
// locks the decimals.
export const useCreateRecurringRule = () =>
  useLedgerMutation(createRecurringRule);
export const useUpdateRecurringRule = () =>
  useLedgerMutation(updateRecurringRule);
export const usePauseRecurringRule = () =>
  useLedgerMutation(pauseRecurringRule);
export const useDeleteRecurringRule = () =>
  useLedgerMutation(deleteRecurringRule);
