import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import { ACCOUNT_LIST, GOALS } from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const ANSWERS: Answers = {
  list_goals: GOALS,
  list_accounts: ACCOUNT_LIST,
  create_goal: null,
  update_goal: null,
  archive_goal: null,
  unarchive_goal: null,
  delete_goal: null,
};

const calls = (command: string) =>
  vi
    .mocked(invoke)
    .mock.calls.filter(([c]) => c === command)
    .map(([, args]) => args);
const dialog = () => within(screen.getByRole("dialog"));
/** The card whose edit button is named `name`. */
const card = async (name: string) =>
  within(
    (await screen.findByRole("button", { name })).closest<HTMLElement>("li")!,
  );
const openMenu = async (goal: string) =>
  fireEvent.keyDown(
    await screen.findByRole("button", { name: `More actions for ${goal}` }),
    { key: "Enter" },
  );

test("each goal shows its account, % done, what's left, the target date and the monthly amount (GOL-3)", async () => {
  renderRoute("/goals", ANSWERS);
  const trip = await card("Edit Japan trip");
  trip.getByText("Bank · by May 5, 2027");
  expect(trip.getByRole("progressbar").getAttribute("aria-valuenow")).toBe(
    "84",
  );
  trip.getByText("₱8,410.75 of ₱10,000.00 · ₱1,589.25 left");
  trip.getByText("Save ₱264.88/month to reach it");
  expect(trip.queryByText("Achieved")).toBeNull();
  // Archived goals are folded away, and the rest keep the backend's order.
  expect(
    screen
      .getAllByRole("button", { name: /^Edit / })
      .map((b) => b.getAttribute("aria-label")),
  ).toEqual(["Edit Japan trip", "Edit New laptop", "Edit Car"]);
});

test("an achieved goal gets the badge and no monthly amount; a passed date says so (GOL-4)", async () => {
  renderRoute("/goals", ANSWERS);
  const laptop = await card("Edit New laptop");
  laptop.getByText("Achieved");
  laptop.getByText("₱320.00 of ₱300.00 · ₱0.00 left");
  expect(laptop.queryByText(/month/)).toBeNull();
  const car = await card("Edit Car");
  car.getByText("Target date passed");
  expect(car.queryByText("Achieved")).toBeNull();
});

test("a goal without a target date names only its account", async () => {
  renderRoute("/goals", {
    ...ANSWERS,
    list_goals: [
      { ...GOALS[0], targetDate: null, monthsLeft: null, monthlyNeeded: null },
    ],
  });
  const trip = await card("Edit Japan trip");
  trip.getByText("Bank");
  expect(trip.queryByText(/month|passed/)).toBeNull();
});

test("with no goals, the empty state offers New goal", async () => {
  renderRoute("/goals", { ...ANSWERS, list_goals: [] });
  await screen.findByText("No goals yet.");
  expect(screen.getAllByRole("button", { name: "New goal" })).toHaveLength(2);
});

test("with only archived goals, the empty state says none are active", async () => {
  renderRoute("/goals", { ...ANSWERS, list_goals: [GOALS[3]] });
  await screen.findByText("No active goals.");
  screen.getByRole("button", { name: "Show archived (1)" });
});

test("New goal offers only active accounts that aren't cards, and the date is optional (GOL-1, GOL-2)", async () => {
  renderRoute("/goals", ANSWERS);
  fireEvent.click(
    (await screen.findAllByRole("button", { name: "New goal" }))[0],
  );
  dialog().getByRole("heading", { name: "New goal" });
  fireEvent.change(dialog().getByRole("textbox", { name: "Name" }), {
    target: { value: "Emergency fund" },
  });
  fireEvent.change(dialog().getByRole("textbox", { name: "Target" }), {
    target: { value: "50,000" },
  });
  const account = dialog().getByRole("combobox", { name: "Account" });
  fireEvent.click(account);
  expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual([
    "Cash",
    "Bank",
    "E-wallet",
  ]);
  fireEvent.click(screen.getByRole("option", { name: "E-wallet" }));
  fireEvent.click(dialog().getByRole("button", { name: "Create goal" }));
  await waitFor(() =>
    expect(calls("create_goal")).toEqual([
      {
        goal: {
          name: "Emergency fund",
          targetAmount: 5000000,
          targetDate: null,
          accountId: 3,
        },
      },
    ]),
  );
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
});

test("the backend's message shows under the field it names", async () => {
  renderRoute("/goals", {
    ...ANSWERS,
    create_goal: () => {
      throw {
        code: "validation",
        field: "name",
        message: "A goal with this name already exists",
      };
    },
  });
  fireEvent.click(
    (await screen.findAllByRole("button", { name: "New goal" }))[0],
  );
  fireEvent.click(dialog().getByRole("button", { name: "Create goal" }));
  await dialog().findByText("A goal with this name already exists");
});

test("clicking a goal edits it, with its date", async () => {
  renderRoute("/goals", ANSWERS);
  fireEvent.click(
    await screen.findByRole("button", { name: "Edit Japan trip" }),
  );
  dialog().getByRole("heading", { name: "Edit goal" });
  expect(
    (dialog().getByRole("textbox", { name: "Target date" }) as HTMLInputElement)
      .value,
  ).toBe("05/05/2027");
  fireEvent.change(dialog().getByRole("textbox", { name: "Name" }), {
    target: { value: "Kyoto trip" },
  });
  fireEvent.click(dialog().getByRole("button", { name: "Save" }));
  await waitFor(() =>
    expect(calls("update_goal")).toEqual([
      {
        id: 1,
        goal: {
          name: "Kyoto trip",
          targetAmount: 1000000,
          targetDate: "2027-05-05",
          accountId: 2,
        },
      },
    ]),
  );
});

test("Archive folds a goal away, and Show archived lists it with Unarchive (GOL-4)", async () => {
  renderRoute("/goals", ANSWERS);
  await openMenu("New laptop");
  fireEvent.click(await screen.findByRole("menuitem", { name: "Archive" }));
  await waitFor(() => expect(calls("archive_goal")).toEqual([{ id: 2 }]));

  const toggle = screen.getByRole("button", { name: "Show archived (1)" });
  expect(screen.queryByRole("button", { name: "Edit Old plan" })).toBeNull();
  fireEvent.click(toggle);
  const old = await card("Edit Old plan");
  old.getByText("Old Wallet");
  await openMenu("Old plan");
  expect(
    screen.getAllByRole("menuitem").map((item) => item.textContent),
  ).toEqual(["Unarchive", "Delete"]);
  fireEvent.click(screen.getByRole("menuitem", { name: "Unarchive" }));
  await waitFor(() => expect(calls("unarchive_goal")).toEqual([{ id: 4 }]));
});

test("an archived goal's edit keeps its archived account", async () => {
  renderRoute("/goals", ANSWERS);
  fireEvent.click(
    await screen.findByRole("button", { name: "Show archived (1)" }),
  );
  fireEvent.click(await screen.findByRole("button", { name: "Edit Old plan" }));
  expect(
    (dialog().getByRole("combobox", { name: "Account" }) as HTMLInputElement)
      .value,
  ).toBe("Old Wallet (archived)");
});

test("Delete asks first", async () => {
  renderRoute("/goals", ANSWERS);
  await openMenu("Car");
  fireEvent.click(await screen.findByRole("menuitem", { name: "Delete" }));
  const confirm = within(
    await screen.findByRole("dialog", { name: "Delete this goal?" }),
  );
  fireEvent.click(confirm.getByRole("button", { name: "Delete" }));
  await waitFor(() => expect(calls("delete_goal")).toEqual([{ id: 3 }]));
});
