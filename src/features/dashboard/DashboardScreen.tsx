import { Plus, ReceiptText } from "lucide-react";
import { Link } from "react-router";
import type { Category } from "@/bindings/Category";
import type { Dashboard } from "@/bindings/Dashboard";
import type { Transaction } from "@/bindings/Transaction";
import { AccountRow } from "@/components/AccountRow";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { Loading, Skeleton } from "@/components/Loading";
import { MoneyText } from "@/components/MoneyText";
import { PeriodSwitcher } from "@/components/PeriodSwitcher";
import { useQuickAdd } from "@/components/QuickAddProvider";
import { StatCard } from "@/components/StatCard";
import { TransactionRow } from "@/components/TransactionRow";
import { Button } from "@/components/ui/button";
import { useCategories } from "@/hooks/useCategories";
import { useDashboard } from "@/hooks/useDashboard";
import { usePeriodQuery } from "@/hooks/usePeriodQuery";
import { formatDayHeader } from "@/lib/dates";

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

/** Consecutive transactions on the same date, newest first as the backend sends them. */
function byDay(transactions: Transaction[]) {
  const days: { date: string; items: Transaction[] }[] = [];
  for (const t of transactions) {
    const last = days[days.length - 1];
    if (last?.date === t.date) last.items.push(t);
    else days.push({ date: t.date, items: [t] });
  }
  return days;
}

/**
 * The dashboard (UI_FLOWS §2): the period's totals, the 5 newest transactions whatever the
 * period, and every active account with the net balance, all from one `get_dashboard` call.
 */
export function DashboardScreen() {
  const [query] = usePeriodQuery();
  const dashboard = useDashboard(query);
  // Loaded alongside the dashboard for the rows' names, not after it.
  const { data: categoryList } = useCategories();
  const data = dashboard.data;

  // An error wins over older numbers: a failed refresh never leaves stale totals on screen.
  const body = dashboard.isError ? (
    <GlassPanel>
      <ErrorState
        onRetry={() => dashboard.refetch()}
        pending={dashboard.isFetching}
      />
    </GlassPanel>
  ) : data ? (
    <DashboardBody data={data} categories={categoryList?.categories ?? []} />
  ) : (
    <Loading className="grid gap-3">
      <div className="grid grid-cols-3 gap-3">
        {[0, 1, 2].map((i) => (
          <GlassPanel key={i} className="grid gap-2.5 p-5">
            <Skeleton className="h-5 w-24" />
            <Skeleton className="h-9 w-44" />
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
  return (
    <>
      <div className="grid grid-cols-3 gap-3">
        <StatCard label="Income" amount={data.income} kind="income" />
        <StatCard label="Expense" amount={data.expense} kind="expense" />
        <StatCard label="Net cashflow" amount={data.net} kind="net" />
      </div>
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
            byDay(data.recent).map(({ date, items }) => (
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
    </>
  );
}
