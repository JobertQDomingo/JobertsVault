import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import { BREAKDOWN, CATEGORIES, PERIOD, SERIES } from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const ANSWERS: Answers = {
  get_period: PERIOD,
  get_timeseries: SERIES,
  get_category_breakdown: BREAKDOWN,
  list_categories: CATEGORIES,
};

const chart = () =>
  screen.findByRole("table", {
    name: "Income, expense and net cashflow by day",
  });

const ranked = () =>
  screen.findByRole("list", { name: "Categories by expenses" });

test("the chart shows the bars for the period in the URL", async () => {
  renderRoute("/reports?period=month&anchor=2026-11-01", ANSWERS);
  const table = await chart();
  expect(within(table).getAllByRole("row")).toHaveLength(31);
  expect(invoke).toHaveBeenCalledWith("get_timeseries", {
    query: { kind: "month", anchor: "2026-11-01" },
  });
  expect(screen.queryByText("Nothing recorded in this period.")).toBeNull();
});

test("a period with nothing recorded says so", async () => {
  const bars = SERIES.bars.map((b) => ({
    ...b,
    income: 0,
    expense: 0,
    net: 0,
  }));
  renderRoute("/reports", { ...ANSWERS, get_timeseries: { ...SERIES, bars } });
  expect(
    await screen.findByText("Nothing recorded in this period."),
  ).toBeTruthy();
});

test("if the chart can't load, Retry asks again", async () => {
  let fail = true;
  renderRoute("/reports", {
    ...ANSWERS,
    get_timeseries: () => {
      if (fail) throw { code: "database", message: "disk I/O error" };
      return SERIES;
    },
  });
  const retry = await screen.findByRole("button", { name: "Retry" });
  fail = false;
  fireEvent.click(retry);
  expect(await chart()).toBeTruthy();
});

test("the breakdown ranks the expense categories with their amounts and shares", async () => {
  renderRoute("/reports", ANSWERS);
  const rows = within(await ranked()).getAllByRole("listitem");
  expect(rows.map((r) => r.textContent)).toEqual([
    "Food & Groceries67.8%−₱2,000.00",
    "Dining Out28.8%−₱850.40",
    // Archived, but its spending still counts.
    "Snacks3.4%−₱100.00",
  ]);
  expect(screen.getByText("Total expenses")).toBeTruthy();
  expect(invoke).toHaveBeenCalledWith("get_category_breakdown", {
    query: { kind: "month", anchor: null },
    kind: "expense",
  });
});

test("clicking a category opens its transactions for the same period", async () => {
  const { router } = renderRoute(
    "/reports?period=week&anchor=2026-11-02",
    ANSWERS,
  );
  fireEvent.click(within(await ranked()).getByText("Dining Out"));
  await waitFor(() =>
    expect(router.state.location.pathname).toBe("/transactions"),
  );
  const params = new URLSearchParams(router.state.location.search);
  expect(Object.fromEntries(params)).toEqual({
    period: "week",
    anchor: "2026-11-02",
    category: "2",
  });
});

test("the switch shows income instead, and stays in the URL", async () => {
  const { router } = renderRoute("/reports", {
    ...ANSWERS,
    get_category_breakdown: (args: { kind: string }) =>
      args.kind === "income"
        ? {
            kind: "income",
            total: 420000,
            categories: [{ categoryId: 3, amount: 420000, share: 1000 }],
            slices: [{ categoryId: 3, amount: 420000, share: 1000 }],
          }
        : BREAKDOWN,
  });
  await ranked();
  fireEvent.click(screen.getByRole("radio", { name: "Income" }));
  const list = await screen.findByRole("list", {
    name: "Categories by income",
  });
  expect(within(list).getByText("Salary")).toBeTruthy();
  expect(within(list).getByText("+₱4,200.00")).toBeTruthy();
  expect(router.state.location.search).toBe("?breakdown=income");
});

test("with nothing in the period, the breakdown says so", async () => {
  renderRoute("/reports", {
    ...ANSWERS,
    get_category_breakdown: {
      kind: "expense",
      total: 0,
      categories: [],
      slices: [],
    },
  });
  expect(
    await screen.findByText("No expenses recorded in this period."),
  ).toBeTruthy();
});
