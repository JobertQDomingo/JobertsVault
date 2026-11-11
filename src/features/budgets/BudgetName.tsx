import type { BudgetPeriod } from "@/bindings/BudgetPeriod";
import type { Category } from "@/bindings/Category";
import { AllSpendingChip, CategoryChip } from "@/components/CategoryChip";
import { EVERY_ONE } from "@/lib/recurrence";

type Covers = { categoryId: number | null; period: BudgetPeriod };

/** "Weekly", "Monthly" or "Yearly", the same words a recurring rule uses. */
export const periodLabel = (period: BudgetPeriod) => EVERY_ONE[period];

/** What a budget covers: its category's name, or "All spending". */
export const budgetName = (
  categoryId: number | null,
  categories: Category[],
) =>
  categoryId === null
    ? "All spending"
    : (categories.find((c) => c.id === categoryId)?.name ?? "");

/** "Dining Out, weekly": names a budget's controls and bar on every screen. */
export const budgetTitle = (budget: Covers, categories: Category[]) =>
  `${budgetName(budget.categoryId, categories)}, ${budget.period}`;

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
      {categoryId === null ? (
        <AllSpendingChip size="sm" />
      ) : (
        category && (
          <CategoryChip color={category.color} icon={category.icon} size="sm" />
        )
      )}
      <span className="truncate">{budgetName(categoryId, categories)}</span>
    </span>
  );
}
