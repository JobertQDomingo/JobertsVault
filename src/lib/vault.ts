// The Vault level's words (UI_FLOWS §8.7–8.8): the backend sends codes, and this file is the one
// place that turns them into text, so the check-in and the Level screen say the same thing.
import type { Account } from "@/bindings/Account";
import type { Category } from "@/bindings/Category";
import type { Condition } from "@/bindings/Condition";
import type { Loan } from "@/bindings/Loan";
import type { ConditionCode } from "@/bindings/ConditionCode";
import type { FocusFields } from "@/bindings/FocusFields";
import type { NextMoveView } from "@/bindings/NextMoveView";
import type { StatName } from "@/bindings/StatName";
import type { StatReason } from "@/bindings/StatReason";
import { addDays, weekdayDate } from "@/lib/dates";

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

/** Each level's conditions, levels 1 to 10, in DATA_MODEL §5.21's order. */
export const LEVEL_CONDITIONS: ConditionCode[][] = [
  ["logged_20"],
  ["payday_rule", "budget"],
  ["nothing_overdue", "none_late"],
  ["earned_month"],
  ["earned_3_months", "no_short_day"],
  ["card_under_mid"],
  ["two_weeks_saved"],
  ["loans_paid", "loan_free_90"],
  ["month_saved"],
  ["stats_80", "no_goal_behind", "loan_free_365"],
];

/** A condition on the Level screen's checklist and path (UI_FLOWS §8.8). */
export const CONDITIONS: Record<ConditionCode, string> = {
  logged_20: "Log on 20 of the last 30 days",
  payday_rule: "Mark your pay as a payday",
  budget: "Set a budget",
  nothing_overdue: "Nothing overdue",
  none_late: "No loan payment paid late in the last 30 days",
  earned_month: "Earn more than you spent last month",
  earned_3_months: "Earn more than you spend 3 months in a row",
  no_short_day: "No pay window runs short in the next 12 months",
  card_under_mid: "Every card under its mid level for 3 cycles",
  two_weeks_saved: "Two weeks of spending saved",
  loans_paid: "Pay off every loan",
  loan_free_90: "90 days without a new loan",
  month_saved: "A month of spending saved",
  stats_80: "Every stat at 80 or more",
  no_goal_behind: "No goal behind",
  loan_free_365: "A year without a new loan",
};

/** An unmet condition's gap, after "·": "₱8,000.00 left", "30 days to go". None when it has none. */
export function conditionGap(
  { code, met, gap, accountId }: Condition,
  { money, account }: Pick<Names, "money" | "account">,
) {
  if (met || gap === null) return null;
  switch (code) {
    case "logged_20":
      return gap ? `${days(gap)} more to log` : null;
    case "earned_3_months":
      return `${gap} of 3 so far`;
    case "card_under_mid":
      return gap && accountId !== null
        ? `${account(accountId)} is ${money(gap)} over now`
        : null;
    case "two_weeks_saved":
    case "month_saved":
      return `${money(gap)} to go`;
    case "loans_paid":
      return `${money(gap)} left`;
    case "loan_free_90":
    case "loan_free_365":
      return `${days(gap)} to go`;
    case "stats_80":
      return `the lowest is ${gap}`;
    default:
      return null;
  }
}

/** What a focus's sentence needs to name things (UI_FLOWS §8.7's offers, §2's strip). */
export type FocusNames = {
  money: (amount: number) => string;
  category: (id: number | null) => string;
  account: (id: number | null) => string;
  /** Active cards: a card is named only when there's more than one. */
  cards: number;
  payment: (id: number | null) => { loan: string; dueDate: string } | undefined;
  today: string;
};

/** The names a focus's sentence needs, from the lists the screen loaded. */
export function focusNames(
  today: string,
  money: FocusNames["money"],
  categories: Category[],
  accounts: Account[],
  loans: Loan[],
): FocusNames {
  return {
    money,
    category: (id) => categories.find((c) => c.id === id)?.name ?? "A category",
    account: (id) => accounts.find((a) => a.id === id)?.name ?? "Your card",
    cards: accounts.filter((a) => a.kind === "credit_card" && !a.archived)
      .length,
    payment: (id) => {
      for (const loan of loans) {
        const p = loan.payments.find((p) => p.id === id);
        if (p) return { loan: loan.name, dueDate: p.dueDate };
      }
    },
    today,
  };
}

/** A card-level focus names its level when its target is one of the card's three. */
export const cardLevel =
  (accounts: Account[]) => (f: Pick<FocusFields, "accountId" | "target">) => {
    const a = accounts.find((a) => a.id === f.accountId);
    if (!a || f.target === null) return null;
    if (f.target === a.spendSoft) return "soft";
    if (f.target === a.spendMid) return "mid";
    return f.target === a.spendHard ? "hard" : null;
  };

/** "week" or "month", from the dates a focus runs for. */
export const focusUnit = (f: Pick<FocusFields, "start" | "end">) =>
  addDays(f.start, 6) === f.end ? "week" : "month";

/** A focus as one sentence, capitalized and with no full stop. `level` names a card level. */
export function focusText(
  f: FocusFields,
  names: FocusNames,
  level?: string | null,
) {
  const unit = focusUnit(f);
  const target = f.target ?? 0;
  const { money } = names;
  switch (f.kind) {
    case "category_cap":
      return `Keep ${names.category(f.categoryId)} under ${money(target)}`;
    case "spending_cap":
      return `Spend under ${money(target)} this ${unit}`;
    case "card_level": {
      const card = names.cards > 1 ? `${names.account(f.accountId)}'s` : "your";
      return level
        ? `Stay under ${card} ${money(target)} ${level} level this cycle`
        : `Stay under ${money(target)} on ${names.account(f.accountId)} this cycle`;
    }
    case "loan_on_time": {
      const due = names.payment(f.loanPaymentId);
      return due
        ? `Pay ${due.loan} by ${weekdayDate(due.dueDate, names.today)}`
        : "Pay a loan payment on time";
    }
    case "logging":
      return `Log on at least ${target} days this ${unit}`;
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
