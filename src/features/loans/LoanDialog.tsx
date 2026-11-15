import { Plus, X } from "lucide-react";
import { useRef, useState } from "react";
import { Controller, useFieldArray, useForm, useWatch } from "react-hook-form";
import { ApiError } from "@/api/client";
import type { Account } from "@/bindings/Account";
import type { Category } from "@/bindings/Category";
import type { Loan } from "@/bindings/Loan";
import type { WeekStart } from "@/bindings/WeekStart";
import { ACCOUNT_ICONS } from "@/components/AccountRow";
import { AmountInput } from "@/components/AmountInput";
import { CategoryChip } from "@/components/CategoryChip";
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
import { Segmented } from "@/components/ui/segmented";
import { notify } from "@/components/ui/toast";
import { useCreateLoan, useUpdateLoan } from "@/hooks/useLoans";
import {
  addMonths,
  formatInputDate,
  formatShortDate,
  isIsoDate,
} from "@/lib/dates";
import { formatMoney, type MoneySettings } from "@/lib/money";
import { named, offered } from "@/lib/records";
import { showSaveError } from "@/lib/saveError";

type Row = {
  /** The stored payment this row edits, or null for a new row. */
  paymentId: number | null;
  /** `YYYY-MM-DD`, or empty before one is chosen. */
  dueDate: string;
  amount: number | null;
  /** Set on a payment marked paid: the row is locked and never sent (F12 step 3). */
  paidOn: string | null;
};

type Values = {
  name: string;
  accountId: string | null;
  categoryId: string | null;
  isNew: "new" | "existing";
  amountBorrowed: number | null;
  /** `YYYY-MM-DD`, or empty for none. */
  borrowedOn: string;
  payments: Row[];
};

const SHOWN = [
  "name",
  "accountId",
  "categoryId",
  "amountBorrowed",
  "borrowedOn",
] as const;
const KINDS = [
  { value: "new", label: "A new loan" },
  { value: "existing", label: "A loan I already have" },
] as const;
// The other forms' rows, with room for "Date borrowed".
const ROW =
  "grid-cols-[112px_minmax(0,1fr)] items-center gap-x-3 [&>p]:col-start-2";
const blankRow: Row = {
  paymentId: null,
  dueDate: "",
  amount: null,
  paidOn: null,
};

/**
 * Add and Edit loan (UI_FLOWS F12): the loan's fields and one row per payment. Rows marked paid
 * are locked. The backend checks every field and row; its message shows under the field it
 * names, and a row's under the list, with that row marked.
 */
export function LoanDialog({
  loan,
  accounts,
  categories,
  today,
  weekStart,
  settings,
  onClose,
}: {
  loan?: Loan;
  accounts: Account[];
  categories: Category[];
  today: string;
  weekStart: WeekStart;
  settings: MoneySettings;
  onClose: () => void;
}) {
  // The active expense category named Loan, else none (DATA_MODEL §4.5).
  const loanCategory = categories.find(
    (c) =>
      c.kind === "expense" && !c.archived && c.name.toLowerCase() === "loan",
  );
  const {
    control,
    register,
    handleSubmit,
    setError,
    setValue,
    getValues,
    formState: { errors },
  } = useForm<Values>({
    defaultValues: loan
      ? {
          name: loan.name,
          accountId: String(loan.accountId),
          categoryId: String(loan.categoryId),
          isNew: loan.isNew ? "new" : "existing",
          amountBorrowed: loan.amountBorrowed,
          borrowedOn: loan.borrowedOn ?? "",
          payments: loan.payments.map((p) => ({
            paymentId: p.id,
            dueDate: p.dueDate,
            amount: p.amount,
            paidOn: p.status === "paid" ? (p.paidOn ?? p.dueDate) : null,
          })),
        }
      : {
          name: "",
          accountId: null,
          categoryId: loanCategory ? String(loanCategory.id) : null,
          isNew: "new",
          amountBorrowed: null,
          borrowedOn: today,
          payments: [blankRow],
        },
  });
  const rows = useFieldArray({ control, name: "payments" });
  const [isNew, payments] = useWatch({
    control,
    name: ["isNew", "payments"],
  });
  // A row's error names it by its place among the rows sent (the unlocked ones), from 1.
  const [rowsError, setRowsError] = useState<{
    message: string;
    index: number | null;
  } | null>(null);
  const create = useCreateLoan();
  const update = useUpdateLoan();
  const saving = useRef(false);
  // On a loan the user already has, rows before the day it's added are paid before (§4.6).
  const cutoff = loan?.addedOn ?? today;

  const accountOptions = offered(
    accounts.filter((a) => a.kind !== "credit_card"),
    loan?.accountId ?? null,
  ).map((a) => {
    const Icon = ACCOUNT_ICONS[a.kind];
    return {
      value: String(a.id),
      label: named(a),
      icon: <Icon aria-hidden className="size-4 text-muted-foreground" />,
    };
  });
  const categoryOptions = offered(
    categories.filter((c) => c.kind === "expense"),
    loan?.categoryId ?? null,
  ).map((c) => ({
    value: String(c.id),
    label: named(c),
    icon: <CategoryChip color={c.color} icon={c.icon} size="sm" />,
  }));

  // The previous row's amount, dated the first payment + n months (F12, UI_FLOWS §11 item 24).
  const addRow = () => {
    const list = getValues("payments");
    const first = list[0]?.dueDate ?? "";
    const index = list.length;
    rows.append(
      {
        ...blankRow,
        dueDate: isIsoDate(first) ? addMonths(first, index) : "",
        amount: list[index - 1]?.amount ?? null,
      },
      { shouldFocus: true, focusName: `payments.${index}.amount` },
    );
  };

  const changeKind = (next: Values["isNew"]) => {
    // A loan the user already has needs no date borrowed, and today's would restart the streak.
    const borrowed = getValues("borrowedOn");
    if (next === "existing" && borrowed === today) setValue("borrowedOn", "");
    if (next === "new" && !borrowed) setValue("borrowedOn", today);
    setValue("isNew", next);
  };

  const save = async (v: Values) => {
    if (saving.current) return;
    saving.current = true;
    setRowsError(null);
    const sent = v.payments
      .map((r, index) => ({ r, index }))
      .filter(({ r }) => !r.paidOn);
    try {
      const fields = {
        name: v.name,
        // Empty choices go as 0, for the backend to name.
        accountId: Number(v.accountId) || 0,
        categoryId: Number(v.categoryId) || 0,
        amountBorrowed: v.amountBorrowed,
        borrowedOn: v.borrowedOn.trim() || null,
        payments: sent.map(({ r }) => ({
          id: r.paymentId,
          dueDate: r.dueDate,
          amount: r.amount ?? 0,
        })),
      };
      if (loan) {
        await update.mutateAsync({ id: loan.id, loan: fields });
        notify.success("Saved");
      } else {
        // Task 12 (LOAN-10): a new loan shows "Before you add this loan" before this save.
        await create.mutateAsync({ isNew: v.isNew === "new", ...fields });
        notify.success("Loan added");
      }
      onClose();
    } catch (error) {
      const detail = error instanceof ApiError ? error.detail : null;
      if (detail?.code === "validation" && detail.field === "payments") {
        const n = /^Payment (\d+)/.exec(detail.message)?.[1];
        setRowsError({
          message: detail.message,
          index: n ? (sent[Number(n) - 1]?.index ?? null) : null,
        });
      } else showSaveError(error, SHOWN, setError);
    } finally {
      saving.current = false;
    }
  };

  const money = (amount: number) => formatMoney(amount, settings);
  const existing = isNew === "existing";

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent title={loan ? "Edit loan" : "Add loan"} size="list">
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
              htmlFor="loan-name"
              error={errors.name?.message}
              className={ROW}
            >
              <Input
                id="loan-name"
                maxLength={50}
                autoFocus
                autoComplete="off"
                placeholder="E-wallet loan"
                {...register("name")}
              />
            </Field>
            <Controller
              control={control}
              name="accountId"
              render={({ field }) => (
                <Field
                  label="Paid from"
                  htmlFor="loan-account"
                  error={errors.accountId?.message}
                  className={ROW}
                >
                  <Combobox
                    id="loan-account"
                    options={accountOptions}
                    value={field.value}
                    onChange={field.onChange}
                    placeholder="Choose an account"
                  />
                </Field>
              )}
            />
            <Controller
              control={control}
              name="categoryId"
              render={({ field }) => (
                <Field
                  label="Category"
                  htmlFor="loan-category"
                  error={errors.categoryId?.message}
                  className={ROW}
                >
                  <Combobox
                    id="loan-category"
                    options={categoryOptions}
                    value={field.value}
                    onChange={field.onChange}
                    placeholder="Choose a category"
                  />
                </Field>
              )}
            />
            {/* Whether it's new can't change once it's added (DATA_MODEL §4.5). */}
            {!loan && (
              <div className={`grid gap-1 ${ROW}`}>
                <span className="text-small font-semibold text-muted-foreground">
                  This is
                </span>
                <Segmented
                  aria-label="This is"
                  value={isNew}
                  onChange={changeKind}
                  options={[...KINDS]}
                  className="justify-self-start"
                />
              </div>
            )}
            <Controller
              control={control}
              name="amountBorrowed"
              render={({ field }) => (
                <Field
                  label="Amount borrowed"
                  htmlFor="loan-borrowed"
                  error={errors.amountBorrowed?.message}
                  help="Optional. Add it to see what the loan costs in interest."
                  className={ROW}
                >
                  <AmountInput
                    id="loan-borrowed"
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
              name="borrowedOn"
              render={({ field }) => (
                <Field
                  label="Date borrowed"
                  htmlFor="loan-borrowedOn"
                  error={errors.borrowedOn?.message}
                  help={existing ? "Optional." : undefined}
                  className={ROW}
                >
                  <DatePicker
                    id="loan-borrowedOn"
                    value={field.value}
                    onChange={field.onChange}
                    weekStart={weekStart}
                  />
                </Field>
              )}
            />
            <h3 className="mt-1 border-t border-hairline pt-3 text-small font-semibold text-muted-foreground">
              Payments
            </h3>
            <ul
              aria-label="Payments"
              aria-describedby={rowsError ? "loan-rows-message" : undefined}
              className="grid max-h-[32vh] gap-2 overflow-y-auto p-0.5"
            >
              {rows.fields.map((row, index) => {
                const current = payments[index] ?? row;
                const n = index + 1;
                const invalid = rowsError?.index === index;
                if (current.paidOn)
                  return (
                    <li
                      key={row.id}
                      className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_112px_36px] items-center gap-2 text-muted-foreground"
                    >
                      <span className="px-3 tabular-nums">
                        {formatInputDate(current.dueDate)}
                      </span>
                      <span className="px-3 text-right font-semibold tabular-nums">
                        {current.amount === null ? "" : money(current.amount)}
                      </span>
                      <span className="text-small">
                        Paid {formatShortDate(current.paidOn)}
                      </span>
                    </li>
                  );
                return (
                  <li
                    key={row.id}
                    className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_112px_36px] items-center gap-2"
                  >
                    <Controller
                      control={control}
                      name={`payments.${index}.dueDate`}
                      render={({ field }) => (
                        <DatePicker
                          aria-label={`Payment ${n} date`}
                          aria-invalid={invalid || undefined}
                          value={field.value}
                          onChange={field.onChange}
                          weekStart={weekStart}
                        />
                      )}
                    />
                    <Controller
                      control={control}
                      name={`payments.${index}.amount`}
                      render={({ field }) => (
                        <AmountInput
                          aria-label={`Payment ${n} amount`}
                          aria-invalid={invalid || undefined}
                          ref={field.ref}
                          value={field.value}
                          onChange={field.onChange}
                          onBlur={field.onBlur}
                          onKeyDown={(e) => {
                            // Enter in the last amount adds a row rather than saving (F12).
                            if (
                              e.key === "Enter" &&
                              index === rows.fields.length - 1
                            ) {
                              e.preventDefault();
                              addRow();
                            }
                          }}
                        />
                      )}
                    />
                    <span className="text-small text-muted-foreground">
                      {existing && current.dueDate && current.dueDate < cutoff
                        ? "Paid before"
                        : ""}
                    </span>
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label={`Remove payment ${n}`}
                      onClick={() => rows.remove(index)}
                    >
                      <X />
                    </Button>
                  </li>
                );
              })}
            </ul>
            {rowsError && (
              <p
                id="loan-rows-message"
                role="alert"
                className="text-small text-danger"
              >
                {rowsError.message}
              </p>
            )}
            <Button
              size="sm"
              variant="ghost"
              className="justify-self-start"
              onClick={addRow}
            >
              <Plus />
              Add payment
            </Button>
          </DialogBody>
          <DialogFooter>
            <Button onClick={onClose}>Cancel</Button>
            <Button variant="primary" type="submit">
              {loan ? "Save" : "Add loan"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
