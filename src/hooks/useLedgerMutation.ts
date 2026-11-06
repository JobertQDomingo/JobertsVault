import { useMutation, useQueryClient } from "@tanstack/react-query";
import { queryKeys } from "@/lib/queryKeys";

/**
 * A write that can change any balance or total, so it refreshes every ledger query (ARCHITECTURE
 * §5.2). The settings too: their decimals lock follows whether any amount is stored. `then`
 * runs even if the screen that started it has closed, unlike callbacks passed to `mutate`.
 */
export function useLedgerMutation<TArgs, TResult>(
  mutationFn: (args: TArgs) => Promise<TResult>,
  then: { onSuccess?: () => void; onError?: (error: Error) => void } = {},
) {
  const client = useQueryClient();
  return useMutation({
    mutationFn,
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: queryKeys.ledger }),
        client.invalidateQueries({ queryKey: queryKeys.settings }),
      ]);
      then.onSuccess?.();
    },
    onError: then.onError,
  });
}
