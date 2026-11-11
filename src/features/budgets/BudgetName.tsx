import type { BudgetPeriod } from "@/bindings/BudgetPeriod";
import type { Category } from "@/bindings/Category";
import { AllSpendingChip, CategoryChip } from "@/components/CategoryChip";

export const PERIODS: Record<BudgetPeriod, string> = {
  weekly: "Weekly",
  monthly: "Monthly",
  yearly: "Yearly",
};

/** What a budget covers: its category's name, or "All spending". */
export const budgetName = (
  categoryId: number | null,
  categories: Category[],
) =>
  categoryId === null
    ? "All spending"
    : (categories.find((c) => c.id === categoryId)?.name ?? "");

/** The chip and name a budget shows on the Budgets screen and the dashboard. */
export function BudgetName({
  categoryId,
  categories,
}: {
  categoryId: number | null;
  categories: Category[];
}) {
  const category = categories.find((c) => c.id === categoryId);
  return (
    <span className="inline-flex min-w-0 items-center gap-2">
      {category ? (
        <CategoryChip color={category.color} icon={category.icon} size="sm" />
      ) : (
        <AllSpendingChip size="sm" />
      )}
      <span className="truncate">{budgetName(categoryId, categories)}</span>
    </span>
  );
}
