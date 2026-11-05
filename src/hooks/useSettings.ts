import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getSettings, updateSettings } from "@/api/settings";
import type { Settings } from "@/bindings/Settings";
import type { SettingsView } from "@/bindings/SettingsView";
import { queryKeys } from "@/lib/queryKeys";

export const useSettings = () =>
  useQuery({ queryKey: queryKeys.settings, queryFn: getSettings });

/**
 * Saves a change to the settings. `update_settings` takes the whole set, so the change is laid
 * over the stored settings when the save runs (they're loaded by then: every screen reads them).
 */
export function useUpdateSettings() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (change: Partial<Settings>) => {
      const { decimalsLocked, ...stored } =
        client.getQueryData<SettingsView>(queryKeys.settings) ??
        ({} as SettingsView);
      return updateSettings({ ...stored, ...change });
    },
    // One at a time, each built on the one before: a quick second change keeps the first, and a
    // refused one isn't carried into the next.
    scope: { id: "settings" },
    onSuccess: (saved) => {
      client.setQueryData(queryKeys.settings, saved);
      // The week start and month start day change the totals (ARCHITECTURE §5.2).
      return client.invalidateQueries({ queryKey: queryKeys.ledger });
    },
  });
}
