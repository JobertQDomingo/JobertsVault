import { Trash2 } from "lucide-react";
import {
  useEffect,
  useRef,
  useState,
  type ComponentProps,
  type ReactNode,
} from "react";
import { Controller, useForm, useWatch } from "react-hook-form";
import type { Account } from "@/bindings/Account";
import type { Category } from "@/bindings/Category";
import type { DueOccurrence } from "@/bindings/DueOccurrence";
import type { Frequency } from "@/bindings/Frequency";
import type { PaymentMethod } from "@/bindings/PaymentMethod";
import type { RecurringMode } from "@/bindings/RecurringMode";
import type { RecurringRule } from "@/bindings/RecurringRule";
import type { RecurringRuleFields } from "@/bindings/RecurringRuleFields";
import type { TransactionFields } from "@/bindings/TransactionFields";
import type { TransactionKind } from "@/bindings/TransactionKind";
import type { WeekStart } from "@/bindings/WeekStart";
import { useAccounts } from "@/hooks/useAccounts";
import { useCategories } from "@/hooks/useCategories";
import { usePaymentMethods } from "@/hooks/usePaymentMethods";
import {
  useCreateRecurringRule,
  usePostOccurrence,
  useUpdateRecurringRule,
} from "@/hooks/useRecurring";
import { useSettings } from "@/hooks/useSettings";
import {
  useCreateTransaction,
  useDeleteTransaction,
  usePayBillDefaults,
  usePayeeSuggestions,
  useTransaction,
  useTransactionDefaults,
  useUpdateTransaction,
} from "@/hooks/useTransactions";
import { named, offered } from "@/lib/records";
import { unitOf } from "@/lib/recurrence";
import { showSaveError } from "@/lib/saveError";
import { ACCOUNT_ICONS } from "./AccountRow";
import { AmountInput } from "./AmountInput";
import { CategoryChip } from "./CategoryChip";
import { ErrorState } from "./ErrorState";
import { Loading, Skeleton } from "./Loading";
import { Autocomplete } from "./ui/autocomplete";
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
 * What quick-add was opened for: nothing (add), a row to edit or duplicate, a credit card to
 * pay (CC-9), a due recurring date to Edit & post (UI_FLOWS F10), or a recurring rule, new
 * (`true`) or to edit: the form plus a schedule section (§8.2).
 */
export type QuickAddRequest = {
  edit?: number;
  duplicate?: number;
  payBill?: number;
  post?: DueOccurrence;
  rule?: RecurringRule | true;
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
  // A rule's schedule; the date is its start date.
  frequency: Frequency;
  interval: string;
  ends: "never" | "on" | "after";
  endDate: string;
  maxOccurrences: string;
  mode: RecurringMode;
};

/** A new rule's schedule, and what every other form carries unused. */
const SCHEDULE = {
  frequency: "monthly",
  interval: "1",
  ends: "never",
  endDate: "",
  maxOccurrences: "",
  mode: "auto",
} as const;

const FREQUENCIES: { value: Frequency; label: string }[] = [
  { value: "daily", label: "Daily" },
  { value: "weekly", label: "Weekly" },
  { value: "monthly", label: "Monthly" },
  { value: "yearly", label: "Yearly" },
];
const ENDS: { value: Values["ends"]; label: string }[] = [
  { value: "never", label: "Never" },
  { value: "on", label: "On a date" },
  { value: "after", label: "Number of times" },
];
/** The end field each choice shows, so an error for one that's hidden is a toast. */
const END_FIELDS = {
  never: [],
  on: ["endDate"],
  after: ["maxOccurrences"],
} as const;
const MODES: { value: RecurringMode; label: string }[] = [
  { value: "auto", label: "Auto-post" },
  { value: "confirm", label: "Confirm" },
];

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

/** A count typed as digits, or 0 so the backend's "Enter a number from…" shows under the field. */
const count = (text: string) =>
  /^\d{1,6}$/.test(text.trim()) ? Number(text) : 0;

/** The form as a rule: the transaction is its template, dated the start date (REC-1). */
const toRule = (v: Values): RecurringRuleFields => ({
  transaction: toFields(v),
  frequency: v.frequency,
  interval: count(v.interval),
  endDate: v.ends === "on" ? v.endDate : null,
  maxOccurrences: v.ends === "after" ? count(v.maxOccurrences) : null,
  mode: v.mode,
});

const titleOf = (request: QuickAddRequest) =>
  request.edit !== undefined
    ? "Edit transaction"
    : request.payBill !== undefined
      ? "Pay credit card"
      : request.post
        ? "Edit & post"
        : request.rule === true
          ? "New rule"
          : request.rule
            ? "Edit rule"
            : "Add transaction";

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
  const editingRule = request.rule === true ? undefined : request.rule;
  // What the form starts from, taken once. After that the row isn't watched, so deleting it (or
  // a refresh) can't pull the form away mid-save.
  const [start, setStart] = useState<{ values: Values; today: string } | null>(
    null,
  );
  // A payment starts from its own defaults (they carry today too), so it needs no others.
  const paying = request.payBill !== undefined;
  const defaults = useTransactionDefaults(!paying);
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

  const own = paying ? payBill : defaults;
  const queries = [
    own,
    accounts,
    categories,
    methods,
    settingsQuery,
    ...(sourceId !== undefined ? [source] : []),
  ];
  const failed = queries.filter((q) => q.isError);
  const today = own.data?.today;
  // The cache can be old (an edit elsewhere, or past midnight), so the starting values wait for
  // this opening's own answers. A failed one still leaves the old data, so a failure stops it too.
  if (
    !start &&
    !failed.length &&
    own.isFetchedAfterMount &&
    today &&
    accounts.data &&
    categories.data &&
    methods.data &&
    settings &&
    (sourceId === undefined || (source.isFetchedAfterMount && source.data))
  )
    setStart({
      values: source.data
        ? // A duplicate is dated today (TXN-2).
          fromRow(source.data, request.duplicate !== undefined ? today : null)
        : request.post
          ? fromRow(request.post.transaction, null)
          : editingRule
            ? fromRule(editingRule)
            : paying && payBill.data
              ? // A transfer to the card for what's owed, from the backend's account (UI_FLOWS F2).
                {
                  ...blank(payBill.data.fromAccountId, today),
                  kind: "transfer",
                  amount: payBill.data.amount,
                  toAccountId: text(request.payBill ?? null),
                  note: "Credit card payment",
                }
              : blank(defaults.data?.accountId ?? null, today),
      today,
    });

  return (
    <>
      <Dialog open onOpenChange={(open) => !open && close()}>
        <DialogContent
          title={titleOf(request)}
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
              postId={request.post?.id}
              schedule={
                request.rule
                  ? {
                      ruleId: editingRule?.id,
                      // A new schedule restarts from the next date (DATA_MODEL §4.1).
                      restart: editingRule?.nextDate ?? start.today,
                    }
                  : undefined
              }
              payment={paying}
              initial={start.values}
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
  ...SCHEDULE,
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

/** The row's (or a due date's) fields, dated `today` when given. */
const fromRow = (t: TransactionFields, today: string | null): Values => ({
  ...SCHEDULE,
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

/** A rule's template and schedule; its date is the start date. */
const fromRule = (r: RecurringRule): Values => ({
  ...fromRow(r.transaction, null),
  frequency: r.frequency,
  interval: String(r.interval),
  ends: r.endDate ? "on" : r.maxOccurrences ? "after" : "never",
  endDate: r.endDate ?? "",
  maxOccurrences: r.maxOccurrences ? String(r.maxOccurrences) : "",
  mode: r.mode,
});

// The mockup's form rows: a 96 px label, the field beside it, its message under the field.
const ROW =
  "grid-cols-[96px_minmax(0,1fr)] items-center gap-x-3 [&>p]:col-start-2";

/** A row whose control names itself (a Segmented), so its label is plain text. */
function Labeled({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className={`grid gap-1 ${ROW}`}>
      <span className="text-small font-semibold text-muted-foreground">
        {label}
      </span>
      {children}
    </div>
  );
}

/** A count with its unit after it, "Every [2] months". Field's aria props reach the input. */
function WithUnit({
  unit,
  ...props
}: ComponentProps<"input"> & { unit: string }) {
  return (
    <div className="flex items-center gap-2">
      <Input
        inputMode="numeric"
        autoComplete="off"
        className="w-20"
        {...props}
      />
      <span className="text-muted-foreground">{unit}</span>
    </div>
  );
}

function TransactionForm({
  editId,
  postId,
  schedule,
  payment,
  initial,
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
  /** Edit & post: saving posts this due recurring date, and changes only it (F10). */
  postId?: number;
  /** A recurring rule (§8.2): the schedule section, and `ruleId` when editing one. */
  schedule?: { ruleId?: number; restart: string };
  /** Pay bill: always a transfer, saved once (the mockup has no type switch or Save & add another). */
  payment: boolean;
  initial: Values;
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
  const post = usePostOccurrence();
  const createRule = useCreateRecurringRule();
  const updateRule = useUpdateRecurringRule();
  const [deleting, setDeleting] = useState(false);
  // An edit, a payment, a post and a rule are saved once; a payment's type is fixed too.
  const addAnother =
    editId === undefined && postId === undefined && !schedule && !payment;
  // The ids an archived record may still be offered for: the row being edited or copied. Save &
  // add another starts a new row, which keeps none.
  const [kept, setKept] = useState(initial);
  // Set before the save starts, so a second Enter in the same moment can't save twice.
  const saving = useRef(false);
  const kind = useWatch({ control, name: "kind" });
  const transfer = kind === "transfer";
  const payees =
    usePayeeSuggestions(useWatch({ control, name: "payee" }), kind).data ?? [];
  const [frequency, interval, ends] = useWatch({
    control,
    name: ["frequency", "interval", "ends"],
  });

  // Editing a rule's frequency or interval starts it again (DATA_MODEL §4.1): from its next
  // date, unless another start date was already chosen, so the dates already done stay done.
  // Put back as it was, the schedule keeps its stored start, so nothing restarts.
  const offerRestart = () => {
    if (schedule?.ruleId === undefined) return;
    const changed =
      getValues("frequency") !== initial.frequency ||
      getValues("interval").trim() !== initial.interval;
    const date = getValues("date");
    if (changed && date === initial.date)
      setValue("date", schedule.restart, { shouldDirty: true });
    else if (!changed && date === schedule.restart)
      setValue("date", initial.date, { shouldDirty: true });
  };

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
    ...(schedule ? (["interval", ...END_FIELDS[ends]] as const) : []),
  ];

  const save = (another: boolean) => {
    if (saving.current) return;
    saving.current = true;
    void handleSubmit(async (values) => {
      try {
        const transaction = toFields(values);
        if (schedule?.ruleId !== undefined)
          await updateRule.mutateAsync({
            id: schedule.ruleId,
            rule: toRule(values),
          });
        else if (schedule) await createRule.mutateAsync(toRule(values));
        else if (editId !== undefined)
          await update.mutateAsync({ id: editId, transaction });
        else if (postId !== undefined)
          await post.mutateAsync({ id: postId, transaction });
        else await create.mutateAsync(transaction);
        notify.success(postId !== undefined ? "Posted" : "Saved");
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
            // Paying a card: the card it pays stays (UI_FLOWS F2).
            disabled={payment && name === "toAccountId"}
          />
        </Field>
      )}
    />
  );

  // A rule's start date sits in its schedule section, and may be in the past or the future.
  const dateField = (
    <Controller
      control={control}
      name="date"
      render={({ field }) => (
        <Field
          label={schedule ? "Starts" : "Date"}
          htmlFor="qa-date"
          error={errors.date?.message}
          className={ROW}
        >
          <DatePicker
            id="qa-date"
            value={field.value}
            onChange={field.onChange}
            weekStart={weekStart}
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
            // Shift+Enter is Save & add another, when there is one.
            save(e.shiftKey && addAnother);
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
          {!schedule && dateField}
          {!transfer && (
            <>
              <Controller
                control={control}
                name="payee"
                render={({ field }) => (
                  <Field
                    label="Payee"
                    htmlFor="qa-payee"
                    error={errors.payee?.message}
                    className={ROW}
                  >
                    <Autocomplete
                      id="qa-payee"
                      placeholder="Optional"
                      value={field.value}
                      onChange={field.onChange}
                      suggestions={payees.map((p) => p.payee)}
                      onPick={(name) => {
                        field.onChange(name);
                        // TXN-6: its last category, unless that's archived now.
                        const id = payees.find(
                          (p) => p.payee === name,
                        )?.categoryId;
                        if (id != null)
                          setValue("categoryId", String(id), {
                            shouldDirty: true,
                          });
                      }}
                    />
                  </Field>
                )}
              />
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
          {schedule && (
            <>
              <h3 className="mt-1 border-t border-hairline pt-3 text-small font-semibold text-muted-foreground">
                Schedule
              </h3>
              <Labeled label="Repeats">
                <Controller
                  control={control}
                  name="frequency"
                  render={({ field }) => (
                    <Segmented
                      aria-label="Repeats"
                      value={field.value}
                      onChange={(next) => {
                        field.onChange(next);
                        offerRestart();
                      }}
                      options={FREQUENCIES}
                      className="justify-self-start"
                    />
                  )}
                />
              </Labeled>
              <Field
                label="Every"
                htmlFor="qa-interval"
                error={errors.interval?.message}
                className={ROW}
              >
                <WithUnit
                  id="qa-interval"
                  unit={unitOf(frequency, interval)}
                  {...register("interval", { onChange: offerRestart })}
                />
              </Field>
              {dateField}
              <Labeled label="Ends">
                <Controller
                  control={control}
                  name="ends"
                  render={({ field }) => (
                    <Segmented
                      aria-label="Ends"
                      value={field.value}
                      onChange={field.onChange}
                      options={ENDS}
                      className="justify-self-start"
                    />
                  )}
                />
              </Labeled>
              {ends === "on" && (
                <Controller
                  control={control}
                  name="endDate"
                  render={({ field }) => (
                    <Field
                      label="Ends on"
                      htmlFor="qa-endDate"
                      error={errors.endDate?.message}
                      className={ROW}
                    >
                      <DatePicker
                        id="qa-endDate"
                        value={field.value}
                        onChange={field.onChange}
                        weekStart={weekStart}
                      />
                    </Field>
                  )}
                />
              )}
              {ends === "after" && (
                <Field
                  label="Ends after"
                  htmlFor="qa-maxOccurrences"
                  error={errors.maxOccurrences?.message}
                  className={ROW}
                >
                  <WithUnit
                    id="qa-maxOccurrences"
                    unit="times"
                    {...register("maxOccurrences")}
                  />
                </Field>
              )}
              <Labeled label="Mode">
                <Controller
                  control={control}
                  name="mode"
                  render={({ field }) => (
                    <Segmented
                      aria-label="Mode"
                      value={field.value}
                      onChange={field.onChange}
                      options={MODES}
                      className="justify-self-start"
                    />
                  )}
                />
              </Labeled>
            </>
          )}
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
          {addAnother && (
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
