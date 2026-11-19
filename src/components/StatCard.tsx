import { cn } from "@/lib/utils";
import { Change } from "./Change";
import { GlassPanel } from "./GlassPanel";
import { MoneyText } from "./MoneyText";

/**
 * A period total (DESIGN_SYSTEM §9.2): a label, a display-size amount, and its % change with the
 * dates it's compared with (DSH-3). The net cashflow card is the only one tinted with the accent
 * (§2.3). `flat` is the `--wash` card for inside a panel, untinted (the check-in, UI_FLOWS §8.7).
 */
export function StatCard({
  label,
  amount,
  kind,
  change,
  vs,
  flat = false,
}: {
  label: string;
  amount: number;
  kind: "income" | "expense" | "net";
  /** Tenths of a percent from the backend, or null when there's nothing to compare with. */
  change: number | null;
  /** The dates compared with, e.g. "Oct 1–6". */
  vs: string;
  flat?: boolean;
}) {
  const Box = flat ? "div" : GlassPanel;
  return (
    <Box
      className={cn(
        "grid gap-1.5 p-5",
        flat ? "rounded-control bg-wash" : kind === "net" && "glass-tint",
      )}
    >
      <p className="font-semibold text-muted-foreground">{label}</p>
      <MoneyText amount={amount} kind={kind} size="display" />
      <p className="flex items-baseline gap-1.5 text-small">
        <Change tenths={change} upIsGood={kind !== "expense"} />
        <span className="text-muted-foreground">vs {vs}</span>
      </p>
    </Box>
  );
}
