import { CalendarX, SearchX } from "lucide-react";
import type { TransactionFilter } from "@/bindings/TransactionFilter";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { Loading, Skeleton } from "@/components/Loading";
import { MoneyText } from "@/components/MoneyText";
import { useQuickAdd } from "@/components/QuickAddProvider";
import { TransactionRow } from "@/components/TransactionRow";
import { Button } from "@/components/ui/button";
import { useAccounts } from "@/hooks/useAccounts";
import { useCategories } from "@/hooks/useCategories";
import { usePeriod } from "@/hooks/usePeriod";
import { usePeriodQuery } from "@/hooks/usePeriodQuery";
import { useTransactions } from "@/hooks/useTransactions";
import { formatDayHeader, formatPeriodLabel } from "@/lib/dates";

/**
 * The transaction list (UI_FLOWS §3, TXN-3): a summary over every match, then the rows grouped by
 * day with each day's net, newest first, 100 at a time as it scrolls. It scrolls inside its one
 * glass panel (DESIGN_SYSTEM §11). `filtered` picks the empty message; `onClearFilters` its button.
 */
export function TransactionList({
  filter,
  filtered,
  onClearFilters,
}: {
  filter: TransactionFilter;
  filtered: boolean;
  onClearFilters: () => void;
}) {
  const list = useTransactions(filter);
  const accounts = useAccounts();
  const categories = useCategories();
  const period = usePeriod(filter.period);
  const [, setPeriod] = usePeriodQuery();
  const { edit, duplicate } = useQuickAdd();

  const queries = [list, accounts, categories, period];
  const failed = queries.filter((q) => q.isError);
  const pages = list.data?.pages;
  const view = period.data;

  let body;
  // An error wins over older rows, as on the dashboard.
  if (failed.length)
    body = (
      <ErrorState
        onRetry={() => failed.forEach((q) => q.refetch())}
        pending={failed.some((q) => q.isFetching)}
      />
    );
  else if (!pages || !accounts.data || !categories.data || !view)
    body = (
      <Loading className="grid gap-2 p-[18px]">
        {Array.from({ length: 8 }, (_, i) => (
          <Skeleton key={i} className="h-10" />
        ))}
      </Loading>
    );
  else if (pages[0].summary.count === 0)
    body = filtered ? (
      <EmptyState
        icon={SearchX}
        message="No transactions match these filters."
        action={<Button onClick={onClearFilters}>Clear filters</Button>}
      />
    ) : (
      <EmptyState
        icon={CalendarX}
        message={`Nothing recorded for ${formatPeriodLabel(view.todayQuery.kind, view.start, view.end)}.`}
        action={
          !view.isCurrent && (
            <Button onClick={() => setPeriod(view.todayQuery)}>
              Go to today
            </Button>
          )
        }
      />
    );
  else {
    const { summary } = pages[0];
    const accountById = new Map(accounts.data.accounts.map((a) => [a.id, a]));
    const categoryById = new Map(
      categories.data.categories.map((c) => [c.id, c]),
    );
    const nets = new Map(
      pages.flatMap((p) => p.dayNets).map((d) => [d.date, d.net]),
    );
    const days: { date: string; items: (typeof pages)[0]["items"] }[] = [];
    for (const t of pages.flatMap((p) => p.items)) {
      const last = days[days.length - 1];
      if (last?.date === t.date) last.items.push(t);
      else days.push({ date: t.date, items: [t] });
    }
    body = (
      <>
        <p className="border-b border-hairline px-[18px] py-3 text-muted-foreground">
          {summary.count} {summary.count === 1 ? "transaction" : "transactions"}{" "}
          · Income <MoneyText amount={summary.income} kind="income" /> · Expense{" "}
          <MoneyText amount={summary.expense} kind="expense" />
        </p>
        <ul
          aria-label="Transactions"
          className="min-h-0 flex-1 overflow-y-auto pb-1.5"
          onScroll={(e) => {
            const el = e.currentTarget;
            // The next 100 load before the end comes into view. Scroll events fire faster than the
            // query reports it's fetching, so repeats are ignored by the query itself.
            if (
              list.hasNextPage &&
              el.scrollTop + el.clientHeight >= el.scrollHeight - 600
            )
              list.fetchNextPage({ cancelRefetch: false });
          }}
        >
          {days.map(({ date, items }) => (
            <li key={date}>
              <h3 className="flex items-center justify-between px-[18px] pt-3 pb-1.5 text-small font-semibold text-muted-foreground">
                <span>{formatDayHeader(date, view.today)}</span>
                <MoneyText amount={nets.get(date) ?? 0} kind="net" />
              </h3>
              {items.map((t) => (
                <TransactionRow
                  key={t.id}
                  transaction={t}
                  category={
                    t.categoryId ? categoryById.get(t.categoryId) : undefined
                  }
                  account={accountById.get(t.accountId)}
                  toAccount={
                    t.toAccountId ? accountById.get(t.toAccountId) : undefined
                  }
                  onOpen={() => edit(t.id)}
                  onDuplicate={() => duplicate(t.id)}
                />
              ))}
            </li>
          ))}
        </ul>
      </>
    );
  }

  return (
    <GlassPanel className="flex min-h-0 flex-1 flex-col overflow-hidden">
      {body}
    </GlassPanel>
  );
}
