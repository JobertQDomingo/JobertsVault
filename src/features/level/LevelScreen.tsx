import { Check, Circle } from "lucide-react";
import type { ActiveFocus } from "@/bindings/ActiveFocus";
import { ErrorState } from "@/components/ErrorState";
import { FocusRows } from "@/components/FocusRows";
import { GlassPanel } from "@/components/GlassPanel";
import { LevelBadge } from "@/components/LevelBadge";
import { LevelPath } from "@/components/LevelPath";
import { Loading, Skeleton } from "@/components/Loading";
import { StatHexagon, StatScores } from "@/components/StatHexagon";
import { useAccounts } from "@/hooks/useAccounts";
import { useCategories } from "@/hooks/useCategories";
import { useVaultStatus } from "@/hooks/useCheckin";
import { useLoans } from "@/hooks/useLoans";
import { useSettings } from "@/hooks/useSettings";
import { formatMoney } from "@/lib/money";
import {
  CONDITIONS,
  conditionGap,
  type FocusNames,
  levelTitle,
  nextMoveText,
  weakest,
} from "@/lib/vault";

const plural = (n: number, one: string, many: string) =>
  n === 1 ? `1 ${one}` : `${n} ${many}`;

/**
 * `/level` (UI_FLOWS §8.8, LVL-3, LVL-4, LVL-7): the level path and the next level's checklist,
 * the stats, and the focus with the streaks. Everything is `get_vault_status`'s, for today, and
 * nothing can be edited; the lists only give the names.
 */
export function LevelScreen() {
  const status = useVaultStatus();
  const categories = useCategories();
  const accounts = useAccounts();
  const loans = useLoans();
  const settings = useSettings();
  const failed = [status, categories, accounts, loans, settings].filter(
    (q) => q.isError,
  );

  if (failed.length)
    return (
      <GlassPanel>
        <ErrorState
          onRetry={() => failed.forEach((q) => q.refetch())}
          pending={failed.some((q) => q.isFetching)}
        />
      </GlassPanel>
    );
  if (
    !status.data ||
    !categories.data ||
    !accounts.data ||
    !loans.data ||
    !settings.data
  )
    return (
      <Loading className="grid gap-3">
        <Skeleton className="h-[220px]" />
        <Skeleton className="h-[400px]" />
      </Loading>
    );

  const s = status.data;
  const list = accounts.data.accounts;
  const account = (id: number | null) =>
    list.find((a) => a.id === id)?.name ?? "Your card";
  const money = (amount: number) => formatMoney(amount, settings.data);
  const names: FocusNames = {
    money,
    category: (id) =>
      categories.data.categories.find((c) => c.id === id)?.name ?? "A category",
    account,
    cards: list.filter((a) => a.kind === "credit_card" && !a.archived).length,
    payment: (id) => {
      for (const loan of loans.data.loans) {
        const p = loan.payments.find((p) => p.id === id);
        if (p) return { loan: loan.name, dueDate: p.dueDate };
      }
    },
    today: s.today,
  };
  // A card-level focus names its level when its target is one of the card's three.
  const cardLevel = (f: ActiveFocus) => {
    const a = list.find((a) => a.id === f.accountId);
    if (!a || f.target === null) return null;
    if (f.target === a.spendSoft) return "soft";
    if (f.target === a.spendMid) return "mid";
    return f.target === a.spendHard ? "hard" : null;
  };
  const move = nextMoveText(s.nextMove, {
    money,
    account,
    loan: (id) => loans.data.loans.find((l) => l.id === id)?.name ?? "A loan",
  });
  // The lowest stat that applies, the first on a tie (DATA_MODEL §5.20); none when all are 100.
  const lowest = s.stats.reduce<(typeof s.stats)[number] | null>(
    (low, st) =>
      st.score !== null && (low === null || st.score < (low.score ?? 0))
        ? st
        : low,
    null,
  );

  return (
    <div className="grid gap-3">
      <GlassPanel className="grid gap-4 p-5">
        <LevelBadge level={s.level} />
        <LevelPath level={s.level} />
        {s.level < 10 && (
          <div className="grid gap-2">
            <h3 className="text-h3">{`Next: ${levelTitle(s.nextLevel.level)}`}</h3>
            <ul className="grid gap-1">
              {s.nextLevel.conditions.map((c) => {
                const gap = conditionGap(c, names);
                return (
                  <li
                    key={c.code}
                    className="flex items-center gap-2 tabular-nums"
                  >
                    {c.met ? (
                      <Check
                        aria-hidden
                        className="size-4 shrink-0 text-success"
                      />
                    ) : (
                      <Circle
                        aria-hidden
                        className="size-4 shrink-0 text-muted-foreground"
                      />
                    )}
                    <span className="sr-only">
                      {c.met ? "Met: " : "Not met: "}
                    </span>
                    {CONDITIONS[c.code]}
                    {gap && (
                      <span className="text-muted-foreground">{`· ${gap}`}</span>
                    )}
                  </li>
                );
              })}
            </ul>
          </div>
        )}
        <p className="tabular-nums">{move}</p>
      </GlassPanel>
      <div className="grid grid-cols-2 items-start gap-3">
        <GlassPanel className="grid gap-3 p-5">
          <h3 className="text-h3">Stats</h3>
          <div className="grid grid-cols-[280px_minmax(0,1fr)] items-center gap-6">
            <StatHexagon stats={s.stats} previous={s.previous?.stats} />
            <div className="grid content-center">
              <StatScores stats={s.stats} previous={s.previous?.stats} />
            </div>
          </div>
          {lowest?.reason && <p>{weakest(lowest.reason)}</p>}
        </GlassPanel>
        <GlassPanel className="grid gap-4 p-5">
          <h3 className="text-h3">Focus</h3>
          {s.focuses.length ? (
            <FocusRows focuses={s.focuses} names={names} level={cardLevel} />
          ) : (
            <p className="text-muted-foreground">
              No focus right now. You'll pick one at your next check-in.
            </p>
          )}
          <div className="grid gap-1 tabular-nums">
            <p>{`${plural(s.focusStreak, "focus", "focuses")} kept in a row`}</p>
            <p>{`${plural(s.checkinStreak, "check-in", "check-ins")} in a row on time`}</p>
            {s.loanFreeDays !== null && (
              <p>{`${plural(s.loanFreeDays, "day", "days")} since your last new loan`}</p>
            )}
          </div>
        </GlassPanel>
      </div>
    </div>
  );
}
