import { screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { renderWithSettings } from "@/test/render";
import { ProgressBar } from "./ProgressBar";
import { StatCard } from "./StatCard";

test("a stat card shows its label and a display-size amount", () => {
  renderWithSettings(<StatCard label="Income" amount={420000} kind="income" />);
  expect(screen.getByText("Income")).toBeTruthy();
  expect(screen.getByText("+₱4,200.00").className).toContain("text-display");
});

test("only the net cashflow card is tinted", () => {
  const { container } = renderWithSettings(
    <>
      <StatCard label="Expense" amount={295040} kind="expense" />
      <StatCard label="Net cashflow" amount={124960} kind="net" />
    </>,
  );
  const [expense, net] = [...container.children] as HTMLElement[];
  expect(expense.className).not.toContain("glass-tint");
  expect(net.className).toContain("glass-tint");
});

const fill = () =>
  screen.getByRole("progressbar").firstElementChild as HTMLElement;

test("the fill turns amber from 80% and red from 100%", () => {
  const { rerender } = renderWithSettings(<ProgressBar percent={79} />);
  expect(fill().className).toContain("bg-primary");
  rerender(<ProgressBar percent={80} />);
  expect(fill().className).toContain("bg-warning");
  rerender(<ProgressBar percent={100} />);
  expect(fill().className).toContain("bg-danger");
});

test("past 100% the fill stops at the end and the value stays valid", () => {
  renderWithSettings(<ProgressBar percent={150} />);
  expect(fill().style.width).toBe("100%");
  const bar = screen.getByRole("progressbar");
  expect(bar.getAttribute("aria-valuenow")).toBe("150");
  expect(bar.getAttribute("aria-valuemax")).toBe("150");
});

test("the percent is written next to the bar", () => {
  renderWithSettings(<ProgressBar percent={6} />);
  expect(screen.getByText("6%")).toBeTruthy();
});

test("the bar is named by its label and never reports below 0", () => {
  renderWithSettings(<ProgressBar percent={-5} label="Utilization" />);
  const bar = screen.getByRole("progressbar", { name: "Utilization" });
  expect(bar.getAttribute("aria-valuenow")).toBe("0");
});
