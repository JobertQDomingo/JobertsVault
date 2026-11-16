import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import {
  archiveGoal,
  createGoal,
  deleteGoal,
  listGoals,
  suggestGoal,
  unarchiveGoal,
  updateGoal,
} from "@/api/goals";
import type { GoalDraft } from "@/bindings/GoalDraft";
import { queryKeys } from "@/lib/queryKeys";
import { useLedgerMutation } from "./useLedgerMutation";

export const useGoals = () =>
  useQuery({ queryKey: queryKeys.goals, queryFn: listGoals });

/** Asks `suggest_goal` 250 ms after typing stops, like the payslip's net (UI_FLOWS §11 item 52). */
export function useGoalSuggestion(draft: GoalDraft | null) {
  const [asked, setAsked] = useState(draft);
  const key = JSON.stringify(draft);
  useEffect(() => {
    const timer = setTimeout(
      () => setAsked(JSON.parse(key) as GoalDraft | null),
      250,
    );
    return () => clearTimeout(timer);
  }, [key]);
  return useQuery({
    queryKey: queryKeys.goalSuggestion(asked),
    queryFn: () => (asked ? suggestGoal(asked) : null),
    enabled: asked !== null,
    placeholderData: keepPreviousData,
  });
}

// Ledger writes: a goal's target locks the decimals and puts its account in use.
export const useCreateGoal = () => useLedgerMutation(createGoal);
export const useUpdateGoal = () => useLedgerMutation(updateGoal);
export const useArchiveGoal = () => useLedgerMutation(archiveGoal);
export const useUnarchiveGoal = () => useLedgerMutation(unarchiveGoal);
export const useDeleteGoal = () => useLedgerMutation(deleteGoal);
