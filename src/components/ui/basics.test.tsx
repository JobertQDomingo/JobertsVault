import { fireEvent, render, screen } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { Button } from "./button";
import { Field, Input } from "./input";
import { Segmented } from "./segmented";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./tabs";

test("a button is a secondary type=button by default", () => {
  render(<Button>Cancel</Button>);
  const button = screen.getByRole("button", { name: "Cancel" });
  expect(button.getAttribute("type")).toBe("button");
  expect(button.className).toContain("bg-field");
});

test("a primary button is solid accent", () => {
  render(<Button variant="primary">Save</Button>);
  expect(screen.getByRole("button", { name: "Save" }).className).toContain(
    "bg-primary",
  );
});

test("asChild styles the child instead", () => {
  render(
    <Button asChild variant="ghost">
      <a href="/transactions">View all</a>
    </Button>,
  );
  expect(screen.getByRole("link", { name: "View all" }).className).toContain(
    "hover:bg-wash",
  );
});

test("a field shows its error under the input and links them", () => {
  render(
    <Field label="Amount" htmlFor="amount" error="Amount must be more than 0">
      <Input id="amount" aria-invalid />
    </Field>,
  );
  const input = screen.getByLabelText("Amount");
  const error = screen.getByRole("alert");
  expect(error.textContent).toBe("Amount must be more than 0");
  expect(input.getAttribute("aria-describedby")).toBe(error.id);
});

test("clicking a tab shows its panel", () => {
  render(
    <Tabs defaultValue="general">
      <TabsList>
        <TabsTrigger value="general">General</TabsTrigger>
        <TabsTrigger value="categories">Categories</TabsTrigger>
      </TabsList>
      <TabsContent value="general">Currency</TabsContent>
      <TabsContent value="categories">Expense categories</TabsContent>
    </Tabs>,
  );
  fireEvent.mouseDown(screen.getByRole("tab", { name: "Categories" }));
  expect(screen.getByRole("tabpanel").textContent).toBe("Expense categories");
});

test("a segmented control changes but never empties", () => {
  const onChange = vi.fn();
  render(
    <Segmented
      aria-label="Type"
      value="expense"
      onChange={onChange}
      options={[
        { value: "expense", label: "Expense" },
        { value: "income", label: "Income" },
      ]}
    />,
  );
  fireEvent.click(screen.getByRole("radio", { name: "Expense" }));
  expect(onChange).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("radio", { name: "Income" }));
  expect(onChange).toHaveBeenCalledWith("income");
});
