import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { Forecast } from "@/bindings/Forecast";
import { CATEGORIES, DASHBOARD, PERIOD } from "@/test/fixtures";
import { renderRoute } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

// Today in the fixtures is Thu, Nov 5, 2026.
const SAFE: Forecast = {
  ...DASHBOARD.safeToSpend,
  safeToSpend: 840000,
  perDay: 84000,
  windowEnd: "2026-11-14",
  daysLeft: 10,
  heldBack: 150000,
  heldFor: { start: "2026-11-30", end: "2026-12-04" },
};

function show(safeToSpend: Forecast) {
  renderRoute("/", {
    get_dashboard: { ...DASHBOARD, safeToSpend },
    get_period: PERIOD,
    list_categories: CATEGORIES,
  });
}

const card = async () =>
  within(
    (
      await screen.findByText(/^Safe to spend/, { selector: "h2" })
    ).closest<HTMLElement>(".glass")!,
  );

test("the card shows what's safe to spend, a day's share and what's held back (SAFE-6)", async () => {
  show(SAFE);
  const c = await card();
  expect(c.getByText("Safe to spend until Sat, Nov 14")).toBeTruthy();
  expect(c.getByText("₱8,400.00")).toBeTruthy();
  expect(c.getByText("₱840.00 a day · 10 days left")).toBeTruthy();
  expect(c.getByText("₱1,500.00 held back for Nov 30 – Dec 4")).toBeTruthy();
  expect(
    c.getByRole("link", { name: "View forecast" }).getAttribute("href"),
  ).toBe("/forecast");
  expect(c.queryByText(/Paid early/)).toBeNull();
});

test("a window ending today says until today", async () => {
  show({ ...SAFE, windowEnd: SAFE.today, daysLeft: 1 });
  const c = await card();
  expect(c.getByText("Safe to spend until today")).toBeTruthy();
});

test("nothing held back, one day left and a window ending next year", async () => {
  show({
    ...SAFE,
    windowEnd: "2027-11-04",
    daysLeft: 1,
    heldBack: 0,
    heldFor: null,
  });
  const c = await card();
  expect(c.getByText("Safe to spend until Thu, Nov 4, 2027")).toBeTruthy();
  expect(c.getByText("₱840.00 a day · 1 day left")).toBeTruthy();
  expect(c.queryByText(/held back/)).toBeNull();
});

test("a short window shows ₱0.00 and when it runs short, with an icon", async () => {
  show({
    ...SAFE,
    safeToSpend: 0,
    perDay: 0,
    short: { amount: 124000, date: "2026-12-04" },
  });
  const c = await card();
  expect(c.getByText("₱0.00")).toBeTruthy();
  const short = c.getByText("Short by ₱1,240.00 on Fri, Dec 4");
  expect(short.className).toContain("text-danger");
  expect(short.querySelector("svg")).toBeTruthy();
  expect(c.queryByText(/held back/)).toBeNull();
});

test("within 3 days of a payday, Record now opens its payslip (PAY-4)", async () => {
  show({ ...SAFE, earlyPayday: { ruleId: 7, date: "2026-11-07" } });
  const c = await card();
  expect(c.getByText("Payday Sat, Nov 7. Paid early?")).toBeTruthy();
  fireEvent.click(c.getByRole("button", { name: "Record now" }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("get_payslip_defaults", {
      ruleId: 7,
      date: "2026-11-07",
    }),
  );
});

test("with no payday rule, the card says how to set one up", async () => {
  show(DASHBOARD.safeToSpend);
  const c = await card();
  expect(
    c.getByText("Mark your pay as a payday to see what's safe to spend."),
  ).toBeTruthy();
  expect(
    c.getByRole("link", { name: "Open Recurring" }).getAttribute("href"),
  ).toBe("/recurring");
});
