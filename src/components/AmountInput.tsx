import { useState, type ComponentProps } from "react";
import { useSettings } from "@/hooks/useSettings";
import { formatAmount, parseAmount } from "@/lib/money";
import { cn } from "@/lib/utils";
import { fieldClasses } from "./ui/input";

/**
 * The amount field (DESIGN_SYSTEM §7): digits, one `.` and pasted text with commas, never
 * negative. It right-aligns, shows separators once you leave it, and reports centavos (or null
 * when there's no amount yet) through `money.ts`.
 */
export function AmountInput({
  value,
  onChange,
  onBlur,
  className,
  ...props
}: Omit<ComponentProps<"input">, "value" | "onChange"> & {
  value: number | null;
  onChange: (minor: number | null) => void;
}) {
  const { data: settings } = useSettings();
  const decimals = settings?.currencyDecimals ?? 2;
  // What's being typed. It's shown only while it still means `value`, so a reset clears it.
  const [draft, setDraft] = useState<string | null>(null);
  const shown =
    draft !== null && parseAmount(draft, decimals) === value
      ? draft
      : value === null
        ? ""
        : formatAmount(value, decimals);

  return (
    <input
      inputMode="decimal"
      autoComplete="off"
      className={cn(
        fieldClasses,
        "text-right font-semibold tabular-nums",
        className,
      )}
      {...props}
      // Until the settings arrive, the number of decimals isn't known.
      disabled={!settings || props.disabled}
      value={shown}
      onChange={(e) => {
        // Never negative: a minus is refused rather than silently dropped.
        if (/[-−]/.test(e.target.value)) return;
        // Pasted symbols and spaces drop out; a keystroke that only adds one of them changes nothing.
        const text = e.target.value.replace(/[^\d.,]/g, "");
        const points = text.split(".").length - 1;
        const fraction = text.split(".")[1] ?? "";
        if (
          text === shown ||
          points > (decimals > 0 ? 1 : 0) ||
          fraction.length > decimals
        )
          return;
        setDraft(text);
        onChange(parseAmount(text, decimals));
      }}
      onBlur={(e) => {
        if (draft !== null && parseAmount(draft, decimals) !== null)
          setDraft(null);
        onBlur?.(e);
      }}
    />
  );
}
