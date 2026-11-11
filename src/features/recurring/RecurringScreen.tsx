import { Ellipsis, Plus, Repeat } from "lucide-react";
import { useState } from "react";
import type { Category } from "@/bindings/Category";
import type { RecurringRule } from "@/bindings/RecurringRule";
import { CategoryChip, TransferChip } from "@/components/CategoryChip";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { Loading, Skeleton } from "@/components/Loading";
import { MoneyText } from "@/components/MoneyText";
import { useQuickAdd } from "@/components/QuickAddProvider";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/ui/dialog";
import { Menu, MenuContent, MenuItem, MenuTrigger } from "@/components/ui/menu";
import { notify } from "@/components/ui/toast";
import { useAccounts } from "@/hooks/useAccounts";
import { useCategories } from "@/hooks/useCategories";
import {
  useDeleteRecurringRule,
  usePauseRecurringRule,
  useRecurringRules,
} from "@/hooks/useRecurring";
import { useTransactionDefaults } from "@/hooks/useTransactions";
import { formatSpan } from "@/lib/dates";
import { rowTitle } from "@/lib/records";
import { describeSchedule } from "@/lib/recurrence";
import { cn } from "@/lib/utils";

// The mockup's badges: Auto is plain, Confirm takes the accent, Paused the warning color.
const BADGES = {
  auto: { label: "Auto", tone: "bg-wash text-muted-foreground" },
  confirm: {
    label: "Confirm",
    tone: "bg-[color-mix(in_srgb,var(--primary)_var(--tint),transparent)] text-primary",
  },
  paused: {
    label: "Paused",
    tone: "bg-[color-mix(in_srgb,var(--warning)_var(--tint),transparent)] text-warning",
  },
};

/**
 * `/recurring` (UI_FLOWS §8.2): every rule with its schedule, next date, amount and mode, or
 * Paused. A row opens the rule's form; ⋯ pauses, resumes or deletes it. + New rule opens the
 * transaction form with a schedule section.
 */
export function RecurringScreen() {
  const rules = useRecurringRules();
  const accounts = useAccounts();
  const categories = useCategories();
  // Only for a next date's year: shown when it isn't this year.
  const defaults = useTransactionDefaults();
  const queries = [rules, accounts, categories, defaults];
  const failed = queries.filter((q) => q.isError);
  const { rule: openRule } = useQuickAdd();
  const pause = usePauseRecurringRule();
  const remove = useDeleteRecurringRule();
  const [deleting, setDeleting] = useState<RecurringRule | null>(null);
  const done = (message: string) => () => notify.success(message);
  const failedWith = (error: Error) => notify.error(error.message);

  let body;
  if (failed.length)
    body = (
      <ErrorState
        onRetry={() => failed.forEach((q) => q.refetch())}
        pending={failed.some((q) => q.isFetching)}
      />
    );
  else if (!rules.data || !accounts.data || !categories.data || !defaults.data)
    body = (
      <Loading className="grid gap-2 px-[18px] pb-4">
        {Array.from({ length: 4 }, (_, i) => (
          <Skeleton key={i} className="h-11" />
        ))}
      </Loading>
    );
  else if (rules.data.length === 0)
    body = (
      <EmptyState
        icon={Repeat}
        message="No recurring transactions yet."
        action={
          <Button variant="primary" onClick={() => openRule()}>
            <Plus />
            New rule
          </Button>
        }
      />
    );
  else {
    const today = defaults.data.today;
    const accountList = accounts.data.accounts;
    const categoryList = categories.data.categories;
    body = rules.data.map((rule) => {
      const title = rowTitle(rule.transaction, accountList, categoryList);
      return (
        <div
          key={rule.id}
          className="flex items-center gap-1 border-t border-hairline pr-3"
        >
          <RuleRow
            rule={rule}
            title={title}
            today={today}
            categories={categoryList}
            onOpen={() => openRule(rule)}
          />
          <Menu>
            <MenuTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                aria-label={`More actions for ${title}`}
              >
                <Ellipsis />
              </Button>
            </MenuTrigger>
            <MenuContent>
              <MenuItem onSelect={() => openRule(rule)}>Edit</MenuItem>
              <MenuItem
                onSelect={() =>
                  pause.mutate(
                    { id: rule.id, paused: !rule.paused },
                    {
                      onSuccess: done(rule.paused ? "Resumed" : "Paused"),
                      onError: failedWith,
                    },
                  )
                }
              >
                {rule.paused ? "Resume" : "Pause"}
              </MenuItem>
              <MenuItem destructive onSelect={() => setDeleting(rule)}>
                Delete
              </MenuItem>
            </MenuContent>
          </Menu>
        </div>
      );
    });
  }

  return (
    <GlassPanel className="overflow-hidden">
      <div className="flex items-center justify-between gap-2 px-[18px] pt-4 pb-2.5">
        <h2 className="text-h3">Recurring</h2>
        <Button variant="primary" size="sm" onClick={() => openRule()}>
          <Plus />
          New rule
        </Button>
      </div>
      {body}
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title="Delete this rule?"
        message="The transactions it already created stay."
        confirmLabel="Delete"
        destructive
        onConfirm={() =>
          deleting &&
          remove.mutate(deleting.id, {
            onSuccess: done("Deleted"),
            onError: failedWith,
          })
        }
      />
    </GlassPanel>
  );
}

function RuleRow({
  rule,
  title,
  today,
  categories,
  onOpen,
}: {
  rule: RecurringRule;
  title: string;
  today: string;
  categories: Category[];
  onOpen: () => void;
}) {
  const t = rule.transaction;
  const category = categories.find((c) => c.id === t.categoryId);
  const transfer = t.kind === "transfer";
  // A paused rule's next date moves when it's resumed, so it isn't shown (UI_FLOWS §8.2).
  const when = !rule.nextDate
    ? " · ended"
    : rule.paused
      ? ""
      : ` · next ${formatSpan(rule.nextDate, rule.nextDate, today)}`;
  const badge = BADGES[rule.paused ? "paused" : rule.mode];
  return (
    <button
      type="button"
      onClick={onOpen}
      className="grid min-h-[52px] min-w-0 flex-1 grid-cols-[32px_minmax(0,1fr)_auto] items-center gap-3 rounded-chip py-1.5 pl-[18px] text-left transition-colors duration-150 outline-none hover:bg-glass-hover focus-visible:ring-3 focus-visible:ring-ring/40"
    >
      {transfer ? (
        <TransferChip />
      ) : (
        <CategoryChip
          color={category?.color ?? "cat-11"}
          icon={category?.icon ?? "ellipsis"}
        />
      )}
      <span className="grid min-w-0">
        <span className="truncate font-semibold">{title}</span>
        <span className="truncate text-small text-muted-foreground">
          {describeSchedule(rule) + when}
        </span>
      </span>
      <span className="grid justify-items-end gap-0.5 pr-2">
        <MoneyText amount={t.amount} kind={t.kind} />
        <span
          className={cn(
            "rounded-pill px-2 text-small font-semibold",
            badge.tone,
          )}
        >
          {badge.label}
        </span>
      </span>
    </button>
  );
}
