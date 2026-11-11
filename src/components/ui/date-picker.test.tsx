import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { expect, test, vi } from "vitest";
import type { WeekStart } from "@/bindings/WeekStart";
import { DatePicker } from "./date-picker";

function Picker({ onChange = vi.fn(), weekStart = "monday" as WeekStart }) {
  const [value, setValue] = useState("2026-10-26");
  return (
    <DatePicker
      aria-label="Date"
      value={value}
      weekStart={weekStart}
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

test("up and down move one day", () => {
  const onChange = vi.fn();
  render(<Picker onChange={onChange} />);
  fireEvent.keyDown(input(), { key: "ArrowDown" });
  expect(onChange).toHaveBeenLastCalledWith("2026-10-25");
  fireEvent.keyDown(input(), { key: "ArrowUp" });
  fireEvent.keyDown(input(), { key: "ArrowUp" });
  expect(onChange).toHaveBeenLastCalledWith("2026-10-27");
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

test("a date that can't be read is passed on as typed, for the backend to explain", () => {
  const onChange = vi.fn();
  render(<Picker onChange={onChange} />);
  fireEvent.change(input(), { target: { value: "02/30/2026" } });
  fireEvent.blur(input());
  expect(onChange).toHaveBeenCalledWith("02/30/2026");
  expect((input() as HTMLInputElement).value).toBe("02/30/2026");
  onChange.mockClear();
  fireEvent.keyDown(input(), { key: "ArrowUp" });
  expect(onChange).not.toHaveBeenCalled();
});

test("Enter reads the typed date before the form saves", () => {
  const onChange = vi.fn();
  render(<Picker onChange={onChange} />);
  fireEvent.change(input(), { target: { value: "10/5/2026" } });
  const notCancelled = fireEvent.keyDown(input(), { key: "Enter" });
  expect(onChange).toHaveBeenCalledWith("2026-10-05");
  expect(notCancelled).toBe(true);
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

test("picking a day passes it on and closes the calendar", () => {
  const onChange = vi.fn();
  render(<Picker onChange={onChange} />);
  fireEvent.click(screen.getByRole("button", { name: "Choose date" }));
  fireEvent.click(screen.getByRole("button", { name: /October 20(th)?,/ }));
  expect(onChange).toHaveBeenCalledWith("2026-10-20");
  expect(screen.queryByRole("grid")).toBeNull();
});
