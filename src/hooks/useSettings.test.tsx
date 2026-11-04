import { invoke } from "@tauri-apps/api/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { expect, test, vi } from "vitest";
import type { Settings } from "@/bindings/Settings";
import { useUpdateSettings } from "./useSettings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

test("saving stores the returned settings and refreshes the ledger", async () => {
  const settings: Settings = {
    currencySymbol: "₱",
    currencyPosition: "before",
    currencyDecimals: 2,
    weekStart: "sunday",
    monthStartDay: 25,
    showPaymentMethod: true,
    onboardingCompleted: true,
    autoUpdateEnabled: true,
  };
  const saved = { ...settings, decimalsLocked: false };
  vi.mocked(invoke).mockResolvedValueOnce(saved);
  const client = new QueryClient();
  client.setQueryData(["ledger", "dashboard"], "old totals");
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );

  const { result } = renderHook(() => useUpdateSettings(), { wrapper });
  await result.current.mutateAsync(settings);

  expect(invoke).toHaveBeenCalledWith("update_settings", { settings });
  expect(client.getQueryData(["settings"])).toEqual(saved);
  // A new week start or month start day changes every total (ARCHITECTURE §5.2).
  expect(client.getQueryState(["ledger", "dashboard"])?.isInvalidated).toBe(
    true,
  );
});
