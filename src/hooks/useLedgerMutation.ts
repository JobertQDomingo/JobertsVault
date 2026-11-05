import { useMutation, useQueryClient } from "@tanstack/react-query";
import { queryKeys } from "@/lib/queryKeys";

/**
 * A write that can change any balance or total, so it refreshes every ledger query (ARCHITECTURE
 * §5.2). `before` runs after the write and before that refresh, e.g. to leave a deleted record's
 * screen so it isn't fetched again.
 */
export function useLedgerMutation<TArgs, TResult>(
  mutationFn: (args: TArgs) => Promise<TResult>,
  before?: (args: TArgs) => void,
) {
  const client = useQueryClient();
  return useMutation({
    mutationFn,
    onSuccess: (_result, args) => {
      before?.(args);
      return client.invalidateQueries({ queryKey: queryKeys.ledger });
    },
  });
}
