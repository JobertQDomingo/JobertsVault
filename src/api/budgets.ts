import type { Budget } from "@/bindings/Budget";
import type { BudgetFields } from "@/bindings/BudgetFields";
import { call } from "./client";

/** All spending first, then in the categories' order, weekly before monthly before yearly. */
export const listBudgets = () => call<Budget[]>("list_budgets");

/** One budget per category (or all spending) and period (BUD-1). */
export const createBudget = (budget: BudgetFields) =>
  call<Budget>("create_budget", { budget });

export const updateBudget = ({
  id,
  budget,
}: {
  id: number;
  budget: BudgetFields;
}) => call<Budget>("update_budget", { id, budget });

export const deleteBudget = (id: number) => call<null>("delete_budget", { id });
