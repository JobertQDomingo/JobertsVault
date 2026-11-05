import { render } from "@testing-library/react";
import { expect, test } from "vitest";
import { AuroraBackground } from "./AuroraBackground";

test("the aurora is decoration only, fixed behind everything", () => {
  const { container } = render(<AuroraBackground />);
  const aurora = container.firstElementChild as HTMLElement;
  expect(aurora.getAttribute("aria-hidden")).toBe("true");
  for (const name of ["fixed", "inset-0", "pointer-events-none", "-z-10"])
    expect(aurora.classList).toContain(name);
});

test("it paints the canvas and all four lights from tokens", () => {
  const { container } = render(<AuroraBackground />);
  const background = (container.firstElementChild as HTMLElement).style
    .background;
  for (const token of [
    "canvas",
    "aurora-1",
    "aurora-2",
    "aurora-3",
    "aurora-4",
  ])
    expect(background).toContain(`var(--${token})`);
});
