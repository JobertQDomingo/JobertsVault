import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { SortableList } from "./SortableList";

const ITEMS = [
  { id: 1, name: "One" },
  { id: 2, name: "Two" },
  { id: 3, name: "Three" },
];

function renderList(
  onReorder: (ids: number[]) => Promise<unknown> = vi.fn(() =>
    Promise.resolve(),
  ),
) {
  render(
    <SortableList
      items={ITEMS}
      label={(item) => item.name}
      onReorder={onReorder}
    >
      {(item, handle) => (
        <div data-testid="row">
          {handle}
          {item.name}
        </div>
      )}
    </SortableList>,
  );
  return onReorder;
}

const order = () => screen.getAllByTestId("row").map((r) => r.textContent);

test("ArrowDown on a handle moves the item down one and reports the new order", async () => {
  const onReorder = renderList();
  const handle = screen.getByRole("button", { name: "Move One" });
  handle.focus();
  fireEvent.keyDown(handle, { key: "ArrowDown" });
  expect(onReorder).toHaveBeenCalledWith([2, 1, 3]);
  expect(order()).toEqual(["Two", "One", "Three"]);
  await waitFor(() =>
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Move One" }),
    ),
  );
});

test("ArrowUp moves it up, and the ends don't move", () => {
  const onReorder = renderList();
  fireEvent.keyDown(screen.getByRole("button", { name: "Move One" }), {
    key: "ArrowUp",
  });
  fireEvent.keyDown(screen.getByRole("button", { name: "Move Three" }), {
    key: "ArrowDown",
  });
  expect(onReorder).not.toHaveBeenCalled();
  fireEvent.keyDown(screen.getByRole("button", { name: "Move Three" }), {
    key: "ArrowUp",
  });
  expect(onReorder).toHaveBeenCalledWith([1, 3, 2]);
});

test("a failed save puts the old order back", async () => {
  renderList(vi.fn(() => Promise.reject(new Error("no"))));
  fireEvent.keyDown(screen.getByRole("button", { name: "Move One" }), {
    key: "ArrowDown",
  });
  await waitFor(() => expect(order()).toEqual(["One", "Two", "Three"]));
});

test("the new order stays while the same items arrive again", () => {
  const onReorder = vi.fn(() => new Promise(() => {}));
  const ui = (items: typeof ITEMS) => (
    <SortableList items={items} label={(i) => i.name} onReorder={onReorder}>
      {(item, handle) => (
        <div data-testid="row">
          {handle}
          {item.name}
        </div>
      )}
    </SortableList>
  );
  const { rerender } = render(ui(ITEMS));
  fireEvent.keyDown(screen.getByRole("button", { name: "Move One" }), {
    key: "ArrowDown",
  });
  rerender(ui([...ITEMS]));
  expect(order()).toEqual(["Two", "One", "Three"]);
  rerender(ui([ITEMS[1], ITEMS[0], ITEMS[2]]));
  expect(order()).toEqual(["Two", "One", "Three"]);
});

test("moves made while a save runs wait for it, then go as one save, and the list never jumps back", async () => {
  const saves: { ids: number[]; done: () => void }[] = [];
  const onReorder = vi.fn(
    (ids: number[]) =>
      new Promise<void>((resolve) => saves.push({ ids, done: resolve })),
  );
  renderList(onReorder);
  const down = () =>
    fireEvent.keyDown(screen.getByRole("button", { name: "Move One" }), {
      key: "ArrowDown",
    });
  down();
  down();
  fireEvent.keyDown(screen.getByRole("button", { name: "Move One" }), {
    key: "ArrowUp",
  });
  down();
  expect(saves.map((s) => s.ids)).toEqual([[2, 1, 3]]);
  expect(order()).toEqual(["Two", "Three", "One"]);
  saves[0].done();
  await waitFor(() => expect(saves).toHaveLength(2));
  expect(saves[1].ids).toEqual([2, 3, 1]);
  expect(order()).toEqual(["Two", "Three", "One"]);
});

test("the handle doesn't announce dnd-kit's Space instructions, which it doesn't use", () => {
  renderList();
  const handle = screen.getByRole("button", { name: "Move One" });
  expect(handle.getAttribute("aria-describedby")).toBeNull();
  expect(handle.getAttribute("aria-roledescription")).toBeNull();
});
