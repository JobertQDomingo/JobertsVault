import { cn } from "@/lib/utils";
import { GlassPanel } from "./GlassPanel";
import { MoneyText } from "./MoneyText";

/**
 * A period total (DESIGN_SYSTEM §9.2): a label and a display-size amount. The net cashflow card is
 * the only one tinted with the accent (§2.3). The % change line arrives in Phase 2 (DSH-3).
 */
export function StatCard({
  label,
  amount,
  kind,
}: {
  label: string;
  amount: number;
  kind: "income" | "expense" | "net";
}) {
  return (
    <GlassPanel
      className={cn("grid gap-1.5 p-5", kind === "net" && "glass-tint")}
    >
      <p className="font-semibold text-muted-foreground">{label}</p>
      <MoneyText amount={amount} kind={kind} size="display" />
    </GlassPanel>
  );
}
