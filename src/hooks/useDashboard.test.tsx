import { invoke } from "@tauri-apps/api/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, expect, test, vi } from "vitest";
import type { PeriodQuery } from "@/bindings/PeriodQuery";
import { useDashboard, usePeriod } from "./useDashboard";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => vi.mocked(invoke).mockReset());

const query: PeriodQuery = {
  kind: "custom",
  start: "2026-10-01",
  end: "2026-10-10",
};

function setup() {
  const client = new QueryClient();
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return { client, wrapper };
}

test.each([
  ["get_dashboard", useDashboard],
  ["get_period", usePeriod],
] as const)(
  "%s loads by query and reloads after a ledger write",
  async (command, hook) => {
    vi.mocked(invoke).mockResolvedValue({});
    const { client, wrapper } = setup();
    const { result } = renderHook(() => hook(query), { wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    await client.invalidateQueries({ queryKey: ["ledger"] });

    expect(invoke).toHaveBeenCalledTimes(2);
    expect(invoke).toHaveBeenCalledWith(command, { query });
  },
);
