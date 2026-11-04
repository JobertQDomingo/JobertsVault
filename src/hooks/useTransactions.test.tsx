import { invoke } from "@tauri-apps/api/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, expect, test, vi } from "vitest";
import type { TransactionFields } from "@/bindings/TransactionFields";
import type { TransactionFilter } from "@/bindings/TransactionFilter";
import type { TransactionList } from "@/bindings/TransactionList";
import { useCreateTransaction, useTransactions } from "./useTransactions";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => vi.mocked(invoke).mockReset());

function setup() {
  const client = new QueryClient();
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return { client, wrapper };
}

const page = (hasMore: boolean): TransactionList => ({
  items: [],
  hasMore,
  summary: { count: 0, income: 0, expense: 0 },
  dayNets: [],
});

test("the list asks for the next page while there is more", async () => {
  const filter: TransactionFilter = {
    period: { kind: "month", anchor: null },
    kind: null,
    accountId: null,
    categoryId: null,
    paymentMethodId: null,
    search: null,
  };
  vi.mocked(invoke)
    .mockResolvedValueOnce(page(true))
    .mockResolvedValueOnce(page(false));
  const { wrapper } = setup();
  const { result } = renderHook(() => useTransactions(filter), { wrapper });

  await waitFor(() => expect(result.current.hasNextPage).toBe(true));
  await result.current.fetchNextPage();
  await waitFor(() => expect(result.current.hasNextPage).toBe(false));

  expect(invoke).toHaveBeenNthCalledWith(1, "list_transactions", {
    filter,
    page: 0,
  });
  expect(invoke).toHaveBeenNthCalledWith(2, "list_transactions", {
    filter,
    page: 1,
  });
});

test("creating a transaction refreshes the ledger", async () => {
  const transaction: TransactionFields = {
    kind: "expense",
    amount: 2550,
    date: "2026-11-03",
    accountId: 1,
    toAccountId: null,
    categoryId: 8,
    payee: null,
    paymentMethodId: null,
    note: null,
  };
  vi.mocked(invoke).mockResolvedValueOnce({ id: 1, ...transaction });
  const { client, wrapper } = setup();
  client.setQueryData(["ledger", "accounts"], "old balances");
  const { result } = renderHook(() => useCreateTransaction(), { wrapper });

  await result.current.mutateAsync(transaction);

  expect(invoke).toHaveBeenCalledWith("create_transaction", { transaction });
  expect(client.getQueryState(["ledger", "accounts"])?.isInvalidated).toBe(
    true,
  );
});
