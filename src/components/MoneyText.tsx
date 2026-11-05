import { ArrowLeftRight } from "lucide-react";
import { useSettings } from "@/hooks/useSettings";
import { formatMoney } from "@/lib/money";
import { cn } from "@/lib/utils";

export type MoneyKind =
  "income" | "expense" | "transfer" | "balance" | "owed" | "net";

function colorOf(amount: number, kind: MoneyKind) {
  if (amount === 0) return "text-muted-foreground";
  switch (kind) {
    case "income":
      return "text-income";
    case "expense":
      return "text-expense";
    case "transfer":
      return "text-transfer";
    case "net":
      return amount > 0 ? "text-income" : "text-expense";
    case "balance":
      return amount < 0 ? "text-expense" : "text-foreground";
    case "owed":
      return "text-foreground";
  }
}

/**
 * An amount in the DESIGN_SYSTEM §7 format. Income and expense always carry a sign and a
 * transfer an icon, so color is never the only cue. `amount` is as stored: an expense is positive.
 */
export function MoneyText({
  amount,
  kind,
  size = "body",
  className,
}: {
  amount: number;
  kind: MoneyKind;
  size?: "body" | "display";
  className?: string;
}) {
  const { data: settings } = useSettings();
  if (!settings) return null;
  const text = formatMoney(kind === "expense" ? -amount : amount, settings, {
    plus: kind === "income" || kind === "net",
  });
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 whitespace-nowrap tabular-nums",
        size === "display" ? "text-display" : "font-semibold",
        colorOf(amount, kind),
        className,
      )}
    >
      {kind === "transfer" && <ArrowLeftRight aria-hidden className="size-4" />}
      {text}
      {kind === "owed" && " owed"}
    </span>
  );
}
