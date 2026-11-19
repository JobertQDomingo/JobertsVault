// The Vault level's words (UI_FLOWS §8.7–8.8): the backend sends codes, and this file is the one
// place that turns them into text, so the check-in and the Level screen say the same thing.
import type { ConditionCode } from "@/bindings/ConditionCode";
import type { NextMoveView } from "@/bindings/NextMoveView";
import type { StatName } from "@/bindings/StatName";
import type { StatReason } from "@/bindings/StatReason";

/** Levels 1 to 10 (DATA_MODEL §5.21). Level 0 has no name. */
export const LEVEL_NAMES = [
  "Tracker",
  "Planner",
  "Bill Keeper",
  "In the black",
  "Steady",
  "Card under control",
  "Two weeks ahead",
  "Debt-free",
  "One month ahead",
  "Vault Master",
];

/** "Level 4 · In the black", or "Level 0" alone. */
export const levelTitle = (level: number) =>
  level === 0 ? "Level 0" : `Level ${level} · ${LEVEL_NAMES[level - 1]}`;

export const STAT_LABELS: Record<StatName, string> = {
  cashflow: "Cashflow",
  bills: "Bills",
  card: "Card",
  debt: "Debt",
  savings: "Savings",
  habit: "Habit",
};

/** What the reason codes say, after "Weakest: <stat>." (DATA_MODEL §5.20). */
export const REASONS: Record<StatReason, string> = {
  no_take_home: "No take-home pay in the last 3 months.",
  spending_over_income: "You spent more than you took home.",
  little_left: "Little of your take-home pay is left after spending.",
  loan_overdue: "A loan payment is overdue.",
  card_overdue: "A card bill is overdue.",
  paid_late: "A loan payment was paid late.",
  card_past_hard: "A card is past its hard level.",
  card_past_mid: "A card is past its mid level.",
  card_past_soft: "A card is past its soft level.",
  card_high_use: "A card uses much of its limit.",
  loans_due: "Loan payments are due this month.",
  loans_left: "Something is left on your loans.",
  goal_behind: "A goal is behind.",
  cushion_low: "Your money covers less than 3 months of spending.",
  few_days_logged: "You logged on few of the last 30 days.",
  late_checkins: "Some check-ins weren't finished on time.",
};

/** The stat each reason belongs to. */
const REASON_STAT: Record<StatReason, StatName> = {
  no_take_home: "cashflow",
  spending_over_income: "cashflow",
  little_left: "cashflow",
  loan_overdue: "bills",
  card_overdue: "bills",
  paid_late: "bills",
  card_past_hard: "card",
  card_past_mid: "card",
  card_past_soft: "card",
  card_high_use: "card",
  loans_due: "debt",
  loans_left: "debt",
  goal_behind: "savings",
  cushion_low: "savings",
  few_days_logged: "habit",
  late_checkins: "habit",
};

/** "Weakest: Savings. Your money covers less than 3 months of spending." */
export const weakest = (reason: StatReason) =>
  `Weakest: ${STAT_LABELS[REASON_STAT[reason]]}. ${REASONS[reason]}`;

/** Why the level fell, after "Level 2 now: " (the first unmet condition, DATA_MODEL §5.21). */
export const DROP_REASONS: Record<ConditionCode, string> = {
  logged_20: "you logged on fewer than 20 of the last 30 days",
  payday_rule: "there's no payday rule",
  budget: "there's no budget",
  nothing_overdue: "a payment is overdue",
  none_late: "a loan payment was paid late in the last 30 days",
  earned_month: "last month you spent more than you earned",
  earned_3_months: "you didn't earn more than you spent 3 months in a row",
  no_short_day: "a pay window runs short",
  card_under_mid: "a card went past its mid level",
  two_weeks_saved: "your money covers less than two weeks of spending",
  loans_paid: "a loan isn't paid off",
  loan_free_90: "you took a new loan in the last 90 days",
  month_saved: "your money covers less than a month of spending",
  stats_80: "a stat is below 80",
  no_goal_behind: "a goal is behind",
  loan_free_365: "you took a new loan in the last year",
};

type Names = {
  money: (amount: number) => string;
  loan: (id: number) => string;
  account: (id: number) => string;
};

const days = (n: number) => (n === 1 ? "1 day" : `${n} days`);

/** A condition as the thing to do, with its gap where it has one (DATA_MODEL §5.21). */
function conditionMove(
  code: ConditionCode,
  gap: number | null,
  accountId: number | null,
  { money, account }: Names,
) {
  switch (code) {
    case "logged_20":
      return gap ? `log on ${days(gap)} more` : "log on 20 of the last 30 days";
    case "payday_rule":
      return "mark your pay as a payday";
    case "budget":
      return "set a budget";
    case "nothing_overdue":
      return "pay what's overdue";
    case "none_late":
      return "pay every loan payment on time for 30 days";
    case "earned_month":
      return "keep this month's spending under your income";
    case "earned_3_months":
      return `earn more than you spend 3 months in a row, ${gap ?? 0} of 3 so far`;
    case "no_short_day":
      return "keep every pay window from running short";
    case "card_under_mid": {
      const card = accountId === null ? "your cards" : account(accountId);
      return gap
        ? `keep ${card} under its mid level, ${money(gap)} over now`
        : `keep ${card} under its mid level`;
    }
    case "two_weeks_saved":
      return gap === null
        ? "save enough to cover two weeks of spending"
        : `save ${money(gap)} more to cover two weeks of spending`;
    case "loans_paid":
      return gap === null
        ? "pay off your loans"
        : `pay off your loans, ${money(gap)} left`;
    case "loan_free_90":
      return gap === null
        ? "go 90 days without a new loan"
        : `go 90 days without a new loan, ${days(gap)} to go`;
    case "month_saved":
      return gap === null
        ? "save enough to cover a month of spending"
        : `save ${money(gap)} more to cover a month of spending`;
    case "stats_80":
      return gap === null
        ? "get every stat to 80"
        : `get every stat to 80, the lowest is ${gap}`;
    case "no_goal_behind":
      return "catch up on the goal that's behind";
    case "loan_free_365":
      return gap === null
        ? "go a year without a new loan"
        : `go a year without a new loan, ${days(gap)} to go`;
  }
}

/** "Next best move: pay off your loans, ₱8,000.00 left." (DATA_MODEL §5.21's table). */
export function nextMoveText(move: NextMoveView, names: Names) {
  switch (move.kind) {
    case "pay_overdue_loan":
      return `Next best move: pay the overdue ${names.loan(move.loanId)} payment, ${names.money(move.amount)}.`;
    case "pay_overdue_card":
      return `Next best move: pay ${names.account(move.accountId)}'s overdue bill, ${names.money(move.amount)}.`;
    case "condition": {
      const { code, gap, accountId } = move.condition;
      return `Next best move: ${conditionMove(code, gap, accountId, names)}.`;
    }
    case "lowest_stat":
      return move.reason
        ? `You're at the top level. ${weakest(move.reason)}`
        : "You're at the top level.";
  }
}
