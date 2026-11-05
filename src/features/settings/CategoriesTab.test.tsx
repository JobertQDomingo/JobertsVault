import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { CategoryList } from "@/bindings/CategoryList";
import { CATEGORIES, PAYMENT_METHODS } from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const ANSWERS: Answers = {
  list_categories: CATEGORIES,
  list_payment_methods: PAYMENT_METHODS,
  reorder_categories: null,
  archive_category: null,
  unarchive_category: null,
  delete_category: null,
  merge_categories: null,
  create_category: ({ category }: { category: object }) => ({
    ...CATEGORIES.categories[0],
    id: 9,
    ...category,
  }),
  update_category: ({ id }: { id: number }) =>
    CATEGORIES.categories.find((c) => c.id === id),
};

const calls = (command: string) =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === command)
    .map(([, args]) => args);
const section = (name: string) => within(screen.getByRole("region", { name }));
const menu = async (name: string) => {
  fireEvent.keyDown(
    await screen.findByRole("button", { name: `More actions for ${name}` }),
    { key: "Enter" },
  );
};
const item = (name: string) => screen.findByRole("menuitem", { name });
const dialog = () => within(screen.getByRole("dialog"));

const open = (answers: Answers = ANSWERS) =>
  renderRoute("/settings?tab=categories", answers);

test("expense and income categories are listed apart, archived ones folded away", async () => {
  open();
  await screen.findByRole("region", { name: "Expense" });
  const names = (kind: string) =>
    section(kind)
      .getAllByRole("button", { name: /^Move / })
      .map((b) => b.getAttribute("aria-label"));
  expect(names("Expense")).toEqual([
    "Move Food & Groceries",
    "Move Dining Out",
    "Move Transportation",
  ]);
  expect(names("Income")).toEqual(["Move Salary"]);
  expect(section("Expense").getAllByText("1 transaction")).toHaveLength(3);
  expect(screen.queryByText("Snacks")).toBeNull();
  fireEvent.click(
    section("Expense").getByRole("button", { name: "Show archived (1)" }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Unarchive Snacks" }));
  await waitFor(() => expect(calls("unarchive_category")).toEqual([{ id: 5 }]));
});

test("reordering sends only that kind's categories", async () => {
  open();
  fireEvent.keyDown(
    await screen.findByRole("button", { name: "Move Food & Groceries" }),
    { key: "ArrowDown" },
  );
  await waitFor(() =>
    expect(calls("reorder_categories")).toEqual([{ ids: [2, 1, 4] }]),
  );
});

test("a new category gets its kind, the default color and a chosen icon", async () => {
  open();
  await screen.findByRole("region", { name: "Income" });
  fireEvent.click(
    section("Income").getByRole("button", { name: "New category" }),
  );
  expect(dialog().getByRole("heading", { name: "New income category" }));
  // The kind's default color is picked to start with (DATA_MODEL §7).
  expect(
    dialog()
      .getByRole("radio", { name: "Color 1" })
      .getAttribute("aria-checked"),
  ).toBe("true");
  const name = dialog().getByRole("textbox", { name: "Name" });
  fireEvent.change(name, { target: { value: "Side gig" } });
  fireEvent.click(dialog().getByRole("radio", { name: "Color 5" }));
  fireEvent.click(dialog().getByRole("radio", { name: "laptop" }));
  fireEvent.click(dialog().getByRole("button", { name: "Create category" }));
  await waitFor(() =>
    expect(calls("create_category")).toEqual([
      {
        category: {
          kind: "income",
          name: "Side gig",
          color: "cat-5",
          icon: "laptop",
        },
      },
    ]),
  );
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
});

test("Edit changes the name, color and icon, and a backend message shows under Name", async () => {
  open({
    ...ANSWERS,
    update_category: () => {
      throw {
        code: "validation",
        field: "name",
        message: "A category with this name already exists",
      };
    },
  });
  await menu("Dining Out");
  fireEvent.click(await item("Edit"));
  const name = dialog().getByRole("textbox", {
    name: "Name",
  }) as HTMLInputElement;
  expect(name.value).toBe("Dining Out");
  expect(
    dialog()
      .getByRole("radio", { name: "utensils" })
      .getAttribute("aria-checked"),
  ).toBe("true");
  fireEvent.change(name, { target: { value: "Salary" } });
  fireEvent.click(dialog().getByRole("button", { name: "Save" }));
  expect(
    await dialog().findByText("A category with this name already exists"),
  ).toBeTruthy();
  expect(calls("update_category")).toEqual([
    { id: 2, category: { name: "Salary", color: "cat-6", icon: "utensils" } },
  ]);
});

test("Archive saves at once, and the backend's refusal is a toast", async () => {
  open({
    ...ANSWERS,
    archive_category: () => {
      throw {
        code: "conflict",
        message: "Keep at least one active income category",
      };
    },
  });
  await menu("Salary");
  fireEvent.click(await item("Archive"));
  expect(
    await screen.findByText("Keep at least one active income category"),
  ).toBeTruthy();
  expect(calls("archive_category")).toEqual([{ id: 3 }]);
});

test("Delete is only offered with no transactions, and asks first", async () => {
  const unused: CategoryList = {
    ...CATEGORIES,
    categories: CATEGORIES.categories.map((c) =>
      c.id === 4 ? { ...c, transactionCount: 0 } : c,
    ),
  };
  open({ ...ANSWERS, list_categories: unused });
  await menu("Dining Out");
  await item("Edit");
  expect(screen.queryByRole("menuitem", { name: "Delete" })).toBeNull();
  fireEvent.keyDown(document.activeElement!, { key: "Escape" });
  await menu("Transportation");
  fireEvent.click(await item("Delete"));
  const confirm = within(
    await screen.findByRole("dialog", { name: "Delete Transportation?" }),
  );
  fireEvent.click(confirm.getByRole("button", { name: "Delete" }));
  await waitFor(() => expect(calls("delete_category")).toEqual([{ id: 4 }]));
});

test("Merge offers only other active categories of the same kind, then confirms (F7)", async () => {
  open();
  await menu("Dining Out");
  fireEvent.click(await item("Merge into…"));
  fireEvent.click(dialog().getByRole("combobox", { name: "Into" }));
  expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual([
    "Food & Groceries",
    "Transportation",
  ]);
  fireEvent.click(screen.getByRole("option", { name: "Food & Groceries" }));
  fireEvent.click(dialog().getByRole("button", { name: "Merge" }));
  const confirm = await screen.findByRole("dialog", {
    name: "Merge categories?",
  });
  expect(confirm.textContent).toContain(
    "Move 1 transaction from 'Dining Out' to 'Food & Groceries' and delete 'Dining Out'?",
  );
  fireEvent.click(within(confirm).getByRole("button", { name: "Merge" }));
  await waitFor(() =>
    expect(calls("merge_categories")).toEqual([{ fromId: 2, intoId: 1 }]),
  );
  // The dialog has closed by now; the result is still shown.
  expect(await screen.findByText("Merged")).toBeTruthy();
});

test("a refused merge says why, though its dialog has closed", async () => {
  open({
    ...ANSWERS,
    merge_categories: () => {
      throw { code: "conflict", message: "Choose an active category" };
    },
  });
  await menu("Dining Out");
  fireEvent.click(await item("Merge into…"));
  fireEvent.click(dialog().getByRole("combobox", { name: "Into" }));
  fireEvent.click(screen.getByRole("option", { name: "Transportation" }));
  fireEvent.click(dialog().getByRole("button", { name: "Merge" }));
  const confirm = await screen.findByRole("dialog", {
    name: "Merge categories?",
  });
  fireEvent.click(within(confirm).getByRole("button", { name: "Merge" }));
  expect(await screen.findByText("Choose an active category")).toBeTruthy();
});
