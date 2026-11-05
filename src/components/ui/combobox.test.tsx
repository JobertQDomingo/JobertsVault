import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { expect, test, vi } from "vitest";
import { Combobox } from "./combobox";
import { Dialog, DialogContent } from "./dialog";

const CATEGORIES = [
  { value: "1", label: "Food & Groceries" },
  { value: "2", label: "Dining Out" },
  { value: "3", label: "Transportation" },
  { value: "4", label: "Digital Subscriptions" },
];

function Form({ onSubmit = vi.fn(), onPick = vi.fn() }) {
  const [value, setValue] = useState<string | null>(null);
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit();
      }}
    >
      <Combobox
        aria-label="Category"
        options={CATEGORIES}
        value={value}
        onChange={(next) => {
          setValue(next);
          onPick(next);
        }}
      />
    </form>
  );
}

const input = () => screen.getByRole("combobox", { name: "Category" });
const type = (text: string) =>
  fireEvent.change(input(), { target: { value: text } });
const key = (k: string) => fireEvent.keyDown(input(), { key: k });

test("typing filters, Enter picks the best match without saving", () => {
  const onSubmit = vi.fn();
  const onPick = vi.fn();
  render(<Form onSubmit={onSubmit} onPick={onPick} />);
  type("din");
  const options = screen.getAllByRole("option");
  expect(options[0].textContent).toBe("Dining Out");
  key("Enter");
  expect(onPick).toHaveBeenCalledWith("2");
  expect(onSubmit).not.toHaveBeenCalled();
  expect(screen.queryByRole("listbox")).toBeNull();
  expect((input() as HTMLInputElement).value).toBe("Dining Out");
});

test("Enter on a closed combobox is left to the form, Shift included", () => {
  const onFormKey = vi.fn();
  render(
    <div onKeyDown={(e) => onFormKey(e.key, e.shiftKey)}>
      <Form />
    </div>,
  );
  // Not cancelled, so the browser submits the form.
  expect(fireEvent.keyDown(input(), { key: "Enter" })).toBe(true);
  fireEvent.keyDown(input(), { key: "Enter", shiftKey: true });
  expect(onFormKey).toHaveBeenLastCalledWith("Enter", true);
});

test("shortcuts still reach the window from a closed combobox", () => {
  const onShortcut = vi.fn();
  window.addEventListener("keydown", onShortcut);
  render(<Form />);
  fireEvent.keyDown(input(), { key: "n", ctrlKey: true });
  window.removeEventListener("keydown", onShortcut);
  expect(onShortcut).toHaveBeenCalledOnce();
  expect(fireEvent.keyDown(input(), { key: "Home" })).toBe(true);
});

test("opening the list starts on the current choice", () => {
  const onPick = vi.fn();
  render(<Form onPick={onPick} />);
  fireEvent.click(input());
  fireEvent.click(screen.getByRole("option", { name: "Transportation" }));
  key("ArrowDown");
  key("Enter");
  expect(onPick).toHaveBeenLastCalledWith("3");
});

test("names that start with what you typed come first", () => {
  render(<Form />);
  type("d");
  const options = screen.getAllByRole("option");
  expect(options.map((o) => o.textContent)).toEqual([
    "Dining Out",
    "Digital Subscriptions",
    "Food & Groceries",
  ]);
  expect(options[0].getAttribute("aria-selected")).toBe("true");
  expect(input().getAttribute("aria-activedescendant")).toBe(options[0].id);
});

test("arrow keys move the highlight", () => {
  const onPick = vi.fn();
  render(<Form onPick={onPick} />);
  type("o");
  key("ArrowDown");
  const highlighted = screen
    .getAllByRole("option")
    .find((o) => o.getAttribute("aria-selected") === "true");
  key("Enter");
  expect(onPick).toHaveBeenCalledWith(
    CATEGORIES.find((c) => c.label === highlighted?.textContent)?.value,
  );
  expect(screen.queryAllByRole("option").length).toBe(0);
});

test("clicking an option picks it", () => {
  const onPick = vi.fn();
  render(<Form onPick={onPick} />);
  fireEvent.click(input());
  fireEvent.click(screen.getByRole("option", { name: "Transportation" }));
  expect(onPick).toHaveBeenCalledWith("3");
});

test("Esc inside a dialog closes only the list", () => {
  const onOpenChange = vi.fn();
  render(
    <Dialog open onOpenChange={onOpenChange}>
      <DialogContent title="Add transaction">
        <Form />
      </DialogContent>
    </Dialog>,
  );
  type("din");
  expect(screen.getByRole("listbox")).toBeTruthy();
  key("Escape");
  expect(screen.queryByRole("listbox")).toBeNull();
  expect(onOpenChange).not.toHaveBeenCalled();
});

test("after a pick the name is selected, so typing starts a new search", () => {
  render(<Form />);
  input().focus();
  type("din");
  key("Enter");
  const field = input() as HTMLInputElement;
  expect([field.selectionStart, field.selectionEnd]).toEqual([
    0,
    "Dining Out".length,
  ]);
});
