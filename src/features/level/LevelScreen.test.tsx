import { invoke } from "@tauri-apps/api/core";
import { act, screen, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { ActiveFocus } from "@/bindings/ActiveFocus";
import type { VaultStatus } from "@/bindings/VaultStatus";
import {
  ACCOUNT_LIST,
  CATEGORIES,
  CHECKIN_STATUS,
  LOANS,
} from "@/test/fixtures";
import { renderRoute } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

// Mon, Nov 16, 2026, at level 4, against the Nov 9 check-in.
const { levelChange: _, ...STATUS } = CHECKIN_STATUS;

const show = (over: Partial<VaultStatus> = {}) =>
  renderRoute("/level", {
    get_vault_status: { ...STATUS, ...over },
    list_categories: CATEGORIES,
    list_accounts: ACCOUNT_LIST,
    list_loans: LOANS,
  });

const focus = (
  over: Partial<ActiveFocus> & Pick<ActiveFocus, "kind">,
): ActiveFocus => ({
  actual: null,
  paidOn: null,
  percent: null,
  state: null,
  categoryId: null,
  accountId: null,
  loanPaymentId: null,
  target: null,
  start: "2026-11-16",
  end: "2026-11-22",
  ...over,
});

const row = (name: string) =>
  screen.getByText(name, { selector: "span" }).parentElement as HTMLElement;

test("the level, its path, the next level's checklist and the move (LVL-3, LVL-4)", async () => {
  show();
  expect(await screen.findByText("Level 4 · In the black")).toBeTruthy();
  expect(
    screen.getByRole("heading", { level: 1, name: "Vault level" }),
  ).toBeTruthy();

  // Each node says its state in words, so color is never the only cue.
  const path = screen.getByRole("list", { name: "Level path" });
  const nodes = within(path)
    .getAllByLabelText(/^Level \d+/)
    .map((n) => n.getAttribute("aria-label"));
  expect(nodes).toHaveLength(10);
  expect(nodes[2]).toBe("Level 3 · Bill Keeper, reached");
  expect(nodes[3]).toBe("Level 4 · In the black, your level");
  expect(nodes[4]).toBe("Level 5 · Steady, not reached");

  expect(
    screen.getByRole("heading", { name: "Next: Level 5 · Steady" }),
  ).toBeTruthy();
  const unmet = screen.getByText(/^Earn more than you spend 3 months/);
  expect(unmet.textContent).toBe(
    "Not met: Earn more than you spend 3 months in a row· 1 of 3 so far",
  );
  expect(screen.getByText(/^No pay window runs short/).textContent).toBe(
    "Met: No pay window runs short in the next 12 months",
  );
  expect(
    screen.getByText(
      "Next best move: earn more than you spend 3 months in a row, 1 of 3 so far.",
    ),
  ).toBeTruthy();
  // Everything is for today, and nothing can be edited.
  expect(screen.queryByRole("group", { name: /period/i })).toBeNull();
  expect(screen.getAllByRole("button").map((b) => b.textContent)).toEqual([
    "AddCtrl+N",
  ]);
});

test("a path node gives its level's name and conditions when it has the keyboard", async () => {
  show();
  const node = await screen.findByLabelText("Level 8 · Debt-free, not reached");
  expect(node.tabIndex).toBe(0);
  act(() => node.focus());
  const tip = await screen.findByRole("tooltip");
  expect(tip.textContent).toBe(
    "Level 8 · Debt-freePay off every loan90 days without a new loan",
  );
});

test("each score is written with its change against the last check-in, and in a hidden table", async () => {
  show();
  await screen.findByText("Level 4 · In the black");
  expect(row("Cashflow").textContent).toBe("Cashflow72▲ Up 6");
  expect(row("Bills").textContent).toBe("Bills100— No change");
  expect(row("Card").textContent).toBe("Card48▼ Down 10");
  const table = screen.getByRole("table", { name: "Stats" });
  expect(
    within(table)
      .getAllByRole("columnheader")
      .map((h) => h.textContent),
  ).toEqual(["Stat", "Score", "Last check-in"]);
  expect(within(table).getAllByRole("row")[5].textContent).toBe("Savings2220");
  // Today's shape and last check-in's outline behind it.
  expect(document.querySelectorAll(".recharts-radar")).toHaveLength(2);
  expect(
    screen.getByText(
      "Weakest: Savings. Your money covers less than 3 months of spending.",
    ),
  ).toBeTruthy();
});

test("before the first check-in there's no outline and no change column", async () => {
  show({ previous: null });
  await screen.findByText("Level 4 · In the black");
  expect(row("Cashflow").textContent).toBe("Cashflow72");
  expect(screen.queryByText(/▲|▼/)).toBeNull();
  const table = screen.getByRole("table", { name: "Stats" });
  expect(within(table).getAllByRole("columnheader")).toHaveLength(2);
  expect(document.querySelectorAll(".recharts-radar")).toHaveLength(1);
});

test("a stat that doesn't apply has no corner and shows —", async () => {
  show({
    stats: STATUS.stats.map((s) =>
      s.stat === "card" ? { ...s, score: null, reason: null } : s,
    ),
  });
  await screen.findByText("Level 4 · In the black");
  expect(row("Card").textContent).toBe("Card——");
  // Five corners: the chart's labels name only the stats that apply.
  expect(
    document.querySelectorAll(".recharts-polar-angle-axis-tick"),
  ).toHaveLength(5);
});

test("level 0 has no name, and its next level is level 1", async () => {
  show({
    level: 0,
    nextLevel: {
      level: 1,
      conditions: [{ code: "logged_20", met: false, gap: 6, accountId: null }],
      met: 0,
      total: 1,
    },
    nextMove: {
      kind: "condition",
      condition: { code: "logged_20", met: false, gap: 6, accountId: null },
    },
  });
  expect(await screen.findByText("Level 0")).toBeTruthy();
  expect(screen.queryByText(/^Level 0 ·/)).toBeNull();
  expect(screen.getByLabelText("Level 1 · Tracker, not reached")).toBeTruthy();
  expect(screen.getByText(/^Log on 20 of the last 30 days/).textContent).toBe(
    "Not met: Log on 20 of the last 30 days· 6 days more to log",
  );
  expect(screen.getByText("Next best move: log on 6 days more.")).toBeTruthy();
});

test("at level 10 there's no next level, and the move names the lowest stat", async () => {
  show({
    level: 10,
    nextLevel: { level: 10, conditions: [], met: 3, total: 3 },
    nextMove: { kind: "lowest_stat", reason: "cushion_low" },
  });
  expect(await screen.findByText("Level 10 · Vault Master")).toBeTruthy();
  expect(screen.queryByRole("heading", { name: /^Next:/ })).toBeNull();
  expect(
    screen.getByText(
      "You're at the top level. Weakest: Savings. Your money covers less than 3 months of spending.",
    ),
  ).toBeTruthy();
  expect(
    screen.getByLabelText("Level 10 · Vault Master, your level"),
  ).toBeTruthy();
});

test("with no focus it says so, and the streaks are listed one per line (LVL-7)", async () => {
  show();
  expect(
    await screen.findByText(
      "No focus right now. You'll pick one at your next check-in.",
    ),
  ).toBeTruthy();
  expect(screen.getByText("2 focuses kept in a row")).toBeTruthy();
  expect(screen.getByText("4 check-ins in a row on time")).toBeTruthy();
  expect(screen.getByText("51 days since your last new loan")).toBeTruthy();
});

test("the loan-free streak is hidden when there's none", async () => {
  show({ loanFreeDays: null, focusStreak: 1, checkinStreak: 1 });
  expect(await screen.findByText("1 focus kept in a row")).toBeTruthy();
  expect(screen.getByText("1 check-in in a row on time")).toBeTruthy();
  expect(screen.queryByText(/since your last new loan/)).toBeNull();
});

test("the active focuses, the month's first, with a cap's bar under its sentence", async () => {
  show({
    focuses: [
      focus({
        kind: "category_cap",
        categoryId: 2,
        target: 150000,
        actual: 64000,
        percent: 42,
        state: "ok",
      }),
      focus({
        kind: "loan_on_time",
        loanPaymentId: 14,
        start: "2026-11-01",
        end: "2026-11-30",
      }),
      focus({
        kind: "card_level",
        accountId: 4,
        target: 400000,
        actual: 231000,
      }),
      focus({ kind: "logging", target: 5, actual: 3 }),
    ],
  });
  const lines = (await screen.findAllByText(/'s focus: /)).map(
    (l) => l.textContent,
  );
  expect(lines).toEqual([
    "This month's focus: Pay E-wallet loan by Sat, Nov 28 · not paid yet",
    "This week's focus: Keep Dining Out under ₱1,500.00 · ₱640.00 so far",
    "This week's focus: Stay under your ₱4,000.00 mid level · ₱2,310.00 this cycle",
    "This week's focus: Log on at least 5 days this week · 3 days so far",
  ]);
  const bar = screen.getByRole("progressbar");
  expect(bar.getAttribute("aria-valuenow")).toBe("42");
  expect(screen.queryByText(/No focus right now/)).toBeNull();
});

test("a failed load is the inline message with Retry", async () => {
  renderRoute("/level", {
    get_vault_status: () => {
      throw { kind: "internal", message: "x" };
    },
    list_categories: CATEGORIES,
    list_accounts: ACCOUNT_LIST,
    list_loans: LOANS,
  });
  expect(await screen.findByRole("button", { name: /retry/i })).toBeTruthy();
});
