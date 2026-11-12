import { ChevronRight, Ellipsis, Flag, Plus } from "lucide-react";
import { useState, type ReactNode } from "react";
import type { Account } from "@/bindings/Account";
import type { GoalProgress } from "@/bindings/GoalProgress";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { Loading, Skeleton } from "@/components/Loading";
import { ProgressBar } from "@/components/ProgressBar";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/ui/dialog";
import { Menu, MenuContent, MenuItem, MenuTrigger } from "@/components/ui/menu";
import { notify } from "@/components/ui/toast";
import { useAccounts } from "@/hooks/useAccounts";
import {
  useArchiveGoal,
  useDeleteGoal,
  useGoals,
  useUnarchiveGoal,
} from "@/hooks/useGoals";
import { useSettings } from "@/hooks/useSettings";
import { formatDate } from "@/lib/dates";
import { formatMoney, type MoneySettings } from "@/lib/money";
import { cn } from "@/lib/utils";
import { GoalDialog } from "./GoalDialog";

/**
 * `/goals` (UI_FLOWS §8.4): each goal as a card with its account's progress, what's left and what
 * to save each month (GOL-3). A reached goal gets an Achieved badge, and any goal can be archived
 * under Show archived (GOL-4). One glass panel holds the flat cards (DESIGN_SYSTEM §2.3).
 */
export function GoalsScreen() {
  const goals = useGoals();
  const accounts = useAccounts();
  const settings = useSettings();
  const queries = [goals, accounts, settings];
  const failed = queries.filter((q) => q.isError);
  const archive = useArchiveGoal();
  const unarchive = useUnarchiveGoal();
  const remove = useDeleteGoal();
  // `{}` is a new goal.
  const [editing, setEditing] = useState<{ goal?: GoalProgress } | null>(null);
  const [deleting, setDeleting] = useState<GoalProgress | null>(null);
  const [showArchived, setShowArchived] = useState(false);
  const newGoal = (
    <Button variant="primary" size="sm" onClick={() => setEditing({})}>
      <Plus />
      New goal
    </Button>
  );

  let body;
  if (failed.length)
    body = (
      <ErrorState
        onRetry={() => failed.forEach((q) => q.refetch())}
        pending={failed.some((q) => q.isFetching)}
      />
    );
  else if (!goals.data || !accounts.data || !settings.data)
    body = (
      <Loading className="grid grid-cols-3 gap-3 px-[18px] pb-[18px]">
        {[0, 1, 2].map((i) => (
          <Skeleton key={i} className="h-[128px]" />
        ))}
      </Loading>
    );
  else {
    const accountList = accounts.data.accounts;
    const money = settings.data;
    const active = goals.data.filter((g) => !g.archived);
    const archived = goals.data.filter((g) => g.archived);
    const cards = (list: GoalProgress[]) => (
      <ul className="grid grid-cols-3 gap-3 px-[18px] pb-[18px]">
        {list.map((goal) => (
          <Card
            key={goal.id}
            goal={goal}
            accounts={accountList}
            settings={money}
            onEdit={() => setEditing({ goal })}
            actions={
              goal.archived ? (
                <MenuItem
                  disabled={unarchive.isPending}
                  onSelect={() =>
                    unarchive.mutate(goal.id, {
                      onSuccess: () => notify.success("Unarchived"),
                      onError: (error) => notify.error(error.message),
                    })
                  }
                >
                  Unarchive
                </MenuItem>
              ) : (
                <>
                  <MenuItem onSelect={() => setEditing({ goal })}>
                    Edit
                  </MenuItem>
                  <MenuItem
                    disabled={archive.isPending}
                    onSelect={() =>
                      archive.mutate(goal.id, {
                        onSuccess: () => notify.success("Archived"),
                        onError: (error) => notify.error(error.message),
                      })
                    }
                  >
                    Archive
                  </MenuItem>
                </>
              )
            }
            onDelete={() => setDeleting(goal)}
          />
        ))}
      </ul>
    );
    body = (
      <>
        {active.length ? (
          cards(active)
        ) : (
          <EmptyState
            icon={Flag}
            message={archived.length ? "No active goals." : "No goals yet."}
            action={newGoal}
          />
        )}
        {archived.length > 0 && (
          <>
            <button
              type="button"
              aria-expanded={showArchived}
              onClick={() => setShowArchived(!showArchived)}
              className="flex w-full items-center gap-1.5 border-t border-hairline px-4 py-2.5 text-muted-foreground outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/40"
            >
              <ChevronRight
                aria-hidden
                className={cn(
                  "size-3.5 transition-transform duration-150",
                  showArchived && "rotate-90",
                )}
              />
              Show archived ({archived.length})
            </button>
            {showArchived && cards(archived)}
          </>
        )}
      </>
    );
  }

  return (
    <GlassPanel className="overflow-hidden">
      <div className="flex items-center justify-between gap-2 px-[18px] pt-4 pb-2.5">
        <h2 className="text-h3">Goals</h2>
        {newGoal}
      </div>
      {body}
      {editing && accounts.data && settings.data && (
        <GoalDialog
          goal={editing.goal}
          accounts={accounts.data.accounts}
          weekStart={settings.data.weekStart}
          onClose={() => setEditing(null)}
        />
      )}
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title="Delete this goal?"
        message="Its account and transactions stay. This can't be undone."
        confirmLabel="Delete"
        destructive
        onConfirm={() =>
          deleting &&
          remove.mutate(deleting.id, {
            onSuccess: () => notify.success("Deleted"),
            onError: (error) => notify.error(error.message),
          })
        }
      />
    </GlassPanel>
  );
}

/** "Save ₱433.34/month to reach it", "Target date passed", or nothing (GOL-3). */
function plan(goal: GoalProgress, money: (amount: number) => string) {
  if (goal.achievedAt) return null;
  if (goal.monthlyNeeded !== null)
    return `Save ${money(goal.monthlyNeeded)}/month to reach it`;
  return goal.monthsLeft === 0 ? "Target date passed" : null;
}

function Card({
  goal,
  accounts,
  settings,
  onEdit,
  actions,
  onDelete,
}: {
  goal: GoalProgress;
  accounts: Account[];
  settings: MoneySettings;
  onEdit: () => void;
  actions: ReactNode;
  onDelete: () => void;
}) {
  const money = (amount: number) => formatMoney(amount, settings);
  const account = accounts.find((a) => a.id === goal.accountId)?.name ?? "";
  const next = plan(goal, money);
  return (
    // The name's button stretches over the whole card, so the progress bar stays outside it.
    <li className="relative grid content-start gap-2.5 rounded-control bg-wash p-4 pt-3 transition-colors duration-150 hover:bg-glass-hover">
      <div className="flex min-h-8 items-center gap-2">
        <button
          type="button"
          onClick={onEdit}
          aria-label={`Edit ${goal.name}`}
          className="min-w-0 flex-1 truncate text-left font-semibold outline-none after:absolute after:inset-0 after:rounded-control focus-visible:after:ring-3 focus-visible:after:ring-ring/40"
        >
          {goal.name}
        </button>
        {goal.achievedAt && (
          <span className="rounded-pill bg-[color-mix(in_srgb,var(--success)_var(--tint),transparent)] px-2 text-small font-semibold text-success">
            Achieved
          </span>
        )}
        <Menu>
          <MenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              aria-label={`More actions for ${goal.name}`}
              className="relative -mr-2"
            >
              <Ellipsis />
            </Button>
          </MenuTrigger>
          <MenuContent>
            {actions}
            <MenuItem destructive onSelect={onDelete}>
              Delete
            </MenuItem>
          </MenuContent>
        </Menu>
      </div>
      <p className="-mt-2 truncate text-small text-muted-foreground">
        {goal.targetDate
          ? `${account} · by ${formatDate(goal.targetDate)}`
          : account}
      </p>
      <ProgressBar
        percent={goal.percent}
        tone="goal"
        done={goal.achievedAt !== null}
        aria-label={`${goal.name} saved`}
      />
      <p className="text-small text-muted-foreground tabular-nums">
        {`${money(goal.progress)} of ${money(goal.targetAmount)} · ${money(goal.remaining)} left`}
      </p>
      {next && <p className="text-small font-semibold tabular-nums">{next}</p>}
    </li>
  );
}
