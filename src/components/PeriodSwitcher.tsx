import { ChevronLeft, ChevronRight } from "lucide-react";
import { Popover } from "radix-ui";
import { useState } from "react";
import { DayPicker, type DateRange } from "react-day-picker";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import { usePeriod } from "@/hooks/usePeriod";
import { usePeriodQuery } from "@/hooks/usePeriodQuery";
import { useSettings } from "@/hooks/useSettings";
import { formatPeriodLabel, toDate, toIso } from "@/lib/dates";
import { GlassPanel } from "./GlassPanel";
import { Button } from "./ui/button";
import { calendarClasses } from "./ui/date-picker";
import { Segmented } from "./ui/segmented";

const KINDS: { value: PeriodQuery["kind"]; label: string }[] = [
  { value: "day", label: "Day" },
  { value: "week", label: "Week" },
  { value: "month", label: "Month" },
  { value: "year", label: "Year" },
  { value: "custom", label: "Custom" },
];

const ENDS =
  "[&>button]:bg-primary [&>button]:font-semibold [&>button]:text-primary-foreground [&>button]:hover:bg-primary-hover";

/**
 * The period bar (DESIGN_SYSTEM §9.2, UI_FLOWS F8): `[Day│Week│Month│Year│Custom] ◀ label ▶ [Today]`,
 * saved in the URL. ◀ ▶ and Today go where `get_period` says; React never works out dates.
 */
export function PeriodSwitcher() {
  const [query, setQuery] = usePeriodQuery();
  // While the next period loads, `view` is the previous one, shown but not acted on.
  const { data: view, isPlaceholderData: loading, isError } = usePeriod(query);
  const { data: settings } = useSettings();
  const [picking, setPicking] = useState(false);
  const [range, setRange] = useState<DateRange | undefined>();

  const go = (next: PeriodQuery | null | undefined) => {
    if (next && !loading) setQuery(next);
  };
  const pickKind = (kind: PeriodQuery["kind"]) => {
    if (kind === "custom") {
      // Needs the backend's today to rule out future days. Each opening starts a fresh range.
      if (!view) return;
      setRange(undefined);
      setPicking(true);
    } else if (kind !== query.kind)
      // Keeps the place: today while looking at the current period, else the same day (a
      // custom range hands over its last day).
      setQuery({
        kind,
        anchor:
          view?.isCurrent && !loading
            ? null
            : query.kind === "custom"
              ? query.end
              : query.anchor,
      });
  };

  return (
    <Popover.Root open={picking} onOpenChange={setPicking}>
      <Popover.Anchor asChild>
        <GlassPanel className="flex w-fit items-center gap-3 rounded-toolbar p-1.5">
          <Segmented
            aria-label="Period"
            value={query.kind}
            options={KINDS}
            onChange={pickKind}
            onReselect={() => query.kind === "custom" && pickKind("custom")}
          />
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              aria-label="Previous period"
              title="Previous period (Alt+←)"
              disabled={!view?.previous}
              onClick={() => go(view?.previous)}
            >
              <ChevronLeft />
            </Button>
            <span
              aria-live="polite"
              className="min-w-[200px] text-center font-semibold tabular-nums"
            >
              {/* The view's own kind (Today keeps it), so a loading switch never mislabels it. */}
              {isError
                ? "Couldn't load this period"
                : view &&
                  formatPeriodLabel(view.todayQuery.kind, view.start, view.end)}
            </span>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Next period"
              title="Next period (Alt+→)"
              disabled={!view?.next}
              onClick={() => go(view?.next)}
            >
              <ChevronRight />
            </Button>
          </div>
          <Button
            size="sm"
            title="Go to today (Alt+Home)"
            // After an error, Today is the way out: back to this month, the default.
            disabled={!isError && (!view || view.isCurrent)}
            onClick={() =>
              go(isError ? { kind: "month", anchor: null } : view?.todayQuery)
            }
          >
            Today
          </Button>
        </GlassPanel>
      </Popover.Anchor>
      <Popover.Portal>
        <Popover.Content
          align="start"
          sideOffset={8}
          aria-label="Custom period"
          className="glass-raised z-50 grid gap-3 rounded-toolbar p-3"
        >
          <DayPicker
            mode="range"
            autoFocus
            selected={range}
            onSelect={setRange}
            defaultMonth={
              range?.from ?? (view ? toDate(view.start) : undefined)
            }
            endMonth={view ? toDate(view.today) : undefined}
            weekStartsOn={settings?.weekStart === "sunday" ? 0 : 1}
            // Until Phase 3 nothing is recorded after today (TXN-8).
            disabled={view ? { after: toDate(view.today) } : undefined}
            classNames={{
              ...calendarClasses,
              // The two ends are solid accent, the days between a flat wash.
              range_start: ENDS,
              range_end: ENDS,
              range_middle: "bg-wash",
            }}
          />
          <div className="flex justify-end gap-2">
            <Button size="sm" onClick={() => setPicking(false)}>
              Cancel
            </Button>
            <Button
              size="sm"
              variant="primary"
              disabled={!range?.from || !range.to}
              onClick={() => {
                if (!range?.from || !range.to) return;
                setQuery({
                  kind: "custom",
                  start: toIso(range.from),
                  end: toIso(range.to),
                });
                setPicking(false);
              }}
            >
              Apply
            </Button>
          </div>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
