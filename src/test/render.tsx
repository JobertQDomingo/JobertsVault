import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import type { ReactNode } from "react";
import type { SettingsView } from "@/bindings/SettingsView";
import { queryKeys } from "@/lib/queryKeys";

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
