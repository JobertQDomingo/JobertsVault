// Dates cross the Rust boundary as 'YYYY-MM-DD' strings. This file is the only place that turns
// them into Date objects (for the date picker) or display text (ARCHITECTURE §2, DESIGN_SYSTEM §8).
import { addDays as addDaysToDate, format, isValid, parse } from "date-fns";

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
