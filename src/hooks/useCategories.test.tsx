import { invoke } from "@tauri-apps/api/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, expect, test, vi } from "vitest";
import { useMergeCategories } from "./useCategories";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => vi.mocked(invoke).mockReset());

test("merging categories sends both ids and refreshes the ledger", async () => {
  vi.mocked(invoke).mockResolvedValueOnce(null);
  const client = new QueryClient();
  client.setQueryData(["ledger", "dashboard"], "old totals");
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  const { result } = renderHook(() => useMergeCategories(), { wrapper });

  await result.current.mutateAsync({ fromId: 8, intoId: 9 });

  expect(invoke).toHaveBeenCalledWith("merge_categories", {
    fromId: 8,
    intoId: 9,
  });
  // Moving transactions changes the totals by category (ARCHITECTURE §5.2).
  expect(client.getQueryState(["ledger", "dashboard"])?.isInvalidated).toBe(
    true,
  );
});
