import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import { BUDGETS, CATEGORIES } from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const ANSWERS: Answers = {
  list_budgets: BUDGETS,
  list_categories: CATEGORIES,
  create_budget: null,
  update_budget: null,
  delete_budget: null,
};

const calls = (command: string) =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === command)
    .map(([, args]) => args);
const dialog = () => within(screen.getByRole("dialog"));
/** The tile whose edit button is named `name`. */
const tile = async (name: string) =>
  within(
    (await screen.findByRole("button", { name })).closest<HTMLElement>("li")!,
  );

test("each budget shows its period, % used, and spent of amount with what's left (BUD-2)", async () => {
  renderRoute("/budgets", ANSWERS);
  const all = await tile("Edit All spending, monthly");
  all.getByText("Monthly");
  expect(all.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("84");
  all.getByText("84%");
  all.getByText("₱2,950.40 of ₱3,500.00 · ₱549.60 left");
  const food = await tile("Edit Food & Groceries, monthly");
  food.getByText("86%");
  food.getByText("₱520.40 of ₱600.00 · ₱79.60 left");
  // The list keeps the backend's order: all spending, then the categories' order.
  expect(
    screen
      .getAllByRole("button", { name: /^Edit / })
      .map((b) => b.getAttribute("aria-label")),
  ).toEqual([
    "Edit All spending, monthly",
    "Edit Food & Groceries, monthly",
    "Edit Dining Out, weekly",
  ]);
});

test("past its limit, a budget says how much over (BUD-3)", async () => {
  renderRoute("/budgets", ANSWERS);
  const dining = await tile("Edit Dining Out, weekly");
  dining.getByText("120%");
  dining.getByText("₱180.00 of ₱150.00 · ₱30.00 over");
});

test("with no budgets, the empty state offers New budget", async () => {
  renderRoute("/budgets", { ...ANSWERS, list_budgets: [] });
  await screen.findByText("No budgets yet.");
  // The header's button and the empty state's.
  expect(screen.getAllByRole("button", { name: "New budget" })).toHaveLength(2);
});

test("New budget starts on all spending, monthly, and offers only active expense categories (BUD-1)", async () => {
  renderRoute("/budgets", ANSWERS);
  fireEvent.click(
    (await screen.findAllByRole("button", { name: "New budget" }))[0],
  );
  expect(dialog().getByRole("heading", { name: "New budget" }));
  const category = dialog().getByRole("combobox", { name: "Category" });
  expect((category as HTMLInputElement).value).toBe("All spending");
  fireEvent.click(category);
  expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual([
    "All spending",
    "Food & Groceries",
    "Dining Out",
    "Transportation",
  ]);
  fireEvent.click(screen.getByRole("option", { name: "Transportation" }));
  fireEvent.click(dialog().getByRole("radio", { name: "Weekly" }));
  fireEvent.change(dialog().getByRole("textbox", { name: "Amount" }), {
    target: { value: "1,500" },
  });
  fireEvent.click(dialog().getByRole("button", { name: "Create budget" }));
  await waitFor(() =>
    expect(calls("create_budget")).toEqual([
      { budget: { categoryId: 4, period: "weekly", amount: 150000 } },
    ]),
  );
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
});

test("the backend's message shows under the field it names", async () => {
  renderRoute("/budgets", {
    ...ANSWERS,
    create_budget: () => {
      throw {
        code: "validation",
        field: "period",
        message: "There's already a monthly budget for all spending",
      };
    },
  });
  fireEvent.click(
    (await screen.findAllByRole("button", { name: "New budget" }))[0],
  );
  fireEvent.change(dialog().getByRole("textbox", { name: "Amount" }), {
    target: { value: "100" },
  });
  fireEvent.click(dialog().getByRole("button", { name: "Create budget" }));
  await dialog().findByText(
    "There's already a monthly budget for all spending",
  );
});

test("clicking a budget edits it, keeping its archived category", async () => {
  const snacks = { ...BUDGETS[1], id: 7, categoryId: 5, period: "yearly" };
  renderRoute("/budgets", {
    ...ANSWERS,
    list_budgets: [...BUDGETS, snacks],
  });
  fireEvent.click(
    await screen.findByRole("button", { name: "Edit Snacks, yearly" }),
  );
  expect(dialog().getByRole("heading", { name: "Edit budget" }));
  const category = dialog().getByRole("combobox", { name: "Category" });
  expect((category as HTMLInputElement).value).toBe("Snacks (archived)");
  expect(
    dialog()
      .getByRole("radio", { name: "Yearly" })
      .getAttribute("aria-checked"),
  ).toBe("true");
  fireEvent.change(dialog().getByRole("textbox", { name: "Amount" }), {
    target: { value: "700" },
  });
  fireEvent.click(dialog().getByRole("button", { name: "Save" }));
  await waitFor(() =>
    expect(calls("update_budget")).toEqual([
      {
        id: 7,
        budget: { categoryId: 5, period: "yearly", amount: 70000 },
      },
    ]),
  );
});

test("Delete asks first", async () => {
  renderRoute("/budgets", ANSWERS);
  fireEvent.keyDown(
    await screen.findByRole("button", {
      name: "More actions for Dining Out, weekly",
    }),
    { key: "Enter" },
  );
  fireEvent.click(await screen.findByRole("menuitem", { name: "Delete" }));
  const confirm = within(
    await screen.findByRole("dialog", { name: "Delete this budget?" }),
  );
  fireEvent.click(confirm.getByRole("button", { name: "Delete" }));
  await waitFor(() => expect(calls("delete_budget")).toEqual([{ id: 2 }]));
});
