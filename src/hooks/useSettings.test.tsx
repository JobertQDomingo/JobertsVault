import { invoke } from "@tauri-apps/api/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, expect, test, vi } from "vitest";
import type { Settings } from "@/bindings/Settings";
import { useUpdateSettings } from "./useSettings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

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

function renderUpdate(client: QueryClient) {
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return renderHook(() => useUpdateSettings(), { wrapper }).result;
}

beforeEach(() => vi.mocked(invoke).mockReset());

test("saves run one at a time, so the last change is the one kept", async () => {
  let finishFirst: (value: unknown) => void = () => {};
  vi.mocked(invoke)
    .mockReturnValueOnce(new Promise((resolve) => (finishFirst = resolve)))
    .mockResolvedValueOnce(saved);
  const result = renderUpdate(new QueryClient());

  const first = result.current.mutateAsync({
    ...settings,
    weekStart: "monday",
  });
  const second = result.current.mutateAsync(settings);
  await new Promise((resolve) => setTimeout(resolve, 10));
  expect(invoke).toHaveBeenCalledTimes(1);

  finishFirst(saved);
  await Promise.all([first, second]);
  expect(invoke).toHaveBeenCalledTimes(2);
});

test("saving stores the returned settings and refreshes the ledger", async () => {
  vi.mocked(invoke).mockResolvedValueOnce(saved);
  const client = new QueryClient();
  client.setQueryData(["ledger", "dashboard"], "old totals");

  const result = renderUpdate(client);
  await result.current.mutateAsync(settings);

  expect(invoke).toHaveBeenCalledWith("update_settings", { settings });
  expect(client.getQueryData(["settings"])).toEqual(saved);
  // A new week start or month start day changes every total (ARCHITECTURE §5.2).
  expect(client.getQueryState(["ledger", "dashboard"])?.isInvalidated).toBe(
    true,
  );
});
