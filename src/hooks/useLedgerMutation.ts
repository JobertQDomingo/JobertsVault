import {
  type QueryClient,
  useMutation,
  useQueryClient,
} from "@tanstack/react-query";
import { getVaultStatus } from "@/api/checkin";
import type { CapState } from "@/bindings/CapState";
import type { CategoryList } from "@/bindings/CategoryList";
import type { Saved } from "@/bindings/Saved";
import type { SettingsView } from "@/bindings/SettingsView";
import type { VaultStatus } from "@/bindings/VaultStatus";
import { notify } from "@/components/ui/toast";
import { formatDate } from "@/lib/dates";
import { formatMoney } from "@/lib/money";
import { queryKeys } from "@/lib/queryKeys";

/**
 * A write that can change any balance or total, so it refreshes every ledger query (ARCHITECTURE
 * §5.2). The settings too: their decimals lock follows whether any amount is stored. `then`
 * runs even if the screen that started it has closed, unlike callbacks passed to `mutate`. A
 * save that moved goals' dates says so (GOL-7), and one that raised a focus cap warns (CHK-8).
 */
export function useLedgerMutation<TArgs, TResult>(
  mutationFn: (args: TArgs) => Promise<TResult>,
  then: {
    onSuccess?: () => void;
    onError?: (error: Error) => void;
    /** Off for a restore: it swaps the data, it doesn't spend toward a focus. */
    nudge?: boolean;
  } = {},
) {
  const client = useQueryClient();
  return useMutation({
    mutationFn,
    // Taken before the write, so a refetch while it runs can't hide or fake a rise.
    onMutate: () =>
      then.nudge === false
        ? undefined
        : client.getQueryData<VaultStatus | null>(queryKeys.vaultStatus),
    onSuccess: async (result, _args, before) => {
      await Promise.all([
        client.invalidateQueries({ queryKey: queryKeys.ledger }),
        client.invalidateQueries({ queryKey: queryKeys.settings }),
      ]);
      tellGoalsMoved(result);
      then.onSuccess?.();
      await tellFocusRaised(client, before);
    },
    onError: then.onError,
  });
}

const RANK: Record<CapState, number> = { ok: 0, near: 1, over: 2 };

/**
 * A warning when this save raised a cap's state, once per step up (UI_FLOWS F18). Only caps
 * have a state, so a card level, a loan payment or logging never warns here. Processing doesn't
 * save through this hook, so the dates it posts never warn either. With no status loaded before
 * the save, there's nothing to compare, so it stays quiet.
 */
async function tellFocusRaised(
  client: QueryClient,
  before: VaultStatus | null | undefined,
) {
  if (!before?.focuses.some((f) => f.state)) return;
  // Already fresh when a screen shows it; fetched only when the refresh left it stale.
  const after = await client
    .fetchQuery({
      queryKey: queryKeys.vaultStatus,
      queryFn: getVaultStatus,
      staleTime: Infinity,
    })
    .catch(() => null);
  const settings = client.getQueryData<SettingsView>(queryKeys.settings);
  if (!after || !settings) return;
  const categories =
    client.getQueryData<CategoryList>(queryKeys.categories)?.categories ?? [];
  const money = (amount: number) => formatMoney(amount, settings);
  for (const f of after.focuses) {
    const was = before.focuses.find(
      (b) => b.kind === f.kind && b.start === f.start && b.end === f.end,
    );
    if (!f.state || !was?.state || RANK[f.state] <= RANK[was.state]) continue;
    const name =
      f.kind === "spending_cap"
        ? "Spending"
        : (categories.find((c) => c.id === f.categoryId)?.name ?? "A category");
    const actual = money(f.actual ?? 0);
    const target = money(f.target ?? 0);
    notify.warning(
      f.state === "over"
        ? `${name} focus: ${actual}, past your ${target} target.`
        : `${name} focus: ${actual} of ${target}.`,
    );
  }
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
