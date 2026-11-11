import { expect, test } from "vitest";
import type { Frequency } from "@/bindings/Frequency";
import { describeSchedule, unitOf } from "./recurrence";

const rule = (frequency: Frequency, interval: number, start: string) => ({
  frequency,
  interval,
  transaction: { date: start },
});

test.each([
  ["daily", 1, "2026-11-03", "Daily"],
  ["daily", 3, "2026-11-03", "Every 3 days"],
  ["weekly", 1, "2026-11-02", "Weekly on Mondays"],
  ["weekly", 2, "2026-11-06", "Every 2 weeks on Fridays"],
  ["monthly", 1, "2026-10-25", "Monthly on the 25th"],
  ["monthly", 1, "2026-01-31", "Monthly on the 31st"],
  ["monthly", 3, "2026-11-02", "Every 3 months on the 2nd"],
  ["yearly", 1, "2028-02-29", "Yearly on Feb 29"],
  ["yearly", 2, "2026-03-15", "Every 2 years on Mar 15"],
] as const)(
  "%s every %i from %s reads %j",
  (frequency, interval, start, text) => {
    expect(describeSchedule(rule(frequency, interval, start))).toBe(text);
  },
);

test("a unit is singular only for 1", () => {
  expect(unitOf("monthly", "1")).toBe("month");
  expect(unitOf("monthly", " 1 ")).toBe("month");
  expect(unitOf("weekly", "2")).toBe("weeks");
  expect(unitOf("daily", "")).toBe("days");
});
