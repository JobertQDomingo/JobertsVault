import { Plus, ReceiptText, Repeat, Target } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";
import type { BudgetProgress } from "@/bindings/BudgetProgress";
import type { Category } from "@/bindings/Category";
import type { Dashboard } from "@/bindings/Dashboard";
import { AccountRow } from "@/components/AccountRow";
import { Banner } from "@/components/Banner";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { IncomeExpenseChart } from "@/components/IncomeExpenseChart";
import { Loading, Skeleton } from "@/components/Loading";
import { MoneyText } from "@/components/MoneyText";
import { PeriodSwitcher } from "@/components/PeriodSwitcher";
import { ProgressBar } from "@/components/ProgressBar";
import { useQuickAdd } from "@/components/QuickAddProvider";
import { StatCard } from "@/components/StatCard";
import { TransactionRow } from "@/components/TransactionRow";
import { Button } from "@/components/ui/button";
import {
  BudgetName,
  budgetTitle,
  periodLabel,
} from "@/features/budgets/BudgetName";
import { DueDialog } from "@/features/recurring/DueDialog";
import { useBackupNow } from "@/hooks/useBackup";
import { useCategories } from "@/hooks/useCategories";
import { useDashboard } from "@/hooks/useDashboard";
import { usePeriodQuery } from "@/hooks/usePeriodQuery";
import { formatDayHeader, formatSpan, groupByDate } from "@/lib/dates";
import { cn } from "@/lib/utils";

const COLUMNS =
  "grid grid-cols-[minmax(0,1.45fr)_minmax(0,1fr)] items-start gap-3";

function CardHeader({
  title,
  to,
  link,
}: {
  title: string;
  to: string;
  link: string;
}) {
  return (
    <div className="flex items-center justify-between gap-2 px-[18px] pt-4 pb-1.5">
      <h2 className="text-h3">{title}</h2>
      <Link to={to} className="font-semibold text-primary hover:underline">
        {link}
      </Link>
    </div>
  );
}

/**
 * The dashboard (UI_FLOWS §2): the period's totals, the 5 newest transactions whatever the
 * period, and every active account with the net balance, all from one `get_dashboard` call.
 */
export function DashboardScreen() {
  const [query] = usePeriodQuery();
  const dashboard = useDashboard(query);
  // Loaded alongside the dashboard for the rows' names, and part of its loading and errors.
  const categories = useCategories();
  const data = dashboard.data;
  const failed = [dashboard, categories].filter((q) => q.isError);

  // An error wins over older numbers: a failed refresh never leaves stale totals on screen.
  const body = failed.length ? (
    <GlassPanel>
      <ErrorState
        onRetry={() => failed.forEach((q) => q.refetch())}
        pending={failed.some((q) => q.isFetching)}
      />
    </GlassPanel>
  ) : data && categories.data ? (
    // While the next period loads, the old numbers stay but dim (after 150 ms, so a quick load
    // doesn't flicker) and are marked busy: they never pass for the new period's.
    <div
      aria-busy={dashboard.isPlaceholderData}
      className={cn(
        "grid gap-3 transition-opacity duration-150",
        dashboard.isPlaceholderData && "opacity-60 delay-150",
      )}
    >
      <DashboardBody data={data} categories={categories.data.categories} />
    </div>
  ) : (
    <Loading className="grid gap-3">
      <div className="grid grid-cols-3 gap-3">
        {[0, 1, 2].map((i) => (
          <GlassPanel key={i} className="grid gap-2.5 p-5">
            <Skeleton className="h-5 w-24" />
            <Skeleton className="h-9 w-44" />
            <Skeleton className="h-4 w-32" />
          </GlassPanel>
        ))}
      </div>
      <div className={COLUMNS}>
        <GlassPanel className="h-80" />
        <GlassPanel className="h-80" />
      </div>
    </Loading>
  );

  return (
    <>
      <PeriodSwitcher />
      {body}
    </>
  );
}

function DashboardBody({
  data,
  categories: categoryList,
}: {
  data: Dashboard;
  categories: Category[];
}) {
  const { add, edit } = useQuickAdd();
  const accounts = new Map(data.accounts.map((a) => [a.id, a]));
  const categories = new Map(categoryList.map((c) => [c.id, c]));
  const { comparison: c } = data;
  const vs = formatSpan(c.compareStart, c.compareEnd, data.today);
  return (
    <>
      {data.backupDue && <BackupWarning />}
      <RecurringDue count={data.recurringDue} today={data.today} />
      <div className="grid grid-cols-3 gap-3">
        <StatCard
          label="Income"
          amount={data.income}
          kind="income"
          change={c.income.change}
          vs={vs}
        />
        <StatCard
          label="Expense"
          amount={data.expense}
          kind="expense"
          change={c.expense.change}
          vs={vs}
        />
        <StatCard
          label="Net cashflow"
          amount={data.net}
          kind="net"
          change={c.net.change}
          vs={vs}
        />
      </div>
      <GlassPanel className="grid gap-2 px-[18px] pt-4 pb-3">
        <h2 className="text-h3">Income vs expense</h2>
        <IncomeExpenseChart series={data.series} height={140} />
      </GlassPanel>
      <div className={COLUMNS}>
        <GlassPanel className="overflow-hidden pb-1.5">
          <CardHeader
            title="Recent transactions"
            to="/transactions"
            link="View all"
          />
          {data.recent.length === 0 ? (
            <EmptyState
              icon={ReceiptText}
              message="No transactions yet. Press Ctrl+N to add your first one."
              action={
                <Button variant="primary" onClick={add}>
                  <Plus />
                  Add transaction
                </Button>
              }
            />
          ) : (
            groupByDate(data.recent).map(({ date, items }) => (
              <section key={date}>
                <h3 className="px-[18px] pt-2.5 pb-1.5 text-small font-semibold text-muted-foreground">
                  {formatDayHeader(date, data.today)}
                </h3>
                {items.map((t) => (
                  <TransactionRow
                    key={t.id}
                    compact
                    transaction={t}
                    category={
                      t.categoryId ? categories.get(t.categoryId) : undefined
                    }
                    account={accounts.get(t.accountId)}
                    toAccount={
                      t.toAccountId ? accounts.get(t.toAccountId) : undefined
                    }
                    onOpen={() => edit(t.id)}
                  />
                ))}
              </section>
            ))
          )}
        </GlassPanel>
        <GlassPanel className="overflow-hidden">
          <CardHeader title="Accounts" to="/accounts" link="Manage" />
          {/* Archived accounts are hidden here but still in the net balance (ACC-5). */}
          {data.accounts
            .filter((a) => !a.archived)
            .map((a) => (
              <Link
                key={a.id}
                to={`/accounts/${a.id}`}
                className="block border-t border-hairline transition-colors duration-150 hover:bg-glass-hover"
              >
                <AccountRow account={a} />
              </Link>
            ))}
          <div className="flex items-center justify-between border-t border-hairline bg-wash px-[18px] py-3 font-semibold">
            <span>Net balance</span>
            <MoneyText amount={data.netBalance} kind="balance" />
          </div>
        </GlassPanel>
      </div>
      {/* The mockup's second column, "Coming up", is DSH-7. */}
      <div className="grid grid-cols-2 items-start gap-3">
        <Budgets budgets={data.budgets} categories={categoryList} />
      </div>
    </>
  );
}

/**
 * Each budget's bar for its own current period, whatever the period shown, like the accounts'
 * balances (DSH-6, DATA_MODEL §9).
 */
function Budgets({
  budgets,
  categories,
}: {
  budgets: BudgetProgress[];
  categories: Category[];
}) {
  return (
    <GlassPanel className="overflow-hidden pb-2">
      <CardHeader title="Budgets" to="/budgets" link="Manage" />
      {budgets.length === 0 ? (
        <EmptyState icon={Target} message="No budgets yet." />
      ) : (
        <ul>
          {budgets.map((b) => (
            <li
              key={b.id}
              className="grid grid-cols-[minmax(0,1fr)_64px_minmax(0,1.25fr)] items-center gap-3 px-[18px] py-2"
            >
              <BudgetName categoryId={b.categoryId} categories={categories} />
              <span className="text-small text-muted-foreground">
                {periodLabel(b.period)}
              </span>
              <ProgressBar
                percent={b.percent}
                aria-label={`${budgetTitle(b, categories)} used`}
              />
            </li>
          ))}
        </ul>
      )}
    </GlassPanel>
  );
}

/** Dates Confirm rules queued, with Review (UI_FLOWS F10). */
function RecurringDue({ count, today }: { count: number; today: string }) {
  const [reviewing, setReviewing] = useState(false);
  // The review closes once the last one is posted or skipped, so new ones don't reopen it.
  if (reviewing && count === 0) setReviewing(false);
  return (
    <>
      {count > 0 && (
        <Banner
          tone="info"
          icon={Repeat}
          action={
            <Button size="sm" onClick={() => setReviewing(true)}>
              Review
            </Button>
          }
        >
          {count === 1
            ? "1 recurring transaction is due."
            : `${count} recurring transactions are due.`}
        </Banner>
      )}
      {reviewing && (
        <DueDialog today={today} onClose={() => setReviewing(false)} />
      )}
    </>
  );
}

/** No backup for more than 7 days, or none at all (BAK-7, UI_FLOWS F9). */
function BackupWarning() {
  const backup = useBackupNow();
  return (
    <Banner
      action={
        <Button
          size="sm"
          onClick={() => backup.mutate()}
          disabled={backup.isPending}
        >
          {backup.isPending ? "Backing up…" : "Back up now"}
        </Button>
      }
    >
      No backup in the last 7 days.
    </Banner>
  );
}
