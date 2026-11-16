import { Plus, Trash2, X } from "lucide-react";
import { useRef, useState } from "react";
import { Controller, useFieldArray, useForm, useWatch } from "react-hook-form";
import { ApiError } from "@/api/client";
import type { Account } from "@/bindings/Account";
import type { Category } from "@/bindings/Category";
import type { Deduction } from "@/bindings/Deduction";
import type { PayslipFields } from "@/bindings/PayslipFields";
import type { WeekStart } from "@/bindings/WeekStart";
import { ACCOUNT_ICONS } from "@/components/AccountRow";
import { AmountInput } from "@/components/AmountInput";
import { CategoryChip } from "@/components/CategoryChip";
import { ErrorState } from "@/components/ErrorState";
import { Loading, Skeleton } from "@/components/Loading";
import { Button } from "@/components/ui/button";
import { Combobox } from "@/components/ui/combobox";
import { DatePicker } from "@/components/ui/date-picker";
import {
  ConfirmDialog,
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
} from "@/components/ui/dialog";
import { Field, Input } from "@/components/ui/input";
import { notify } from "@/components/ui/toast";
import { useAccounts } from "@/hooks/useAccounts";
import { useCategories } from "@/hooks/useCategories";
import {
  usePayslip,
  usePayslipDefaults,
  usePayslipNet,
  useRecordPayslip,
  useUpdatePayslip,
} from "@/hooks/usePayslips";
import { useSettings } from "@/hooks/useSettings";
import { useDeleteTransaction } from "@/hooks/useTransactions";
import { formatMoney } from "@/lib/money";
import { named, offered } from "@/lib/records";
import { showSaveError } from "@/lib/saveError";
import { cn } from "@/lib/utils";

/** A payday to record (its rule and scheduled date), or a payslip to edit (any of its rows). */
export type PayslipRequest = { ruleId: number; date: string } | { id: number };

type Line = { categoryId: string | null; amount: number | null };
type Values = {
  date: string;
  accountId: string | null;
  gross: number | null;
  deductions: Line[];
  note: string;
};

/**
 * The PAY-3 deduction categories, listed first in a line's category list (UI_FLOWS F14). Matched
 * by name, so a renamed one is listed with the other expense categories (UI_FLOWS §11).
 */
const DEDUCTIONS = [
  "SSS",
  "PhilHealth",
  "Pag-IBIG",
  "Withholding tax",
  "Salary loan",
  "Other deductions",
];

const NET_ERROR = "Deductions can't add up to the gross pay or more";

// The other forms' rows: a 96 px label, the field beside it, its message under the field.
const ROW =
  "grid-cols-[96px_minmax(0,1fr)] items-center gap-x-3 [&>p]:col-start-2";

/** Empty choices and amounts go as 0, for the backend to name. */
const toFields = (v: Values): PayslipFields => ({
  date: v.date,
  accountId: Number(v.accountId) || 0,
  gross: v.gross ?? 0,
  deductions: v.deductions.map((d) => ({
    categoryId: Number(d.categoryId) || 0,
    amount: d.amount ?? 0,
  })),
  note: v.note.trim() || null,
});

const toLines = (deductions: Deduction[]): Line[] =>
  deductions.map((d) => ({
    categoryId: String(d.categoryId),
    amount: d.amount,
  }));

/**
 * Record or edit a payslip (UI_FLOWS F14): the gross pay and its deduction lines, with the net
 * from the backend as the user types (`preview_payslip`).
 */
export function PayslipDialog({
  request,
  onClose,
}: {
  request: PayslipRequest;
  onClose: () => void;
}) {
  const editId = "id" in request ? request.id : undefined;
  // Taken once, so a refresh can't pull the form away mid-save.
  const [start, setStart] = useState<{
    values: Values;
    /** The payslip's gross row, when editing. */
    id?: number;
    categoryId: number;
    /** The records an edit may keep though they're archived. */
    kept: { accountId: number | null; categoryIds: number[] };
  } | null>(null);
  // Not watched once the form starts, so deleting the payslip doesn't refetch it (NotFound).
  const defaults = usePayslipDefaults(
    "ruleId" in request ? request.ruleId : 0,
    "ruleId" in request ? request.date : "",
    editId === undefined && !start,
  );
  const stored = usePayslip(start ? undefined : editId);
  const accounts = useAccounts();
  const categories = useCategories();
  const settings = useSettings();
  const own = editId === undefined ? defaults : stored;
  const queries = [own, accounts, categories, settings];
  const failed = queries.filter((q) => q.isError);
  if (!start && !failed.length && own.isFetchedAfterMount && own.data) {
    const p = own.data;
    setStart({
      values: {
        date: p.date,
        accountId: String(p.accountId),
        gross: p.gross,
        deductions: toLines(p.deductions),
        note: stored.data?.note ?? "",
      },
      id: stored.data?.id,
      categoryId: p.categoryId,
      kept: stored.data
        ? {
            accountId: stored.data.accountId,
            categoryIds: stored.data.deductions.map((d) => d.categoryId),
          }
        : { accountId: null, categoryIds: [] },
    });
  }
  const category = categories.data?.categories.find(
    (c) => c.id === start?.categoryId,
  );
  const title =
    editId !== undefined ? "Edit payslip" : `Payslip · ${category?.name ?? ""}`;

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        title={title}
        size="list"
        onOpenAutoFocus={(e) => e.preventDefault()}
      >
        {!start && failed.length ? (
          <ErrorState
            onRetry={() => failed.forEach((q) => q.refetch())}
            pending={failed.some((q) => q.isFetching)}
          />
        ) : start && accounts.data && categories.data && settings.data ? (
          <PayslipForm
            ruleId={"ruleId" in request ? request.ruleId : undefined}
            payday={"ruleId" in request ? request.date : undefined}
            editId={start.id}
            initial={start.values}
            kept={start.kept}
            accounts={accounts.data.accounts}
            categories={categories.data.categories}
            weekStart={settings.data.weekStart}
            money={(amount) => formatMoney(amount, settings.data)}
            onClose={onClose}
          />
        ) : (
          <Loading className="grid gap-3 px-[22px] py-4">
            {Array.from({ length: 5 }, (_, i) => (
              <Skeleton key={i} className="h-9" />
            ))}
          </Loading>
        )}
      </DialogContent>
    </Dialog>
  );
}

function PayslipForm({
  ruleId,
  payday,
  editId,
  initial,
  kept,
  accounts,
  categories,
  weekStart,
  money,
  onClose,
}: {
  ruleId?: number;
  /** The payday's scheduled date, which `record_payslip` settles. */
  payday?: string;
  /** The gross row's id when editing. */
  editId?: number;
  initial: Values;
  kept: { accountId: number | null; categoryIds: number[] };
  accounts: Account[];
  categories: Category[];
  weekStart: WeekStart;
  money: (amount: number) => string;
  onClose: () => void;
}) {
  const {
    control,
    register,
    handleSubmit,
    setError,
    formState: { errors },
  } = useForm<Values>({ defaultValues: initial });
  const lines = useFieldArray({ control, name: "deductions" });
  const values = useWatch({ control }) as Values;
  const net = usePayslipNet(toFields(values)).data?.net;
  // A message about the lines as a whole (more than 30), shown under Net pay.
  const [listError, setListError] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const record = useRecordPayslip();
  const update = useUpdatePayslip();
  const remove = useDeleteTransaction();
  const saving = useRef(false);

  const accountOptions = offered(
    accounts.filter((a) => a.kind !== "credit_card"),
    kept.accountId,
  ).map((a) => {
    const Icon = ACCOUNT_ICONS[a.kind];
    return {
      value: String(a.id),
      label: named(a),
      icon: <Icon aria-hidden className="size-4 text-muted-foreground" />,
    };
  });
  const rank = (c: Category) => {
    const i = DEDUCTIONS.indexOf(c.name);
    return i === -1 ? DEDUCTIONS.length : i;
  };
  const categoryOptions = offered(
    categories.filter((c) => c.kind === "expense"),
    ...kept.categoryIds,
  )
    // Stable, so the other expense categories keep their own order.
    .sort((a, b) => rank(a) - rank(b))
    .map((c) => ({
      value: String(c.id),
      label: named(c),
      icon: <CategoryChip color={c.color} icon={c.icon} size="sm" />,
    }));

  const shown = [
    "date",
    "accountId",
    "gross",
    "note",
    ...values.deductions.flatMap(
      (_, i) =>
        [`deductions.${i}.categoryId`, `deductions.${i}.amount`] as const,
    ),
  ] as const;

  const save = async (v: Values) => {
    if (saving.current) return;
    saving.current = true;
    setListError(null);
    try {
      const payslip = toFields(v);
      if (editId !== undefined)
        await update.mutateAsync({ id: editId, payslip });
      else if (ruleId !== undefined && payday !== undefined)
        await record.mutateAsync({ ruleId, date: payday, payslip });
      notify.success("Payslip saved");
      onClose();
    } catch (error) {
      const detail = error instanceof ApiError ? error.detail : null;
      if (detail?.code === "validation" && detail.field === "deductions")
        setListError(detail.message);
      else showSaveError(error, shown, setError);
    } finally {
      saving.current = false;
    }
  };

  const short = net !== undefined && net <= 0;
  const netMessage = short ? NET_ERROR : listError;
  const count = initial.deductions.length;

  return (
    <>
      <form
        noValidate
        onSubmit={(e) => {
          e.preventDefault();
          void handleSubmit(save)();
        }}
      >
        <DialogBody>
          <div className="grid grid-cols-2 gap-3">
            <Controller
              control={control}
              name="date"
              render={({ field }) => (
                <Field
                  label="Date"
                  htmlFor="payslip-date"
                  error={errors.date?.message}
                >
                  <DatePicker
                    id="payslip-date"
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
                  htmlFor="payslip-account"
                  error={errors.accountId?.message}
                >
                  <Combobox
                    id="payslip-account"
                    options={accountOptions}
                    value={field.value}
                    onChange={field.onChange}
                    placeholder="Choose an account"
                  />
                </Field>
              )}
            />
          </div>
          <Controller
            control={control}
            name="gross"
            render={({ field }) => (
              <Field
                label="Gross pay"
                htmlFor="payslip-gross"
                error={errors.gross?.message}
                className={ROW}
              >
                <AmountInput
                  id="payslip-gross"
                  ref={field.ref}
                  autoFocus
                  value={field.value}
                  onChange={field.onChange}
                  onBlur={field.onBlur}
                />
              </Field>
            )}
          />
          <h3 className="mt-1 text-small font-semibold text-muted-foreground">
            Deductions
          </h3>
          {lines.fields.length > 0 && (
            <ul
              aria-label="Deductions"
              className="grid max-h-[32vh] gap-2 overflow-y-auto p-0.5"
            >
              {lines.fields.map((line, index) => {
                const n = index + 1;
                const categoryError =
                  errors.deductions?.[index]?.categoryId?.message;
                const amountError = errors.deductions?.[index]?.amount?.message;
                return (
                  <li key={line.id} className="grid gap-1">
                    <div className="grid grid-cols-[minmax(0,1fr)_140px_36px] items-center gap-2">
                      <Controller
                        control={control}
                        name={`deductions.${index}.categoryId`}
                        render={({ field }) => (
                          <Combobox
                            aria-label={`Deduction ${n} category`}
                            aria-invalid={!!categoryError || undefined}
                            options={categoryOptions}
                            value={field.value}
                            onChange={field.onChange}
                            placeholder="Choose a category"
                          />
                        )}
                      />
                      <Controller
                        control={control}
                        name={`deductions.${index}.amount`}
                        render={({ field }) => (
                          <AmountInput
                            aria-label={`Deduction ${n} amount`}
                            aria-invalid={!!amountError || undefined}
                            ref={field.ref}
                            value={field.value}
                            onChange={field.onChange}
                            onBlur={field.onBlur}
                          />
                        )}
                      />
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={`Remove deduction ${n}`}
                        onClick={() => lines.remove(index)}
                      >
                        <X />
                      </Button>
                    </div>
                    {(categoryError ?? amountError) && (
                      <p role="alert" className="text-small text-danger">
                        {categoryError ?? amountError}
                      </p>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
          <Button
            size="sm"
            variant="ghost"
            className="justify-self-start"
            onClick={() =>
              lines.append(
                { categoryId: null, amount: null },
                {
                  shouldFocus: true,
                  focusName: `deductions.${lines.fields.length}.amount`,
                },
              )
            }
          >
            <Plus />
            Add deduction
          </Button>
          <Field
            label="Note"
            htmlFor="payslip-note"
            error={errors.note?.message}
            className={ROW}
          >
            <Input
              id="payslip-note"
              autoComplete="off"
              placeholder="Optional"
              {...register("note")}
            />
          </Field>
          <div
            className="grid gap-1 rounded-[12px] border border-hairline bg-wash px-3 py-2"
            aria-live="polite"
          >
            <div className="flex items-center justify-between font-semibold">
              <span>Net pay</span>
              <span
                className={cn("tabular-nums", short && "text-danger")}
                data-testid="payslip-net"
              >
                {net === undefined ? "" : money(net)}
              </span>
            </div>
            {netMessage && (
              <p role="alert" className="text-small text-danger">
                {netMessage}
              </p>
            )}
          </div>
        </DialogBody>
        <DialogFooter>
          {editId !== undefined && (
            <Button
              variant="destructive"
              className="mr-auto"
              onClick={() => setDeleting(true)}
            >
              <Trash2 />
              Delete
            </Button>
          )}
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" type="submit">
            Save
          </Button>
        </DialogFooter>
      </form>
      {editId !== undefined && (
        <ConfirmDialog
          open={deleting}
          onOpenChange={setDeleting}
          title="Delete this payslip?"
          // The payday stays settled (DATA_MODEL §4.7), so its pay leaves the forecast for good.
          message={`${
            count === 0
              ? "Its gross pay is deleted."
              : `Its gross pay and ${count} ${count === 1 ? "deduction" : "deductions"} are deleted.`
          } Its payday won't be offered again, so to fix a mistake, edit the payslip instead.`}
          confirmLabel="Delete"
          destructive
          onConfirm={() =>
            remove.mutate(editId, {
              onSuccess: () => {
                notify.success("Deleted");
                onClose();
              },
              onError: (error) => notify.error(error.message),
            })
          }
        />
      )}
    </>
  );
}
