import { CalendarDays } from "lucide-react";
import { Popover } from "radix-ui";
import { useState, type ComponentProps } from "react";
import { DayPicker } from "react-day-picker";
import type { WeekStart } from "@/bindings/WeekStart";
import {
  addDays,
  formatInputDate,
  isIsoDate,
  parseInputDate,
  toDate,
  toIso,
} from "@/lib/dates";
import { cn } from "@/lib/utils";
import { Button } from "./button";
import { fieldClasses } from "./input";

type InputProps = Pick<
  ComponentProps<"input">,
  "id" | "aria-label" | "aria-invalid" | "aria-describedby" | "disabled"
>;

/** The calendar's look, shared with the period switcher's range picker. */
export const calendarClasses = {
  months: "relative",
  month_caption: "flex h-8 items-center px-1 font-semibold",
  nav: "absolute top-0 right-0 z-10 flex gap-1",
  button_previous:
    "grid size-8 place-items-center rounded-chip hover:bg-wash aria-disabled:pointer-events-none aria-disabled:opacity-40",
  button_next:
    "grid size-8 place-items-center rounded-chip hover:bg-wash aria-disabled:pointer-events-none aria-disabled:opacity-40",
  chevron: "size-4 fill-current",
  month_grid: "mt-2 border-collapse",
  weekday: "size-8 text-small font-normal text-muted-foreground",
  day: "p-0 text-center",
  day_button:
    "size-8 rounded-chip tabular-nums transition-colors duration-150 hover:bg-wash",
  today: "[&>button]:font-semibold",
  outside: "text-muted-foreground",
  disabled: "opacity-40 [&>button]:pointer-events-none",
};

/**
 * A `MM/DD/YYYY` input with a calendar (DESIGN_SYSTEM §8, §9.1). ↑/↓ move one day, and any day
 * can be picked, future ones too (TXN-8). Weeks start on `weekStart`. Typed text that isn't a date
 * still goes to `onChange`, so the backend's "Enter a real date" shows under the field.
 */
export function DatePicker({
  value,
  onChange,
  weekStart,
  className,
  ...inputProps
}: InputProps & {
  value: string;
  onChange: (iso: string) => void;
  weekStart: WeekStart;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  // What's typed, while it's being typed; null shows `value`.
  const [draft, setDraft] = useState<string | null>(null);
  const valid = isIsoDate(value);
  const change = (next: string) => {
    if (next !== value) onChange(next);
  };
  const commit = () => {
    if (draft === null) return;
    change(parseInputDate(draft) ?? draft.trim());
    setDraft(null);
  };

  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Anchor asChild>
        <div className={cn("relative", className)}>
          <input
            {...inputProps}
            value={draft ?? formatInputDate(value)}
            placeholder="MM/DD/YYYY"
            autoComplete="off"
            className={cn(fieldClasses, "pr-10 tabular-nums")}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commit}
            onKeyDown={(e) => {
              if (e.key === "ArrowUp" || e.key === "ArrowDown") {
                e.preventDefault();
                if (!valid) return;
                setDraft(null);
                change(addDays(value, e.key === "ArrowUp" ? 1 : -1));
              } else if (e.key === "Enter") commit(); // and let the form save
            }}
          />
          <Popover.Trigger asChild>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Choose date"
              className="absolute top-0.5 right-0.5"
              disabled={inputProps.disabled}
            >
              <CalendarDays />
            </Button>
          </Popover.Trigger>
        </div>
      </Popover.Anchor>
      <Popover.Portal>
        <Popover.Content
          align="end"
          sideOffset={4}
          className="glass-raised z-50 rounded-toolbar p-3"
        >
          <DayPicker
            mode="single"
            required
            autoFocus
            selected={valid ? toDate(value) : undefined}
            defaultMonth={valid ? toDate(value) : undefined}
            weekStartsOn={weekStart === "monday" ? 1 : 0}
            onSelect={(date) => {
              change(toIso(date));
              setOpen(false);
            }}
            classNames={{
              ...calendarClasses,
              selected:
                "[&>button]:bg-primary [&>button]:font-semibold [&>button]:text-primary-foreground [&>button]:hover:bg-primary-hover",
            }}
          />
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
