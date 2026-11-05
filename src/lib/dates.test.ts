import { expect, test } from "vitest";
import {
  addDays,
  formatInputDate,
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
