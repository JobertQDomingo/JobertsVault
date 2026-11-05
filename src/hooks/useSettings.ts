import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getSettings, updateSettings } from "@/api/settings";
import type { Settings } from "@/bindings/Settings";
import type { SettingsView } from "@/bindings/SettingsView";
import { queryKeys } from "@/lib/queryKeys";

export const useSettings = () =>
  useQuery({ queryKey: queryKeys.settings, queryFn: getSettings });

/**
 * Saves a change to the settings. `update_settings` takes the whole set, so the change is laid
 * over the stored settings when the save runs. Without them loaded it refuses rather than send a
 * partial set.
 */
export function useUpdateSettings() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (change: Partial<Settings>) => {
      // A reload already on its way read the old values; its answer mustn't land after this one.
      await client.cancelQueries({ queryKey: queryKeys.settings });
      const current = client.getQueryData<SettingsView>(queryKeys.settings);
      if (!current) throw new Error("The settings haven't loaded yet.");
      const { decimalsLocked, ...stored } = current;
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
