import { expect, test } from "vitest";
import {
  addDays,
  formatInputDate,
  formatDayHeader,
  formatPeriodLabel,
  parseInputDate,
  toDate,
  toIso,
} from "./dates";

test("a date string round-trips through a Date", () => {
  const date = toDate("2026-10-26");
  expect([date.getFullYear(), date.getMonth(), date.getDate()]).toEqual([
    2026, 9, 26,
  ]);
  expect(toIso(date)).toBe("2026-10-26");
});

test("the date input shows US order", () => {
  expect(formatInputDate("2026-01-05")).toBe("01/05/2026");
});

test("typed US dates are read, with or without leading zeros", () => {
  expect(parseInputDate("1/5/2026")).toBe("2026-01-05");
  expect(parseInputDate(" 10/26/2026 ")).toBe("2026-10-26");
});

test("impossible or other-format dates are refused", () => {
  for (const text of [
    "02/30/2026",
    "13/01/2026",
    "2026-01-05",
    "abc",
    "",
    "1/5/26",
  ])
    expect(parseInputDate(text), text).toBeNull();
});

test("adding days crosses months, years and leap days", () => {
  expect(addDays("2026-02-28", 1)).toBe("2026-03-01");
  expect(addDays("2024-02-28", 1)).toBe("2024-02-29");
  expect(addDays("2026-01-01", -1)).toBe("2025-12-31");
});

test("text that isn't a stored date is shown as typed", () => {
  expect(formatInputDate("")).toBe("");
  expect(formatInputDate("13/05/2026")).toBe("13/05/2026");
});

test("period labels follow DESIGN_SYSTEM §8", () => {
  expect(formatPeriodLabel("day", "2026-10-26", "2026-10-26")).toBe(
    "Mon, Oct 26, 2026",
  );
  expect(formatPeriodLabel("week", "2026-10-26", "2026-11-01")).toBe(
    "Oct 26 – Nov 1, 2026",
  );
  expect(formatPeriodLabel("week", "2025-12-29", "2026-01-04")).toBe(
    "Dec 29, 2025 – Jan 4, 2026",
  );
  expect(formatPeriodLabel("month", "2026-10-01", "2026-10-31")).toBe(
    "October 2026",
  );
  expect(formatPeriodLabel("month", "2026-09-25", "2026-10-24")).toBe(
    "Sep 25 – Oct 24, 2026",
  );
  expect(formatPeriodLabel("year", "2026-01-01", "2026-12-31")).toBe("2026");
  expect(formatPeriodLabel("custom", "2026-10-01", "2026-10-10")).toBe(
    "Oct 1 – Oct 10, 2026",
  );
});

test("a one-day custom range reads like a day", () => {
  expect(formatPeriodLabel("custom", "2026-10-01", "2026-10-01")).toBe(
    "Thu, Oct 1, 2026",
  );
});

test("day headers: Today, Yesterday, else the weekday and date, with the year if it's another", () => {
  expect(formatDayHeader("2026-11-05", "2026-11-05")).toBe("Today");
  expect(formatDayHeader("2026-11-04", "2026-11-05")).toBe("Yesterday");
  expect(formatDayHeader("2025-12-31", "2026-01-01")).toBe("Yesterday");
  expect(formatDayHeader("2026-10-26", "2026-11-05")).toBe("Mon, Oct 26");
  expect(formatDayHeader("2025-10-26", "2026-11-05")).toBe("Sun, Oct 26, 2025");
});
