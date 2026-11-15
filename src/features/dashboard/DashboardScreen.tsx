import {
  CalendarClock,
  CreditCard,
  OctagonAlert,
  Plus,
  ReceiptText,
  Repeat,
  Target,
} from "lucide-react";
import { useLayoutEffect, useRef, useState } from "react";
import { Link } from "react-router";
import type { BudgetProgress } from "@/bindings/BudgetProgress";
import type { CardDue } from "@/bindings/CardDue";
import type { Category } from "@/bindings/Category";
import type { Account } from "@/bindings/Account";
import type { Dashboard } from "@/bindings/Dashboard";
import type { Transaction } from "@/bindings/Transaction";
import type { LoanDue } from "@/bindings/LoanDue";
import type { UpcomingDate } from "@/bindings/UpcomingDate";
import { AccountRow } from "@/components/AccountRow";
import { CategoryChip, TransferChip } from "@/components/CategoryChip";
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
import { SafeToSpendCard } from "./SafeToSpendCard";
import { useBackupNow } from "@/hooks/useBackup";
import { useCategories } from "@/hooks/useCategories";
import { useDashboard } from "@/hooks/useDashboard";
import { usePeriodQuery } from "@/hooks/usePeriodQuery";
import {
  formatDate,
  formatDayHeader,
  formatDue,
  formatOverdue,
  formatSpan,
  groupByDate,
} from "@/lib/dates";
import { rowTitle } from "@/lib/records";
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
      {data.loansDue.map((due) => (
        <LoanDueBanner
          key={due.loanId}
          due={due}
          account={accounts.get(due.accountId)}
          today={data.today}
        />
      ))}
      {data.backupDue && <BackupWarning />}
      {data.cardsDue.map((due) => (
        <CardDueBanner
          key={due.accountId}
          due={due}
          name={accounts.get(due.accountId)?.name}
        />
      ))}
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
      {/* Stretched, so Recent transactions fills the height of the column beside it. */}
      <div className={cn(COLUMNS, "items-stretch")}>
        <GlassPanel className="flex flex-col overflow-hidden pb-1.5">
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
            <RecentRows
              recent={data.recent}
              today={data.today}
              accounts={accounts}
              categories={categories}
              onOpen={(t) => edit(t.id, t.payslipId)}
            />
          )}
        </GlassPanel>
        <div className="grid content-start gap-3">
          <SafeToSpendCard forecast={data.safeToSpend} />
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
            {data.loansLeft && (
              <Link
                to="/debt-free"
                className="block border-t border-hairline px-[18px] py-2.5 text-small text-muted-foreground transition-colors duration-150 hover:bg-glass-hover"
              >
                Loans left:{" "}
                <MoneyText amount={data.loansLeft.totalLeft} kind="balance" /> ·{" "}
                {/* Everything left is overdue: no date already passed (DATA_MODEL §5.11). */}
                {data.loansLeft.overdue ? (
                  <span className="font-semibold text-danger">all overdue</span>
                ) : (
                  `debt-free ${formatDate(data.loansLeft.debtFreeOn)}`
                )}
              </Link>
            )}
          </GlassPanel>
        </div>
      </div>
      <div className="grid grid-cols-2 items-start gap-3">
        <Budgets budgets={data.budgets} categories={categoryList} />
        <ComingUp
          upcoming={data.upcoming}
          accounts={data.accounts}
          categories={categoryList}
          today={data.today}
        />
      </div>
    </>
  );
}

// Recent transactions always shows at least this many, when there are (DSH-8).
const MIN_RECENT = 5;

/**
 * The newest transactions, whatever the period (DSH-8), grouped by day: as many whole rows as
 * fit beside the right column, and at least 5. The rows that don't fit are invisible, so they're
 * out of sight, focus and screen readers, and never cut in half.
 */
function RecentRows({
  recent,
  today,
  accounts,
  categories,
  onOpen,
}: {
  recent: Transaction[];
  today: string;
  accounts: Map<number, Account>;
  categories: Map<number, Category>;
  onOpen: (t: Transaction) => void;
}) {
  const area = useRef<HTMLDivElement>(null);
  const [fit, setFit] = useState({ minHeight: 0, shown: recent.length });
  useLayoutEffect(() => {
    const el = area.current;
    if (!el) return;
    const measure = () => {
      const top = el.getBoundingClientRect().top;
      const bottoms = [
        ...el.querySelectorAll<HTMLElement>("[data-recent-row]"),
      ].map((row) => row.getBoundingClientRect().bottom - top);
      const minHeight = bottoms[Math.min(MIN_RECENT, bottoms.length) - 1] ?? 0;
      const shown = Math.max(
        MIN_RECENT,
        bottoms.filter((bottom) => bottom <= el.clientHeight + 0.5).length,
      );
      setFit((was) =>
        was.minHeight === minHeight && was.shown === shown
          ? was
          : { minHeight, shown },
      );
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [recent]);

  // The rows are placed out of the panel's flow, so the right column alone sets its height.
  return (
    <div
      ref={area}
      className="relative flex-1"
      style={{ minHeight: fit.minHeight }}
    >
      <div className="absolute inset-x-0 top-0">
        {groupByDate(recent).map(({ date, items }) => {
          // Its first row's place in the whole list.
          const first = recent.indexOf(items[0]);
          return (
            <section key={date}>
              <h3
                className={cn(
                  "px-[18px] pt-2.5 pb-1.5 text-small font-semibold text-muted-foreground",
                  first >= fit.shown && "invisible",
                )}
              >
                {formatDayHeader(date, today)}
              </h3>
              {items.map((t, i) => (
                <div
                  key={t.id}
                  data-recent-row
                  className={cn(first + i >= fit.shown && "invisible")}
                >
                  <TransactionRow
                    compact
                    transaction={t}
                    category={
                      t.categoryId ? categories.get(t.categoryId) : undefined
                    }
                    account={accounts.get(t.accountId)}
                    toAccount={
                      t.toAccountId ? accounts.get(t.toAccountId) : undefined
                    }
                    onOpen={() => onOpen(t)}
                  />
                </div>
              ))}
            </section>
          );
        })}
      </div>
    </div>
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

// Past this many dates, the panel links to the rest, so it stays near the Budgets panel's height.
const COMING_UP_ROWS = 8;

/**
 * Each date a recurring rule falls on in the next 7 days, soonest first (DSH-7): what it is, the
 * day, the account and whether it posts by itself or waits to be confirmed (UI_FLOWS §2).
 */
function ComingUp({
  upcoming,
  accounts,
  categories,
  today,
}: {
  upcoming: UpcomingDate[];
  accounts: Account[];
  categories: Category[];
  today: string;
}) {
  const more = upcoming.length - COMING_UP_ROWS;
  return (
    <GlassPanel className="overflow-hidden pb-2">
      <CardHeader title="Coming up" to="/recurring" link="View all" />
      {upcoming.length === 0 ? (
        <EmptyState
          icon={CalendarClock}
          message="Nothing in the next 7 days."
        />
      ) : (
        <ul>
          {upcoming
            .slice(0, COMING_UP_ROWS)
            .map(({ ruleId, mode, transaction: t }) => {
              const category = categories.find((c) => c.id === t.categoryId);
              const transfer = t.kind === "transfer";
              const account = accounts.find((a) => a.id === t.accountId)?.name;
              const when = [
                formatDayHeader(t.date, today),
                // A transfer's title already names both accounts.
                !transfer && account,
                mode === "confirm" ? "needs confirming" : "auto",
              ]
                .filter(Boolean)
                .join(" · ");
              return (
                <li
                  key={`${ruleId}-${t.date}`}
                  className="grid grid-cols-[32px_minmax(0,1fr)_auto] items-center gap-3 px-[18px] py-1.5"
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
                    <span className="truncate font-semibold">
                      {rowTitle(t, accounts, categories)}
                    </span>
                    <span className="truncate text-small text-muted-foreground">
                      {when}
                    </span>
                  </span>
                  <MoneyText amount={t.amount} kind={t.kind} />
                </li>
              );
            })}
        </ul>
      )}
      {more > 0 && (
        <Link
          to="/recurring"
          className="block px-[18px] pt-1 font-semibold text-primary hover:underline"
        >
          + {more} more
        </Link>
      )}
    </GlassPanel>
  );
}

/** Dates Confirm rules queued, with Review (UI_FLOWS F10). */
function RecurringDue({ count, today }: { count: number; today: string }) {
  const [reviewing, setReviewing] = useState(false);
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

/**
 * A loan's next unpaid payment, due within 5 days or overdue, with Pay (LOAN-4, F13), and what
 * to move to its account first when that's short (DATA_MODEL §5.11).
 */
function LoanDueBanner({
  due,
  account,
  today,
}: {
  due: LoanDue;
  account?: Account;
  today: string;
}) {
  const { markPaid } = useQuickAdd();
  const late = due.days < 0;
  return (
    <Banner
      tone={late ? "danger" : "warning"}
      icon={late ? OctagonAlert : CalendarClock}
      detail={
        due.moveFirst !== null &&
        account && (
          <>
            {account.name} has{" "}
            <MoneyText amount={account.balance} kind="balance" />. Move{" "}
            <MoneyText amount={due.moveFirst} kind="balance" /> there first.
          </>
        )
      }
      action={
        <Button size="sm" onClick={() => markPaid(due.paymentId)}>
          Pay
        </Button>
      }
    >
      {due.name}: <MoneyText amount={due.amount} kind="balance" />{" "}
      {late
        ? formatOverdue(due.dueDate, today) +
          (due.moreOverdue > 0 ? `, and ${due.moreOverdue} more overdue` : "")
        : formatDue(due.dueDate, today)}
    </Banner>
  );
}

/** A card payment due within 5 days, with something owed (CC-7). */
function CardDueBanner({ due, name }: { due: CardDue; name?: string }) {
  const { payBill } = useQuickAdd();
  const when =
    due.days === 0
      ? "today"
      : due.days === 1
        ? "tomorrow"
        : `in ${due.days} days`;
  return (
    <Banner
      icon={CreditCard}
      action={
        <Button size="sm" onClick={() => payBill(due.accountId)}>
          Pay bill
        </Button>
      }
    >
      {name} payment due {when}:{" "}
      <MoneyText amount={due.amountOwed} kind="owed" />
    </Banner>
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
