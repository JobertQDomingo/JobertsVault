import {
  ChevronRight,
  Ellipsis,
  HandCoins,
  Plus,
  TriangleAlert,
} from "lucide-react";
import { useState, type ReactNode } from "react";
import type { Account } from "@/bindings/Account";
import type { Loan } from "@/bindings/Loan";
import type { LoanList } from "@/bindings/LoanList";
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
import { useCategories } from "@/hooks/useCategories";
import {
  useArchiveLoan,
  useDeleteLoan,
  useLoans,
  useUnarchiveLoan,
} from "@/hooks/useLoans";
import { useSettings } from "@/hooks/useSettings";
import { useTransactionDefaults } from "@/hooks/useTransactions";
import {
  formatDue,
  formatLongDate,
  formatOverdue,
  formatPeriodLabel,
} from "@/lib/dates";
import { formatMoney, type MoneySettings } from "@/lib/money";
import { cn } from "@/lib/utils";
import { LoanDialog } from "./LoanDialog";
import { PaymentsDialog } from "./PaymentsDialog";

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

/**
 * `/debt-free` (UI_FLOWS §8.5): when the loans end and how far along they are (LOAN-5 to LOAN-8),
 * one flat card per loan inside the Loans panel, and Money freed up. Every figure is the
 * backend's (`list_loans`).
 */
export function DebtFreeScreen() {
  const loans = useLoans();
  const accounts = useAccounts();
  const categories = useCategories();
  const settings = useSettings();
  // `today` for "due tomorrow" and the form's defaults, as the backend gives it.
  const defaults = useTransactionDefaults();
  const queries = [loans, accounts, categories, settings, defaults];
  const failed = queries.filter((q) => q.isError);
  const archive = useArchiveLoan();
  const unarchive = useUnarchiveLoan();
  const remove = useDeleteLoan();
  // `{}` is a new loan.
  const [editing, setEditing] = useState<{ loan?: Loan } | null>(null);
  const [opened, setOpened] = useState<number | null>(null);
  const [deleting, setDeleting] = useState<Loan | null>(null);
  const [showArchived, setShowArchived] = useState(false);
  const addLoan = (
    <Button variant="primary" size="sm" onClick={() => setEditing({})}>
      <Plus />
      Add loan
    </Button>
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
    !loans.data ||
    !accounts.data ||
    !categories.data ||
    !settings.data ||
    !defaults.data
  )
    return (
      <Loading className="grid gap-3">
        <Skeleton className="h-[120px]" />
        <Skeleton className="h-[260px]" />
      </Loading>
    );

  const money = settings.data;
  const today = defaults.data.today;
  const list = loans.data.loans;
  const active = list.filter((l) => !l.archived);
  const archived = list.filter((l) => l.archived);
  const open = list.find((l) => l.id === opened);
  const settled = (message: string) => ({
    onSuccess: () => notify.success(message),
    onError: (error: Error) => notify.error(error.message),
  });
  const cards = (shown: Loan[]) => (
    <ul className="grid grid-cols-2 gap-3 px-[18px] pb-[18px]">
      {shown.map((loan) => (
        <LoanCard
          key={loan.id}
          loan={loan}
          today={today}
          accounts={accounts.data.accounts}
          settings={money}
          onOpen={() => setOpened(loan.id)}
          actions={
            loan.archived ? (
              <MenuItem
                disabled={unarchive.isPending}
                onSelect={() =>
                  unarchive.mutate(loan.id, settled("Unarchived"))
                }
              >
                Unarchive
              </MenuItem>
            ) : (
              <>
                <MenuItem onSelect={() => setEditing({ loan })}>Edit</MenuItem>
                {loan.paidOff && (
                  <MenuItem
                    disabled={archive.isPending}
                    onSelect={() =>
                      archive.mutate(loan.id, settled("Archived"))
                    }
                  >
                    Archive
                  </MenuItem>
                )}
                {!loan.payments.some((p) => p.status === "paid") && (
                  <MenuItem destructive onSelect={() => setDeleting(loan)}>
                    Delete
                  </MenuItem>
                )}
              </>
            )
          }
        />
      ))}
    </ul>
  );

  return (
    <div className="grid gap-3">
      {list.length === 0 ? (
        <GlassPanel>
          <EmptyState
            icon={HandCoins}
            message="No loans. Keep it that way."
            action={addLoan}
          />
        </GlassPanel>
      ) : (
        <>
          <Summary data={loans.data} settings={money} />
          <GlassPanel className="overflow-hidden">
            <div className="flex items-center justify-between gap-2 px-[18px] pt-4 pb-2.5">
              <h2 className="text-h3">Loans</h2>
              {addLoan}
            </div>
            {active.length ? (
              cards(active)
            ) : (
              <p className="px-[18px] pb-4 text-muted-foreground">
                No active loans.
              </p>
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
          </GlassPanel>
          <FreedUp data={loans.data} settings={money} />
        </>
      )}
      {open && (
        <PaymentsDialog
          loan={open}
          categories={categories.data.categories}
          settings={money}
          onEdit={() => {
            setOpened(null);
            setEditing({ loan: open });
          }}
          onClose={() => setOpened(null)}
        />
      )}
      {editing && (
        <LoanDialog
          loan={editing.loan}
          accounts={accounts.data.accounts}
          categories={categories.data.categories}
          today={today}
          weekStart={money.weekStart}
          settings={money}
          onClose={() => setEditing(null)}
        />
      )}
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(isOpen) => !isOpen && setDeleting(null)}
        title="Delete this loan?"
        message="Its payments are deleted. This can't be undone."
        confirmLabel="Delete"
        destructive
        onConfirm={() =>
          deleting && remove.mutate(deleting.id, settled("Deleted"))
        }
      />
    </div>
  );
}

/** The top panel (LOAN-5 to LOAN-8): the debt-free date, the totals, the streak and the interest. */
function Summary({
  data,
  settings,
}: {
  data: LoanList;
  settings: MoneySettings;
}) {
  const s = data.summary;
  const money = (amount: number) => formatMoney(amount, settings);
  const unpaid = data.loans.flatMap((l) =>
    l.payments.filter((p) => p.status === "unpaid"),
  );
  // Everything left is overdue: say what's owed, not a date already passed (DATA_MODEL §5.11).
  const allOverdue = unpaid.length > 0 && unpaid.every((p) => p.overdue);
  const withInterest = data.loans.filter((l) => l.interest !== null).length;
  let headline: ReactNode;
  if (s.debtFreeOn === null)
    headline = <h2 className="text-h3 text-success">You're debt-free.</h2>;
  else if (allOverdue)
    headline = (
      <h2 className="flex items-center gap-2 text-h3 text-danger">
        <TriangleAlert aria-hidden className="size-5" />
        {`${money(s.totalLeft)} overdue. Pay it to be debt-free.`}
      </h2>
    );
  else
    headline = (
      <div className="flex items-baseline justify-between gap-3">
        <h2 className="text-h3">Debt-free on {formatLongDate(s.debtFreeOn)}</h2>
        <span className="text-small text-muted-foreground tabular-nums">
          {s.daysToGo === 0
            ? "Last payment today"
            : `${plural(s.daysToGo ?? 0, "day")} to go`}
        </span>
      </div>
    );
  const extras = [
    s.streakDays !== null &&
      `${plural(s.streakDays, "day")} since your last new loan`,
    s.interest !== null &&
      `${money(s.interest)} in interest on ${plural(withInterest, "loan")}`,
  ].filter(Boolean);
  return (
    <GlassPanel className="grid gap-2 px-[18px] py-4">
      {headline}
      <div className="flex items-center gap-4">
        <span className="text-small whitespace-nowrap text-muted-foreground tabular-nums">
          <span className="font-semibold text-foreground">
            {money(s.totalLeft)}
          </span>
          {` left · ${money(s.totalPaid)} paid`}
        </span>
        <ProgressBar
          percent={s.percentPaid}
          tone="goal"
          done={s.debtFreeOn === null}
          label={`${s.percentPaid}% paid`}
          aria-label="Loans paid"
          className="flex-1"
        />
      </div>
      {extras.length > 0 && (
        <p className="text-small text-muted-foreground tabular-nums">
          {extras.join(" · ")}
        </p>
      )}
    </GlassPanel>
  );
}

function LoanCard({
  loan,
  today,
  accounts,
  settings,
  onOpen,
  actions,
}: {
  loan: Loan;
  today: string;
  accounts: Account[];
  settings: MoneySettings;
  onOpen: () => void;
  actions: ReactNode;
}) {
  const money = (amount: number) => formatMoney(amount, settings);
  const account = accounts.find((a) => a.id === loan.accountId)?.name ?? "";
  const done = loan.payments.filter((p) => p.status !== "unpaid").length;
  const next = loan.next;
  return (
    // The name's button stretches over the whole card, so the progress bar stays outside it.
    <li className="relative grid content-start gap-2.5 rounded-control bg-wash p-4 pt-3 transition-colors duration-150 hover:bg-glass-hover">
      <div className="flex min-h-8 items-center gap-2">
        <button
          type="button"
          onClick={onOpen}
          aria-label={`Open ${loan.name}`}
          className="min-w-0 flex-1 truncate text-left font-semibold outline-none after:absolute after:inset-0 after:rounded-control focus-visible:after:ring-3 focus-visible:after:ring-ring/40"
        >
          {loan.name}
        </button>
        {loan.paidOff && (
          <span className="rounded-pill bg-[color-mix(in_srgb,var(--success)_var(--tint),transparent)] px-2 text-small font-semibold text-success">
            Paid off
          </span>
        )}
        <Menu>
          <MenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              aria-label={`More actions for ${loan.name}`}
              className="relative -mr-2"
            >
              <Ellipsis />
            </Button>
          </MenuTrigger>
          <MenuContent>{actions}</MenuContent>
        </Menu>
      </div>
      <p className="-mt-2 truncate text-small text-muted-foreground">
        {`${account} · ${done} of ${loan.payments.length} paid`}
      </p>
      <ProgressBar
        percent={loan.percentPaid}
        tone="goal"
        done={loan.paidOff}
        aria-label={`${loan.name} paid`}
      />
      <p className="text-small text-muted-foreground tabular-nums">
        {loan.interest === null
          ? `${money(loan.left)} left`
          : `${money(loan.left)} left · ${money(loan.interest)} interest`}
      </p>
      {next &&
        (next.overdue ? (
          <p className="flex items-center gap-1 text-small font-semibold text-danger tabular-nums">
            <TriangleAlert aria-hidden className="size-3.5 shrink-0" />
            {`Next ${money(next.amount)} ${formatOverdue(next.dueDate, today)}`}
          </p>
        ) : (
          <p className="text-small text-muted-foreground tabular-nums">
            Next{" "}
            <span className="font-semibold text-foreground">
              {money(next.amount)}
            </span>{" "}
            {formatDue(next.dueDate, today)}
          </p>
        ))}
    </li>
  );
}

/** Money freed up (LOAN-7): this month to the month after the debt-free date. */
function FreedUp({
  data,
  settings,
}: {
  data: LoanList;
  settings: MoneySettings;
}) {
  const months = data.summary.freedUp;
  if (!months.length) return null;
  const money = (amount: number) => formatMoney(amount, settings);
  return (
    <GlassPanel className="overflow-hidden">
      <h2 className="px-[18px] pt-4 pb-2.5 text-h3">Money freed up</h2>
      <ul className="max-h-[320px] overflow-y-auto pb-2">
        {months.map((m, i) => (
          <li
            key={m.start}
            className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 border-t border-hairline px-[18px] py-2.5"
          >
            <div className="min-w-0">
              <p className="truncate font-semibold">
                {formatPeriodLabel("month", m.start, m.end)}
              </p>
              <p className="truncate text-small text-muted-foreground tabular-nums">
                {i === 0
                  ? "This month"
                  : m.freed > 0
                    ? `${money(m.freed)} less than this month`
                    : m.freed < 0
                      ? `${money(-m.freed)} more than this month`
                      : "Same as this month"}
              </p>
            </div>
            <span className="font-semibold tabular-nums">
              {m.due === 0 ? money(0) : `${money(m.due)} to loans`}
            </span>
          </li>
        ))}
      </ul>
    </GlassPanel>
  );
}
