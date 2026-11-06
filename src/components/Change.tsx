import { formatShare } from "@/lib/money";
import { cn } from "@/lib/utils";

/**
 * A % change from the backend, in tenths of a percent (DESIGN_SYSTEM §7): `▲ 12.3%` or `▼ 4.0%`,
 * green when the direction is good and red when it's bad, or `—` when there was nothing to compare
 * with. The arrow is spelled out for screen readers, so color is never the only cue.
 */
export function Change({
  tenths,
  upIsGood,
}: {
  tenths: number | null;
  /** True for income and net cashflow; false for expense, where up is bad. */
  upIsGood: boolean;
}) {
  if (tenths === null)
    return (
      <span title="Nothing to compare with" className="text-muted-foreground">
        —
      </span>
    );
  if (tenths === 0)
    return <span className="text-muted-foreground tabular-nums">0.0%</span>;
  const up = tenths > 0;
  return (
    <span
      className={cn(
        "font-semibold whitespace-nowrap tabular-nums",
        up === upIsGood ? "text-income" : "text-expense",
      )}
    >
      <span aria-hidden>{up ? "▲ " : "▼ "}</span>
      <span className="sr-only">{up ? "Up " : "Down "}</span>
      {formatShare(Math.abs(tenths))}
    </span>
  );
}
