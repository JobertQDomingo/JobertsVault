import { useQuery } from "@tanstack/react-query";
import {
  archiveGoal,
  createGoal,
  deleteGoal,
  listGoals,
  unarchiveGoal,
  updateGoal,
} from "@/api/goals";
import { queryKeys } from "@/lib/queryKeys";
import { useLedgerMutation } from "./useLedgerMutation";

export const useGoals = () =>
  useQuery({ queryKey: queryKeys.goals, queryFn: listGoals });

// Ledger writes: a goal's target locks the decimals and puts its account in use.
export const useCreateGoal = () => useLedgerMutation(createGoal);
export const useUpdateGoal = () => useLedgerMutation(updateGoal);
export const useArchiveGoal = () => useLedgerMutation(archiveGoal);
export const useUnarchiveGoal = () => useLedgerMutation(unarchiveGoal);
export const useDeleteGoal = () => useLedgerMutation(deleteGoal);
