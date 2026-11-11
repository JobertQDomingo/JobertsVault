import { useRef } from "react";
import { Controller, useForm } from "react-hook-form";
import type { BudgetPeriod } from "@/bindings/BudgetPeriod";
import type { BudgetProgress } from "@/bindings/BudgetProgress";
import type { Category } from "@/bindings/Category";
import { AmountInput } from "@/components/AmountInput";
import { AllSpendingChip, CategoryChip } from "@/components/CategoryChip";
import { Button } from "@/components/ui/button";
import { Combobox } from "@/components/ui/combobox";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
} from "@/components/ui/dialog";
import { Field } from "@/components/ui/input";
import { Segmented } from "@/components/ui/segmented";
import { notify } from "@/components/ui/toast";
import { useCreateBudget, useUpdateBudget } from "@/hooks/useBudgets";
import { named, offered } from "@/lib/records";
import { showSaveError } from "@/lib/saveError";
import { PERIODS } from "./BudgetName";

type Values = {
  /** A category's id, or ALL for all spending. */
  categoryId: string;
  period: BudgetPeriod;
  amount: number | null;
};

const ALL = "all";
const SHOWN = ["categoryId", "period", "amount"] as const;
// The quick-add form's rows: a 96 px label, the field beside it, its message under the field.
const ROW =
  "grid-cols-[96px_minmax(0,1fr)] items-center gap-x-3 [&>p]:col-start-2";
const PERIOD_OPTIONS = (Object.keys(PERIODS) as BudgetPeriod[]).map(
  (value) => ({ value, label: PERIODS[value] }),
);

/**
 * New and edit budget (UI_FLOWS §8.3): all spending or an expense category, the period and the
 * limit (BUD-1). The backend checks every field, and its message shows under the field it names.
 */
export function BudgetDialog({
  budget,
  categories,
  onClose,
}: {
  budget?: BudgetProgress;
  categories: Category[];
  onClose: () => void;
}) {
  const { control, handleSubmit, setError, formState } = useForm<Values>({
    defaultValues: budget
      ? {
          categoryId: budget.categoryId?.toString() ?? ALL,
          period: budget.period,
          amount: budget.amount,
        }
      : { categoryId: ALL, period: "monthly", amount: null },
  });
  const { errors } = formState;
  const create = useCreateBudget();
  const update = useUpdateBudget();
  const saving = useRef(false);
  const options = [
    { value: ALL, label: "All spending", icon: <AllSpendingChip size="sm" /> },
    ...offered(
      categories.filter((c) => c.kind === "expense"),
      budget?.categoryId ?? null,
    ).map((c) => ({
      value: String(c.id),
      label: named(c),
      icon: <CategoryChip color={c.color} icon={c.icon} size="sm" />,
    })),
  ];

  const save = async (v: Values) => {
    if (saving.current) return;
    saving.current = true;
    try {
      const fields = {
        categoryId: v.categoryId === ALL ? null : Number(v.categoryId),
        period: v.period,
        // An empty amount goes as 0, for the backend to name.
        amount: v.amount ?? 0,
      };
      if (budget) await update.mutateAsync({ id: budget.id, budget: fields });
      else await create.mutateAsync(fields);
      notify.success("Saved");
      onClose();
    } catch (error) {
      showSaveError(error, SHOWN, setError);
    } finally {
      saving.current = false;
    }
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent title={budget ? "Edit budget" : "New budget"}>
        <form
          noValidate
          onSubmit={(e) => {
            e.preventDefault();
            void handleSubmit(save)();
          }}
        >
          <DialogBody>
            <Controller
              control={control}
              name="categoryId"
              render={({ field }) => (
                <Field
                  label="Category"
                  htmlFor="budget-category"
                  error={errors.categoryId?.message}
                  className={ROW}
                >
                  <Combobox
                    id="budget-category"
                    options={options}
                    value={field.value}
                    onChange={field.onChange}
                  />
                </Field>
              )}
            />
            <div className={`grid gap-1 ${ROW}`}>
              <span className="text-small font-semibold text-muted-foreground">
                Period
              </span>
              <Controller
                control={control}
                name="period"
                render={({ field }) => (
                  <Segmented
                    aria-label="Period"
                    options={PERIOD_OPTIONS}
                    value={field.value}
                    onChange={field.onChange}
                    className="justify-self-start"
                  />
                )}
              />
              {errors.period && (
                <p role="alert" className="text-small text-danger">
                  {errors.period.message}
                </p>
              )}
            </div>
            <Controller
              control={control}
              name="amount"
              render={({ field }) => (
                <Field
                  label="Amount"
                  htmlFor="budget-amount"
                  error={errors.amount?.message}
                  className={ROW}
                >
                  <AmountInput
                    id="budget-amount"
                    ref={field.ref}
                    value={field.value}
                    onChange={field.onChange}
                    onBlur={field.onBlur}
                    autoFocus
                  />
                </Field>
              )}
            />
            <p className="text-small text-muted-foreground">
              Each period starts again from the full amount. What's left doesn't
              carry over.
            </p>
          </DialogBody>
          <DialogFooter>
            <Button onClick={onClose}>Cancel</Button>
            <Button variant="primary" type="submit">
              {budget ? "Save" : "Create budget"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
