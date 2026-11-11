import { format } from "date-fns";
import type { Frequency } from "@/bindings/Frequency";
import { toDate } from "./dates";

const UNITS: Record<Frequency, [string, string]> = {
  daily: ["day", "days"],
  weekly: ["week", "weeks"],
  monthly: ["month", "months"],
  yearly: ["year", "years"],
};

const EVERY_ONE: Record<Frequency, string> = {
  daily: "Daily",
  weekly: "Weekly",
  monthly: "Monthly",
  yearly: "Yearly",
};

/** "Every 2 [months]": singular only when the typed interval is 1. */
export const unitOf = (frequency: Frequency, interval: string) =>
  UNITS[frequency][interval.trim() === "1" ? 0 : 1];

/**
 * A rule's schedule in words, from its start date (UI_FLOWS §8.2): "Monthly on the 25th",
 * "Every 2 weeks on Fridays", "Yearly on Mar 15", "Daily".
 */
export function describeSchedule(rule: {
  frequency: Frequency;
  interval: number;
  transaction: { date: string };
}) {
  const start = toDate(rule.transaction.date);
  const every =
    rule.interval === 1
      ? EVERY_ONE[rule.frequency]
      : `Every ${rule.interval} ${UNITS[rule.frequency][1]}`;
  const on = {
    daily: "",
    weekly: ` on ${format(start, "EEEE")}s`,
    monthly: ` on the ${format(start, "do")}`,
    yearly: ` on ${format(start, "MMM d")}`,
  }[rule.frequency];
  return every + on;
}
