import { CalendarRange, ChevronRight, TriangleAlert } from "lucide-react";
import { Fragment, useState } from "react";
import { Link, useSearchParams } from "react-router";
import type { Account } from "@/bindings/Account";
import type { Forecast } from "@/bindings/Forecast";
import type { ForecastGrain } from "@/bindings/ForecastGrain";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { Loading, Skeleton } from "@/components/Loading";
import { MoneyText } from "@/components/MoneyText";
import { Button } from "@/components/ui/button";
import { Segmented } from "@/components/ui/segmented";
import { SafeToSpendCard } from "@/features/dashboard/SafeToSpendCard";
import { useAccounts } from "@/hooks/useAccounts";
import { useForecast } from "@/hooks/useForecast";
import { useSettings } from "@/hooks/useSettings";
import { formatShortDate, formatSpan, weekdayDate } from "@/lib/dates";
import { formatMoney, type MoneySettings } from "@/lib/money";
import { cn } from "@/lib/utils";

const VIEWS: { value: ForecastGrain; label: string }[] = [
  { value: "window", label: "Pay windows" },
  { value: "month", label: "Months" },
];

/**
 * `/forecast` (UI_FLOWS §8.6, SAFE-7): Safe to spend at full width, then the next 12 months by pay
 * window or by month (`?view=months`). Every figure is `get_forecast`'s.
 */
export function ForecastScreen() {
  const [params, setParams] = useSearchParams();
  const grain: ForecastGrain =
    params.get("view") === "months" ? "month" : "window";
  const forecast = useForecast(grain);
  const accounts = useAccounts();
  const settings = useSettings();
  const failed = [forecast, accounts, settings].filter((q) => q.isError);

  if (failed.length)
    return (
      <GlassPanel>
        <ErrorState
          onRetry={() => failed.forEach((q) => q.refetch())}
          pending={failed.some((q) => q.isFetching)}
        />
      </GlassPanel>
    );
  if (!forecast.data || !accounts.data || !settings.data)
    return (
      <Loading className="grid gap-3">
        <Skeleton className="h-[120px]" />
        <Skeleton className="h-[480px]" />
      </Loading>
    );

  if (forecast.data.safeToSpend === null)
    return (
      <GlassPanel>
        <EmptyState
          icon={CalendarRange}
          message="Mark your pay as a payday to see what's safe to spend."
          action={
            <Button asChild>
              <Link to="/recurring">Open Recurring</Link>
            </Button>
          }
        />
      </GlassPanel>
    );

  return (
    <div className="grid gap-3">
      <SafeToSpendCard forecast={forecast.data} viewLink={false} />
      <div className="flex">
        <GlassPanel className="rounded-toolbar p-1.5">
          <Segmented
            aria-label="View"
            value={grain}
            options={VIEWS}
            onChange={(next) =>
              setParams(next === "month" ? { view: "months" } : {})
            }
          />
        </GlassPanel>
      </div>
      <GlassPanel className="overflow-hidden">
        {/* Keyed by view, so the other view starts with nothing open. */}
        <ForecastTable
          key={grain}
          forecast={forecast.data}
          grain={grain}
          accounts={accounts.data.accounts}
          settings={settings.data}
        />
      </GlassPanel>
    </div>
  );
}

/** One `ForecastRow` per window or month (DESIGN_SYSTEM §9.2); one opens at a time. */
function ForecastTable({
  forecast,
  grain,
  accounts,
  settings,
}: {
  forecast: Forecast;
  grain: ForecastGrain;
  accounts: Account[];
  settings: MoneySettings;
}) {
  const [open, setOpen] = useState<string | null>(null);
  const { today } = forecast;
  const money = (amount: number) => formatMoney(amount, settings);
  const accountName = new Map(accounts.map((a) => [a.id, a.name]));
  const cell = "px-3 py-2";

  return (
    <table className="w-full text-right tabular-nums">
      <thead className="text-small text-muted-foreground">
        <tr>
          <th scope="col" className="py-2.5 pl-[18px] text-left font-semibold">
            {grain === "month" ? "Month" : "Window"}
          </th>
          {["Start", "Pay in", "Bills", "Left"].map((label) => (
            <th
              key={label}
              scope="col"
              className="px-3 py-2.5 font-semibold last:pr-[18px]"
            >
              {label}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {forecast.rows.map((row, i) => {
          const isOpen = open === row.start;
          return (
            <Fragment key={row.start}>
              <tr
                onClick={() => setOpen(isOpen ? null : row.start)}
                className="h-[52px] cursor-pointer border-t border-hairline transition-colors duration-150 hover:bg-wash"
              >
                <td className="py-2 pl-[18px] text-left">
                  <button
                    type="button"
                    aria-expanded={isOpen}
                    className="inline-flex items-center gap-1.5 rounded-control font-semibold outline-none focus-visible:ring-3 focus-visible:ring-ring/40"
                  >
                    <ChevronRight
                      aria-hidden
                      className={cn(
                        "size-3.5 text-muted-foreground transition-transform duration-150",
                        isOpen && "rotate-90",
                      )}
                    />
                    {/* The current row starts today (UI_FLOWS §8.6). */}
                    {formatSpan(i === 0 ? today : row.start, row.end, today)}
                    {row.lowest && (
                      <span className="rounded-pill bg-wash px-2 text-small font-semibold text-muted-foreground">
                        lowest
                      </span>
                    )}
                  </button>
                  {row.received.map((r) => (
                    <p
                      key={r.payslipId}
                      className="pl-5 text-small text-muted-foreground"
                    >
                      {`${money(r.net)} received ${formatShortDate(r.date)}`}
                    </p>
                  ))}
                </td>
                <td className={cell}>
                  <MoneyText amount={row.startAmount} kind="balance" />
                </td>
                <td className={cell}>{row.in ? money(row.in) : "—"}</td>
                <td className={cell}>{row.out ? money(row.out) : "—"}</td>
                <td className={cn(cell, "pr-[18px]")}>
                  {row.short ? (
                    <span className="inline-flex items-center gap-1 font-semibold whitespace-nowrap text-danger">
                      <TriangleAlert aria-hidden className="size-4 shrink-0" />
                      {`${money(row.left)} Short`}
                    </span>
                  ) : (
                    <MoneyText amount={row.left} kind="balance" />
                  )}
                </td>
              </tr>
              {isOpen &&
                (row.movements.length ? (
                  row.movements.map((m, k) => (
                    // A rule's overdue dates can share a date and id, so by position.
                    <tr key={k} className="bg-wash text-small">
                      <td className="py-1.5 pl-[38px] text-left">
                        {`${weekdayDate(m.date, today)} · ${m.label}`}
                      </td>
                      <td
                        colSpan={3}
                        className="px-3 py-1.5 text-left text-muted-foreground"
                      >
                        {accountName.get(m.accountId)}
                      </td>
                      <td className="py-1.5 pr-[18px] pl-3">
                        <MoneyText
                          amount={m.amount}
                          kind={m.moneyIn ? "income" : "expense"}
                        />
                      </td>
                    </tr>
                  ))
                ) : (
                  <tr className="bg-wash text-small">
                    <td
                      colSpan={5}
                      className="py-1.5 pl-[38px] text-left text-muted-foreground"
                    >
                      Nothing due.
                    </td>
                  </tr>
                ))}
            </Fragment>
          );
        })}
      </tbody>
    </table>
  );
}
