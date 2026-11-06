// Dates cross the Rust boundary as 'YYYY-MM-DD' strings. This file is the only place that turns
// them into Date objects (for the date picker) or display text (ARCHITECTURE §2, DESIGN_SYSTEM §8).
import { addDays as addDaysToDate, format, isValid, parse } from "date-fns";
import type { Grain } from "@/bindings/Grain";
import type { PeriodQuery } from "@/bindings/PeriodQuery";

const ISO = "yyyy-MM-dd";

/** Local midnight on that day. */
export const toDate = (iso: string) => parse(iso, ISO, new Date());

export const toIso = (date: Date) => format(date, ISO);

/** Whether `text` is a real `YYYY-MM-DD` date. */
export const isIsoDate = (text: string) =>
  /^\d{4}-\d{2}-\d{2}$/.test(text) && isValid(toDate(text));

/** What the date input shows: always US order (DESIGN_SYSTEM §8). Other text is shown as it is. */
export const formatInputDate = (iso: string) =>
  isIsoDate(iso) ? format(toDate(iso), "MM/dd/yyyy") : iso;

/** A typed `M/D/YYYY` date, or null if it isn't one (e.g. Feb 30). */
export function parseInputDate(text: string): string | null {
  const trimmed = text.trim();
  if (!/^\d{1,2}\/\d{1,2}\/\d{4}$/.test(trimmed)) return null;
  const date = parse(trimmed, "M/d/yyyy", new Date());
  return isValid(date) ? toIso(date) : null;
}

export const addDays = (iso: string, days: number) =>
  toIso(addDaysToDate(toDate(iso), days));

/** A date without its year, e.g. `Nov 5` (a card's due date). */
export const formatShortDate = (iso: string) => format(toDate(iso), "MMM d");

/** Consecutive items on the same date, in the order given (newest first, as the backend sends). */
export function groupByDate<T extends { date: string }>(items: T[]) {
  const days: { date: string; items: T[] }[] = [];
  for (const item of items) {
    const last = days[days.length - 1];
    if (last?.date === item.date) last.items.push(item);
    else days.push({ date: item.date, items: [item] });
  }
  return days;
}

/** A list's day header (DESIGN_SYSTEM §8), relative to the backend's `today`. */
export function formatDayHeader(iso: string, today: string) {
  if (iso === today) return "Today";
  if (iso === addDays(today, -1)) return "Yesterday";
  const date = toDate(iso);
  return date.getFullYear() === toDate(today).getFullYear()
    ? format(date, "EEE, MMM d")
    : format(date, "EEE, MMM d, yyyy");
}

/** `Oct 26 – Nov 1, 2026`, or with both years when they differ (DESIGN_SYSTEM §8). */
function formatRange(start: Date, end: Date) {
  return start.getFullYear() === end.getFullYear()
    ? `${format(start, "MMM d")} – ${format(end, "MMM d, yyyy")}`
    : `${format(start, "MMM d, yyyy")} – ${format(end, "MMM d, yyyy")}`;
}

/** The period switcher's label (DESIGN_SYSTEM §8), from `get_period`'s dates. */
export function formatPeriodLabel(
  kind: PeriodQuery["kind"],
  start: string,
  end: string,
) {
  const from = toDate(start);
  // A one-day custom range reads like a day.
  if (kind === "day" || start === end) return format(from, "EEE, MMM d, yyyy");
  switch (kind) {
    case "year":
      return format(from, "yyyy");
    case "month":
      // A month that starts on the 1st is a calendar month; with month_start_day > 1 it's a range.
      if (from.getDate() === 1) return format(from, "MMMM yyyy");
  }
  return formatRange(from, toDate(end));
}

/**
 * A chart bar's axis label, from its first day: `Mon` (`weekdays`, for a week of days), `Nov 2`
 * for a day or a week, and `Jan`, or `Jan 2026` when the chart spans `years`.
 */
export function formatBarLabel(
  start: string,
  grain: Grain,
  options: { weekdays?: boolean; years?: boolean } = {},
) {
  const date = toDate(start);
  if (grain === "month")
    return format(date, options.years ? "MMM yyyy" : "MMM");
  return format(date, grain === "day" && options.weekdays ? "EEE" : "MMM d");
}

/** A chart bar's dates in its tooltip: a day, a whole calendar month by name, or the range. */
export function formatBarDates(start: string, end: string) {
  const wholeMonth =
    start.endsWith("-01") &&
    start.slice(0, 7) === end.slice(0, 7) &&
    addDays(end, 1).endsWith("-01");
  return formatPeriodLabel(wholeMonth ? "month" : "custom", start, end);
}
