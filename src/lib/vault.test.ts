import { expect, test } from "vitest";
import type { ConditionCode } from "@/bindings/ConditionCode";
import { conditionGap, CONDITIONS, LEVEL_CONDITIONS } from "./vault";

const names = {
  money: (amount: number) => `₱${amount}`,
  account: (id: number) => `Card ${id}`,
};
const gap = (
  code: ConditionCode,
  value: number | null,
  met = false,
  accountId: number | null = null,
) => conditionGap({ code, met, gap: value, accountId }, names);

test("an unmet condition's gap, after · (UI_FLOWS §8.8)", () => {
  expect(gap("logged_20", 1)).toBe("1 day more to log");
  expect(gap("earned_3_months", 0)).toBe("0 of 3 so far");
  expect(gap("card_under_mid", 300, false, 4)).toBe("Card 4 is ₱300 over now");
  expect(gap("two_weeks_saved", 50)).toBe("₱50 to go");
  expect(gap("loans_paid", 800)).toBe("₱800 left");
  expect(gap("loan_free_365", 120)).toBe("120 days to go");
  expect(gap("stats_80", 62)).toBe("the lowest is 62");
});

test("a met condition, one with no figure yet and one with no gap show none", () => {
  expect(gap("loans_paid", 0, true)).toBeNull();
  expect(gap("month_saved", null)).toBeNull();
  expect(gap("budget", null)).toBeNull();
  // Under its mid level this cycle, though an earlier cycle went past it.
  expect(gap("card_under_mid", 0, false, 4)).toBeNull();
});

test("every condition code belongs to exactly one level and has its words", () => {
  const codes = LEVEL_CONDITIONS.flat();
  expect(new Set(codes).size).toBe(codes.length);
  expect(codes.sort()).toEqual(Object.keys(CONDITIONS).sort());
});
