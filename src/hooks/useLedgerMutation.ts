import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { Saved } from "@/bindings/Saved";
import { notify } from "@/components/ui/toast";
import { formatDate } from "@/lib/dates";
import { queryKeys } from "@/lib/queryKeys";

/**
 * A write that can change any balance or total, so it refreshes every ledger query (ARCHITECTURE
 * §5.2). The settings too: their decimals lock follows whether any amount is stored. `then`
 * runs even if the screen that started it has closed, unlike callbacks passed to `mutate`. A
 * save that moved goals' dates says so (GOL-7).
 */
export function useLedgerMutation<TArgs, TResult>(
  mutationFn: (args: TArgs) => Promise<TResult>,
  then: { onSuccess?: () => void; onError?: (error: Error) => void } = {},
) {
  const client = useQueryClient();
  return useMutation({
    mutationFn,
    onSuccess: async (result) => {
      await Promise.all([
        client.invalidateQueries({ queryKey: queryKeys.ledger }),
        client.invalidateQueries({ queryKey: queryKeys.settings }),
      ]);
      tellGoalsMoved(result);
      then.onSuccess?.();
    },
    onError: then.onError,
  });
}

/** One line per goal moved (UI_FLOWS §8.4). Results of other writes have no `goalsMoved`. */
function tellGoalsMoved(result: unknown) {
  // Picked from the generated type, so a renamed field fails the typecheck.
  const moved = (result as Partial<Pick<Saved<object>, "goalsMoved">> | null)
    ?.goalsMoved;
  if (!moved?.length) return;
  notify.info(
    moved
      .map(
        (g) =>
          `${g.name} now aims for ${formatDate(g.to)} (was ${formatDate(g.from)}), so it stays realistic.`,
      )
      .join("\n"),
  );
}
