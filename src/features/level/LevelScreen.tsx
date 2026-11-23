import { Check, Circle, X } from "lucide-react";
import { Tooltip } from "radix-ui";
import type { GradedFocus } from "@/bindings/GradedFocus";
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
  cardLevel,
  focusNames,
  focusText,
  levelTitle,
  nextMoveText,
  weakest,
} from "@/lib/vault";

const plural = (n: number, one: string, many: string) =>
  n === 1 ? `1 ${one}` : `${n} ${many}`;

/**
 * "Last 8" and the latest graded focuses, oldest first, as 16 px check and x icons (LVL-8). Each is
 * a tab stop whose label and tooltip name the focus and its result, so color is never the only cue.
 */
function FocusRecord({
  record,
  text,
}: {
  /** Latest first, void ones already left out (DATA_MODEL §5.23). */
  record: GradedFocus[];
  text: (f: GradedFocus) => string;
}) {
  return (
    <Tooltip.Provider delayDuration={300}>
      <div className="flex items-center gap-2.5">
        <span className="text-small text-muted-foreground">Last 8</span>
        <ul aria-label="Last 8 focuses" className="flex gap-1">
          {[...record].reverse().map((f) => {
            const kept = f.result === "kept";
            const label = `${text(f)} · ${kept ? "Kept" : "Missed"}`;
            return (
              <li key={`${f.kind}-${f.start}-${f.end}`}>
                <Tooltip.Root>
                  <Tooltip.Trigger asChild>
                    <span
                      tabIndex={0}
                      role="img"
                      aria-label={label}
                      className="block rounded-full outline-none focus-visible:ring-3 focus-visible:ring-ring/40"
                    >
                      {kept ? (
                        <Check aria-hidden className="size-4 text-success" />
                      ) : (
                        <X aria-hidden className="size-4 text-danger" />
                      )}
                    </span>
                  </Tooltip.Trigger>
                  <Tooltip.Portal>
                    <Tooltip.Content
                      sideOffset={6}
                      className="glass-raised z-50 max-w-72 rounded-toolbar px-3 py-2 text-small tabular-nums"
                    >
                      {label}
                    </Tooltip.Content>
                  </Tooltip.Portal>
                </Tooltip.Root>
              </li>
            );
          })}
        </ul>
      </div>
    </Tooltip.Provider>
  );
}

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
  const money = (amount: number) => formatMoney(amount, settings.data);
  const names = focusNames(
    s.today,
    money,
    categories.data.categories,
    list,
    loans.data.loans,
  );
  const level = cardLevel(list);
  const move = nextMoveText(s.nextMove, {
    money,
    account: names.account,
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
            <h2 className="text-h3">{`Next: ${levelTitle(s.nextLevel.level)}`}</h2>
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
          <h2 className="text-h3">Stats</h2>
          {/* gap-3 leaves room for the trend column: 280 + 12 + 248 fills the 540 px inside. */}
          <div className="grid grid-cols-[280px_minmax(0,1fr)] items-center gap-3">
            <StatHexagon stats={s.stats} previous={s.previous?.stats} />
            <div className="grid content-center">
              <StatScores
                stats={s.stats}
                previous={s.previous?.stats}
                history={s.history}
              />
            </div>
          </div>
          {lowest?.reason && <p>{weakest(lowest.reason)}</p>}
        </GlassPanel>
        <GlassPanel className="grid gap-4 p-5">
          <h2 className="text-h3">Focus</h2>
          {s.focuses.length ? (
            <FocusRows focuses={s.focuses} names={names} level={level} />
          ) : (
            <p className="text-muted-foreground">
              No focus right now. You'll pick one at your next check-in.
            </p>
          )}
          {s.focusRecord.length > 0 && (
            <FocusRecord
              record={s.focusRecord}
              text={(f) => focusText(f, names, level(f))}
            />
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
