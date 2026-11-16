import { useEffect, useRef } from "react";
import { Controller, useForm, useWatch } from "react-hook-form";
import { Link } from "react-router";
import type { Account } from "@/bindings/Account";
import type { GoalProgress } from "@/bindings/GoalProgress";
import type { GoalSuggestion } from "@/bindings/GoalSuggestion";
import type { SettingsView } from "@/bindings/SettingsView";
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
import {
  useCreateGoal,
  useGoalSuggestion,
  useUpdateGoal,
} from "@/hooks/useGoals";
import { formatDate } from "@/lib/dates";
import { formatMoney } from "@/lib/money";
import { named, offered } from "@/lib/records";
import { showSaveError } from "@/lib/saveError";

type Values = {
  name: string;
  targetAmount: number | null;
  /** `YYYY-MM-DD`, or empty: only a goal from before Phase 4 opens without one (GOL-6). */
  targetDate: string;
  /** An account's id, or empty before one is chosen. */
  accountId: string;
};

const SHOWN = ["name", "targetAmount", "targetDate", "accountId"] as const;
// The quick-add form's rows: a 96 px label, the field beside it, its message under the field.
const ROW =
  "grid-cols-[96px_minmax(0,1fr)] items-center gap-x-3 [&>p]:col-start-2";

/**
 * New and edit goal (UI_FLOWS §8.4): a name, the target, a target date, and the account whose
 * balance is the progress (GOL-1, GOL-2). Under the date, the backend's suggestion: what can be set
 * aside each month and the earliest realistic date, where a new goal's date starts (GOL-5). The
 * backend checks every field, and its message shows under the field it names (GOL-6).
 */
export function GoalDialog({
  goal,
  accounts,
  settings,
  onClose,
}: {
  goal?: GoalProgress;
  accounts: Account[];
  settings: SettingsView;
  onClose: () => void;
}) {
  const { control, register, handleSubmit, setError, setValue, formState } =
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
  const [targetAmount, accountId] = useWatch({
    control,
    name: ["targetAmount", "accountId"],
  });
  const draft =
    targetAmount !== null && targetAmount > 0 && Number(accountId) > 0
      ? {
          targetAmount,
          accountId: Number(accountId),
          goalId: goal?.id ?? null,
        }
      : null;
  const suggestion = useGoalSuggestion(draft);
  // A new goal's date follows the suggestion until the user picks one (UI_FLOWS §11 item 53).
  const picked = useRef(goal !== undefined);
  const earliest = suggestion.isPlaceholderData
    ? null
    : suggestion.data?.earliestDate;
  useEffect(() => {
    if (earliest && !picked.current) setValue("targetDate", earliest);
  }, [earliest, setValue]);
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
                  help={
                    draft &&
                    suggestion.data !== undefined && (
                      <SuggestionLine
                        suggestion={suggestion.data}
                        settings={settings}
                      />
                    )
                  }
                  className={ROW}
                >
                  <DatePicker
                    id="goal-date"
                    value={field.value}
                    onChange={(date) => {
                      picked.current = true;
                      field.onChange(date);
                    }}
                    weekStart={settings.weekStart}
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

/** The line under the date: the backend's monthly room and earliest date, or why there's none. */
function SuggestionLine({
  suggestion,
  settings,
}: {
  suggestion: GoalSuggestion | null;
  settings: SettingsView;
}) {
  if (suggestion === null)
    return (
      <>
        Mark your pay as a payday first, so the app can suggest a realistic
        date.{" "}
        <Link
          to="/recurring"
          className="font-semibold text-primary hover:underline"
        >
          Open Recurring
        </Link>
      </>
    );
  if (suggestion.earliestDate === null)
    return "You can't set anything aside each month right now, so no date is realistic yet.";
  return `You can set aside about ${formatMoney(suggestion.monthlyRoom, settings)} a month. The earliest realistic date is ${formatDate(suggestion.earliestDate)}.`;
}
