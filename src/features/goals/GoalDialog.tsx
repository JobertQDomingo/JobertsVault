import { useRef } from "react";
import { Controller, useForm } from "react-hook-form";
import type { Account } from "@/bindings/Account";
import type { GoalProgress } from "@/bindings/GoalProgress";
import type { WeekStart } from "@/bindings/WeekStart";
import { AmountInput } from "@/components/AmountInput";
import { ACCOUNT_ICONS } from "@/components/AccountRow";
import { Button } from "@/components/ui/button";
import { Combobox } from "@/components/ui/combobox";
import { DatePicker } from "@/components/ui/date-picker";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
} from "@/components/ui/dialog";
import { Field, Input } from "@/components/ui/input";
import { notify } from "@/components/ui/toast";
import { useCreateGoal, useUpdateGoal } from "@/hooks/useGoals";
import { named, offered } from "@/lib/records";
import { showSaveError } from "@/lib/saveError";

type Values = {
  name: string;
  targetAmount: number | null;
  /** `YYYY-MM-DD`, or empty for no target date. */
  targetDate: string;
  /** An account's id, or empty before one is chosen. */
  accountId: string;
};

const SHOWN = ["name", "targetAmount", "targetDate", "accountId"] as const;
// The quick-add form's rows: a 96 px label, the field beside it, its message under the field.
const ROW =
  "grid-cols-[96px_minmax(0,1fr)] items-center gap-x-3 [&>p]:col-start-2";

/**
 * New and edit goal (UI_FLOWS §8.4): a name, the target, an optional date, and the account whose
 * balance is the progress (GOL-1, GOL-2). The backend checks every field, and its message shows
 * under the field it names.
 */
export function GoalDialog({
  goal,
  accounts,
  weekStart,
  onClose,
}: {
  goal?: GoalProgress;
  accounts: Account[];
  weekStart: WeekStart;
  onClose: () => void;
}) {
  const { control, register, handleSubmit, setError, formState } =
    useForm<Values>({
      defaultValues: goal
        ? {
            name: goal.name,
            targetAmount: goal.targetAmount,
            targetDate: goal.targetDate ?? "",
            accountId: String(goal.accountId),
          }
        : { name: "", targetAmount: null, targetDate: "", accountId: "" },
    });
  const { errors } = formState;
  const create = useCreateGoal();
  const update = useUpdateGoal();
  const saving = useRef(false);
  const options = offered(
    accounts.filter((a) => a.kind !== "credit_card"),
    goal?.accountId ?? null,
  ).map((a) => {
    const Icon = ACCOUNT_ICONS[a.kind];
    return {
      value: String(a.id),
      label: named(a),
      icon: <Icon aria-hidden className="size-4 text-muted-foreground" />,
    };
  });

  const save = async (v: Values) => {
    if (saving.current) return;
    saving.current = true;
    try {
      const fields = {
        name: v.name,
        // Empty fields go as 0, for the backend to name.
        targetAmount: v.targetAmount ?? 0,
        targetDate: v.targetDate.trim() || null,
        accountId: Number(v.accountId) || 0,
      };
      if (goal) await update.mutateAsync({ id: goal.id, goal: fields });
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
      <DialogContent title={goal ? "Edit goal" : "New goal"}>
        <form
          noValidate
          onSubmit={(e) => {
            e.preventDefault();
            void handleSubmit(save)();
          }}
        >
          <DialogBody>
            <Field
              label="Name"
              htmlFor="goal-name"
              error={errors.name?.message}
              className={ROW}
            >
              <Input
                id="goal-name"
                maxLength={50}
                autoFocus
                autoComplete="off"
                {...register("name")}
              />
            </Field>
            <Controller
              control={control}
              name="targetAmount"
              render={({ field }) => (
                <Field
                  label="Target"
                  htmlFor="goal-target"
                  error={errors.targetAmount?.message}
                  className={ROW}
                >
                  <AmountInput
                    id="goal-target"
                    ref={field.ref}
                    value={field.value}
                    onChange={field.onChange}
                    onBlur={field.onBlur}
                  />
                </Field>
              )}
            />
            <Controller
              control={control}
              name="targetDate"
              render={({ field }) => (
                <Field
                  label="Target date"
                  htmlFor="goal-date"
                  error={errors.targetDate?.message}
                  help="Optional. With a date, the goal says how much to save each month."
                  className={ROW}
                >
                  <DatePicker
                    id="goal-date"
                    value={field.value}
                    onChange={field.onChange}
                    weekStart={weekStart}
                  />
                </Field>
              )}
            />
            <Controller
              control={control}
              name="accountId"
              render={({ field }) => (
                <Field
                  label="Account"
                  htmlFor="goal-account"
                  error={errors.accountId?.message}
                  className={ROW}
                >
                  <Combobox
                    id="goal-account"
                    options={options}
                    value={field.value}
                    onChange={field.onChange}
                    placeholder="Choose an account"
                  />
                </Field>
              )}
            />
            <p className="text-small text-muted-foreground">
              The account's balance is the goal's progress, so save toward it by
              adding money to that account.
            </p>
          </DialogBody>
          <DialogFooter>
            <Button onClick={onClose}>Cancel</Button>
            <Button variant="primary" type="submit">
              {goal ? "Save" : "Create goal"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
