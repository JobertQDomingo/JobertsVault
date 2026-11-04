import { invoke } from "@tauri-apps/api/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, expect, test, vi } from "vitest";
import type { NewAccount } from "@/bindings/NewAccount";
import { useCreateAccount } from "./useAccounts";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => vi.mocked(invoke).mockReset());

test("creating an account sends it and refreshes the ledger", async () => {
  const account: NewAccount = {
    kind: "bank",
    name: "Bank",
    openingBalance: 100000,
  };
  vi.mocked(invoke).mockResolvedValueOnce({ id: 2, ...account });
  const client = new QueryClient();
  client.setQueryData(["ledger", "dashboard"], "old totals");
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  const { result } = renderHook(() => useCreateAccount(), { wrapper });

  await result.current.mutateAsync(account);

  expect(invoke).toHaveBeenCalledWith("create_account", { account });
  // A new opening balance changes the net balance (ARCHITECTURE §5.2).
  expect(client.getQueryState(["ledger", "dashboard"])?.isInvalidated).toBe(
    true,
  );
});
