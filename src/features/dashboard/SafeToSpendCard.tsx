import { CalendarRange, TriangleAlert } from "lucide-react";
import { Link } from "react-router";
import type { Forecast } from "@/bindings/Forecast";
import { EmptyState } from "@/components/EmptyState";
import { GlassPanel } from "@/components/GlassPanel";
import { MoneyText } from "@/components/MoneyText";
import { useQuickAdd } from "@/components/QuickAddProvider";
import { Button } from "@/components/ui/button";
import { useSettings } from "@/hooks/useSettings";
import { formatSpan, weekdayDate } from "@/lib/dates";
import { formatMoney } from "@/lib/money";

/**
 * What's safe to spend today until the next payday, the lowest point over the next 12 months
 * (SAFE-5, SAFE-6, UI_FLOWS §2), from `get_forecast`'s summary. It never works out a figure.
 */
export function SafeToSpendCard({
  forecast,
  viewLink = true,
}: {
  forecast: Forecast;
  /** Off on the Forecast screen itself. */
  viewLink?: boolean;
}) {
  const { payslip } = useQuickAdd();
  const { data: settings } = useSettings();
  if (!settings) return null;
  const money = (amount: number) => formatMoney(amount, settings);
  const { today, safeToSpend, windowEnd, short, earlyPayday } = forecast;
  const viewForecast = viewLink && (
    <Link to="/forecast" className="font-semibold text-primary hover:underline">
      View forecast
    </Link>
  );

  if (safeToSpend === null || windowEnd === null)
    return (
      <GlassPanel className="overflow-hidden">
        <div className="flex items-center justify-between gap-2 px-[18px] pt-4">
          <h2 className="text-h3">Safe to spend</h2>
          {viewForecast}
        </div>
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

  const days = forecast.daysLeft === 1 ? "1 day" : `${forecast.daysLeft} days`;
  return (
    <GlassPanel className="grid gap-1 px-[18px] py-4">
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-muted-foreground">
          Safe to spend until {weekdayDate(windowEnd, today)}
        </h2>
        {viewForecast}
      </div>
      {/* What's left, not income, so never green (DESIGN_SYSTEM). */}
      <MoneyText
        amount={safeToSpend}
        kind="balance"
        size="display"
        className="text-foreground"
      />
      <p className="text-small text-muted-foreground tabular-nums">
        {`${money(forecast.perDay ?? 0)} a day · ${days} left`}
      </p>
      {short ? (
        <p className="flex items-center gap-1 text-small font-semibold text-danger tabular-nums">
          <TriangleAlert aria-hidden className="size-3.5 shrink-0" />
          {`Short by ${money(short.amount)} on ${weekdayDate(short.date, today)}`}
        </p>
      ) : (
        forecast.heldFor &&
        forecast.heldBack !== null && (
          <p className="text-small text-muted-foreground tabular-nums">
            {`${money(forecast.heldBack)} held back for ${formatSpan(forecast.heldFor.start, forecast.heldFor.end, today)}`}
          </p>
        )
      )}
      {earlyPayday && (
        <p className="flex items-center gap-2 text-small">
          {`Payday ${weekdayDate(earlyPayday.date, today)}. Paid early?`}
          <Button
            size="sm"
            variant="ghost"
            className="text-primary"
            onClick={() =>
              payslip({ ruleId: earlyPayday.ruleId, date: earlyPayday.date })
            }
          >
            Record now
          </Button>
        </p>
      )}
    </GlassPanel>
  );
}
