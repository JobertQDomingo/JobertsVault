import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { Forecast } from "@/bindings/Forecast";
import type { ForecastMovement } from "@/bindings/ForecastMovement";
import type { ForecastRow } from "@/bindings/ForecastRow";
import { ACCOUNT_LIST, DASHBOARD, TODAY } from "@/test/fixtures";
import { renderRoute } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

// Today in the fixtures is Thu, Nov 5, 2026, inside the window that started on payday Nov 1.
const move = (
  over: Partial<ForecastMovement> & Pick<ForecastMovement, "date" | "label">,
): ForecastMovement => ({
  kind: "recurring",
  id: 1,
  amount: 100000,
  moneyIn: false,
  accountId: 2,
  ...over,
});

const row = (
  over: Partial<ForecastRow> & Pick<ForecastRow, "start" | "end">,
): ForecastRow => ({
  startAmount: 0,
  in: 0,
  out: 0,
  left: 0,
  short: false,
  lowest: false,
  received: [],
  movements: [],
  ...over,
});

const WINDOWS: Forecast = {
  ...DASHBOARD.safeToSpend,
  safeToSpend: 0,
  perDay: 0,
  windowEnd: "2026-11-14",
  daysLeft: 10,
  heldBack: 990000,
  heldFor: { start: "2026-12-15", end: "2026-12-29" },
  short: { amount: 60000, date: "2026-12-20" },
  rows: [
    row({
      start: "2026-11-01",
      end: "2026-11-14",
      startAmount: 1250000,
      out: 260000,
      left: 990000,
      received: [{ payslipId: 40, date: "2026-11-01", net: 1050000 }],
      movements: [
        move({
          date: "2026-11-10",
          kind: "loan",
          id: 14,
          label: "E-wallet loan",
          accountId: 3,
        }),
        move({
          date: "2026-11-12",
          label: "Utilities · Water bill",
          amount: 160000,
        }),
      ],
    }),
    row({
      start: "2026-11-15",
      end: "2026-11-29",
      startAmount: 990000,
      in: 1050000,
      out: 1200000,
      left: 840000,
      movements: [
        move({
          date: "2026-11-15",
          kind: "payday",
          label: "Salary (expected)",
          amount: 1050000,
          moneyIn: true,
        }),
        move({
          date: "2026-11-20",
          kind: "card",
          id: 4,
          label: "Credit card payment",
          amount: 1200000,
          accountId: 4,
        }),
      ],
    }),
    row({
      start: "2026-11-30",
      end: "2026-12-14",
      startAmount: 840000,
      left: 840000,
    }),
    row({
      start: "2026-12-15",
      end: "2026-12-29",
      startAmount: 840000,
      out: 900000,
      left: -60000,
      short: true,
      lowest: true,
      movements: [
        move({
          date: "2026-12-20",
          kind: "transaction",
          label: "Food & Groceries",
          amount: 900000,
        }),
      ],
    }),
  ],
};

const MONTHS: Forecast = {
  ...WINDOWS,
  rows: [
    row({ start: TODAY, end: "2026-11-30", startAmount: 1250000 }),
    row({ start: "2026-12-01", end: "2026-12-31", startAmount: 840000 }),
  ],
};

function show(url = "/forecast", forecast?: Forecast) {
  return renderRoute(url, {
    get_forecast: ({ grain }: { grain: string }) =>
      forecast ?? (grain === "month" ? MONTHS : WINDOWS),
    list_accounts: ACCOUNT_LIST,
  });
}

const rowButton = (name: RegExp) => screen.findByRole("button", { name });

test("rows show their dates, figures and the pay received (SAFE-7)", async () => {
  show();
  // The current window starts today.
  const first = (await rowButton(/^Nov 5–14/)).closest("tr")!;
  const cells = within(first);
  expect(cells.getByText("₱12,500.00")).toBeTruthy();
  expect(cells.getByText("—")).toBeTruthy();
  expect(cells.getByText("₱2,600.00")).toBeTruthy();
  expect(cells.getByText("₱9,900.00")).toBeTruthy();
  expect(cells.getByText("₱10,500.00 received Nov 1")).toBeTruthy();
  expect(screen.getByRole("columnheader", { name: "Window" })).toBeTruthy();
  expect(invoke).toHaveBeenCalledWith("get_forecast", { grain: "window" });
  // The top panel is the Safe to spend card, without a link to this screen.
  expect(screen.getByText("Safe to spend until Sat, Nov 14")).toBeTruthy();
  expect(screen.queryByRole("link", { name: "View forecast" })).toBeNull();
});

test("the lowest row has a badge, and a short row says Short in danger with an icon", async () => {
  show();
  const short = (await rowButton(/^Dec 15–29/)).closest("tr")!;
  expect(within(short).getByText("lowest")).toBeTruthy();
  const left = within(short).getByText(/Short/).closest("td")!;
  expect(left.textContent).toBe("−₱600.00 Short");
  expect(left.querySelector(".text-danger svg")).toBeTruthy();
  const plain = (await rowButton(/^Nov 15–29/)).closest("tr")!;
  expect(within(plain).queryByText("lowest")).toBeNull();
  expect(within(plain).queryByText(/Short/)).toBeNull();
});

test("clicking a row lists its movements, and opening another closes it", async () => {
  show();
  const second = await rowButton(/^Nov 15–29/);
  fireEvent.click(second.closest("tr")!);
  expect(second.getAttribute("aria-expanded")).toBe("true");
  const pay = screen
    .getByText("Sun, Nov 15 · Salary (expected)")
    .closest("tr")!;
  expect(within(pay).getByText("Bank")).toBeTruthy();
  expect(within(pay).getByText("+₱10,500.00")).toBeTruthy();
  const card = screen
    .getByText("Fri, Nov 20 · Credit card payment")
    .closest("tr")!;
  expect(within(card).getByText("Credit card")).toBeTruthy();
  expect(within(card).getByText("−₱12,000.00")).toBeTruthy();

  const first = await rowButton(/^Nov 5–14/);
  fireEvent.click(first);
  expect(first.getAttribute("aria-expanded")).toBe("true");
  expect(second.getAttribute("aria-expanded")).toBe("false");
  expect(screen.queryByText(/Salary \(expected\)/)).toBeNull();
  expect(screen.getByText("Tue, Nov 10 · E-wallet loan")).toBeTruthy();

  fireEvent.click(first);
  expect(first.getAttribute("aria-expanded")).toBe("false");
  expect(screen.queryByText(/E-wallet loan/)).toBeNull();
});

test("an open row with nothing due says so", async () => {
  show();
  fireEvent.click(await rowButton(/^Nov 30 – Dec 14/));
  expect(screen.getByText("Nothing due.")).toBeTruthy();
});

test("Months is kept in the URL and asks for months", async () => {
  const { router } = show("/forecast?view=months");
  fireEvent.click(await rowButton(/^Nov 5–30/));
  expect(screen.getByRole("columnheader", { name: "Month" })).toBeTruthy();
  expect(invoke).toHaveBeenCalledWith("get_forecast", { grain: "month" });

  fireEvent.click(screen.getByRole("radio", { name: "Pay windows" }));
  expect(router.state.location.search).toBe("");
  await rowButton(/^Nov 5–14/);
  // The other view starts with nothing open.
  expect(screen.queryByText("Nothing due.")).toBeNull();
  expect(screen.queryAllByRole("button", { expanded: true })).toHaveLength(0);

  fireEvent.click(screen.getByRole("radio", { name: "Months" }));
  expect(router.state.location.search).toBe("?view=months");
});

test("an unknown view shows the pay windows", async () => {
  show("/forecast?view=weeks");
  await rowButton(/^Nov 5–14/);
  expect(
    screen
      .getByRole("radio", { name: "Pay windows" })
      .getAttribute("aria-checked"),
  ).toBe("true");
});

test("with no payday rule the screen is one empty state", async () => {
  show("/forecast", DASHBOARD.safeToSpend);
  expect(
    await screen.findByText(
      "Mark your pay as a payday to see what's safe to spend.",
    ),
  ).toBeTruthy();
  expect(
    screen.getByRole("link", { name: "Open Recurring" }).getAttribute("href"),
  ).toBe("/recurring");
  expect(screen.queryByText("Pay windows")).toBeNull();
  expect(screen.queryByText(/^Safe to spend/)).toBeNull();
});
