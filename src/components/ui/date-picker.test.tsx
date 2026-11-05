import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { expect, test, vi } from "vitest";
import type { WeekStart } from "@/bindings/WeekStart";
import { DatePicker } from "./date-picker";

function Picker({
  onChange = vi.fn(),
  weekStart = "monday" as WeekStart,
  max = "2026-10-27",
}) {
  const [value, setValue] = useState("2026-10-26");
  return (
    <DatePicker
      aria-label="Date"
      value={value}
      weekStart={weekStart}
      max={max}
      onChange={(next) => {
        setValue(next);
        onChange(next);
      }}
    />
  );
}

const input = () => screen.getByRole("textbox", { name: "Date" });

test("shows the date in US order", () => {
  render(<Picker />);
  expect((input() as HTMLInputElement).value).toBe("10/26/2026");
});

test("up and down move one day, and up stops at the latest allowed day", () => {
  const onChange = vi.fn();
  render(<Picker onChange={onChange} />);
  fireEvent.keyDown(input(), { key: "ArrowDown" });
  expect(onChange).toHaveBeenLastCalledWith("2026-10-25");
  fireEvent.keyDown(input(), { key: "ArrowUp" });
  fireEvent.keyDown(input(), { key: "ArrowUp" });
  expect(onChange).toHaveBeenLastCalledWith("2026-10-27");
  onChange.mockClear();
  fireEvent.keyDown(input(), { key: "ArrowUp" });
  expect(onChange).not.toHaveBeenCalled();
  expect((input() as HTMLInputElement).value).toBe("10/27/2026");
});

test("a typed date is read when you leave the field", () => {
  const onChange = vi.fn();
  render(<Picker onChange={onChange} />);
  fireEvent.change(input(), { target: { value: "10/5/2026" } });
  fireEvent.blur(input());
  expect(onChange).toHaveBeenCalledWith("2026-10-05");
  expect((input() as HTMLInputElement).value).toBe("10/05/2026");
});

test("an impossible date is dropped and the field shows the date again", () => {
  const onChange = vi.fn();
  render(<Picker onChange={onChange} />);
  fireEvent.change(input(), { target: { value: "02/30/2026" } });
  fireEvent.blur(input());
  expect(onChange).not.toHaveBeenCalled();
  expect((input() as HTMLInputElement).value).toBe("10/26/2026");
});

test("the calendar starts the week on the week start setting", () => {
  const { unmount } = render(<Picker weekStart="monday" />);
  fireEvent.click(screen.getByRole("button", { name: "Choose date" }));
  expect(
    screen
      .getAllByRole("columnheader", { hidden: true })[0]
      .getAttribute("aria-label"),
  ).toBe("Monday");
  unmount();
  render(<Picker weekStart="sunday" />);
  fireEvent.click(screen.getByRole("button", { name: "Choose date" }));
  expect(
    screen
      .getAllByRole("columnheader", { hidden: true })[0]
      .getAttribute("aria-label"),
  ).toBe("Sunday");
});

test("days after the latest allowed day can't be picked, earlier ones can", () => {
  const onChange = vi.fn();
  render(<Picker onChange={onChange} />);
  fireEvent.click(screen.getByRole("button", { name: "Choose date" }));
  const day = (n: number) =>
    screen.getByRole("button", {
      name: new RegExp(`October ${n}(st|nd|rd|th)?,`),
    });
  expect((day(28) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(day(20));
  expect(onChange).toHaveBeenCalledWith("2026-10-20");
  expect(screen.queryByRole("grid")).toBeNull();
});

test("next month looks disabled at the latest allowed month", () => {
  render(<Picker />);
  fireEvent.click(screen.getByRole("button", { name: "Choose date" }));
  const next = screen.getByRole("button", { name: /next month/i });
  // react-day-picker marks it aria-disabled, not disabled.
  expect(next.getAttribute("aria-disabled")).toBe("true");
  expect(next.className).toContain("aria-disabled:opacity-40");
});
