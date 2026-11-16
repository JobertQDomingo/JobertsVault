import { TriangleAlert } from "lucide-react";
import { useState } from "react";
import { ApiError } from "@/api/client";
import type { LoanDraft } from "@/bindings/LoanDraft";
import type { WeekStart } from "@/bindings/WeekStart";
import { AmountInput } from "@/components/AmountInput";
import { GlassPanel } from "@/components/GlassPanel";
import { Button } from "@/components/ui/button";
import { DatePicker } from "@/components/ui/date-picker";
import { Field, Input } from "@/components/ui/input";
import { useLoanPreview } from "@/hooks/useLoans";
import { addMonths, formatDate, formatSpan } from "@/lib/dates";
import { formatMoney, type MoneySettings } from "@/lib/money";
import { cn } from "@/lib/utils";

const FIELDS = ["amount", "count", "firstDate"] as const;

/**
 * Before you borrow (UI_FLOWS F15, LOAN-9): a loan the user is thinking about, monthly from the
 * first payment. Try it shows what `preview_loan` says it would do, and nothing is saved.
 */
export function BeforeYouBorrow({
  today,
  weekStart,
  settings,
}: {
  today: string;
  weekStart: WeekStart;
  settings: MoneySettings;
}) {
  const [amount, setAmount] = useState<number | null>(null);
  const [count, setCount] = useState("");
  // A form default the user can change; the backend checks it.
  const [firstDate, setFirstDate] = useState(() => addMonths(today, 1));
  const [asked, setAsked] = useState<LoanDraft | null>(null);
  const result = useLoanPreview(asked);
  // A change clears the result and its errors, so nothing shown is for other values.
  const edit =
    <T,>(set: (value: T) => void, current: T) =>
    (value: T) => {
      if (value === current) return;
      set(value);
      setAsked(null);
    };
  const detail = result.error instanceof ApiError ? result.error.detail : null;
  const fieldError = (field: (typeof FIELDS)[number]) =>
    detail?.code === "validation" && detail.field === field
      ? detail.message
      : undefined;
  const otherError =
    result.error && !FIELDS.some(fieldError) ? result.error.message : null;
  const money = (minor: number) => formatMoney(minor, settings);
  const p = result.isError ? undefined : result.data;
  // left is carried window to window, so every window from the first payment on changes.
  const changed = p?.windows.filter((w) => w.leftBefore !== w.leftAfter) ?? [];

  return (
    <GlassPanel className="overflow-hidden">
      <h2 className="px-[18px] pt-4 pb-2.5 text-h3">Before you borrow</h2>
      <form
        noValidate
        className="grid gap-3 px-[18px] pb-4"
        onSubmit={(e) => {
          e.preventDefault();
          // Blank fields go as 0 and "", for the backend to name.
          const draft = {
            amount: amount ?? 0,
            count: Number.parseInt(count, 10) || 0,
            firstDate,
          };
          // The same draft has the same key, so ask again, say after a failure.
          if (JSON.stringify(draft) === JSON.stringify(asked))
            void result.refetch();
          else setAsked(draft);
        }}
      >
        <div className="grid grid-cols-[minmax(0,1fr)_96px_minmax(0,1fr)] items-start gap-2">
          <Field
            label="Payment"
            htmlFor="borrow-amount"
            error={fieldError("amount")}
          >
            <AmountInput
              id="borrow-amount"
              value={amount}
              onChange={edit(setAmount, amount)}
            />
          </Field>
          <Field
            label="How many"
            htmlFor="borrow-count"
            error={fieldError("count")}
          >
            <Input
              id="borrow-count"
              inputMode="numeric"
              autoComplete="off"
              value={count}
              onChange={(e) =>
                /^\d{0,3}$/.test(e.target.value) &&
                edit(setCount, count)(e.target.value)
              }
            />
          </Field>
          <Field
            label="First payment"
            htmlFor="borrow-first"
            error={fieldError("firstDate")}
          >
            <DatePicker
              id="borrow-first"
              value={firstDate}
              onChange={edit(setFirstDate, firstDate)}
              weekStart={weekStart}
            />
          </Field>
        </div>
        <Button size="sm" type="submit" className="justify-self-start">
          Try it
        </Button>
        {otherError && (
          <p role="alert" className="text-small text-danger">
            {otherError}
          </p>
        )}
        {p?.debtFreeAfter && (
          <div
            aria-label="What it would do"
            className="grid gap-1 text-small text-muted-foreground tabular-nums"
          >
            <p>
              {"Debt-free: "}
              <span className="font-semibold text-foreground">
                {`${p.debtFreeBefore ? formatDate(p.debtFreeBefore) : "today"} → ${formatDate(p.debtFreeAfter)}`}
              </span>
              {p.daysLater === 0
                ? " (same day)"
                : ` (${p.daysLater} ${p.daysLater === 1 ? "day" : "days"} later)`}
            </p>
            {p.safeBefore !== null && p.safeAfter !== null && (
              <p>
                {"Safe to spend now: "}
                <span className="font-semibold text-foreground">
                  {`${money(p.safeBefore)} → ${money(p.safeAfter)}`}
                </span>
              </p>
            )}
            {changed.length > 0 && (
              <ul
                aria-label="Pay windows it changes"
                className="mt-1 grid max-h-[180px] gap-1 overflow-y-auto"
              >
                {changed.map((w) => {
                  const span = formatSpan(w.start, w.end, today);
                  return (
                    <li
                      key={w.start}
                      className={cn(
                        w.shortBy !== null &&
                          "flex items-center gap-1 font-semibold text-danger",
                      )}
                    >
                      {w.shortBy !== null ? (
                        <>
                          <TriangleAlert
                            aria-hidden
                            className="size-3.5 shrink-0"
                          />
                          {`${span} would run short by ${money(w.shortBy)}`}
                        </>
                      ) : (
                        `${span}: ${money(w.leftBefore)} → ${money(w.leftAfter)}`
                      )}
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
        )}
      </form>
    </GlassPanel>
  );
}
