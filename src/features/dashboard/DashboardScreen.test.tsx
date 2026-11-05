import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import { CATEGORIES, DASHBOARD, PERIOD } from "@/test/fixtures";
import { renderRoute, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const ANSWERS: Answers = {
  get_dashboard: DASHBOARD,
  get_period: PERIOD,
  list_categories: CATEGORIES,
};

/** The glass panel holding a heading or label. */
const panel = (name: string) =>
  within(screen.getByText(name).closest<HTMLElement>(".glass")!);

test("the stat cards show the period's income, expense and net cashflow", async () => {
  renderRoute("/", ANSWERS);
  await screen.findByText("Net cashflow");
  expect(panel("Income").getByText("+₱4,200.00")).toBeTruthy();
  expect(panel("Expense").getByText("−₱2,950.40")).toBeTruthy();
  expect(panel("Net cashflow").getByText("+₱1,249.60")).toBeTruthy();
});

test("the 5 newest transactions, grouped by day", async () => {
  renderRoute("/", ANSWERS);
  await screen.findByText("Recent transactions");
  const recent = panel("Recent transactions");
  for (const text of [
    "Today",
    "Yesterday",
    "Mon, Oct 26",
    "Bank → Credit card",
    "Dining Out · Corner Cafe",
  ])
    expect(recent.getByText(text)).toBeTruthy();
  expect(await recent.findByText("Food & Groceries · Market")).toBeTruthy();
  expect(recent.getAllByRole("button")).toHaveLength(5);
});

test("clicking a recent transaction opens it for editing", async () => {
  renderRoute("/", ANSWERS);
  fireEvent.click(await screen.findByText("Food & Groceries · Market"));
  expect(
    await screen.findByRole("dialog", { name: "Edit transaction" }),
  ).toBeTruthy();
});

test("View all goes to Transactions, Manage to Accounts", async () => {
  const { router } = renderRoute("/", ANSWERS);
  fireEvent.click(await screen.findByRole("link", { name: "View all" }));
  await waitFor(() =>
    expect(router.state.location.pathname).toBe("/transactions"),
  );
  await router.navigate("/");
  fireEvent.click(await screen.findByRole("link", { name: "Manage" }));
  await waitFor(() => expect(router.state.location.pathname).toBe("/accounts"));
});

test("accounts: the active ones, the card's details and the backend's net balance", async () => {
  renderRoute("/", ANSWERS);
  await screen.findByText("Net balance");
  const accounts = panel("Net balance");
  const names = accounts
    .getAllByRole("link")
    .filter((a) => a.getAttribute("href")?.startsWith("/accounts/"))
    .map((a) => a.querySelector("p")?.textContent);
  expect(names).toEqual(["Cash", "Bank", "E-wallet", "Credit card"]);
  expect(accounts.getByText("₱640.00 owed")).toBeTruthy();
  expect(accounts.getByText(/Due Nov 5/)).toBeTruthy();
  // The net balance includes the archived Old Wallet, so it isn't the rows' sum.
  expect(accounts.getByText("₱8,285.75")).toBeTruthy();
});

test("clicking an account opens its details", async () => {
  const { router } = renderRoute("/", ANSWERS);
  fireEvent.click(await screen.findByRole("link", { name: /^Bank/ }));
  await waitFor(() =>
    expect(router.state.location.pathname).toBe("/accounts/2"),
  );
});

test("with no transactions yet, it says how to add one and the totals are 0", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: { ...DASHBOARD, income: 0, expense: 0, net: 0, recent: [] },
  });
  expect(
    await screen.findByText(
      "No transactions yet. Press Ctrl+N to add your first one.",
    ),
  ).toBeTruthy();
  expect(screen.getAllByText("₱0.00")).toHaveLength(3);
  fireEvent.click(
    panel("Recent transactions").getByRole("button", {
      name: /Add transaction/,
    }),
  );
  expect(
    await screen.findByRole("dialog", { name: "Add transaction" }),
  ).toBeTruthy();
});

test("if the dashboard can't load, Retry asks again", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: () => {
      throw { code: "database", message: "disk I/O error" };
    },
  });
  fireEvent.click(await screen.findByRole("button", { name: "Retry" }));
  await waitFor(() =>
    expect(
      vi.mocked(invoke).mock.calls.filter(([c]) => c === "get_dashboard"),
    ).toHaveLength(2),
  );
});

test("switching period keeps the totals until the new ones arrive", async () => {
  renderRoute("/", {
    ...ANSWERS,
    get_dashboard: ({ query }: { query: PeriodQuery }) =>
      query.kind === "month" && query.anchor
        ? new Promise(() => {})
        : DASHBOARD,
  });
  await screen.findByText("Net cashflow");
  fireEvent.click(screen.getByRole("button", { name: "Previous period" }));
  await waitFor(() =>
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("get_dashboard", {
      query: { kind: "month", anchor: "2026-10-01" },
    }),
  );
  expect(panel("Income").getByText("+₱4,200.00")).toBeTruthy();
});

test("if a refresh fails, the error replaces the old numbers", async () => {
  let fail = false;
  const { client } = renderRoute("/", {
    ...ANSWERS,
    get_dashboard: () => {
      if (fail) throw { code: "database", message: "disk I/O error" };
      return DASHBOARD;
    },
  });
  await screen.findByText("Net cashflow");
  fail = true;
  await client.invalidateQueries({ queryKey: ["ledger"] });
  expect(await screen.findByRole("button", { name: "Retry" })).toBeTruthy();
  expect(screen.queryByText("Net cashflow")).toBeNull();
});
