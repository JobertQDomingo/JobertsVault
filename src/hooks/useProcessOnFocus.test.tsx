import { invoke } from "@tauri-apps/api/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, expect, test, vi } from "vitest";
import { toMidnight, useProcessOnFocus } from "./useProcessOnFocus";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

function setup() {
  const client = new QueryClient();
  client.setQueryData(["ledger", "onScreen"], 1);
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  const view = renderHook(() => useProcessOnFocus(), { wrapper });
  const stale = () =>
    client.getQueryState(["ledger", "onScreen"])?.isInvalidated;
  return { view, stale, client };
}

test.each([
  [0, false],
  [2, true],
])(
  "a focus processes the dates due; %i posted or queued refreshes the ledger: %s",
  async (processed, refreshed) => {
    vi.mocked(invoke).mockResolvedValue(processed);
    const { stale } = setup();
    expect(invoke).not.toHaveBeenCalled();

    window.dispatchEvent(new Event("focus"));

    expect(invoke).toHaveBeenCalledWith("process_recurring", undefined);
    await waitFor(() => expect(stale()).toBe(refreshed));
  },
);

test("a failure still refreshes, since the rules before it may have posted", async () => {
  vi.mocked(invoke).mockRejectedValueOnce({
    code: "database",
    message: "busy",
  });
  const { stale } = setup();
  window.dispatchEvent(new Event("focus"));
  await waitFor(() => expect(stale()).toBe(true));
});

test("it stops listening when the app shell goes", () => {
  vi.mocked(invoke).mockResolvedValue(0);
  const { view } = setup();
  view.unmount();
  window.dispatchEvent(new Event("focus"));
  expect(invoke).not.toHaveBeenCalled();
});

test("a focus that posted nothing still asks for the check-in again (CHK-2)", async () => {
  vi.mocked(invoke).mockResolvedValue(0);
  const { client } = setup();
  client.setQueryData(["ledger", "checkin"], null);
  window.dispatchEvent(new Event("focus"));
  await waitFor(() =>
    expect(client.getQueryState(["ledger", "checkin"])?.isInvalidated).toBe(
      true,
    ),
  );
});

test("the next local midnight is a second after it, whatever the time now", () => {
  expect(toMidnight(new Date(2026, 10, 20, 23, 59, 59))).toBe(2000);
  expect(toMidnight(new Date(2026, 10, 30, 0, 0, 0))).toBe(86_401_000);
});

test("at midnight it processes and asks again, then waits for the next one (ADR 0014)", async () => {
  vi.useFakeTimers({ now: new Date(2026, 10, 20, 23, 59, 0) });
  try {
    vi.mocked(invoke).mockResolvedValue(0);
    setup();
    await vi.advanceTimersByTimeAsync(59_000);
    expect(invoke).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(2_000);
    expect(invoke).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(86_400_000);
    expect(invoke).toHaveBeenCalledTimes(2);
  } finally {
    vi.useRealTimers();
  }
});
