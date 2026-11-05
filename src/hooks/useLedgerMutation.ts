import { useMutation, useQueryClient } from "@tanstack/react-query";
import { queryKeys } from "@/lib/queryKeys";

/**
 * A write that can change any balance or total, so it refreshes every ledger query (ARCHITECTURE
 * §5.2). The settings too: their decimals lock follows whether any amount is stored.
 */
export function useLedgerMutation<TArgs, TResult>(
  mutationFn: (args: TArgs) => Promise<TResult>,
) {
  const client = useQueryClient();
  return useMutation({
    mutationFn,
    onSuccess: () =>
      Promise.all([
        client.invalidateQueries({ queryKey: queryKeys.ledger }),
        client.invalidateQueries({ queryKey: queryKeys.settings }),
      ]),
  });
}
