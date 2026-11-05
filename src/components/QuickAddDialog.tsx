import { Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Controller, useForm, useWatch } from "react-hook-form";
import type { Account } from "@/bindings/Account";
import type { Category } from "@/bindings/Category";
import type { PaymentMethod } from "@/bindings/PaymentMethod";
import type { Transaction } from "@/bindings/Transaction";
import type { TransactionFields } from "@/bindings/TransactionFields";
import type { TransactionKind } from "@/bindings/TransactionKind";
import type { WeekStart } from "@/bindings/WeekStart";
import { useAccounts } from "@/hooks/useAccounts";
import { useCategories } from "@/hooks/useCategories";
import { usePaymentMethods } from "@/hooks/usePaymentMethods";
import { useSettings } from "@/hooks/useSettings";
import {
  useCreateTransaction,
  useDeleteTransaction,
  usePayBillDefaults,
  useTransaction,
  useTransactionDefaults,
  useUpdateTransaction,
} from "@/hooks/useTransactions";
import { named, offered } from "@/lib/records";
import { showSaveError } from "@/lib/saveError";
import { ACCOUNT_ICONS } from "./AccountRow";
import { AmountInput } from "./AmountInput";
import { CategoryChip } from "./CategoryChip";
import { ErrorState } from "./ErrorState";
import { Loading, Skeleton } from "./Loading";
import { Button } from "./ui/button";
import { Combobox } from "./ui/combobox";
import { DatePicker } from "./ui/date-picker";
import {
  ConfirmDialog,
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
} from "./ui/dialog";
import { Field, Input } from "./ui/input";
import { Segmented } from "./ui/segmented";
import { notify } from "./ui/toast";

/**
 * What quick-add was opened for: nothing (add), a row to edit or duplicate, or a credit card to
 * pay (CC-9).
 */
export type QuickAddRequest = {
  edit?: number;
  duplicate?: number;
  payBill?: number;
};

type Values = {
  kind: TransactionKind;
  amount: number | null;
  categoryId: string | null;
  accountId: string | null;
  toAccountId: string | null;
  date: string;
  payee: string;
  paymentMethodId: string | null;
  note: string;
};

const KINDS: { value: TransactionKind; label: string }[] = [
  { value: "expense", label: "Expense" },
  { value: "income", label: "Income" },
  { value: "transfer", label: "Transfer" },
];
const ALT_KINDS: Record<string, TransactionKind> = {
  e: "expense",
  i: "income",
  t: "transfer",
};

const text = (id: number | null) => (id === null ? null : String(id));
const num = (id: string | null) => (id ? Number(id) : null);

/** A transfer sends only its own fields; a hidden method keeps whatever the row had. */
function toFields(v: Values): TransactionFields {
  const transfer = v.kind === "transfer";
  return {
    kind: v.kind,
    // Empty is 0, so the backend's "Enter an amount above 0" shows under Amount.
    amount: v.amount ?? 0,
    date: v.date,
    accountId: num(v.accountId) ?? 0,
    toAccountId: transfer ? num(v.toAccountId) : null,
    categoryId: transfer ? null : num(v.categoryId),
    payee: transfer ? null : v.payee.trim() || null,
    paymentMethodId: transfer ? null : num(v.paymentMethodId),
    note: v.note.trim() || null,
  };
}

/**
 * The quick-add and edit dialog (UI_FLOWS §4). It waits for what the form starts from, then
 * shows it; closing a form with changes asks "Discard changes?" first.
 */
export function QuickAddDialog({
  request,
  onClose,
}: {
  request: QuickAddRequest;
  onClose: () => void;
}) {
  const sourceId = request.edit ?? request.duplicate;
  // What the form starts from, taken once. After that the row isn't watched, so deleting it (or
  // a refresh) can't pull the form away mid-save.
  const [start, setStart] = useState<{ values: Values; today: string } | null>(
    null,
  );
  const defaults = useTransactionDefaults();
  const source = useTransaction(start ? undefined : sourceId);
  const payBill = usePayBillDefaults(start ? undefined : request.payBill);
  const accounts = useAccounts();
  const categories = useCategories();
  const methods = usePaymentMethods();
  const settingsQuery = useSettings();
  const settings = settingsQuery.data;
  const [dirty, setDirty] = useState(false);
  const [discarding, setDiscarding] = useState(false);
  const close = () => (dirty ? setDiscarding(true) : onClose());

  const queries = [
    defaults,
    accounts,
    categories,
    methods,
    settingsQuery,
    ...(sourceId !== undefined ? [source] : []),
    ...(request.payBill !== undefined ? [payBill] : []),
  ];
  const failed = queries.filter((q) => q.isError);
  // The cache can be old (an edit elsewhere, or past midnight), so the starting values wait for
  // this opening's own answers. A failed one still leaves the old data, so a failure stops it too.
  if (
    !start &&
    !failed.length &&
    defaults.isFetchedAfterMount &&
    defaults.data &&
    accounts.data &&
    categories.data &&
    methods.data &&
    settings &&
    (sourceId === undefined || (source.isFetchedAfterMount && source.data)) &&
    (request.payBill === undefined ||
      (payBill.isFetchedAfterMount && payBill.data))
  )
    setStart({
      values: source.data
        ? // A duplicate is dated today (TXN-2).
          fromRow(
            source.data,
            request.duplicate !== undefined ? defaults.data.today : null,
          )
        : request.payBill !== undefined && payBill.data
          ? // A transfer to the card for what's owed, from the backend's account (UI_FLOWS F2).
            {
              ...blank(payBill.data.fromAccountId, payBill.data.today),
              kind: "transfer",
              amount: payBill.data.amount,
              toAccountId: String(request.payBill),
              note: "Credit card payment",
            }
          : blank(defaults.data.accountId, defaults.data.today),
      today: defaults.data.today,
    });

  return (
    <>
      <Dialog open onOpenChange={(open) => !open && close()}>
        <DialogContent
          title={
            request.edit !== undefined
              ? "Edit transaction"
              : request.payBill !== undefined
                ? "Pay credit card"
                : "Add transaction"
          }
          // The form puts the cursor in Amount itself.
          onOpenAutoFocus={(e) => e.preventDefault()}
        >
          {!start && failed.length ? (
            <ErrorState
              onRetry={() => failed.forEach((q) => q.refetch())}
              pending={failed.some((q) => q.isFetching)}
            />
          ) : start && settings ? (
            <TransactionForm
              editId={request.edit}
              payment={request.payBill !== undefined}
              initial={start.values}
              today={start.today}
              weekStart={settings.weekStart}
              showMethod={settings.showPaymentMethod}
              accounts={accounts.data?.accounts ?? []}
              categories={categories.data?.categories ?? []}
              methods={methods.data ?? []}
              onDirtyChange={setDirty}
              onCancel={close}
              onDone={onClose}
            />
          ) : (
            <Loading className="grid gap-3 px-[22px] py-4">
              {Array.from({ length: 6 }, (_, i) => (
                <Skeleton key={i} className="h-9" />
              ))}
            </Loading>
          )}
        </DialogContent>
      </Dialog>
      <ConfirmDialog
        open={discarding}
        onOpenChange={setDiscarding}
        title="Discard changes?"
        message="What you typed will be lost."
        confirmLabel="Discard"
        destructive
        onConfirm={onClose}
      />
    </>
  );
}

const blank = (accountId: number | null, today: string): Values => ({
  kind: "expense",
  amount: null,
  categoryId: null,
  accountId: text(accountId),
  toAccountId: null,
  date: today,
  payee: "",
  paymentMethodId: null,
  note: "",
});

/** The row's fields, dated `today` when given. */
const fromRow = (t: Transaction, today: string | null): Values => ({
  kind: t.kind,
  amount: t.amount,
  categoryId: text(t.categoryId),
  accountId: text(t.accountId),
  toAccountId: text(t.toAccountId),
  date: today ?? t.date,
  payee: t.payee ?? "",
  paymentMethodId: text(t.paymentMethodId),
  note: t.note ?? "",
});

// The mockup's form rows: a 96 px label, the field beside it, its message under the field.
const ROW =
  "grid-cols-[96px_minmax(0,1fr)] items-center gap-x-3 [&>p]:col-start-2";

function TransactionForm({
  editId,
  payment,
  initial,
  today,
  weekStart,
  showMethod,
  accounts,
  categories,
  methods,
  onDirtyChange,
  onCancel,
  onDone,
}: {
  editId?: number;
  /** Pay bill: always a transfer, saved once (the mockup has no type switch or Save & add another). */
  payment: boolean;
  initial: Values;
  today: string;
  weekStart: WeekStart;
  showMethod: boolean;
  accounts: Account[];
  categories: Category[];
  methods: PaymentMethod[];
  onDirtyChange: (dirty: boolean) => void;
  onCancel: () => void;
  onDone: () => void;
}) {
  const form = useForm<Values>({ defaultValues: initial });
  const {
    control,
    register,
    handleSubmit,
    setError,
    setValue,
    getValues,
    reset,
  } = form;
  const { errors, isDirty } = form.formState;
  const create = useCreateTransaction();
  const update = useUpdateTransaction();
  const remove = useDeleteTransaction();
  const [deleting, setDeleting] = useState(false);
  // The ids an archived record may still be offered for: the row being edited or copied. Save &
  // add another starts a new row, which keeps none.
  const [kept, setKept] = useState(initial);
  // Set before the save starts, so a second Enter in the same moment can't save twice.
  const saving = useRef(false);
  const kind = useWatch({ control, name: "kind" });
  const transfer = kind === "transfer";

  useEffect(() => onDirtyChange(isDirty), [isDirty, onDirtyChange]);

  const changeKind = (next: TransactionKind) => {
    // An expense category can't stay on an income, and the other way round. A transfer only
    // hides it, so going back keeps it.
    const category = categories.find(
      (c) => String(c.id) === getValues("categoryId"),
    );
    if (next !== "transfer" && category && category.kind !== next)
      setValue("categoryId", null, { shouldDirty: true });
    setValue("kind", next, { shouldDirty: true });
  };

  // The fields on screen, so a message always lands where it can be seen.
  const shown: (keyof Values)[] = [
    "amount",
    "date",
    "accountId",
    "note",
    ...(transfer
      ? (["toAccountId"] as const)
      : ([
          "categoryId",
          "payee",
          ...(showMethod ? (["paymentMethodId"] as const) : []),
        ] as const)),
  ];

  const save = (another: boolean) => {
    if (saving.current) return;
    saving.current = true;
    void handleSubmit(async (values) => {
      try {
        const transaction = toFields(values);
        if (editId !== undefined)
          await update.mutateAsync({ id: editId, transaction });
        else await create.mutateAsync(transaction);
        notify.success("Saved");
        if (another) {
          // Keeps the type, account and date; clears the rest (UI_FLOWS §4).
          const next = {
            ...blank(num(values.accountId), values.date),
            kind: values.kind,
          };
          reset(next);
          setKept(next);
          // The same input stays, so it can take the cursor straight away.
          document.getElementById("qa-amount")?.focus();
        } else onDone();
      } catch (error) {
        showSaveError(error, shown, setError);
      }
    })().finally(() => {
      saving.current = false;
    });
  };

  const accountOptions = offered(
    accounts,
    num(kept.accountId),
    num(kept.toAccountId),
  ).map((a) => {
    const Icon = ACCOUNT_ICONS[a.kind];
    return {
      value: String(a.id),
      label: named(a),
      icon: <Icon aria-hidden className="size-4 text-muted-foreground" />,
    };
  });
  const account = (name: "accountId" | "toAccountId", label: string) => (
    <Controller
      control={control}
      name={name}
      render={({ field }) => (
        <Field
          label={label}
          htmlFor={`qa-${name}`}
          error={errors[name]?.message}
          className={ROW}
        >
          <Combobox
            id={`qa-${name}`}
            options={accountOptions}
            value={field.value}
            onChange={field.onChange}
            placeholder="Choose an account"
          />
        </Field>
      )}
    />
  );

  return (
    <>
      <form
        noValidate
        onSubmit={(e) => {
          e.preventDefault();
          save(false);
        }}
        onKeyDown={(e) => {
          const altKind =
            e.altKey && !e.ctrlKey ? ALT_KINDS[e.key.toLowerCase()] : undefined;
          if (altKind) {
            e.preventDefault();
            // Sonner's Alt+T (jump to notifications) listens on the document; this one wins here.
            e.stopPropagation();
            if (!payment) changeKind(altKind);
          } else if (
            e.key === "Enter" &&
            // An open list takes Enter itself (it picks), and a focused button clicks.
            !e.defaultPrevented &&
            !(e.target instanceof HTMLButtonElement)
          ) {
            e.preventDefault();
            // Shift+Enter is Save & add another, which an edit or a payment doesn't have.
            save(e.shiftKey && editId === undefined && !payment);
          }
        }}
      >
        <DialogBody>
          {!payment && (
            <Controller
              control={control}
              name="kind"
              render={({ field }) => (
                <Segmented
                  aria-label="Type"
                  value={field.value}
                  onChange={changeKind}
                  options={KINDS}
                  className="justify-self-start"
                />
              )}
            />
          )}
          <Controller
            control={control}
            name="amount"
            render={({ field }) => (
              <Field
                label="Amount"
                htmlFor="qa-amount"
                error={errors.amount?.message}
                className={ROW}
              >
                <AmountInput
                  id="qa-amount"
                  ref={field.ref}
                  autoFocus
                  value={field.value}
                  onChange={field.onChange}
                  onBlur={field.onBlur}
                />
              </Field>
            )}
          />
          {transfer ? (
            <>
              {account("accountId", "From")}
              {account("toAccountId", "To")}
            </>
          ) : (
            <>
              <Controller
                control={control}
                name="categoryId"
                render={({ field }) => (
                  <Field
                    label="Category"
                    htmlFor="qa-category"
                    error={errors.categoryId?.message}
                    className={ROW}
                  >
                    <Combobox
                      id="qa-category"
                      options={offered(
                        categories.filter((c) => c.kind === kind),
                        num(kept.categoryId),
                      ).map((c) => ({
                        value: String(c.id),
                        label: named(c),
                        icon: (
                          <CategoryChip
                            color={c.color}
                            icon={c.icon}
                            size="sm"
                          />
                        ),
                      }))}
                      value={field.value}
                      onChange={field.onChange}
                      placeholder="Choose a category"
                    />
                  </Field>
                )}
              />
              {account("accountId", "Account")}
            </>
          )}
          <Controller
            control={control}
            name="date"
            render={({ field }) => (
              <Field
                label="Date"
                htmlFor="qa-date"
                error={errors.date?.message}
                className={ROW}
              >
                <DatePicker
                  id="qa-date"
                  value={field.value}
                  onChange={field.onChange}
                  weekStart={weekStart}
                  // Until Phase 3 nothing is recorded after today (TXN-8).
                  max={today}
                />
              </Field>
            )}
          />
          {!transfer && (
            <>
              <Field
                label="Payee"
                htmlFor="qa-payee"
                error={errors.payee?.message}
                className={ROW}
              >
                <Input
                  id="qa-payee"
                  autoComplete="off"
                  placeholder="Optional"
                  {...register("payee")}
                />
              </Field>
              {showMethod && (
                <Controller
                  control={control}
                  name="paymentMethodId"
                  render={({ field }) => (
                    <Field
                      label="Method"
                      htmlFor="qa-method"
                      error={errors.paymentMethodId?.message}
                      className={ROW}
                    >
                      <Combobox
                        id="qa-method"
                        options={offered(
                          methods,
                          num(kept.paymentMethodId),
                        ).map((m) => ({
                          value: String(m.id),
                          label: named(m),
                        }))}
                        value={field.value}
                        onChange={field.onChange}
                        placeholder="Optional"
                      />
                    </Field>
                  )}
                />
              )}
            </>
          )}
          <Field
            label="Note"
            htmlFor="qa-note"
            error={errors.note?.message}
            className={ROW}
          >
            <Input
              id="qa-note"
              autoComplete="off"
              placeholder="Optional"
              {...register("note")}
            />
          </Field>
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
          <Button onClick={onCancel}>Cancel</Button>
          {editId === undefined && !payment && (
            <Button
              title="Shift+Enter"
              aria-keyshortcuts="Shift+Enter"
              onClick={() => save(true)}
            >
              Save &amp; add another
            </Button>
          )}
          <Button
            variant="primary"
            type="submit"
            title="Enter"
            aria-keyshortcuts="Enter"
          >
            Save
          </Button>
        </DialogFooter>
      </form>
      {/* Outside the form: React events cross portals, and the form's keys mustn't act on it. */}
      {editId !== undefined && (
        <ConfirmDialog
          open={deleting}
          onOpenChange={setDeleting}
          title="Delete this transaction?"
          message="This can't be undone."
          confirmLabel="Delete"
          destructive
          onConfirm={() =>
            remove.mutate(editId, {
              onSuccess: () => {
                notify.success("Deleted");
                onDone();
              },
              onError: (error) => notify.error(error.message),
            })
          }
        />
      )}
    </>
  );
}
