import { fireEvent, screen } from "@testing-library/react";
import { useState } from "react";
import { expect, test, vi } from "vitest";
import { formatShortDate } from "@/lib/dates";
import { renderWithSettings } from "@/test/render";
import { AmountInput } from "./AmountInput";
import { MoneyText } from "./MoneyText";

const money = (ui: React.ReactElement, overrides = {}) => {
  const { container } = renderWithSettings(ui, overrides);
  return container.firstElementChild as HTMLElement;
};

test("each kind shows its sign and meaning color", () => {
  const cases = [
    [<MoneyText amount={123450} kind="income" />, "+₱1,234.50", "text-income"],
    [<MoneyText amount={8520} kind="expense" />, "−₱85.20", "text-expense"],
    [
      <MoneyText amount={120000} kind="transfer" />,
      "₱1,200.00",
      "text-transfer",
    ],
    [<MoneyText amount={32000} kind="balance" />, "₱320.00", "text-foreground"],
    [<MoneyText amount={-4500} kind="balance" />, "−₱45.00", "text-expense"],
    [
      <MoneyText amount={64000} kind="owed" />,
      "₱640.00 owed",
      "text-foreground",
    ],
    [<MoneyText amount={50000} kind="net" />, "+₱500.00", "text-income"],
    [<MoneyText amount={-50000} kind="net" />, "−₱500.00", "text-expense"],
  ] as const;
  for (const [ui, text, color] of cases) {
    const el = money(ui);
    expect(el.textContent).toBe(text);
    expect(el.className).toContain(color);
    expect(el.className).toContain("tabular-nums");
  }
});

test("zero is muted and unsigned", () => {
  for (const kind of ["income", "expense", "net", "balance"] as const) {
    const el = money(<MoneyText amount={0} kind={kind} />);
    expect(el.textContent).toBe("₱0.00");
    expect(el.className).toContain("text-muted-foreground");
  }
});

test("follows the currency settings", () => {
  const el = money(<MoneyText amount={1234} kind="expense" />, {
    currencySymbol: "€",
    currencyPosition: "after",
    currencyDecimals: 0,
  });
  expect(el.textContent).toBe("−1,234 €");
});

test("a transfer carries an icon, not only a color", () => {
  const el = money(<MoneyText amount={100} kind="transfer" />);
  expect(el.querySelector("svg[aria-hidden=true]")).toBeTruthy();
});

test("display size is for stat cards and details", () => {
  expect(
    money(<MoneyText amount={1} kind="net" size="display" />).className,
  ).toContain("text-display");
});

test("short dates read like Nov 5", () => {
  expect(formatShortDate("2026-11-05")).toBe("Nov 5");
});

function Amount({ onChange = vi.fn(), start = null as number | null }) {
  const [value, setValue] = useState<number | null>(start);
  return (
    <>
      <AmountInput
        aria-label="Amount"
        value={value}
        onChange={(next) => {
          setValue(next);
          onChange(next);
        }}
      />
      <button onClick={() => setValue(null)}>Reset</button>
    </>
  );
}

const input = () => screen.getByLabelText("Amount") as HTMLInputElement;
const typeText = (text: string) =>
  fireEvent.change(input(), { target: { value: text } });

test("typed text becomes centavos, with separators after leaving", () => {
  const onChange = vi.fn();
  renderWithSettings(<Amount onChange={onChange} />);
  typeText("1,234.5");
  expect(onChange).toHaveBeenLastCalledWith(123450);
  fireEvent.blur(input());
  expect(input().value).toBe("1,234.50");
  expect(input().className).toContain("text-right");
  expect(input().className).toContain("tabular-nums");
});

test("letters, minus signs, a second point and extra decimals are ignored", () => {
  const onChange = vi.fn();
  renderWithSettings(<Amount onChange={onChange} />);
  typeText("12.5");
  onChange.mockClear();
  for (const text of ["12.5a", "-12.5", "12.5.", "12.555"]) typeText(text);
  expect(onChange).not.toHaveBeenCalled();
  expect(input().value).toBe("12.5");
});

test("whole-number currencies refuse a decimal point", () => {
  const onChange = vi.fn();
  renderWithSettings(<Amount onChange={onChange} />, { currencyDecimals: 0 });
  typeText("12.");
  expect(onChange).not.toHaveBeenCalled();
  typeText("1250");
  expect(onChange).toHaveBeenLastCalledWith(1250);
});

test("pasted text with a symbol and commas is read", () => {
  const onChange = vi.fn();
  renderWithSettings(<Amount onChange={onChange} />);
  typeText("₱2,500");
  expect(onChange).toHaveBeenLastCalledWith(250000);
});

test("an empty field is no amount", () => {
  const onChange = vi.fn();
  renderWithSettings(<Amount onChange={onChange} start={500} />);
  typeText("");
  expect(onChange).toHaveBeenLastCalledWith(null);
});

test("a reset clears what was being typed", () => {
  renderWithSettings(<Amount />);
  typeText("250");
  fireEvent.click(screen.getByRole("button", { name: "Reset" }));
  expect(input().value).toBe("");
});

test("a pasted negative amount is refused, not flipped", () => {
  const onChange = vi.fn();
  renderWithSettings(<Amount onChange={onChange} />);
  for (const text of ["-500", "−1,234.50"]) typeText(text);
  expect(onChange).not.toHaveBeenCalled();
  expect(input().value).toBe("");
});
