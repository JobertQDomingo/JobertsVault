import { useId, type ReactNode } from "react";
import { cn } from "@/lib/utils";

/**
 * An 8 px pill on a `--wash` track (DESIGN_SYSTEM §9.2): accent below 80%, amber from 80%, red from
 * 100% (BUD-3). A goal's bar has no thresholds (`tone="goal"`): accent, and green once `done`. The
 * percent is always written next to it; `label` replaces that text, e.g. "6% of ₱10,000.00 limit".
 */
export function ProgressBar({
  percent,
  label,
  tone = "limit",
  done = false,
  className,
  "aria-label": ariaLabel,
}: {
  percent: number;
  tone?: "limit" | "goal";
  done?: boolean;
  label?: ReactNode;
  className?: string;
  "aria-label"?: string;
}) {
  const labelId = useId();
  const limitColor =
    percent >= 100 ? "bg-danger" : percent >= 80 ? "bg-warning" : "bg-primary";
  const goalColor = done ? "bg-success" : "bg-primary";
  const color = tone === "goal" ? goalColor : limitColor;
  return (
    <div className={cn("flex items-center gap-2.5", className)}>
      <div
        role="progressbar"
        aria-label={ariaLabel}
        aria-labelledby={ariaLabel ? undefined : labelId}
        aria-valuenow={Math.max(0, percent)}
        aria-valuemin={0}
        // A card can go past its limit, so the scale grows with it.
        aria-valuemax={Math.max(100, percent)}
        className="h-2 min-w-15 flex-1 overflow-hidden rounded-pill bg-wash shadow-[inset_0_0_0_1px_var(--hairline)]"
      >
        <div
          className={cn("h-full rounded-pill", color)}
          style={{ width: `${Math.min(100, Math.max(0, percent))}%` }}
        />
      </div>
      <span
        id={labelId}
        // A fixed minimum, so bars in a list end level whatever their percent.
        className="min-w-10 text-right text-small whitespace-nowrap text-muted-foreground tabular-nums"
      >
        {label ?? `${percent}%`}
      </span>
    </div>
  );
}
