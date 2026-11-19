import { invoke } from "@tauri-apps/api/core";
import {
  act,
  cleanup,
  fireEvent,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { Checkin } from "@/bindings/Checkin";
import {
  ACCOUNT_LIST,
  CATEGORIES,
  CHECKIN,
  LOANS,
  MONTHLY_CHECKIN,
} from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const NAMES: Answers = {
  list_categories: CATEGORIES,
  list_accounts: ACCOUNT_LIST,
  list_loans: LOANS,
};

function open(checkin: Checkin | (() => Checkin | null), more: Answers = {}) {
  return renderRoute("/check-in", {
    ...NAMES,
    get_checkin: checkin,
    ...more,
  });
}

const path = (router: { state: { location: { pathname: string } } }) =>
  router.state.location.pathname;
const button = (name: string | RegExp) => screen.getByRole("button", { name });
const say = () => screen.getByRole("heading", { level: 2 }).textContent;
const stepLine = () => screen.getByText(/^Step \d+ of \d+/).textContent;
const next = () => fireEvent.click(button(/^Next/));

/** Clicks Next until the step indicator names `name`. */
async function goTo(name: string) {
  await screen.findByText(/^Step \d+ of/);
  for (let i = 0; i < 10 && !stepLine()?.endsWith(name); i++) next();
  expect(stepLine()).toContain(name);
}

test("a due check-in takes over every route, with the period in its title (CHK-2)", async () => {
  const { router } = renderRoute("/accounts", {
    ...NAMES,
    get_checkin: CHECKIN,
  });
  await screen.findByText(/^Step 1 of/);
  expect(screen.getByText("Weekly check-in · Nov 9–15")).toBeTruthy();
  expect(path(router)).toBe("/check-in");
  // No sidebar, and nothing that closes it.
  expect(screen.queryByRole("navigation")).toBeNull();
  expect(
    screen.queryByRole("button", { name: /close|later|skip/i }),
  ).toBeNull();
  expect(stepLine()).toBe("Step 1 of 6 · Start");
  expect(screen.queryByRole("button", { name: "Back" })).toBeNull();
});

test("a merged check-in is titled the month's and has Goals and the Balance check", async () => {
  open(MONTHLY_CHECKIN);
  await screen.findByText("Monthly check-in · October 2026");
  expect(stepLine()).toBe("Step 1 of 8 · Start");
  await goTo("Goals");
  expect(screen.getByText("Japan trip")).toBeTruthy();
  next();
  expect(stepLine()).toBe("Step 6 of 8 · Balance check");
});

test("with no check-in due, /check-in goes to the dashboard", async () => {
  const { router } = renderRoute("/check-in", { get_checkin: null });
  await waitFor(() => expect(path(router)).toBe("/"));
});

test("a check-in that can't be loaded opens the app normally (ADR 0012)", async () => {
  const { router } = renderRoute("/", {
    get_checkin: () => {
      throw { code: "database", message: "busy" };
    },
  });
  await screen.findByRole("navigation");
  expect(path(router)).toBe("/");
});

test("one that comes due while the app is open takes over on the next focus", async () => {
  let due: Checkin | null = null;
  const { router } = renderRoute("/", {
    ...NAMES,
    get_checkin: () => due,
    process_recurring: 0,
  });
  await screen.findByRole("navigation");
  due = CHECKIN;
  act(() => {
    window.dispatchEvent(new Event("focus"));
  });
  await waitFor(() => expect(path(router)).toBe("/check-in"));
});

test("Esc, Ctrl+1 to Ctrl+4 and Ctrl+, don't leave it, and Ctrl+N opens quick-add over it", async () => {
  const { router } = open(CHECKIN);
  await screen.findByText(/^Step 1 of/);
  for (const key of ["Escape"]) fireEvent.keyDown(window, { key });
  for (const [key, code] of [
    ["1", "Digit1"],
    ["2", "Digit2"],
    ["3", "Digit3"],
    ["4", "Digit4"],
    [",", "Comma"],
  ])
    fireEvent.keyDown(window, { key, code, ctrlKey: true });
  await new Promise((r) => setTimeout(r, 20));
  expect(path(router)).toBe("/check-in");
  fireEvent.keyDown(window, { key: "n", code: "KeyN", ctrlKey: true });
  expect(await screen.findByRole("dialog")).toBeTruthy();
  expect(path(router)).toBe("/check-in");
});

test("after a save the figures reload and the step stays (CHK-2)", async () => {
  let checkin = CHECKIN;
  const { client } = open(() => checkin);
  await goTo("Loans and bills");
  expect(say()).toBe("1 loan payment paid, ₱1,000.00.");
  checkin = {
    ...CHECKIN,
    loans: {
      ...CHECKIN.loans,
      paid: [
        ...CHECKIN.loans.paid,
        { ...CHECKIN.loans.paid[0], paymentId: 22 },
      ],
      paidTotal: 200000,
    },
  };
  await act(() => client.invalidateQueries({ queryKey: ["ledger"] }));
  await waitFor(() => expect(say()).toBe("2 loan payments paid, ₱2,000.00."));
  expect(stepLine()).toContain("Loans and bills");
});

test("the step is kept in the URL, Enter is Next and Alt+← is Back", async () => {
  const { router } = renderRoute("/check-in?step=card", {
    ...NAMES,
    get_checkin: CHECKIN,
  });
  await screen.findByText("Step 3 of 6 · Card");
  expect(say()).toBe("You're under your soft level this cycle.");
  fireEvent.keyDown(document.body, { key: "Enter" });
  expect(stepLine()).toBe("Step 4 of 6 · Loans and bills");
  expect(router.state.location.search).toBe("?step=loans");
  fireEvent.keyDown(window, { key: "ArrowLeft", altKey: true });
  expect(stepLine()).toBe("Step 3 of 6 · Card");
});

test("the Start step leads with the win, then the best and the last focus (CHK-6)", async () => {
  open(CHECKIN);
  await screen.findByText(/^Step 1 of/);
  expect(say()).toBe("Food & Groceries fell by ₱640.00 from the week before.");
  expect(
    screen.getByText("Your lowest-spending week of the last 12."),
  ).toBeTruthy();
  expect(
    screen.getByText(
      "Last week's focus: keep Dining Out under ₱1,500.00. You spent ₱1,720.00, past your ₱1,500.00 target.",
    ),
  ).toBeTruthy();
  expect(screen.getByLabelText("Missed")).toBeTruthy();
});

test("with no win and no focus, Start says how the week went", async () => {
  open({
    ...CHECKIN,
    start: { win: null, personalBest: null, lastFocus: null },
  });
  await screen.findByText(/^Step 1 of/);
  expect(say()).toBe("Here's how last week went.");
});

test("Money in and out shows a budget broken by how much, and one kept", async () => {
  open(CHECKIN);
  await goTo("Money in and out");
  expect(screen.getByText("₱30.00 over")).toBeTruthy();
  expect(screen.getByText("kept")).toBeTruthy();
});

test("the Goals step shows only when the backend sends it (CHK-3)", async () => {
  open({ ...CHECKIN, goals: [] });
  await screen.findByText("Step 1 of 7 · Start");
  await goTo("Goals");
  expect(say()).toBe("No goals yet.");
});

test("Next waits for an answer on every Balance check row, and they're never sent (CHK-4)", async () => {
  open(MONTHLY_CHECKIN);
  await goTo("Balance check");
  const nextButton = button(/^Next/) as HTMLButtonElement;
  expect(nextButton.disabled).toBe(true);
  for (const name of ["Cash", "E-wallet"]) {
    fireEvent.click(
      within(screen.getByRole("radiogroup", { name })).getByText("Matches"),
    );
  }
  expect(nextButton.disabled).toBe(true);
  fireEvent.click(
    within(screen.getByRole("radiogroup", { name: "Credit card" })).getByText(
      "Doesn't match",
    ),
  );
  expect(screen.getByText(/Add what's missing with \+ Add/)).toBeTruthy();
  expect(nextButton.disabled).toBe(false);
  const sent = vi.mocked(invoke).mock.calls.map(([command]) => command);
  expect(sent.every((c) => c.startsWith("get_") || c.startsWith("list_"))).toBe(
    true,
  );
});

test("Stats and level shows the scores, their change, the level and the next move", async () => {
  open(CHECKIN);
  await goTo("Stats and level");
  expect(say()).toBe("You're still at level 4.");
  expect(screen.getByText("Level 4 · In the black")).toBeTruthy();
  expect(
    screen.getByText(
      "Next best move: earn more than you spend 3 months in a row, 1 of 3 so far.",
    ),
  ).toBeTruthy();
  expect(
    screen.getByText("4 check-ins in a row on time · 2 focuses kept in a row"),
  ).toBeTruthy();
  expect(
    screen.getByText("Against the last check-in, Mon, Nov 9"),
  ).toBeTruthy();
});

test("a level that fell names why", async () => {
  open({
    ...CHECKIN,
    status: {
      ...CHECKIN.status,
      level: 2,
      levelChange: "down",
      levelReason: "nothing_overdue",
    },
  });
  await goTo("Stats and level");
  expect(say()).toBe("Level 2 now: a payment is overdue.");
});

async function pickFocus() {
  await goTo("Pick a focus");
  expect(say()).toBe("Pick one focus for this week.");
  expect(button("Finish").hasAttribute("disabled")).toBe(true);
  const radios = screen.getAllByRole("radio");
  expect(radios.map((r) => r.getAttribute("aria-checked"))).toEqual([
    "false",
    "false",
    "false",
  ]);
  expect(screen.getByText("Pay E-wallet loan by Sat, Nov 28")).toBeTruthy();
  fireEvent.click(screen.getByText("Keep Dining Out under ₱1,500.00"));
  expect(radios[1].getAttribute("aria-checked")).toBe("true");
  expect(button("Finish").hasAttribute("disabled")).toBe(false);
}

test("Finish waits for a pick, saves it and opens the dashboard (CHK-7, F17)", async () => {
  let done = false;
  const complete = vi.fn(() => {
    done = true;
    return [];
  });
  const { router } = open(() => (done ? null : CHECKIN), {
    complete_checkin: complete,
  });
  await pickFocus();
  fireEvent.click(button("Finish"));
  await waitFor(() => expect(path(router)).toBe("/"));
  expect(complete).toHaveBeenCalledWith({ focus: CHECKIN.offers[1] });
  expect(await screen.findByText("Check-in done")).toBeTruthy();
});

test("↑ and ↓ move the pick between the offers", async () => {
  open(CHECKIN);
  await goTo("Pick a focus");
  const group = screen.getByRole("radiogroup");
  fireEvent.keyDown(group, { key: "ArrowDown" });
  fireEvent.keyDown(group, { key: "ArrowDown" });
  fireEvent.keyDown(group, { key: "ArrowUp" });
  fireEvent.keyDown(group, { key: "ArrowUp" });
  const checked = screen
    .getAllByRole("radio")
    .map((r) => r.getAttribute("aria-checked"));
  expect(checked).toEqual(["false", "false", "true"]);
});

test("an account left as Doesn't match opens its details after Finish", async () => {
  let done = false;
  const { router } = open(() => (done ? null : MONTHLY_CHECKIN), {
    complete_checkin: () => {
      done = true;
      return [];
    },
    get_account: () => new Promise(() => {}),
  });
  await goTo("Balance check");
  for (const [name, answer] of [
    ["Cash", "Matches"],
    ["E-wallet", "Doesn't match"],
    ["Credit card", "Doesn't match"],
  ])
    fireEvent.click(
      within(screen.getByRole("radiogroup", { name })).getByText(answer),
    );
  await goTo("Pick a focus");
  fireEvent.click(screen.getAllByRole("radio")[0]);
  fireEvent.click(button("Finish"));
  await waitFor(() => expect(path(router)).toBe("/accounts/3"));
  expect(
    await screen.findByText(
      "Check-in done. Check this account against your real balance.",
    ),
  ).toBeTruthy();
});

test("a refused Finish reloads the offers, clears a pick that's gone, and offers a way on (ADR 0014)", async () => {
  // The day changed: the offers now run for the next week.
  const moved = {
    ...CHECKIN,
    offers: CHECKIN.offers.map((o) => ({
      ...o,
      start: "2026-11-23",
      end: "2026-11-29",
    })),
  };
  let loads = 0;
  const { router } = open(() => (loads++ === 0 ? CHECKIN : moved), {
    complete_checkin: () => {
      throw {
        code: "validation",
        field: "focus",
        message: "Pick one of the offered focuses.",
      };
    },
  });
  await pickFocus();
  expect(screen.queryByText("Open the app anyway")).toBeNull();
  fireEvent.click(button("Finish"));
  expect(
    await screen.findByText("Pick one of the offered focuses."),
  ).toBeTruthy();
  await waitFor(() =>
    expect(
      screen.getAllByRole("radio").map((r) => r.getAttribute("aria-checked")),
    ).toEqual(["false", "false", "false"]),
  );
  expect(stepLine()).toContain("Pick a focus");
  expect(button("Finish").hasAttribute("disabled")).toBe(true);
  // A second try closes the first failure's toast, so only one way out is ever on screen.
  fireEvent.click(screen.getAllByRole("radio")[0]);
  fireEvent.click(button("Finish"));
  await waitFor(() =>
    expect(
      screen.getAllByText("Pick one of the offered focuses."),
    ).toHaveLength(1),
  );
  await new Promise((r) => setTimeout(r, 450));
  expect(screen.getAllByText("Open the app anyway")).toHaveLength(1);
  fireEvent.click(screen.getByText("Open the app anyway"));
  await waitFor(() => expect(path(router)).toBe("/"));
  await screen.findByRole("navigation");

  // The lifted guard lived in memory only, so the next start asks again.
  cleanup();
  const restart = renderRoute("/", { ...NAMES, get_checkin: moved });
  await waitFor(() => expect(path(restart.router)).toBe("/check-in"));
  expect(await screen.findByText(/^Step 1 of/)).toBeTruthy();
});

test("a reload that fails keeps an open check-in, rather than opening the app", async () => {
  let fail = false;
  const { router, client } = open(() => {
    if (fail) throw { code: "database", message: "busy" };
    return CHECKIN;
  });
  await screen.findByText(/^Step 1 of/);
  fail = true;
  await act(() => client.invalidateQueries({ queryKey: ["ledger"] }));
  await new Promise((r) => setTimeout(r, 20));
  expect(path(router)).toBe("/check-in");
});

test("a Balance check that appears after a save can't be skipped from a later step (CHK-4)", async () => {
  renderRoute("/check-in?step=focus", {
    ...NAMES,
    get_checkin: MONTHLY_CHECKIN,
  });
  await screen.findByText(/^Step \d+ of/);
  expect(stepLine()).toBe("Step 6 of 8 · Balance check");
});

test("overdue counts every overdue payment, not only one per loan", async () => {
  open({
    ...CHECKIN,
    loans: {
      ...CHECKIN.loans,
      loansDue: [
        {
          loanId: 2,
          name: "Bank loan",
          accountId: 2,
          paymentId: 21,
          amount: 100000,
          dueDate: "2026-10-10",
          days: -37,
          moreOverdue: 2,
          moveFirst: null,
        },
      ],
    },
  });
  await goTo("Loans and bills");
  expect(say()).toBe("3 loan payments are overdue.");
});

test("settings that fail to load show the error with Retry, not a skeleton forever", async () => {
  renderRoute(
    "/check-in",
    {
      ...NAMES,
      get_checkin: CHECKIN,
      get_settings: () => {
        throw { code: "database", message: "busy" };
      },
    },
    { seedSettings: false },
  );
  expect(await screen.findByRole("button", { name: /Retry/ })).toBeTruthy();
});

test("holding Enter moves one step, not through them all", async () => {
  open(CHECKIN);
  await screen.findByText(/^Step 1 of/);
  fireEvent.keyDown(document.body, { key: "Enter" });
  fireEvent.keyDown(document.body, { key: "Enter", repeat: true });
  fireEvent.keyDown(document.body, { key: "Enter", repeat: true });
  expect(stepLine()).toBe("Step 2 of 6 · Money in and out");
});
