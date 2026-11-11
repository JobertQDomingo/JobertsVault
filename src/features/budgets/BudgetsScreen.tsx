import { Ellipsis, Plus, Target } from "lucide-react";
import { useState } from "react";
import type { BudgetProgress } from "@/bindings/BudgetProgress";
import type { Category } from "@/bindings/Category";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { Loading, Skeleton } from "@/components/Loading";
import { ProgressBar } from "@/components/ProgressBar";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/ui/dialog";
import { Menu, MenuContent, MenuItem, MenuTrigger } from "@/components/ui/menu";
import { notify } from "@/components/ui/toast";
import { useBudgets, useDeleteBudget } from "@/hooks/useBudgets";
import { useCategories } from "@/hooks/useCategories";
import { useSettings } from "@/hooks/useSettings";
import { formatMoney, type MoneySettings } from "@/lib/money";
import { cn } from "@/lib/utils";
import { BudgetDialog } from "./BudgetDialog";
import { BudgetName, budgetName, PERIODS } from "./BudgetName";

/**
 * `/budgets` (UI_FLOWS §8.3): each budget's current period as a tile with its progress (BUD-2,
 * BUD-3). A tile opens its form; ⋯ edits or deletes it. One glass panel holds the flat tiles, so
 * any number of budgets stays inside the blur budget (DESIGN_SYSTEM §2.3).
 */
export function BudgetsScreen() {
  const budgets = useBudgets();
  const categories = useCategories();
  const settings = useSettings();
  const queries = [budgets, categories, settings];
  const failed = queries.filter((q) => q.isError);
  const remove = useDeleteBudget();
  // `{}` is a new budget.
  const [editing, setEditing] = useState<{ budget?: BudgetProgress } | null>(
    null,
  );
  const [deleting, setDeleting] = useState<BudgetProgress | null>(null);
  const newBudget = (
    <Button variant="primary" size="sm" onClick={() => setEditing({})}>
      <Plus />
      New budget
    </Button>
  );

  let body;
  if (failed.length)
    body = (
      <ErrorState
        onRetry={() => failed.forEach((q) => q.refetch())}
        pending={failed.some((q) => q.isFetching)}
      />
    );
  else if (!budgets.data || !categories.data || !settings.data)
    body = (
      <Loading className="grid grid-cols-3 gap-3 px-[18px] pb-[18px]">
        {[0, 1, 2].map((i) => (
          <Skeleton key={i} className="h-[104px]" />
        ))}
      </Loading>
    );
  else if (budgets.data.length === 0)
    body = (
      <EmptyState icon={Target} message="No budgets yet." action={newBudget} />
    );
  else {
    const categoryList = categories.data.categories;
    const money = settings.data;
    body = (
      <ul className="grid grid-cols-3 gap-3 px-[18px] pb-[18px]">
        {budgets.data.map((budget) => (
          <Tile
            key={budget.id}
            budget={budget}
            categories={categoryList}
            settings={money}
            onEdit={() => setEditing({ budget })}
            onDelete={() => setDeleting(budget)}
          />
        ))}
      </ul>
    );
  }

  return (
    <GlassPanel className="overflow-hidden">
      <div className="flex items-center justify-between gap-2 px-[18px] pt-4 pb-2.5">
        <h2 className="text-h3">Budgets</h2>
        {newBudget}
      </div>
      {body}
      {editing && categories.data && (
        <BudgetDialog
          budget={editing.budget}
          categories={categories.data.categories}
          onClose={() => setEditing(null)}
        />
      )}
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title="Delete this budget?"
        message="Its transactions stay. This can't be undone."
        confirmLabel="Delete"
        destructive
        onConfirm={() =>
          deleting &&
          remove.mutate(deleting.id, {
            onSuccess: () => notify.success("Deleted"),
            onError: (error) => notify.error(error.message),
          })
        }
      />
    </GlassPanel>
  );
}

/** "₱320.00 of ₱400.00 · ₱80.00 left", or "… · ₱30.00 over" past the limit (UI_FLOWS §8.3). */
function budgetCaption(budget: BudgetProgress, settings: MoneySettings) {
  const money = (amount: number) => formatMoney(amount, settings);
  const rest = budget.remaining;
  return `${money(budget.spent)} of ${money(budget.amount)} · ${money(Math.abs(rest))} ${rest < 0 ? "over" : "left"}`;
}

function Tile({
  budget,
  categories,
  settings,
  onEdit,
  onDelete,
}: {
  budget: BudgetProgress;
  categories: Category[];
  settings: MoneySettings;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const title = `${budgetName(budget.categoryId, categories)}, ${budget.period}`;
  return (
    // The name's button stretches over the whole tile, so the progress bar stays outside it.
    <li className="relative grid gap-2.5 rounded-control bg-wash p-4 pt-3 transition-colors duration-150 hover:bg-glass-hover">
      <div className="flex min-h-8 items-center gap-2">
        <button
          type="button"
          onClick={onEdit}
          aria-label={`Edit ${title}`}
          className="min-w-0 flex-1 text-left font-semibold outline-none after:absolute after:inset-0 after:rounded-control focus-visible:after:ring-3 focus-visible:after:ring-ring/40"
        >
          <BudgetName categoryId={budget.categoryId} categories={categories} />
        </button>
        <span className="text-small text-muted-foreground">
          {PERIODS[budget.period]}
        </span>
        <Menu>
          <MenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              aria-label={`More actions for ${title}`}
              className="relative -mr-2"
            >
              <Ellipsis />
            </Button>
          </MenuTrigger>
          <MenuContent>
            <MenuItem onSelect={onEdit}>Edit</MenuItem>
            <MenuItem destructive onSelect={onDelete}>
              Delete
            </MenuItem>
          </MenuContent>
        </Menu>
      </div>
      <ProgressBar percent={budget.percent} aria-label={`${title} used`} />
      <p
        className={cn(
          "text-small text-muted-foreground tabular-nums",
          budget.remaining < 0 && "text-danger",
        )}
      >
        {budgetCaption(budget, settings)}
      </p>
    </li>
  );
}
