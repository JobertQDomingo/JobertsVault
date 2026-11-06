import { invoke } from "@tauri-apps/api/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { getSettings } from "@/api/settings";
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
  comparisonMode: "to_date",
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
  const client = new QueryClient();
  client.setQueryData(["settings"], saved);
  const result = renderUpdate(client);

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
  client.setQueryData(["settings"], saved);
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

test("a change is laid over the stored settings", async () => {
  vi.mocked(invoke).mockResolvedValueOnce({ ...saved, weekStart: "monday" });
  const client = new QueryClient();
  client.setQueryData(["settings"], saved);
  const result = renderUpdate(client);
  await result.current.mutateAsync({ weekStart: "monday" });
  expect(invoke).toHaveBeenCalledWith("update_settings", {
    settings: { ...settings, weekStart: "monday" },
  });
});

test("a change with no settings loaded fails clearly instead of sending a partial set", async () => {
  const result = renderUpdate(new QueryClient());
  await expect(
    result.current.mutateAsync({ onboardingCompleted: true }),
  ).rejects.toThrow("The settings haven't loaded yet");
  expect(invoke).not.toHaveBeenCalled();
});

test("a save cancels a settings reload already on its way, so its old answer can't overwrite", async () => {
  let answerReload: (value: unknown) => void = () => {};
  vi.mocked(invoke).mockImplementation((command: string) =>
    command === "get_settings"
      ? new Promise((resolve) => (answerReload = resolve))
      : Promise.resolve({ ...saved, weekStart: "monday" }),
  );
  const client = new QueryClient();
  client.setQueryData(["settings"], saved);
  const reload = client.fetchQuery({
    queryKey: ["settings"],
    queryFn: getSettings,
    staleTime: 0,
  });
  const result = renderUpdate(client);
  await result.current.mutateAsync({ weekStart: "monday" });
  answerReload(saved); // read before the save
  await reload.catch(() => {});
  expect(client.getQueryData(["settings"])).toEqual({
    ...saved,
    weekStart: "monday",
  });
});
