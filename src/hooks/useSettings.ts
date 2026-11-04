import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getSettings, updateSettings } from "@/api/settings";
import { queryKeys } from "@/lib/queryKeys";

export const useSettings = () =>
  useQuery({ queryKey: queryKeys.settings, queryFn: getSettings });

export function useUpdateSettings() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: updateSettings,
    onSuccess: (saved) => {
      client.setQueryData(queryKeys.settings, saved);
      // The week start and month start day change the totals (ARCHITECTURE §5.2).
      return client.invalidateQueries({ queryKey: queryKeys.ledger });
    },
  });
}
