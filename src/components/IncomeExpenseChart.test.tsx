import { cloneElement, type ReactElement } from "react";
import { expect, test, vi } from "vitest";
import { SERIES } from "@/test/fixtures";
import { renderWithSettings } from "@/test/render";
import { IncomeExpenseChart } from "./IncomeExpenseChart";

// jsdom has no layout, so give the chart a fixed size instead of measuring its box.
vi.mock("recharts", async (importOriginal) => ({
  ...(await importOriginal<typeof import("recharts")>()),
  ResponsiveContainer: ({ children }: { children: ReactElement }) =>
    cloneElement(children as ReactElement<{ width: number; height: number }>, {
      width: 800,
      height: 272,
    }),
}));

const yTicks = (container: HTMLElement) =>
  [
    ...container.querySelectorAll(
      ".recharts-yAxis-tick-labels .recharts-cartesian-axis-tick-value",
    ),
  ].map((t) => t.textContent);

/** The net line's path: empty when nothing is drawn. */
const net = (container: HTMLElement) =>
  container.querySelector(".recharts-line-curve")?.getAttribute("d") ?? "";

const EMPTY = {
  ...SERIES,
  bars: SERIES.bars.map((b) => ({ ...b, income: 0, expense: 0, net: 0 })),
};

test("an empty period's chart labels only ₱0 and draws no net line", () => {
  const { container } = renderWithSettings(
    <IncomeExpenseChart series={EMPTY} height={272} />,
  );
  expect(yTicks(container)).toEqual(["₱0"]);
  expect(net(container)).toBe("");
});

test("a period with figures has a scaled axis and a net line", () => {
  const { container } = renderWithSettings(
    <IncomeExpenseChart series={SERIES} height={272} />,
  );
  expect(yTicks(container).length).toBeGreaterThan(2);
  expect(net(container)).not.toBe("");
});
