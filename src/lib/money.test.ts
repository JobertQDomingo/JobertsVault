import { describe, expect, test } from "vitest";
import {
  formatAmount,
  formatMoney,
  MAX_AMOUNT,
  parseAmount,
  type MoneySettings,
} from "./money";

const peso: MoneySettings = {
  currencySymbol: "₱",
  currencyPosition: "before",
  currencyDecimals: 2,
};

describe("parseAmount", () => {
  test("reads typed and pasted amounts", () => {
    expect(parseAmount("1,234.5", 2)).toBe(123450);
    expect(parseAmount("1234.50", 2)).toBe(123450);
    expect(parseAmount(" 12 ", 2)).toBe(1200);
    expect(parseAmount(".5", 2)).toBe(50);
    expect(parseAmount("12.", 2)).toBe(1200);
    expect(parseAmount("0", 2)).toBe(0);
    expect(parseAmount("1,234", 0)).toBe(1234);
  });

  test("reads up to the largest amount", () => {
    expect(parseAmount("999,999,999,999", 0)).toBe(MAX_AMOUNT);
    expect(parseAmount("9,999,999,999.99", 2)).toBe(MAX_AMOUNT);
    expect(parseAmount("10,000,000,000", 2)).toBeNull();
    // Past 2^53 a JS number can't hold it exactly, so it must be refused, not rounded.
    expect(parseAmount("99999999999999999999", 0)).toBeNull();
  });

  test.each(["", ".", "-5", "+5", "1.2.3", "₱5", "1e3", "12.3,4"])(
    "rejects %j",
    (text) => {
      expect(parseAmount(text, 2)).toBeNull();
    },
  );

  test.each(["12,50", "1,2345", "1,2,3", ",5", "1234,567", "1,"])(
    "takes a comma only as a thousands separator, so rejects %j",
    (text) => {
      expect(parseAmount(text, 2)).toBeNull();
    },
  );

  test("reads amounts grouped in thousands", () => {
    expect(parseAmount("12,345,678.90", 2)).toBe(1234567890);
  });

  test("never rounds extra fraction digits", () => {
    expect(parseAmount("12.5", 0)).toBeNull();
    expect(parseAmount("1.234", 2)).toBeNull();
  });
});

describe("formatAmount", () => {
  test("adds thousands separators and the decimals", () => {
    expect(formatAmount(123450, 2)).toBe("1,234.50");
    expect(formatAmount(0, 2)).toBe("0.00");
    expect(formatAmount(5, 2)).toBe("0.05");
    expect(formatAmount(1234, 0)).toBe("1,234");
    expect(formatAmount(MAX_AMOUNT, 2)).toBe("9,999,999,999.99");
  });

  test("marks negatives with a true minus sign", () => {
    expect(formatAmount(-123450, 2)).toBe("−1,234.50");
  });
});

describe("formatMoney", () => {
  test("puts the symbol before the amount and the minus before the symbol", () => {
    expect(formatMoney(123450, peso)).toBe("₱1,234.50");
    expect(formatMoney(-123450, peso)).toBe("−₱1,234.50");
    expect(formatMoney(0, peso)).toBe("₱0.00");
  });

  test("adds a plus only when asked, and never to zero", () => {
    expect(formatMoney(50000, peso, { plus: true })).toBe("+₱500.00");
    expect(formatMoney(0, peso, { plus: true })).toBe("₱0.00");
    expect(formatMoney(-50000, peso, { plus: true })).toBe("−₱500.00");
  });

  test("puts the symbol after the amount when set", () => {
    const after: MoneySettings = { ...peso, currencyPosition: "after" };
    expect(formatMoney(123450, after)).toBe("1,234.50\u00A0₱");
    expect(formatMoney(-123450, after)).toBe("−1,234.50\u00A0₱");
  });

  test("shows no decimals when the currency has none", () => {
    expect(formatMoney(1234, { ...peso, currencyDecimals: 0 })).toBe("₱1,234");
  });
});
