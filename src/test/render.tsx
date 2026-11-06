import { invoke } from "@tauri-apps/api/core";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import type { ReactNode } from "react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { vi } from "vitest";
import type { SettingsView } from "@/bindings/SettingsView";
import { createQueryClient } from "@/lib/queryClient";
import { queryKeys } from "@/lib/queryKeys";
import { Toaster } from "@/components/ui/toast";
import { routes } from "@/routes";

export const SETTINGS: SettingsView = {
  decimalsLocked: false,
  currencySymbol: "₱",
  currencyPosition: "before",
  currencyDecimals: 2,
  weekStart: "monday",
  monthStartDay: 1,
  showPaymentMethod: true,
  onboardingCompleted: true,
  autoUpdateEnabled: true,
  comparisonMode: "to_date",
};

/** Renders inside a QueryClient that already holds the settings, as the app does once started. */
export function renderWithSettings(
  ui: ReactNode,
  overrides: Partial<SettingsView> = {},
) {
  const client = new QueryClient({
    defaultOptions: { queries: { staleTime: Infinity, retry: false } },
  });
  client.setQueryData(queryKeys.settings, { ...SETTINGS, ...overrides });
  return render(
    <QueryClientProvider client={client}>{ui}</QueryClientProvider>,
  );
}

/** A command's answer: a value, or a function of its arguments (throw to reject). */
export type Answers = Record<string, unknown>;

/**
 * Renders the app's routes at `url`, the way the running app does once started. `invoke` (mocked
 * by the test file) answers each command from `answers`; any other command rejects. The settings
 * start in the cache unless `seedSettings` is false (then they load like any query).
 */
export function renderRoute(
  url: string,
  answers: Answers = {},
  { seedSettings = true } = {},
) {
  vi.mocked(invoke).mockImplementation((command: string, args?: unknown) =>
    Promise.resolve().then(() => {
      const all: Answers = { get_settings: SETTINGS, ...answers };
      // A command the test didn't expect fails loudly instead of looking like an empty load.
      if (!(command in all)) throw new Error(`unanswered command: ${command}`);
      const answer = all[command];
      return typeof answer === "function" ? answer(args) : answer;
    }),
  );
  const client = createQueryClient();
  if (seedSettings) client.setQueryData(queryKeys.settings, SETTINGS);
  const router = createMemoryRouter(routes, { initialEntries: [url] });
  render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
      <Toaster />
    </QueryClientProvider>,
  );
  return { router, client };
}
