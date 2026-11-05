import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { expect, test } from "vitest";
import { queryKeys } from "@/lib/queryKeys";
import { SETTINGS } from "@/test/render";
import { useTransactionFilter } from "./useTransactionFilter";

function setup(url: string, showPaymentMethod = true) {
  const result = {
    current: null as unknown as ReturnType<typeof useTransactionFilter>,
  };
  function Probe() {
    result.current = useTransactionFilter();
    return null;
  }
  const router = createMemoryRouter([{ path: "/", element: <Probe /> }], {
    initialEntries: [url],
  });
  const client = new QueryClient({
    defaultOptions: { queries: { staleTime: Infinity } },
  });
  client.setQueryData(queryKeys.settings, { ...SETTINGS, showPaymentMethod });
  render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  const params = () =>
    Object.fromEntries(new URLSearchParams(router.state.location.search));
  return { result, router, params };
}

const NONE = {
  kind: null,
  accountId: null,
  categoryId: null,
  paymentMethodId: null,
  search: null,
};

test("reads every filter and the period from the URL", () => {
  const { result } = setup(
    "/?period=week&anchor=2026-10-26&search=cafe&type=expense&account=4&category=2&method=3",
  );
  expect(result.current[0]).toEqual({
    period: { kind: "week", anchor: "2026-10-26" },
    kind: "expense",
    accountId: 4,
    categoryId: 2,
    paymentMethodId: 3,
    search: "cafe",
  });
});

test("ignores values that aren't real filters", () => {
  const { result } = setup(
    "/?type=bogus&account=abc&category=-1&method=2.5&search=",
  );
  expect(result.current[0]).toEqual({
    period: { kind: "month", anchor: null },
    ...NONE,
  });
});

test("a change keeps the period and the other filters", () => {
  const { result, params } = setup("/?period=year&search=cafe&type=expense");
  act(() => result.current[1]({ accountId: 2 }));
  expect(params()).toEqual({
    period: "year",
    search: "cafe",
    type: "expense",
    account: "2",
  });
  act(() => result.current[1]({ kind: null }));
  expect(params()).toEqual({ period: "year", search: "cafe", account: "2" });
});

test("clearing keeps only the period", () => {
  const { result, params } = setup(
    "/?period=day&search=cafe&type=income&account=2&category=3&method=1",
  );
  act(() => result.current[2]());
  expect(params()).toEqual({ period: "day" });
});

test("typing a search replaces the history entry instead of adding one per key", () => {
  const { result, router } = setup("/?period=month");
  act(() => result.current[1]({ accountId: 2 }));
  const after = router.state.historyAction;
  act(() => result.current[1]({ search: "c" }, { replace: true }));
  act(() => result.current[1]({ search: "ca" }, { replace: true }));
  expect(after).toBe("PUSH");
  expect(router.state.historyAction).toBe("REPLACE");
  expect(result.current[0].search).toBe("ca");
});

test("a search of only spaces is no search, and spaces around one don't count", () => {
  expect(setup("/?search=%20%20").result.current[0].search).toBeNull();
  expect(setup("/?search=%20cafe%20").result.current[0].search).toBe("cafe");
});

test("with the payment method field off, a method in the URL is ignored", () => {
  expect(
    setup("/?method=2", false).result.current[0].paymentMethodId,
  ).toBeNull();
});
