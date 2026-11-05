import { ToggleGroup } from "radix-ui";
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

/**
 * A flat `--wash` track whose selected segment is solid accent (DESIGN_SYSTEM §9.1). Exactly
 * one segment is always selected. Arrow keys move between segments.
 */
export function Segmented<V extends string>({
  value,
  onChange,
  onReselect,
  options,
  className,
  disabled,
  "aria-label": ariaLabel,
}: {
  value: V;
  onChange: (value: V) => void;
  /** The selected segment was clicked again (the period bar reopens Custom's picker). */
  onReselect?: () => void;
  options: { value: V; label: ReactNode }[];
  className?: string;
  disabled?: boolean;
  "aria-label": string;
}) {
  return (
    <ToggleGroup.Root
      type="single"
      value={value}
      // Radix sends "" when the selected segment is clicked again; keep the selection instead.
      onValueChange={(next: string) =>
        next ? onChange(next as V) : onReselect?.()
      }
      aria-label={ariaLabel}
      disabled={disabled}
      className={cn(
        "inline-flex rounded-control bg-wash p-[3px]",
        // Radix marks only the items as disabled, so the group is dimmed here.
        disabled && "opacity-50",
        className,
      )}
    >
      {options.map((option) => (
        <ToggleGroup.Item
          key={option.value}
          value={option.value}
          className="h-[30px] rounded-[9px] px-3 whitespace-nowrap text-muted-foreground transition-colors duration-150 hover:text-foreground data-[state=on]:bg-primary data-[state=on]:font-semibold data-[state=on]:text-primary-foreground data-[state=on]:shadow-[inset_0_1px_0_rgb(255_255_255/.25)]"
        >
          {option.label}
        </ToggleGroup.Item>
      ))}
    </ToggleGroup.Root>
  );
}
