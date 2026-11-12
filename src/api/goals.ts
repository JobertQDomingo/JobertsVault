import type { Goal } from "@/bindings/Goal";
import type { GoalFields } from "@/bindings/GoalFields";
import type { GoalProgress } from "@/bindings/GoalProgress";
import { call } from "./client";

/** Every goal with its account's progress as of today (GOL-3), in the order they were created. */
export const listGoals = () => call<GoalProgress[]>("list_goals");

/** A goal is linked to one account that isn't a credit card (GOL-1, GOL-2). */
export const createGoal = (goal: GoalFields) =>
  call<Goal>("create_goal", { goal });

export const updateGoal = ({ id, goal }: { id: number; goal: GoalFields }) =>
  call<Goal>("update_goal", { id, goal });

export const deleteGoal = (id: number) => call<null>("delete_goal", { id });
