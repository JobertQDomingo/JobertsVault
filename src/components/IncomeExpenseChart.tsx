import type { ReactNode } from "react";
import {
  Bar,
  CartesianGrid,
  ComposedChart,
  Line,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
  type TooltipContentProps,
} from "recharts";
import type {
  NameType,
  ValueType,
} from "recharts/types/component/DefaultTooltipContent";
import type { Bar as ChartBar } from "@/bindings/Bar";
import type { Timeseries } from "@/bindings/Timeseries";
import { useSettings } from "@/hooks/useSettings";
import { formatBarDates, formatBarLabel } from "@/lib/dates";
import { formatCompact } from "@/lib/money";
import { cn } from "@/lib/utils";
import { MoneyText } from "./MoneyText";

const TICK = { fill: "var(--muted-foreground)", fontSize: 12 };

/** A legend or tooltip key: a swatch for a bar, a short stroke for the net line. */
function Key({ color, line }: { color: string; line?: boolean }) {
  return (
    <span
      aria-hidden
      className={cn(
        "inline-block shrink-0",
        line ? "h-0.5 w-3.5 rounded-full" : "size-2.5 rounded-[3px]",
        color,
      )}
    />
  );
}

const KEYS = [
  { label: "Income", color: "bg-income" },
  { label: "Expense", color: "bg-expense" },
  { label: "Net cashflow", color: "bg-primary", line: true },
];

function Figure({ index, children }: { index: number; children: ReactNode }) {
  const { label, color, line } = KEYS[index];
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="flex items-center gap-1.5 text-muted-foreground">
        <Key color={color} line={line} />
        {label}
      </span>
      {children}
    </div>
  );
}

/** Flat and solid: it floats inside the chart's glass panel, so it's never blurred (§2.3). */
function BarTooltip({
  active,
  payload,
}: TooltipContentProps<ValueType, NameType>) {
  const bar = payload?.[0]?.payload as ChartBar | undefined;
  if (!active || !bar) return null;
  return (
    <div className="grid min-w-52 gap-1 rounded-control border border-edge bg-glass-solid px-3 py-2 text-small shadow-raised">
      <p className="font-semibold">{formatBarDates(bar.start, bar.end)}</p>
      <Figure index={0}>
        <MoneyText amount={bar.income} kind="income" />
      </Figure>
      <Figure index={1}>
        <MoneyText amount={bar.expense} kind="expense" />
      </Figure>
      <Figure index={2}>
        <MoneyText amount={bar.net} kind="net" />
      </Figure>
    </div>
  );
}

/**
 * Income vs expense over time (RPT-1, UI_FLOWS §8.1): income rises above the baseline and expense
 * hangs below it, so the two never rely on green and red alone, with net cashflow as a line. The
 * bars are the backend's; a hidden table gives screen readers the same figures.
 */
export function IncomeExpenseChart({
  series,
  height,
}: {
  series: Timeseries;
  height: number;
}) {
  const { data: settings } = useSettings();
  if (!settings) return null;
  const { bars, grain } = series;
  const empty = bars.every((b) => b.income === 0 && b.expense === 0);
  const years =
    bars[0]?.start.slice(0, 4) !== bars[bars.length - 1]?.end.slice(0, 4);
  // Expense is drawn below zero: a direction on the chart, not a new figure. The net line stops at
  // today, so days still to come don't read as a net of 0.
  const rows = bars.map((b) => ({
    ...b,
    below: -b.expense,
    line: b.start > series.today ? null : b.net,
  }));
  return (
    <div className="grid gap-2">
      <div aria-hidden className="flex gap-4 text-small text-muted-foreground">
        {KEYS.map((k) => (
          <span key={k.label} className="flex items-center gap-1.5">
            <Key color={k.color} line={k.line} />
            {k.label}
          </span>
        ))}
      </div>
      <div aria-hidden style={{ height }} className="relative">
        <ResponsiveContainer>
          <ComposedChart
            data={rows}
            stackOffset="sign"
            accessibilityLayer={false}
            margin={{ top: 4, right: 4, bottom: 0, left: 0 }}
          >
            <CartesianGrid vertical={false} stroke="var(--hairline)" />
            <XAxis
              dataKey="start"
              tickFormatter={(start: string) =>
                formatBarLabel(start, grain, {
                  weekdays: bars.length <= 7,
                  years,
                })
              }
              tick={TICK}
              tickLine={false}
              axisLine={false}
              interval="preserveStartEnd"
              minTickGap={16}
            />
            <YAxis
              tickFormatter={(minor: number) => formatCompact(minor, settings)}
              tick={TICK}
              tickLine={false}
              axisLine={false}
              allowDecimals={false}
              width={64}
            />
            <ReferenceLine y={0} stroke="var(--field-edge)" />
            <Tooltip
              content={(props) => <BarTooltip {...props} />}
              cursor={{ fill: "var(--wash)" }}
              isAnimationActive={false}
            />
            <Bar
              dataKey="income"
              stackId="flow"
              fill="var(--income)"
              maxBarSize={24}
              radius={[4, 4, 0, 0]}
              isAnimationActive={false}
            />
            <Bar
              dataKey="below"
              stackId="flow"
              fill="var(--expense)"
              maxBarSize={24}
              radius={[4, 4, 0, 0]}
              isAnimationActive={false}
            />
            <Line
              dataKey="line"
              type="linear"
              stroke="var(--primary)"
              strokeWidth={2}
              // A single point has no line to show it.
              dot={bars.length === 1 && { r: 4, fill: "var(--primary)" }}
              activeDot={{ r: 4, stroke: "var(--glass-solid)", strokeWidth: 2 }}
              isAnimationActive={false}
            />
          </ComposedChart>
        </ResponsiveContainer>
        {empty && (
          <p className="absolute inset-0 grid place-items-center text-muted-foreground">
            Nothing recorded in this period.
          </p>
        )}
      </div>
      <table className="sr-only">
        <caption>Income, expense and net cashflow by {grain}</caption>
        <thead>
          <tr>
            <th scope="col">Dates</th>
            {KEYS.map((k) => (
              <th key={k.label} scope="col">
                {k.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {bars.map((b) => (
            <tr key={b.start}>
              <th scope="row">{formatBarDates(b.start, b.end)}</th>
              <td>
                <MoneyText amount={b.income} kind="income" />
              </td>
              <td>
                <MoneyText amount={b.expense} kind="expense" />
              </td>
              <td>
                <MoneyText amount={b.net} kind="net" />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
