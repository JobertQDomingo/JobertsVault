import { Check, CreditCard, TriangleAlert, X } from "lucide-react";
import type { KeyboardEvent, ReactNode } from "react";
import type { Account } from "@/bindings/Account";
import type { BudgetKept } from "@/bindings/BudgetKept";
import type { Category } from "@/bindings/Category";
import type { CategoryLine } from "@/bindings/CategoryLine";
import type { Checkin } from "@/bindings/Checkin";
import type { Comparison } from "@/bindings/Comparison";
import type { FocusFields } from "@/bindings/FocusFields";
import type { GradedFocus } from "@/bindings/GradedFocus";
import type { Offer } from "@/bindings/Offer";
import type { SettingsView } from "@/bindings/SettingsView";
import type { Win } from "@/bindings/Win";
import { ACCOUNT_ICONS } from "@/components/AccountRow";
import { CategoryChip } from "@/components/CategoryChip";
import { Change } from "@/components/Change";
import { LevelBadge } from "@/components/LevelBadge";
import { MoneyText } from "@/components/MoneyText";
import { ProgressBar } from "@/components/ProgressBar";
import {
  LEVEL_RANK,
  levelAmount,
  reachedOrPast,
  SpendLevelTag,
} from "@/components/SpendLevel";
import { StatCard } from "@/components/StatCard";
import { StatHexagon } from "@/components/StatHexagon";
import { Segmented } from "@/components/ui/segmented";
import { BudgetName } from "@/features/budgets/BudgetName";
import { ForecastTable } from "@/features/forecast/ForecastScreen";
import { behindText } from "@/features/goals/GoalsScreen";
import {
  addDays,
  formatDate,
  formatDue,
  formatOverdue,
  formatSpan,
  weekdayDate,
} from "@/lib/dates";
import { formatMoney } from "@/lib/money";
import { cn } from "@/lib/utils";
import { DROP_REASONS, nextMoveText, STAT_LABELS } from "@/lib/vault";

/** Everything a step needs: the check-in, the names it refers to, and the screen's own state. */
export type Ctx = {
  data: Checkin;
  settings: SettingsView;
  categories: Category[];
  accounts: Account[];
  loans: {
    id: number;
    name: string;
    payments: { id: number; dueDate: string }[];
  }[];
  /** The Balance check's answers, by account id. Held by the screen, never sent (CHK-4). */
  answers: Record<number, Answer>;
  answer: (accountId: number, value: Answer) => void;
  /** The picked offer, or none. */
  picked: Offer | undefined;
  pick: (offer: Offer) => void;
};

export type Answer = "match" | "mismatch";

export type Step = { say: string; body: ReactNode };

/** The period shown: a merged check-in is the monthly one (UI_FLOWS §8.7). */
export const unitOf = (data: Checkin) =>
  data.kinds.includes("month") ? "month" : "week";

const money = (ctx: Ctx, amount: number) => formatMoney(amount, ctx.settings);
const days = (n: number) => (n === 1 ? "1 day" : `${n} days`);
const plural = (n: number, one: string, many: string) =>
  n === 1 ? `1 ${one}` : `${n} ${many}`;

/** A flat `--wash` card inside the panel (UI_FLOWS §8.7). */
function Flat({
  title,
  className,
  children,
}: {
  title?: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  return (
    <section
      className={cn("grid gap-2 rounded-control bg-wash p-4", className)}
    >
      {title && <h3 className="flex items-center gap-2 text-h3">{title}</h3>}
      {children}
    </section>
  );
}

const Muted = ({ children }: { children: ReactNode }) => (
  <p className="text-muted-foreground tabular-nums">{children}</p>
);

const categoryName = (ctx: Ctx, id: number | null) =>
  ctx.categories.find((c) => c.id === id)?.name ?? "A category";
const accountName = (ctx: Ctx, id: number | null) =>
  ctx.accounts.find((a) => a.id === id)?.name ?? "Your card";
const payment = (ctx: Ctx, id: number | null) =>
  ctx.loans
    .map((l) => ({ loan: l, p: l.payments.find((p) => p.id === id) }))
    .find((x) => x.p);
const loanName = (ctx: Ctx, id: number) =>
  ctx.loans.find((l) => l.id === id)?.name ?? "A loan";

// ── Start ────────────────────────────────────────────────────────────────────────────────

function winText(win: Win, ctx: Ctx) {
  const unit = unitOf(ctx.data);
  switch (win.code) {
    case "focus_kept":
      return "You kept your focus.";
    case "stat_rose":
      return `Your ${STAT_LABELS[win.stat]} stat rose from ${win.from} to ${win.to}.`;
    case "category_fell":
      return `${categoryName(ctx, win.categoryId)} fell by ${money(ctx, win.by)} from the ${unit} before.`;
    case "spending_fell":
      return `You spent less than the ${unit} before.`;
    case "loans_paid_on_time":
      return "Every loan payment was paid on time.";
  }
}

/** A focus as one sentence, capitalized and with no full stop (UI_FLOWS §8.7's offers). */
function focusText(f: FocusFields & Partial<Pick<Offer, "level">>, ctx: Ctx) {
  const unit = addDays(f.start, 6) === f.end ? "week" : "month";
  const target = f.target ?? 0;
  switch (f.kind) {
    case "category_cap":
      return `Keep ${categoryName(ctx, f.categoryId)} under ${money(ctx, target)}`;
    case "spending_cap":
      return `Spend under ${money(ctx, target)} this ${unit}`;
    case "card_level": {
      // Named only when there's more than one card to tell apart.
      const card =
        ctx.data.cards.length > 1
          ? `${accountName(ctx, f.accountId)}'s`
          : "your";
      return f.level
        ? `Stay under ${card} ${money(ctx, target)} ${f.level} level this cycle`
        : `Stay under ${money(ctx, target)} on ${accountName(ctx, f.accountId)} this cycle`;
    }
    case "loan_on_time": {
      const due = payment(ctx, f.loanPaymentId);
      return due?.p
        ? `Pay ${due.loan.name} by ${weekdayDate(due.p.dueDate, ctx.data.today)}`
        : "Pay a loan payment on time";
    }
    case "logging":
      return `Log on at least ${target} days this ${unit}`;
  }
}

function resultText(f: GradedFocus, ctx: Ctx) {
  const actual = f.actual ?? 0;
  if (f.result === "void")
    return "That loan payment was removed, so it doesn't count.";
  if (f.kind === "loan_on_time")
    return f.result === "kept"
      ? "You paid it on time."
      : "It wasn't paid on time.";
  if (f.kind === "logging")
    return f.result === "kept"
      ? `You kept it: ${days(actual)}.`
      : `You logged on ${days(actual)}.`;
  if (f.result === "kept") return `You kept it: ${money(ctx, actual)}.`;
  return f.kind === "card_level"
    ? `It reached ${money(ctx, actual)}.`
    : `You spent ${money(ctx, actual)}, past your ${money(ctx, f.target ?? 0)} target.`;
}

export function start(ctx: Ctx): Step {
  const { win, personalBest, lastFocus } = ctx.data.start;
  const unit = unitOf(ctx.data);
  const focus = lastFocus && (
    <Flat>
      <p className="flex items-start gap-2">
        {lastFocus.result === "kept" ? (
          <Check
            aria-label="Kept"
            className="mt-0.5 size-4 shrink-0 text-success"
          />
        ) : lastFocus.result === "missed" ? (
          <X
            aria-label="Missed"
            className="mt-0.5 size-4 shrink-0 text-danger"
          />
        ) : null}
        <span className="tabular-nums">
          {`Last ${addDays(lastFocus.start, 6) === lastFocus.end ? "week" : "month"}'s focus: `}
          {`${focusText(lastFocus, ctx).replace(/^./, (c) => c.toLowerCase())}. `}
          {resultText(lastFocus, ctx)}
        </span>
      </p>
    </Flat>
  );
  return {
    say: win ? winText(win, ctx) : `Here's how last ${unit} went.`,
    body: (
      <>
        {personalBest && (
          <Muted>
            {personalBest === "lowest_expense"
              ? `Your lowest-spending ${unit} of the last 12.`
              : `Your highest net cashflow ${unit} of the last 12.`}
          </Muted>
        )}
        {focus}
      </>
    ),
  };
}

// ── Money in and out, and Your year ─────────────────────────────────────────────────────────

function Totals({
  ctx,
  comparison: c,
  top,
  budgets,
  unit,
}: {
  ctx: Ctx;
  comparison: Comparison;
  top: CategoryLine[];
  budgets: BudgetKept[];
  unit: string;
}) {
  const vs = formatSpan(c.compareStart, c.compareEnd, ctx.data.today);
  return (
    <>
      <div className="grid grid-cols-3 gap-3">
        <StatCard
          flat
          label="Income"
          amount={c.income.current}
          kind="income"
          change={c.income.change}
          vs={vs}
        />
        <StatCard
          flat
          label="Expense"
          amount={c.expense.current}
          kind="expense"
          change={c.expense.change}
          vs={vs}
        />
        <StatCard
          flat
          label="Net cashflow"
          amount={c.net.current}
          kind="net"
          change={c.net.change}
          vs={vs}
        />
      </div>
      <Flat title="Largest categories">
        {top.length === 0 ? (
          <Muted>{`No spending this ${unit}.`}</Muted>
        ) : (
          <ul className="grid gap-2">
            {top.map((line) => {
              const category = ctx.categories.find(
                (x) => x.id === line.categoryId,
              );
              return (
                <li
                  key={line.categoryId}
                  className="grid grid-cols-[32px_minmax(0,1fr)_auto_72px] items-center gap-3"
                >
                  <CategoryChip
                    color={category?.color ?? "cat-11"}
                    icon={category?.icon ?? "ellipsis"}
                  />
                  <span className="truncate font-semibold">
                    {category?.name}
                  </span>
                  <MoneyText amount={line.current} kind="expense" />
                  <span className="text-right text-small">
                    <Change tenths={line.change} upIsGood={false} />
                  </span>
                </li>
              );
            })}
          </ul>
        )}
      </Flat>
      <Flat title="Budgets">
        {budgets.length === 0 ? (
          <Muted>{`No ${unit === "year" ? "yearly" : `${unit}ly`} budgets.`}</Muted>
        ) : (
          <ul className="grid gap-2">
            {budgets.map((b) => (
              <li
                key={b.budgetId}
                className="flex items-center justify-between gap-3"
              >
                <BudgetName
                  categoryId={b.categoryId}
                  categories={ctx.categories}
                />
                {b.kept ? (
                  <span className="inline-flex items-center gap-1 font-semibold text-success">
                    <Check aria-hidden className="size-4" />
                    kept
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1 font-semibold text-danger tabular-nums">
                    <TriangleAlert aria-hidden className="size-4" />
                    {`${money(ctx, b.over)} over`}
                  </span>
                )}
              </li>
            ))}
          </ul>
        )}
      </Flat>
    </>
  );
}

export function moneyStep(ctx: Ctx): Step {
  const { comparison: c, topCategories, budgets } = ctx.data.money;
  const unit = unitOf(ctx.data);
  const net = c.net.current;
  return {
    say:
      net > 0
        ? `You earned ${money(ctx, net)} more than you spent.`
        : net < 0
          ? `You spent ${money(ctx, -net)} more than you earned.`
          : c.income.current === 0
            ? `Nothing came in or went out this ${unit}.`
            : "You spent exactly what you earned.",
    body: (
      <Totals
        ctx={ctx}
        comparison={c}
        top={topCategories}
        budgets={budgets}
        unit={unit}
      />
    ),
  };
}

export function yearStep(ctx: Ctx): Step {
  const y = ctx.data.year;
  if (!y) return { say: "", body: null };
  const c = y.comparison;
  const paidOff = y.loansPaidOff.map((id) => loanName(ctx, id));
  return {
    say: `${c.currentStart.slice(0, 4)}: you earned ${money(ctx, c.income.current)} and spent ${money(ctx, c.expense.current)}.`,
    body: (
      <>
        <Totals
          ctx={ctx}
          comparison={c}
          top={y.topCategories}
          budgets={y.budgets}
          unit="year"
        />
        <Flat title="Loans paid off">
          <Muted>
            {paidOff.length
              ? paidOff.join(", ")
              : "No loans paid off this year."}
          </Muted>
        </Flat>
        {y.firstLevel !== null && y.lastLevel !== null && (
          <Muted>{`You started the year at level ${y.firstLevel} and ended it at level ${y.lastLevel}.`}</Muted>
        )}
      </>
    ),
  };
}

// ── Card ─────────────────────────────────────────────────────────────────────────────────────

export function cardStep(ctx: Ctx): Step {
  const cards = ctx.data.cards.filter((a) => a.card);
  if (cards.length === 0) return { say: "No credit card.", body: null };
  // The card furthest along names the sentence (the first on a tie).
  const top = cards.reduce((a, b) =>
    LEVEL_RANK[b.card!.spendLevel] > LEVEL_RANK[a.card!.spendLevel] ? b : a,
  );
  const level = top.card!.spendLevel;
  const one = cards.length === 1;
  let say: string;
  if (level === "none")
    say = one
      ? "You're under your soft level this cycle."
      : "Your cards are under their soft levels this cycle.";
  else {
    const past =
      reachedOrPast(top.card!.cycleSpending, levelAmount(top, level)) ===
      "past";
    say = one
      ? past
        ? `You're past your ${level} level this cycle.`
        : `You've reached your ${level} level this cycle.`
      : past
        ? `${top.name} is past its ${level} level this cycle.`
        : `${top.name} has reached its ${level} level this cycle.`;
  }
  return {
    say,
    body: cards.map((a) => {
      const card = a.card!;
      return (
        <Flat
          key={a.id}
          title={
            <>
              <span className="grid size-8 place-items-center rounded-chip bg-wash [&_svg]:size-4">
                <CreditCard aria-hidden />
              </span>
              {a.name}
            </>
          }
        >
          <div className="flex items-baseline justify-between gap-3">
            <span className="flex items-baseline gap-2">
              <MoneyText
                amount={card.cycleSpending}
                kind="balance"
                size="display"
              />
              <span className="text-muted-foreground">this cycle</span>
            </span>
            <SpendLevelTag level={card.spendLevel} />
          </div>
          {a.spendSoft !== null && (
            <Muted>
              {`Soft ${money(ctx, a.spendSoft)} · Mid ${money(ctx, a.spendMid ?? 0)} · Hard ${money(ctx, a.spendHard ?? 0)}`}
            </Muted>
          )}
          <Muted>
            {[
              `${money(ctx, card.amountOwed)} owed`,
              a.creditLimit !== null &&
                `${card.utilizationPercent}% of your limit`,
              formatDue(card.dueDate, ctx.data.today),
            ]
              .filter(Boolean)
              .join(" · ")}
          </Muted>
        </Flat>
      );
    }),
  };
}

// ── Loans and bills ──────────────────────────────────────────────────────────────────────────

export function loansStep(ctx: Ctx): Step {
  const { paid, paidTotal, loansDue, loansLeft, debtFreeMoved, forecast } =
    ctx.data.loans;
  const { today } = ctx.data;
  const unit = unitOf(ctx.data);
  // Each row is a loan's earliest overdue payment; its others are `moreOverdue` (a count, not money).
  const overdue = loansDue
    .filter((d) => d.days < 0)
    .reduce((n, d) => n + 1 + d.moreOverdue, 0);
  const paidLine = `${plural(paid.length, "loan payment", "loan payments")} paid, ${money(ctx, paidTotal)}.`;
  const say = overdue
    ? overdue === 1
      ? "A loan payment is overdue."
      : `${overdue} loan payments are overdue.`
    : paid.length
      ? paidLine
      : loansLeft
        ? `No loan payments were paid this ${unit}.`
        : "No loans to pay.";
  const short = forecast.short;
  return {
    say,
    body: (
      <>
        {(loansLeft || loansDue.length > 0 || paid.length > 0) && (
          <Flat title="Loans">
            {overdue > 0 && paid.length > 0 && <Muted>{paidLine}</Muted>}
            {loansDue.map((d) => (
              <p
                key={d.paymentId}
                className={cn(
                  "flex items-center gap-1.5 tabular-nums",
                  d.days < 0 && "font-semibold text-danger",
                )}
              >
                {d.days < 0 && (
                  <TriangleAlert aria-hidden className="size-4 shrink-0" />
                )}
                {`${d.name}: ${money(ctx, d.amount)} ${d.days < 0 ? formatOverdue(d.dueDate, today) : formatDue(d.dueDate, today)}`}
              </p>
            ))}
            {loansLeft && (
              <Muted>
                {`Loans left: ${money(ctx, loansLeft.totalLeft)} · `}
                {loansLeft.overdue
                  ? "all overdue"
                  : `debt-free ${formatDate(loansLeft.debtFreeOn)}`}
              </Muted>
            )}
            {debtFreeMoved && (
              <Muted>{`It moved from ${formatDate(debtFreeMoved.from)}.`}</Muted>
            )}
          </Flat>
        )}
        <Flat title="Coming up">
          {forecast.safeToSpend === null || forecast.windowEnd === null ? (
            <Muted>Mark your pay as a payday to see what's coming.</Muted>
          ) : (
            <>
              {forecast.nextPayday && (
                <p>{`Next payday: ${weekdayDate(forecast.nextPayday.date, today)}`}</p>
              )}
              <Muted>
                {`${money(ctx, forecast.safeToSpend)} safe to spend until ${forecast.windowEnd === today ? "today" : weekdayDate(forecast.windowEnd, today)} · ${money(ctx, forecast.perDay ?? 0)} a day`}
              </Muted>
              {short && (
                <p className="flex items-center gap-1 font-semibold text-danger tabular-nums">
                  <TriangleAlert aria-hidden className="size-4 shrink-0" />
                  {`Short by ${money(ctx, short.amount)} on ${weekdayDate(short.date, today)}`}
                </p>
              )}
              <div className="-mx-4 -mb-2">
                <ForecastTable
                  forecast={forecast}
                  grain="window"
                  accounts={ctx.accounts}
                  settings={ctx.settings}
                />
              </div>
            </>
          )}
        </Flat>
      </>
    ),
  };
}

// ── Goals ────────────────────────────────────────────────────────────────────────────────────

export function goalsStep(ctx: Ctx): Step {
  const goals = ctx.data.goals ?? [];
  const behind = goals.filter((g) => g.behind).length;
  return {
    say:
      goals.length === 0
        ? "No goals yet."
        : behind
          ? behind === 1
            ? "1 goal is behind."
            : `${behind} goals are behind.`
          : "Your goals are on track.",
    body: goals.map((g) => {
      // Named when it moved by itself since the period shown began (DATA_MODEL §5.19).
      const moved =
        g.movedOn !== null &&
        g.movedOn >= ctx.data.period.start &&
        g.movedFrom &&
        g.targetDate;
      const late = behindText(g);
      return (
        <Flat key={g.id} title={g.name}>
          <ProgressBar
            percent={g.percent}
            tone="goal"
            done={g.achievedAt !== null}
            aria-label={`${g.name} saved`}
          />
          <Muted>{`${money(ctx, g.progress)} of ${money(ctx, g.targetAmount)} · ${money(ctx, g.remaining)} left`}</Muted>
          {moved && (
            <p>{`Now aims for ${formatDate(g.targetDate!)} (was ${formatDate(g.movedFrom!)})`}</p>
          )}
          {late && (
            <p className="text-small font-semibold text-warning">{late}</p>
          )}
        </Flat>
      );
    }),
  };
}

// ── Balance check ────────────────────────────────────────────────────────────────────────────

export function balanceStep(ctx: Ctx): Step {
  return {
    say: "Do these match your real accounts?",
    body: (
      <Flat className="gap-0 py-1">
        {(ctx.data.balances ?? []).map((b, i) => {
          const account = ctx.accounts.find((a) => a.id === b.accountId);
          const Icon = account ? ACCOUNT_ICONS[account.kind] : CreditCard;
          const off = ctx.answers[b.accountId] === "mismatch";
          return (
            <div
              key={b.accountId}
              className={cn(
                "grid grid-cols-[32px_minmax(0,1fr)_auto_auto] items-center gap-3 py-2.5",
                i > 0 && "border-t border-hairline",
              )}
            >
              <span className="grid size-8 place-items-center rounded-chip bg-wash [&_svg]:size-4">
                <Icon aria-hidden />
              </span>
              <div className="min-w-0">
                <p className="truncate font-semibold">{account?.name}</p>
                {off && (
                  <p className="text-small text-muted-foreground">
                    Add what's missing with + Add. The balance updates here.
                  </p>
                )}
              </div>
              {b.amountOwed !== null ? (
                <span className="font-semibold tabular-nums">{`${money(ctx, b.amountOwed)} owed`}</span>
              ) : (
                <MoneyText amount={b.balance} kind="balance" />
              )}
              <Segmented
                aria-label={account?.name ?? "Account"}
                value={ctx.answers[b.accountId] ?? null}
                onChange={(v) => ctx.answer(b.accountId, v)}
                options={[
                  { value: "match", label: "Matches" },
                  { value: "mismatch", label: "Doesn't match" },
                ]}
              />
            </div>
          );
        })}
      </Flat>
    ),
  };
}

// ── Stats and level ──────────────────────────────────────────────────────────────────────────

export function statsStep(ctx: Ctx): Step {
  const s = ctx.data.status;
  const prev = s.previous?.stats;
  const say =
    s.levelChange === "up"
      ? `You reached level ${s.level}.`
      : s.levelChange === "down"
        ? s.levelReason
          ? `Level ${s.level} now: ${DROP_REASONS[s.levelReason]}.`
          : `Level ${s.level} now.`
        : `You're still at level ${s.level}.`;
  const move = nextMoveText(s.nextMove, {
    money: (amount) => money(ctx, amount),
    loan: (id) => loanName(ctx, id),
    account: (id) => accountName(ctx, id),
  });
  return {
    say,
    body: (
      <>
        <Flat className="grid-cols-[280px_minmax(0,1fr)] items-center gap-10 px-6 py-5">
          <StatHexagon stats={s.stats} previous={prev} />
          <div className="grid content-center">
            {s.stats.map(({ stat, score }, i) => {
              const was = prev?.[i] ?? null;
              const change =
                score !== null && was !== null ? score - was : null;
              return (
                <div
                  key={stat}
                  className="grid h-8 grid-cols-[76px_36px_48px] items-center gap-2"
                >
                  <span>{STAT_LABELS[stat]}</span>
                  <b className="text-right font-semibold tabular-nums">
                    {score ?? "—"}
                  </b>
                  {prev && <Points change={change} />}
                </div>
              );
            })}
            {s.previous && (
              <p className="mt-2 text-small text-muted-foreground">
                {`Against the last check-in, ${weekdayDate(s.previous.completedOn, ctx.data.today)}`}
              </p>
            )}
          </div>
        </Flat>
        <Flat className="gap-2.5 px-6">
          <LevelBadge level={s.level} />
          <p className="tabular-nums">{move}</p>
          <p className="text-small text-muted-foreground">
            {`${plural(s.checkinStreak, "check-in", "check-ins")} in a row on time · ${plural(s.focusStreak, "focus", "focuses")} kept in a row`}
          </p>
        </Flat>
      </>
    ),
  };
}

/** `▲ 6`, `▼ 10` or `—` against the last check-in, with the arrow spelled out. */
function Points({ change }: { change: number | null }) {
  if (!change)
    return (
      <span className="text-muted-foreground">{change === 0 ? "0" : "—"}</span>
    );
  const up = change > 0;
  return (
    <span
      className={cn(
        "font-semibold tabular-nums",
        up ? "text-income" : "text-expense",
      )}
    >
      <span aria-hidden>{up ? "▲ " : "▼ "}</span>
      <span className="sr-only">{up ? "Up " : "Down "}</span>
      {Math.abs(change)}
    </span>
  );
}

// ── Pick a focus ─────────────────────────────────────────────────────────────────────────────

const HELPS: Record<Offer["kind"], string> = {
  category_cap: "",
  spending_cap: "",
  card_level: "Helps your Card stat",
  loan_on_time: "Helps your Bills stat",
  logging: "Helps your Habit stat",
};

export function focusStep(ctx: Ctx): Step {
  const { offers } = ctx.data;
  const unit = unitOf(ctx.data);
  const at = ctx.picked ? offers.indexOf(ctx.picked) : -1;
  // ↑ ↓ move the pick between the cards, like a radio group.
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[
      e.key
    ];
    if (!step) return;
    e.preventDefault();
    const next = at < 0 ? 0 : (at + step + offers.length) % offers.length;
    ctx.pick(offers[next]);
    (e.currentTarget.children[next] as HTMLElement | undefined)?.focus();
  };
  return {
    say: `Pick one focus for this ${unit}.`,
    body: (
      <div
        role="radiogroup"
        aria-label="Focus"
        onKeyDown={onKeyDown}
        className="grid gap-3"
      >
        {offers.map((o, i) => {
          const on = o === ctx.picked;
          const basis =
            o.kind === "category_cap"
              ? `That's what you spent the ${unit} before`
              : o.kind === "spending_cap"
                ? `Your Safe to spend, ${money(ctx, o.basis ?? 0)} a day`
                : HELPS[o.kind];
          return (
            <div
              key={i}
              role="radio"
              aria-checked={on}
              tabIndex={on || (at < 0 && i === 0) ? 0 : -1}
              onClick={() => ctx.pick(o)}
              onKeyDown={(e) => {
                if (e.key === " ") {
                  e.preventDefault();
                  ctx.pick(o);
                }
              }}
              className={cn(
                "relative grid cursor-pointer gap-0.5 rounded-control border-2 bg-wash py-3 pr-12 pl-4 outline-none focus-visible:ring-3 focus-visible:ring-ring/40",
                on ? "border-primary" : "border-transparent",
              )}
            >
              <b className="font-semibold tabular-nums">{focusText(o, ctx)}</b>
              <span className="text-small text-muted-foreground tabular-nums">
                {basis}
              </span>
              <span
                aria-hidden
                className={cn(
                  "absolute top-3 right-3 grid size-5 place-items-center rounded-full border",
                  on
                    ? "border-primary bg-primary text-primary-foreground"
                    : "border-field-edge",
                )}
              >
                {on && <Check className="size-3" />}
              </span>
            </div>
          );
        })}
      </div>
    ),
  };
}
