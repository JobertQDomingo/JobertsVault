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

/** The declarations of the rule whose selector is exactly `selector`. */
function rule(selector: string) {
  const start = css.indexOf(`\n${selector} {`);
  expect(start, selector).toBeGreaterThanOrEqual(0);
  return css.slice(start, css.indexOf("\n}", start));
}

test("a panel inside a panel is a flat tint", () => {
  // DESIGN_SYSTEM §2.3, §9.1 Toolbar.
  const flat = rule(":is(.glass, .glass-raised) .glass");
  expect(flat).toContain("background: var(--wash)");
  expect(flat).toMatch(/backdrop-filter:\s*none/);
});

test("a raised element inside a panel keeps its material but isn't blurred again", () => {
  // An inline menu must stay readable over the card (§2.1), with one layer of blur (§2.3).
  const raised = rule(":is(.glass, .glass-raised) .glass-raised");
  expect(raised).toMatch(/backdrop-filter:\s*none/);
  expect(raised).not.toContain("background");
});

test("Windows and the switch share one transparency-off variant (§2.4, SET-8)", () => {
  const variant = rule("@custom-variant transparency-off");
  expect(variant).toContain("@media (prefers-reduced-transparency: reduce)");
  expect(variant).toContain(
    "&:where(.reduce-transparency, .reduce-transparency *)",
  );
  // Nothing else asks Windows, so the switch and the setting can't drift apart.
  expect(css.split("prefers-reduced-transparency")).toHaveLength(2);
});

test("transparency off makes glass solid and blurs nothing", () => {
  const off = /@variant transparency-off \{\s*([^}]*)\}/;
  expect(rule(".glass,\n.glass-raised").match(off)?.[1]).toContain(
    "background: var(--glass-solid)",
  );
  expect(rule("*").match(off)?.[1]).toContain("backdrop-filter: none");
  expect(
    rule(":is(.glass-warning, .glass-info, .glass-danger)").match(off)?.[1],
  ).toContain("var(--glass-solid)");
});

test("the danger banner has its own tint, deeper in dark mode (§3.3, LOAN-4)", () => {
  // The light pink --danger at 14% read as plain glass over the dark aurora.
  for (const block of [":root", ".dark"])
    expect(tokensIn(block)).toEqual(
      expect.arrayContaining(["--danger-banner", "--danger-banner-mix"]),
    );
  const dark = rule(".dark");
  expect(dark).toContain("--danger-banner: #dc2626;");
  expect(dark).toContain("--danger-banner-mix: 30%;");
  expect(rule(".glass-danger")).toContain("--banner: var(--danger-banner)");
  expect(rule(".glass-danger")).toContain(
    "--banner-mix: var(--danger-banner-mix)",
  );
  // Each tone sets its own amount, so a banner never inherits another's.
  for (const tone of [".glass-warning", ".glass-info"])
    expect(rule(tone)).toContain("--banner-mix: 14%");
});

test("a banner's border is tinted with its color (§3.3)", () => {
  expect(rule(":is(.glass-warning, .glass-info, .glass-danger)")).toContain(
    "border-color: color-mix(in srgb, var(--banner) 35%, var(--edge))",
  );
});
