import { useQuery } from "@tanstack/react-query";
import {
  createBudget,
  deleteBudget,
  listBudgets,
  updateBudget,
} from "@/api/budgets";
import { queryKeys } from "@/lib/queryKeys";
import { useLedgerMutation } from "./useLedgerMutation";

export const useBudgets = () =>
  useQuery({ queryKey: queryKeys.budgets, queryFn: listBudgets });

// Ledger writes: a budget's amount locks the decimals.
export const useCreateBudget = () => useLedgerMutation(createBudget);
export const useUpdateBudget = () => useLedgerMutation(updateBudget);
export const useDeleteBudget = () => useLedgerMutation(deleteBudget);
