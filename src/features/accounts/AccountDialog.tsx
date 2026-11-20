import { useRef } from "react";
import { Controller, useForm, useWatch } from "react-hook-form";
import type { Account } from "@/bindings/Account";
import type { AccountFields } from "@/bindings/AccountFields";
import type { AccountKind } from "@/bindings/AccountKind";
import { ACCOUNT_ICONS, ACCOUNT_KINDS } from "@/components/AccountRow";
import { AmountInput } from "@/components/AmountInput";
import { Button } from "@/components/ui/button";
import { Combobox } from "@/components/ui/combobox";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
} from "@/components/ui/dialog";
import { Field, Input } from "@/components/ui/input";
import { notify } from "@/components/ui/toast";
import { useCreateAccount, useUpdateAccount } from "@/hooks/useAccounts";
import { useSettings } from "@/hooks/useSettings";
import { formatMoney, parseAmount } from "@/lib/money";
import { showSaveError } from "@/lib/saveError";

type Values = {
  kind: AccountKind;
  name: string;
  openingBalance: number | null;
  creditLimit: number | null;
  statementDay: string | null;
  dueDay: string | null;
  spendSoft: number | null;
  spendMid: number | null;
  spendHard: number | null;
};

const LEVELS = [
  { name: "spendSoft", label: "Soft", start: "3000" },
  { name: "spendMid", label: "Mid", start: "4000" },
  { name: "spendHard", label: "Hard", start: "5000" },
] as const;

const KIND_OPTIONS = (Object.keys(ACCOUNT_KINDS) as AccountKind[]).map(
  (kind) => {
    const Icon = ACCOUNT_ICONS[kind];
    return {
      value: kind,
      label: ACCOUNT_KINDS[kind],
      icon: <Icon aria-hidden className="size-4 text-muted-foreground" />,
    };
  },
);
const DAYS = Array.from({ length: 31 }, (_, i) => ({
  value: String(i + 1),
  label: `Day ${i + 1}`,
}));

// The quick-add form's rows: a 96 px label, the field beside it, its message under the field.
const ROW =
  "grid-cols-[96px_minmax(0,1fr)] items-center gap-x-3 [&>p]:col-start-2";

/**
 * Only a credit card sends its limit, days (CC-1) and spending levels (CC-10); empty amounts go
 * as 0 for the backend to name.
 */
function toFields(v: Values): AccountFields {
  const card = v.kind === "credit_card";
  return {
    name: v.name,
    openingBalance: v.openingBalance ?? 0,
    creditLimit: card ? v.creditLimit : null,
    statementDay: card && v.statementDay ? Number(v.statementDay) : null,
    dueDay: card && v.dueDay ? Number(v.dueDay) : null,
    spendSoft: card ? (v.spendSoft ?? 0) : null,
    spendMid: card ? (v.spendMid ?? 0) : null,
    spendHard: card ? (v.spendHard ?? 0) : null,
  };
}

/**
 * The new and edit account dialog (UI_FLOWS §5.2). The type is chosen once: editing shows it
 * locked. The backend checks every field, and its message shows under the field it names.
 */
export function AccountDialog({
  account,
  kind = "cash",
  onClose,
}: {
  account?: Account;
  /** A new account's starting type. */
  kind?: AccountKind;
  onClose: () => void;
}) {
  // A new card starts at 3,000, 4,000 and 5,000 in the main unit (DATA_MODEL §2). The app
  // waits for the settings before any screen, so they're here.
  const settings = useSettings().data;
  const decimals = settings?.currencyDecimals ?? 2;
  const { control, register, handleSubmit, setError, formState } =
    useForm<Values>({
      defaultValues: account
        ? {
            kind: account.kind,
            name: account.name,
            openingBalance: account.openingBalance,
            creditLimit: account.creditLimit,
            statementDay: account.statementDay?.toString() ?? null,
            dueDay: account.dueDay?.toString() ?? null,
            spendSoft: account.spendSoft,
            spendMid: account.spendMid,
            spendHard: account.spendHard,
          }
        : {
            kind,
            name: "",
            openingBalance: null,
            creditLimit: null,
            statementDay: null,
            dueDay: null,
            spendSoft: parseAmount(LEVELS[0].start, decimals),
            spendMid: parseAmount(LEVELS[1].start, decimals),
            spendHard: parseAmount(LEVELS[2].start, decimals),
          },
    });
  const { errors } = formState;
  const create = useCreateAccount();
  const update = useUpdateAccount();
  const saving = useRef(false);
  const card = useWatch({ control, name: "kind" }) === "credit_card";
  const shown: (keyof Values)[] = card
    ? [
        "name",
        "openingBalance",
        "creditLimit",
        "statementDay",
        "dueDay",
        "spendSoft",
        "spendMid",
        "spendHard",
      ]
    : ["name", "openingBalance"];

  const save = async (values: Values) => {
    if (saving.current) return;
    saving.current = true;
    try {
      const fields = toFields(values);
      if (account)
        await update.mutateAsync({ id: account.id, account: fields });
      else await create.mutateAsync({ kind: values.kind, ...fields });
      notify.success("Saved");
      onClose();
    } catch (error) {
      showSaveError(error, shown, setError);
    } finally {
      saving.current = false;
    }
  };

  const day = (name: "statementDay" | "dueDay", label: string) => (
    <Controller
      control={control}
      name={name}
      render={({ field }) => (
        <Field
          label={label}
          htmlFor={`acct-${name}`}
          error={errors[name]?.message}
          className={ROW}
        >
          <Combobox
            id={`acct-${name}`}
            options={DAYS}
            value={field.value}
            onChange={field.onChange}
            placeholder="Choose a day"
          />
        </Field>
      )}
    />
  );
  const amount = (
    name: "openingBalance" | "creditLimit",
    label: string,
    help?: string,
  ) => (
    <Controller
      control={control}
      name={name}
      render={({ field }) => (
        <Field
          label={label}
          htmlFor={`acct-${name}`}
          error={errors[name]?.message}
          help={help}
          className={ROW}
        >
          <AmountInput
            id={`acct-${name}`}
            ref={field.ref}
            value={field.value}
            onChange={field.onChange}
            onBlur={field.onBlur}
          />
        </Field>
      )}
    />
  );

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent title={account ? "Edit account" : "New account"}>
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
              htmlFor="acct-name"
              error={errors.name?.message}
              className={ROW}
            >
              <Input
                id="acct-name"
                autoComplete="off"
                autoFocus
                {...register("name")}
              />
            </Field>
            <Controller
              control={control}
              name="kind"
              render={({ field }) => (
                <Field label="Type" htmlFor="acct-kind" className={ROW}>
                  <Combobox
                    id="acct-kind"
                    options={KIND_OPTIONS}
                    value={field.value}
                    onChange={field.onChange}
                    // It decides how the balance works, so it can't change (UI_FLOWS §5.2).
                    disabled={account !== undefined}
                  />
                </Field>
              )}
            />
            {/* A card's field is its opening amount owed, which is today's only on a new card
                (UI_FLOWS §5.2, §11 item 94). */}
            {amount(
              "openingBalance",
              !card
                ? "Opening balance"
                : account
                  ? "Opening amount owed"
                  : "Amount owed now",
              account?.card && settings
                ? `Owed today: ${formatMoney(account.card.amountOwed, settings)}`
                : undefined,
            )}
            {card && (
              <>
                {amount("creditLimit", "Credit limit")}
                {day("statementDay", "Statement closes")}
                {day("dueDay", "Payment due")}
                {/* CC-10: three amounts on one row (UI_FLOWS §5.2). */}
                <div className={`grid ${ROW}`}>
                  <span className="text-small font-semibold text-muted-foreground">
                    Spending levels
                  </span>
                  <div className="grid grid-cols-3 gap-2">
                    {LEVELS.map(({ name, label }) => (
                      <Controller
                        key={name}
                        control={control}
                        name={name}
                        render={({ field }) => (
                          <Field
                            label={label}
                            htmlFor={`acct-${name}`}
                            error={errors[name]?.message}
                          >
                            <AmountInput
                              id={`acct-${name}`}
                              ref={field.ref}
                              value={field.value}
                              onChange={field.onChange}
                              onBlur={field.onBlur}
                            />
                          </Field>
                        )}
                      />
                    ))}
                  </div>
                  <p className="text-small text-muted-foreground">
                    Warnings for each statement cycle's spending. They never
                    block a purchase.
                  </p>
                </div>
              </>
            )}
            {(card || !account) && (
              <p className="text-small text-muted-foreground">
                {[
                  card && "If a month is shorter, the last day is used.",
                  !account && "The type can't be changed later.",
                ]
                  .filter(Boolean)
                  .join(" ")}
              </p>
            )}
          </DialogBody>
          <DialogFooter>
            <Button onClick={onClose}>Cancel</Button>
            <Button variant="primary" type="submit">
              {account ? "Save" : "Create account"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
