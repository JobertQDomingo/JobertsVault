import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import { CATEGORIES, PAYMENT_METHODS } from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

// Cash (used), Credit Card (used), Old card (archived); a fourth, unused one for Delete.
const METHODS = [
  ...PAYMENT_METHODS,
  { id: 4, name: "GCash", sortOrder: 4, archived: false, inUse: false },
];
const ANSWERS: Answers = {
  list_categories: CATEGORIES,
  list_payment_methods: METHODS,
  reorder_payment_methods: null,
  archive_payment_method: null,
  unarchive_payment_method: null,
  delete_payment_method: null,
  create_payment_method: ({ name }: { name: string }) => ({
    id: 9,
    name,
    sortOrder: 9,
    archived: false,
    inUse: false,
  }),
  update_payment_method: ({ id, name }: { id: number; name: string }) => ({
    ...METHODS.find((m) => m.id === id),
    name,
  }),
};

const calls = (command: string) =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === command)
    .map(([, args]) => args);
const menu = async (name: string) =>
  fireEvent.keyDown(
    await screen.findByRole("button", { name: `More actions for ${name}` }),
    { key: "Enter" },
  );
const item = (name: string) => screen.findByRole("menuitem", { name });
const dialog = () => within(screen.getByRole("dialog"));
const open = (answers: Answers = ANSWERS) =>
  renderRoute("/settings?tab=methods", answers);

test("lists the active methods in order, reorders, and unarchives", async () => {
  open();
  fireEvent.keyDown(await screen.findByRole("button", { name: "Move Cash" }), {
    key: "ArrowDown",
  });
  await waitFor(() =>
    expect(calls("reorder_payment_methods")).toEqual([{ ids: [2, 1, 4] }]),
  );
  expect(screen.queryByText("Old card")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Show archived (1)" }));
  fireEvent.click(screen.getByRole("button", { name: "Unarchive Old card" }));
  await waitFor(() =>
    expect(calls("unarchive_payment_method")).toEqual([{ id: 3 }]),
  );
});

test("New method saves its name, and the backend's message shows under it", async () => {
  open({
    ...ANSWERS,
    create_payment_method: ({ name }: { name: string }) => {
      if (name === "Cash")
        throw {
          code: "validation",
          field: "name",
          message: "A payment method with this name already exists",
        };
      return { id: 9, name, sortOrder: 9, archived: false, inUse: false };
    },
  });
  fireEvent.click(await screen.findByRole("button", { name: "New method" }));
  const name = dialog().getByRole("textbox", { name: "Name" });
  fireEvent.change(name, { target: { value: "Cash" } });
  fireEvent.click(dialog().getByRole("button", { name: "Create method" }));
  expect(
    await dialog().findByText("A payment method with this name already exists"),
  ).toBeTruthy();
  fireEvent.change(name, { target: { value: "Maya" } });
  fireEvent.click(dialog().getByRole("button", { name: "Create method" }));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(calls("create_payment_method")).toEqual([
    { name: "Cash" },
    { name: "Maya" },
  ]);
});

test("Rename and Archive", async () => {
  open();
  await menu("Credit Card");
  fireEvent.click(await item("Rename"));
  const name = dialog().getByRole("textbox", {
    name: "Name",
  }) as HTMLInputElement;
  expect(name.value).toBe("Credit Card");
  fireEvent.change(name, { target: { value: "Visa" } });
  fireEvent.click(dialog().getByRole("button", { name: "Save" }));
  await waitFor(() =>
    expect(calls("update_payment_method")).toEqual([{ id: 2, name: "Visa" }]),
  );
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  await menu("Credit Card");
  fireEvent.click(await item("Archive"));
  await waitFor(() =>
    expect(calls("archive_payment_method")).toEqual([{ id: 2 }]),
  );
});

test("Delete is only offered while unused, and asks first", async () => {
  open();
  await menu("Cash");
  await item("Rename");
  expect(screen.queryByRole("menuitem", { name: "Delete" })).toBeNull();
  fireEvent.keyDown(document.activeElement!, { key: "Escape" });
  await menu("GCash");
  fireEvent.click(await item("Delete"));
  const confirm = within(
    await screen.findByRole("dialog", { name: "Delete GCash?" }),
  );
  fireEvent.click(confirm.getByRole("button", { name: "Delete" }));
  await waitFor(() =>
    expect(calls("delete_payment_method")).toEqual([{ id: 4 }]),
  );
});
