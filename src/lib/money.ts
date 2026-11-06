// The only code that converts between text and minor units (ARCHITECTURE §5.3). It works on
// digit strings, never floats, so amounts stay exact (ADR 0002).

import type { Settings } from "@/bindings/Settings";

export type Decimals = Settings["currencyDecimals"];

/** The money settings (DATA_MODEL §3.5), as `get_settings` returns them. */
export type MoneySettings = Pick<
  Settings,
  "currencySymbol" | "currencyPosition" | "currencyDecimals"
>;

/** The largest amount any field accepts (DATA_MODEL §6). */
export const MAX_AMOUNT = 999_999_999_999;

const MINUS = "\u2212";

/**
 * Reads typed or pasted text: digits, one ".", and commas as thousands separators (DESIGN_SYSTEM §7).
 * Anything else, or more than MAX_AMOUNT, is null. Never rounds.
 */
export function parseAmount(text: string, decimals: Decimals): number | null {
  // The whole part is plain digits or grouped in threes ("12,345"), so "12,50" is refused.
  const match = /^(\d*|\d{1,3}(?:,\d{3})+)(?:\.(\d*))?$/.exec(text.trim());
  if (!match) return null;
  const whole = match[1].replace(/,/g, "");
  const fraction = match[2] ?? "";
  if ((whole === "" && fraction === "") || fraction.length > decimals)
    return null;
  const digits = (whole + fraction.padEnd(decimals, "0")).replace(
    /^0+(?=\d)/,
    "",
  );
  // More digits than MAX_AMOUNT has can't be valid, and could lose precision as a number.
  if (digits.length > String(MAX_AMOUNT).length) return null;
  const minor = Number.parseInt(digits, 10);
  return minor <= MAX_AMOUNT ? minor : null;
}

/** The number alone, e.g. "1,234.50" or "−1,234.50": what the amount input shows. */
export function formatAmount(minor: number, decimals: Decimals): string {
  const digits = String(Math.abs(minor)).padStart(decimals + 1, "0");
  const whole = digits
    .slice(0, digits.length - decimals)
    .replace(/\B(?=(\d{3})+$)/g, ",");
  const fraction = decimals > 0 ? `.${digits.slice(-decimals)}` : "";
  return `${minor < 0 ? MINUS : ""}${whole}${fraction}`;
}

/** An amount with its symbol (DESIGN_SYSTEM §7). `plus` marks gains, such as income, with "+". */
export function formatMoney(
  minor: number,
  settings: MoneySettings,
  options: { plus?: boolean } = {},
): string {
  const amount = formatAmount(Math.abs(minor), settings.currencyDecimals);
  const sign = minor < 0 ? MINUS : options.plus && minor > 0 ? "+" : "";
  return settings.currencyPosition === "before"
    ? `${sign}${settings.currencySymbol}${amount}`
    : `${sign}${amount}\u00A0${settings.currencySymbol}`;
}

/**
 * A chart axis label (DESIGN_SYSTEM §7): `₱850`, `₱12.5k`, `₱1.2M`, `−₱5k`. One decimal at most,
 * rounded half up, using whole-number maths only.
 */
export function formatCompact(minor: number, settings: MoneySettings): string {
  const unit = 10 ** settings.currencyDecimals;
  const abs = Math.abs(minor);
  const [size, suffix] =
    abs >= unit * 1_000_000
      ? [unit * 1_000_000, "M"]
      : abs >= unit * 1_000
        ? [unit * 1_000, "k"]
        : [unit, ""];
  const scaled = abs * 10;
  const rest = scaled % size;
  const tenths = (scaled - rest) / size + (2 * rest >= size ? 1 : 0);
  const fraction = tenths % 10;
  const amount = `${formatAmount((tenths - fraction) / 10, 0)}${fraction ? `.${fraction}` : ""}${suffix}`;
  const sign = minor < 0 ? MINUS : "";
  return settings.currencyPosition === "before"
    ? `${sign}${settings.currencySymbol}${amount}`
    : `${sign}${amount}\u00A0${settings.currencySymbol}`;
}
