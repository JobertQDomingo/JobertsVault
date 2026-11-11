import { invoke } from "@tauri-apps/api/core";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import type { TransactionFilter } from "@/bindings/TransactionFilter";
import {
  ACCOUNT_LIST,
  CATEGORIES,
  LIST,
  PAYMENT_METHODS,
  PERIOD,
} from "@/test/fixtures";
import { renderRoute, SETTINGS, type Answers } from "@/test/render";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

const EMPTY = {
  ...LIST,
  items: [],
  summary: { count: 0, income: 0, expense: 0 },
  dayNets: [],
};
const ANSWERS: Answers = {
  get_period: PERIOD,
  list_transactions: LIST,
  list_accounts: ACCOUNT_LIST,
  list_categories: CATEGORIES,
  list_payment_methods: PAYMENT_METHODS,
};

/** The filter of the latest `list_transactions` call. */
const lastFilter = () =>
  (
    vi
      .mocked(invoke)
      .mock.calls.filter(([c]) => c === "list_transactions")
      .slice(-1)[0]?.[1] as
      { filter: TransactionFilter; page: number } | undefined
  )?.filter;
const params = (router: { state: { location: { search: string } } }) =>
  Object.fromEntries(new URLSearchParams(router.state.location.search));
const pick = (filter: string, option: string) => {
  fireEvent.click(screen.getByRole("combobox", { name: filter }));
  fireEvent.click(screen.getByRole("option", { name: option }));
};

test("the summary covers every match, and each day shows its net", async () => {
  renderRoute("/transactions", ANSWERS);
  expect(
    await screen.findByText(
      (_, el) =>
        el?.tagName === "P" &&
        el.textContent ===
          "4 transactions · Income +₱4,200.00 · Expense −₱97.70",
    ),
  ).toBeTruthy();
  const today = screen.getByRole("heading", { name: /^Today/ });
  expect(today.textContent).toBe("Today−₱85.20");
  expect(screen.getByRole("heading", { name: /^Yesterday/ }).textContent).toBe(
    "Yesterday+₱4,187.50",
  );
  expect(screen.getByText("Market · weekly shop")).toBeTruthy();
  expect(screen.getByText("Bank → Credit card")).toBeTruthy();
});

/** A row dated after today (TXN-8), which the backend lists first and counts in no figure. */
const RENT = {
  ...LIST.items[0],
  id: 9,
  date: "2026-11-20",
  payee: "Landlord",
  note: null,
};

test("rows after today wait above Today under Upcoming, muted and with no day net", async () => {
  renderRoute("/transactions", {
    ...ANSWERS,
    list_transactions: { ...LIST, items: [RENT, ...LIST.items] },
  });
  const upcoming = await screen.findByRole("heading", { name: "Upcoming" });
  expect(screen.getByRole("heading", { name: "Fri, Nov 20" }).textContent).toBe(
    "Fri, Nov 20",
  );
  const today = screen.getByRole("heading", { name: /^Today/ });
  expect(
    upcoming.compareDocumentPosition(today) & Node.DOCUMENT_POSITION_FOLLOWING,
  ).toBeTruthy();
  const row = (text: string) => screen.getByText(text).closest(".group");
  expect(row("Landlord")?.classList).toContain("text-muted-foreground");
  expect(row("Market · weekly shop")?.classList).not.toContain(
    "text-muted-foreground",
  );
});

test("a period with only upcoming rows lists them, not the empty message", async () => {
  renderRoute("/transactions", {
    ...ANSWERS,
    list_transactions: { ...EMPTY, items: [RENT] },
  });
  expect(await screen.findByRole("heading", { name: "Upcoming" })).toBeTruthy();
  expect(screen.getByText("Landlord")).toBeTruthy();
  expect(screen.queryByText(/Nothing recorded/)).toBeNull();
});

test("clicking a row opens it for editing, and ⧉ duplicates it", async () => {
  renderRoute("/transactions", ANSWERS);
  fireEvent.click(await screen.findByText("Market · weekly shop"));
  expect(
    await screen.findByRole("dialog", { name: "Edit transaction" }),
  ).toBeTruthy();
  fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  fireEvent.click(screen.getAllByRole("button", { name: "Duplicate" })[0]);
  expect(
    await screen.findByRole("dialog", { name: "Add transaction" }),
  ).toBeTruthy();
});

test("search, type, account and category filter the list and live in the URL", async () => {
  const { router } = renderRoute("/transactions?period=week", ANSWERS);
  await screen.findByText("Market · weekly shop");
  fireEvent.change(
    screen.getByRole("searchbox", { name: "Search payee or note" }),
    {
      target: { value: "cafe" },
    },
  );
  pick("Type", "Expense");
  pick("Account", "Credit card");
  pick("Category", "Dining Out");
  await waitFor(() =>
    expect(lastFilter()).toEqual({
      period: { kind: "week", anchor: null },
      kind: "expense",
      accountId: 4,
      categoryId: 2,
      paymentMethodId: null,
      search: "cafe",
    }),
  );
  expect(params(router)).toEqual({
    period: "week",
    search: "cafe",
    type: "expense",
    account: "4",
    category: "2",
  });
  // Archived accounts and methods aren't offered (DESIGN_SYSTEM §9.1).
  fireEvent.click(screen.getByRole("combobox", { name: "Account" }));
  expect(screen.queryByRole("option", { name: "Old Wallet" })).toBeNull();
});

test("the method filter follows the setting, and is ignored while it's off", async () => {
  renderRoute("/transactions?method=2", {
    ...ANSWERS,
    get_settings: { ...SETTINGS, showPaymentMethod: false },
  });
  await screen.findByText("Market · weekly shop");
  await waitFor(() =>
    expect(screen.queryByRole("combobox", { name: "Method" })).toBeNull(),
  );
  await waitFor(() => expect(lastFilter()?.paymentMethodId).toBeNull());
});

test("the method filter, while the setting is on", async () => {
  const { router } = renderRoute("/transactions", ANSWERS);
  await screen.findByText("Market · weekly shop");
  fireEvent.click(screen.getByRole("combobox", { name: "Method" }));
  expect(screen.queryByRole("option", { name: "Old card" })).toBeNull();
  fireEvent.click(screen.getByRole("option", { name: "Credit Card" }));
  await waitFor(() => expect(params(router).method).toBe("2"));
});

test("Clear resets the filters and keeps the period", async () => {
  const { router } = renderRoute(
    "/transactions?period=year&search=cafe&type=income&account=2",
    ANSWERS,
  );
  await screen.findByText("Market · weekly shop");
  fireEvent.click(screen.getByRole("button", { name: "Clear" }));
  await waitFor(() => expect(params(router)).toEqual({ period: "year" }));
});

test("Ctrl+F focuses the search box", async () => {
  renderRoute("/transactions", ANSWERS);
  await screen.findByText("Market · weekly shop");
  fireEvent.keyDown(window, { key: "f", ctrlKey: true });
  expect(document.activeElement).toBe(screen.getByRole("searchbox"));
});

test("scrolling to the end asks for the next 100 once", async () => {
  renderRoute("/transactions", {
    ...ANSWERS,
    list_transactions: ({ page }: { page: number }) =>
      page === 0 ? { ...LIST, hasMore: true } : new Promise(() => {}),
  });
  await screen.findByText("Market · weekly shop");
  const list = screen.getByRole("list", { name: "Transactions" });
  fireEvent.scroll(list);
  fireEvent.scroll(list);
  await waitFor(() =>
    expect(
      vi
        .mocked(invoke)
        .mock.calls.filter(
          ([c, a]) =>
            c === "list_transactions" && (a as { page: number }).page === 1,
        ),
    ).toHaveLength(1),
  );
});

test("an empty period says so, and Go to today goes back", async () => {
  const { router } = renderRoute(
    "/transactions?period=month&anchor=2026-10-01",
    {
      ...ANSWERS,
      get_period: {
        ...PERIOD,
        start: "2026-10-01",
        end: "2026-10-31",
        isCurrent: false,
      },
      list_transactions: EMPTY,
    },
  );
  expect(
    await screen.findByText("Nothing recorded for October 2026."),
  ).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Go to today" }));
  await waitFor(() =>
    expect(params(router)).toEqual({ period: "month", anchor: "2026-11-01" }),
  );
});

test("this period being empty offers no Go to today", async () => {
  renderRoute("/transactions", { ...ANSWERS, list_transactions: EMPTY });
  expect(
    await screen.findByText("Nothing recorded for November 2026."),
  ).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Go to today" })).toBeNull();
});

test("no matches for the filters offers Clear filters", async () => {
  const { router } = renderRoute("/transactions?search=zzz", {
    ...ANSWERS,
    list_transactions: EMPTY,
  });
  expect(
    await screen.findByText("No transactions match these filters."),
  ).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Clear filters" }));
  await waitFor(() => expect(params(router)).toEqual({}));
});

test("if the list can't load, Retry asks again", async () => {
  let fail = true;
  renderRoute("/transactions", {
    ...ANSWERS,
    list_transactions: () => {
      if (fail) throw { code: "database", message: "disk I/O error" };
      return LIST;
    },
  });
  const panel = within(
    (await screen.findByRole("button", { name: "Retry" })).closest<HTMLElement>(
      ".glass",
    )!,
  );
  fail = false;
  fireEvent.click(panel.getByRole("button", { name: "Retry" }));
  expect(await screen.findByText("Market · weekly shop")).toBeTruthy();
});

test("a new filter starts the list at the top, with the old rows marked busy until it loads", async () => {
  renderRoute("/transactions", {
    ...ANSWERS,
    list_transactions: ({ filter }: { filter: TransactionFilter }) =>
      filter.kind ? new Promise(() => {}) : LIST,
  });
  await screen.findByText("Market · weekly shop");
  const before = screen.getByRole("list", { name: "Transactions" });
  pick("Type", "Expense");
  await waitFor(() => expect(lastFilter()?.kind).toBe("expense"));
  const after = screen.getByRole("list", { name: "Transactions" });
  expect(after).not.toBe(before); // a fresh list, scrolled to the top
  expect(after.closest("[aria-busy]")?.getAttribute("aria-busy")).toBe("true");
});

test("with the method field off, a method in the URL leaves nothing to clear", async () => {
  renderRoute("/transactions?method=2", {
    ...ANSWERS,
    get_settings: { ...SETTINGS, showPaymentMethod: false },
    list_transactions: EMPTY,
  });
  expect(
    await screen.findByText("Nothing recorded for November 2026."),
  ).toBeTruthy();
  expect(
    (screen.getByRole("button", { name: "Clear" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
});

test("an archived account in the URL is named in the Account box", async () => {
  renderRoute("/transactions?account=5", ANSWERS);
  await screen.findByText("Market · weekly shop");
  expect(
    (screen.getByRole("combobox", { name: "Account" }) as HTMLInputElement)
      .value,
  ).toBe("Old Wallet (archived)");
});

test("the empty message waits for the new period's name", async () => {
  let release = () => {};
  const october = new Promise((done) => {
    release = () =>
      done({
        ...PERIOD,
        start: "2026-10-01",
        end: "2026-10-31",
        isCurrent: false,
      });
  });
  renderRoute("/transactions", {
    ...ANSWERS,
    get_period: ({ query }: { query: { anchor: string | null } }) =>
      query.anchor === "2026-10-01" ? october : PERIOD,
    list_transactions: ({ filter }: { filter: TransactionFilter }) =>
      filter.period.kind === "month" &&
      "anchor" in filter.period &&
      filter.period.anchor
        ? EMPTY
        : LIST,
  });
  await screen.findByText("Market · weekly shop");
  fireEvent.click(screen.getByRole("button", { name: "Previous period" }));
  await waitFor(() =>
    expect(lastFilter()?.period).toEqual({
      kind: "month",
      anchor: "2026-10-01",
    }),
  );
  // October's (empty) list has arrived; its name hasn't yet.
  await waitFor(() =>
    expect(screen.queryByText("Market · weekly shop")).toBeNull(),
  );
  expect(screen.queryByText(/Nothing recorded for November/)).toBeNull();
  release();
  expect(
    await screen.findByText("Nothing recorded for October 2026."),
  ).toBeTruthy();
});
