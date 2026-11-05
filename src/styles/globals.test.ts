import { expect, test } from "vitest";
import css from "./globals.css?raw";

/** The custom properties declared in the first block that starts with `selector {`. */
function tokensIn(selector: string) {
  const start = css.indexOf(`${selector} {`);
  expect(start, `${selector} block`).toBeGreaterThanOrEqual(0);
  const block = css.slice(start, css.indexOf("\n}", start));
  return [...block.matchAll(/^\s*(--[\w-]+):/gm)].map((m) => m[1]).sort();
}

// DESIGN_SYSTEM §2.2 and §3.1–§3.4: every one needs a light and a dark value.
const DESIGN_TOKENS = [
  "canvas",
  "aurora-1",
  "aurora-2",
  "aurora-3",
  "aurora-4",
  "grain",
  "glass",
  "glass-strong",
  "glass-solid",
  "glass-hover",
  "field",
  "field-edge",
  "edge",
  "hairline",
  "wash",
  "overlay",
  "shadow-panel",
  "shadow-raised",
  "foreground",
  "muted-foreground",
  "primary",
  "primary-hover",
  "primary-foreground",
  "ring",
  "income",
  "expense",
  "transfer",
  "warning",
  "danger",
  "danger-foreground",
  "success",
  ...Array.from({ length: 12 }, (_, i) => `cat-${i + 1}`),
  "tint",
].map((name) => `--${name}`);

test("light and dark define the same tokens", () => {
  expect(tokensIn(".dark")).toEqual(tokensIn(":root"));
});

test("every design token is defined", () => {
  expect(tokensIn(":root")).toEqual(expect.arrayContaining(DESIGN_TOKENS));
});

test("every type style is defined", () => {
  // DESIGN_SYSTEM §4
  for (const style of [
    "display",
    "h1",
    "h2",
    "h3",
    "body",
    "body-strong",
    "small",
  ])
    expect(css).toContain(`--text-${style}:`);
});

test("glass inside glass is never blurred again", () => {
  // Either level nested in either level (DESIGN_SYSTEM §2.3).
  expect(css).toContain(
    ":is(.glass, .glass-raised) :is(.glass, .glass-raised) {",
  );
});

test("transparency off makes glass solid", () => {
  const start = css.indexOf("@media (prefers-reduced-transparency: reduce)");
  expect(start).toBeGreaterThanOrEqual(0);
  const block = css.slice(start, css.indexOf("\n}", start));
  for (const part of [".glass", ".glass-raised", "var(--glass-solid)"])
    expect(block).toContain(part);
  expect(block).toMatch(/backdrop-filter:\s*none/);
});
