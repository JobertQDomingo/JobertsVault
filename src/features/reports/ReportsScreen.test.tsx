import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import { PERIOD, SERIES } from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const ANSWERS: Answers = { get_period: PERIOD, get_timeseries: SERIES };

const chart = () =>
  screen.findByRole("table", {
    name: "Income, expense and net cashflow by day",
  });

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
