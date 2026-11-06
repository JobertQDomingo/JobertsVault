import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { Change } from "./Change";

test("up and down carry an arrow, a word for screen readers and a color", () => {
  const { container } = render(
    <>
      <Change tenths={123} upIsGood />
      <Change tenths={-40} upIsGood />
      <Change tenths={123} upIsGood={false} />
    </>,
  );
  const [up, down, badUp] = [...container.children];
  expect(up.textContent).toBe("▲ Up 12.3%");
  expect(up.className).toContain("text-income");
  expect(down.textContent).toBe("▼ Down 4.0%");
  expect(down.className).toContain("text-expense");
  // Spending more is bad.
  expect(badUp.className).toContain("text-expense");
});

test("nothing to compare with is a dash, and no change has no arrow", () => {
  render(
    <>
      <Change tenths={null} upIsGood />
      <Change tenths={0} upIsGood />
    </>,
  );
  expect(screen.getByTitle("Nothing to compare with").textContent).toBe("—");
  expect(screen.getByText("0.0%")).toBeTruthy();
});
