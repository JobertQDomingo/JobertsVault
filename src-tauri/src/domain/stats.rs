//! The six stats and their reasons (LVL-1, DATA_MODEL §5.20). Integers only: every division
//! rounds down. Where the number divided can be below 0, that's toward minus infinity
//! (`div_euclid`); elsewhere it's never below 0, so `/` is the same.

use super::account::SpendLevel;
use super::money::Money;

/// The raw figures for `today`, as the queries give them. The shared figures of §5.20 are
/// worked out from these by the methods below.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatInputs {
    /// The latest completed months counted, 0 to 3: those that ended before the earliest
    /// transaction's date are left out.
    pub months: i64,
    /// `I`: income over those months (§5.2).
    pub income: Money,
    /// `E`: expense over those months (§5.2).
    pub expense: Money,
    /// `D`: the expenses in `E` with `payslip_id` set.
    pub deductions: Money,
    /// Σ `amount_borrowed` of new loans whose `borrowed_on` is in those months.
    pub borrowed: Money,
    /// Σ balance today over every cash, bank and e-wallet account, archived ones included.
    /// Signed: below 0 counts as 0.
    pub cash_balance: Money,
    /// Dates from `today − 29` to `today` with at least one transaction of any kind.
    pub logged_days: i64,
    /// Unpaid loan payments dated before today.
    pub overdue_loan_payments: i64,
    /// Active cards whose latest statement still has something to pay and is past its due date.
    pub overdue_cards: i64,
    /// Loan payments whose expense is dated after their `due_date`, with the expense
    /// dated from `today − 89` to `today`.
    pub paid_late_90: i64,
    /// Any loan at all, paid-off and archived ones included.
    pub has_loan: bool,
    /// Every active credit card.
    pub cards: Vec<CardInput>,
    /// §5.11's `total left` over every loan.
    pub loan_total_left: Money,
    /// §5.11's `due(current month)`, paid payments included (LOAN-7).
    pub loan_due_this_month: Money,
    /// Active goals that are Behind (§5.17).
    pub goals_behind: i64,
    /// Of the 4 latest completed weeks, those whose week row was finished on time (`checkin::on_time`).
    /// 0 to 4. Only for `get_checkin`'s status and the snapshot Finish stores, the check-in being
    /// finished counts as finished today; elsewhere only existing rows count.
    pub on_time_weeks: i64,
}

/// One active card, for the Card stat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardInput {
    /// Breaks a tie: the lowest id gives the score.
    pub account_id: i64,
    /// The level its spending this cycle has reached (§5.18).
    pub level: SpendLevel,
    /// A whole percent (§5.1), over 100 past the limit.
    pub utilization_percent: i64,
}

/// The main thing holding a stat down (§5.20's reason table, in its order). On the wire each
/// is its snake_case name there (`no_take_home`), not camelCase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatReason {
    NoTakeHome,
    SpendingOverIncome,
    LittleLeft,
    LoanOverdue,
    CardOverdue,
    PaidLate,
    CardPastHard,
    CardPastMid,
    CardPastSoft,
    CardHighUse,
    LoansDue,
    LoansLeft,
    GoalBehind,
    CushionLow,
    FewDaysLogged,
    LateCheckins,
}

/// A score from 0 to 100, and its reason when it's below 100.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stat {
    pub score: i64,
    pub reason: Option<StatReason>,
}

/// Each stat is `None` when it doesn't apply ("—"). Habit always applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub cashflow: Option<Stat>,
    pub bills: Option<Stat>,
    pub card: Option<Stat>,
    pub debt: Option<Stat>,
    pub savings: Option<Stat>,
    pub habit: Stat,
}

impl StatInputs {
    /// `max(0, I − D − borrowed)`.
    pub fn take_home(&self) -> i64 {
        (self.income.0 - self.deductions.0 - self.borrowed.0).max(0)
    }

    /// `E − D`.
    pub fn spending(&self) -> i64 {
        self.expense.0 - self.deductions.0
    }

    /// `take_home ÷ months`, 0 with no month.
    pub fn avg_take_home(&self) -> i64 {
        per_month(self.take_home(), self.months)
    }

    /// `spending ÷ months`, 0 with no month.
    pub fn avg_spending(&self) -> i64 {
        per_month(self.spending(), self.months)
    }

    /// `max(0, cash_balance)`.
    pub fn money(&self) -> i64 {
        self.cash_balance.0.max(0)
    }
}

pub fn stats(input: &StatInputs) -> Stats {
    Stats {
        cashflow: cashflow(input),
        bills: bills(input),
        card: card(&input.cards),
        debt: debt(input),
        savings: savings(input),
        habit: habit(input),
    }
}

fn per_month(total: i64, months: i64) -> i64 {
    if months > 0 {
        total.div_euclid(months)
    } else {
        0
    }
}

/// A reason only when the score is below 100.
fn stat(score: i64, reason: StatReason) -> Stat {
    Stat {
        score,
        reason: (score < 100).then_some(reason),
    }
}

fn cashflow(input: &StatInputs) -> Option<Stat> {
    if input.months == 0 || (input.income.0 == 0 && input.expense.0 == 0) {
        return None;
    }
    let take_home = input.take_home();
    let spending = input.spending();
    if take_home == 0 {
        return Some(stat(0, StatReason::NoTakeHome));
    }
    // In i128: the product can pass i64 for large amounts.
    let left = i128::from(take_home - spending) * 500;
    let score = (50 + left.div_euclid(i128::from(take_home))).clamp(0, 100) as i64;
    let reason = if spending > take_home {
        StatReason::SpendingOverIncome
    } else {
        StatReason::LittleLeft
    };
    Some(stat(score, reason))
}

fn bills(input: &StatInputs) -> Option<Stat> {
    if !input.has_loan && input.cards.is_empty() {
        return None;
    }
    let overdue = input.overdue_loan_payments + input.overdue_cards;
    let score = (100 - 25 * overdue - 10 * input.paid_late_90).max(0);
    let reason = if input.overdue_loan_payments > 0 {
        StatReason::LoanOverdue
    } else if input.overdue_cards > 0 {
        StatReason::CardOverdue
    } else {
        StatReason::PaidLate
    };
    Some(stat(score, reason))
}

fn card(cards: &[CardInput]) -> Option<Stat> {
    cards
        .iter()
        .map(|c| (one_card(c), c.account_id))
        .min_by_key(|(s, id)| (s.score, *id))
        .map(|(s, _)| s)
}

pub(crate) fn one_card(card: &CardInput) -> Stat {
    let (level_points, past) = match card.level {
        SpendLevel::None => (100, StatReason::CardHighUse),
        SpendLevel::Soft => (70, StatReason::CardPastSoft),
        SpendLevel::Mid => (40, StatReason::CardPastMid),
        SpendLevel::Hard => (10, StatReason::CardPastHard),
    };
    let u = card.utilization_percent;
    let use_points = if u <= 10 {
        100
    } else if u >= 90 {
        0
    } else {
        (90 - u) * 100 / 80
    };
    let score = (level_points * 60 + use_points * 40) / 100;
    // The level is the reason when it costs at least as many points as the use does.
    let reason = if (100 - level_points) * 60 >= (100 - use_points) * 40 {
        past
    } else {
        StatReason::CardHighUse
    };
    stat(score, reason)
}

fn debt(input: &StatInputs) -> Option<Stat> {
    if input.loan_total_left.0 == 0 {
        return Some(stat(100, StatReason::LoansLeft));
    }
    if input.months == 0 {
        return None;
    }
    let due = input.loan_due_this_month.0;
    if due == 0 {
        return Some(stat(95, StatReason::LoansLeft));
    }
    let avg = input.avg_take_home();
    let score = if avg == 0 {
        0
    } else {
        let share = i128::from(due) * 100 / i128::from(avg);
        (95 - share * 95 / 90).clamp(0, 95) as i64
    };
    Some(stat(score, StatReason::LoansDue))
}

fn savings(input: &StatInputs) -> Option<Stat> {
    if input.months == 0 {
        return None;
    }
    let money = input.money();
    let avg = input.avg_spending();
    let cushion = if avg > 0 {
        (i128::from(money) * 100 / (3 * i128::from(avg))).min(100) as i64
    } else if money > 0 {
        100
    } else {
        0
    };
    let behind = 15 * input.goals_behind;
    let reason = if behind > 100 - cushion {
        StatReason::GoalBehind
    } else {
        StatReason::CushionLow
    };
    Some(stat((cushion - behind).max(0), reason))
}

fn habit(input: &StatInputs) -> Stat {
    let days = (input.logged_days * 70 / 25).min(70);
    // Capped so an odd period edge (§5.19) can't push Habit past 100.
    let checkins = input.on_time_weeks.min(4) * 30 / 4;
    let reason = if 70 - days >= 30 - checkins {
        StatReason::FewDaysLogged
    } else {
        StatReason::LateCheckins
    };
    stat(days + checkins, reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use StatReason::*;

    /// Three months, 3,000 take-home and 2,700 spent (900 a month): Cashflow 100.
    fn base() -> StatInputs {
        StatInputs {
            months: 3,
            income: Money(3000),
            expense: Money(2700),
            deductions: Money(0),
            borrowed: Money(0),
            cash_balance: Money(0),
            logged_days: 0,
            overdue_loan_payments: 0,
            overdue_cards: 0,
            paid_late_90: 0,
            has_loan: false,
            cards: vec![],
            loan_total_left: Money(0),
            loan_due_this_month: Money(0),
            goals_behind: 0,
            on_time_weeks: 0,
        }
    }

    fn s(score: i64, reason: Option<StatReason>) -> Option<Stat> {
        Some(Stat { score, reason })
    }

    fn card_at(id: i64, level: SpendLevel, u: i64) -> CardInput {
        CardInput {
            account_id: id,
            level,
            utilization_percent: u,
        }
    }

    // Cashflow

    fn cash(income: i64, expense: i64) -> Option<Stat> {
        cashflow(&StatInputs {
            income: Money(income),
            expense: Money(expense),
            ..base()
        })
    }

    #[test]
    fn cashflow_is_100_at_10_percent_left_50_at_even_and_0_at_10_percent_over() {
        assert_eq!(cash(1000, 900), s(100, None));
        assert_eq!(cash(1000, 0), s(100, None));
        assert_eq!(cash(1000, 901), s(99, Some(LittleLeft)));
        assert_eq!(cash(1000, 1000), s(50, Some(LittleLeft)));
        assert_eq!(cash(1000, 1001), s(49, Some(SpendingOverIncome)));
        assert_eq!(cash(1000, 1100), s(0, Some(SpendingOverIncome)));
        assert_eq!(cash(1000, 5000), s(0, Some(SpendingOverIncome)));
    }

    #[test]
    fn cashflow_rounds_down_toward_minus_infinity() {
        // 50 + (−1 × 500) ÷ 300 = 50 − 2: −1.67 rounds down to −2, not −1.
        assert_eq!(cash(300, 301), s(48, Some(SpendingOverIncome)));
        // 50 + (−1 × 500) ÷ 1000 = 50 − 1: −0.5 rounds down to −1.
        assert_eq!(cash(1000, 1001), s(49, Some(SpendingOverIncome)));
        // 50 + (1 × 500) ÷ 1000 = 50: 0.5 rounds down to 0.
        assert_eq!(cash(1000, 999), s(50, Some(LittleLeft)));
    }

    #[test]
    fn cashflow_is_none_with_no_month_or_nothing_logged() {
        let none = StatInputs {
            months: 0,
            ..base()
        };
        assert_eq!(cashflow(&none), None);
        assert_eq!(cash(0, 0), None);
        // Only one side being 0 still applies.
        assert_eq!(cash(0, 100), s(0, Some(NoTakeHome)));
        assert_eq!(cash(100, 0), s(100, None));
    }

    #[test]
    fn cashflow_leaves_deductions_out_of_both_sides() {
        // Gross 1,200, deductions 200, other spending 900: take-home 1,000, spending 900.
        let input = StatInputs {
            income: Money(1200),
            expense: Money(1100),
            deductions: Money(200),
            ..base()
        };
        assert_eq!(cashflow(&input), s(100, None));
    }

    #[test]
    fn cashflow_is_0_when_take_home_is_0() {
        // Deductions take the whole gross.
        let input = StatInputs {
            income: Money(1000),
            expense: Money(1000),
            deductions: Money(1000),
            ..base()
        };
        assert_eq!(cashflow(&input), s(0, Some(NoTakeHome)));
    }

    #[test]
    fn borrowed_money_is_not_take_home_pay() {
        // 1,000 earned and 5,000 borrowed (logged as income), 1,000 spent: take-home 1,000.
        let input = StatInputs {
            income: Money(6000),
            expense: Money(1000),
            borrowed: Money(5000),
            ..base()
        };
        assert_eq!(input.take_home(), 1000);
        assert_eq!(cashflow(&input), s(50, Some(LittleLeft)));
        // Borrowed above income: take-home is 0, never negative.
        let input = StatInputs {
            income: Money(3000),
            expense: Money(1000),
            borrowed: Money(5000),
            ..base()
        };
        assert_eq!(input.take_home(), 0);
        assert_eq!(cashflow(&input), s(0, Some(NoTakeHome)));
    }

    // Bills

    fn bill(loans: i64, cards: i64, late: i64) -> Option<Stat> {
        bills(&StatInputs {
            has_loan: true,
            overdue_loan_payments: loans,
            overdue_cards: cards,
            paid_late_90: late,
            ..base()
        })
    }

    #[test]
    fn bills_take_25_per_overdue_and_10_per_late_payment() {
        assert_eq!(bill(0, 0, 0), s(100, None));
        assert_eq!(bill(0, 0, 1), s(90, Some(PaidLate)));
        assert_eq!(bill(0, 1, 0), s(75, Some(CardOverdue)));
        assert_eq!(bill(1, 1, 1), s(40, Some(LoanOverdue)));
        assert_eq!(bill(4, 0, 0), s(0, Some(LoanOverdue)));
        assert_eq!(bill(3, 2, 5), s(0, Some(LoanOverdue)));
        assert_eq!(bill(0, 0, 10), s(0, Some(PaidLate)));
    }

    #[test]
    fn a_loan_overdue_is_named_before_a_card_overdue() {
        assert_eq!(bill(1, 3, 0).unwrap().reason, Some(LoanOverdue));
        assert_eq!(bill(0, 1, 3).unwrap().reason, Some(CardOverdue));
    }

    #[test]
    fn bills_is_none_with_no_loan_and_no_card() {
        assert_eq!(bills(&base()), None);
        let a_card = StatInputs {
            cards: vec![card_at(1, SpendLevel::None, 0)],
            ..base()
        };
        assert_eq!(bills(&a_card), s(100, None));
    }

    // Card

    fn one(level: SpendLevel, u: i64) -> Option<Stat> {
        card(&[card_at(1, level, u)])
    }

    #[test]
    fn card_use_is_full_to_10_percent_and_empty_from_90() {
        assert_eq!(one(SpendLevel::None, 0), s(100, None));
        assert_eq!(one(SpendLevel::None, 10), s(100, None));
        // U = (90 − 11) × 100 ÷ 80 = 98; 60 + 39 = 99.
        assert_eq!(one(SpendLevel::None, 11), s(99, Some(CardHighUse)));
        // U = 50; 60 + 20 = 80.
        assert_eq!(one(SpendLevel::None, 50), s(80, Some(CardHighUse)));
        // U = 1; (6000 + 40) ÷ 100 = 60.
        assert_eq!(one(SpendLevel::None, 89), s(60, Some(CardHighUse)));
        assert_eq!(one(SpendLevel::None, 90), s(60, Some(CardHighUse)));
        assert_eq!(one(SpendLevel::None, 150), s(60, Some(CardHighUse)));
    }

    #[test]
    fn card_levels_cost_30_60_and_90_of_their_60_points() {
        assert_eq!(one(SpendLevel::Soft, 0), s(82, Some(CardPastSoft)));
        assert_eq!(one(SpendLevel::Mid, 0), s(64, Some(CardPastMid)));
        assert_eq!(one(SpendLevel::Hard, 0), s(46, Some(CardPastHard)));
        assert_eq!(one(SpendLevel::Hard, 90), s(6, Some(CardPastHard)));
    }

    #[test]
    fn the_card_reason_is_the_level_when_it_costs_at_least_as_much_as_the_use() {
        // Soft costs 30 × 60 = 1800; U = 55 costs 45 × 40 = 1800: a tie goes to the level.
        // u = 46 gives U = (90 − 46) × 100 ÷ 80 = 55.
        assert_eq!(
            one(SpendLevel::Soft, 46).unwrap().reason,
            Some(CardPastSoft)
        );
        // u = 47 gives U = 53, costing 1880: now the use.
        assert_eq!(one(SpendLevel::Soft, 47).unwrap().reason, Some(CardHighUse));
        // Mid costs 3600, the most any use can (100 × 40 = 4000) nearly reaches.
        assert_eq!(one(SpendLevel::Mid, 90).unwrap().reason, Some(CardHighUse));
        assert_eq!(one(SpendLevel::Mid, 80).unwrap().reason, Some(CardPastMid));
        assert_eq!(
            one(SpendLevel::Hard, 90).unwrap().reason,
            Some(CardPastHard)
        );
    }

    #[test]
    fn with_several_cards_the_lowest_wins_and_a_tie_goes_to_the_lowest_id() {
        let cards = [
            card_at(3, SpendLevel::None, 0),
            card_at(2, SpendLevel::None, 50),
            card_at(1, SpendLevel::Soft, 0),
        ];
        assert_eq!(card(&cards), s(80, Some(CardHighUse)));
        // Both score 60: id 5 for its use, id 4 for its level. Id 4 gives the reason.
        // Mid at u = 18: U = 90, (2400 + 3600) ÷ 100 = 60.
        let tie = [
            card_at(5, SpendLevel::None, 90),
            card_at(4, SpendLevel::Mid, 18),
        ];
        assert_eq!(card(&tie), s(60, Some(CardPastMid)));
    }

    #[test]
    fn card_is_none_with_no_active_card() {
        assert_eq!(card(&[]), None);
    }

    // Debt

    fn owing(due: i64) -> StatInputs {
        // Average take-home pay: 1,000 a month.
        StatInputs {
            income: Money(3000),
            expense: Money(0),
            loan_total_left: Money(50_000),
            loan_due_this_month: Money(due),
            ..base()
        }
    }

    #[test]
    fn debt_is_100_with_nothing_left_and_95_with_nothing_due() {
        assert_eq!(debt(&base()), s(100, None));
        // Nothing left counts even with no completed month.
        let new = StatInputs {
            months: 0,
            ..base()
        };
        assert_eq!(debt(&new), s(100, None));
        assert_eq!(debt(&owing(0)), s(95, Some(LoansLeft)));
    }

    #[test]
    fn debt_falls_from_95_to_0_as_loans_take_up_to_90_percent_of_take_home() {
        // share 1 → 95 − 95 ÷ 90 = 94.
        assert_eq!(debt(&owing(10)), s(94, Some(LoansDue)));
        // share 0 (9 × 100 ÷ 1000 rounds down) → 95, but something is still due.
        assert_eq!(debt(&owing(9)), s(95, Some(LoansDue)));
        // share 45 → 95 − 47 = 48.
        assert_eq!(debt(&owing(450)), s(48, Some(LoansDue)));
        // share 89 → 95 − 93 = 2; 90 → 0.
        assert_eq!(debt(&owing(890)), s(2, Some(LoansDue)));
        assert_eq!(debt(&owing(900)), s(0, Some(LoansDue)));
        assert_eq!(debt(&owing(5000)), s(0, Some(LoansDue)));
    }

    #[test]
    fn debt_is_0_with_something_due_and_no_take_home() {
        let input = StatInputs {
            income: Money(0),
            ..owing(100)
        };
        assert_eq!(debt(&input), s(0, Some(LoansDue)));
    }

    #[test]
    fn debt_averages_take_home_over_the_months_counted() {
        // One month of 1,000: due 450 is share 45.
        let one_month = StatInputs {
            months: 1,
            income: Money(1000),
            ..owing(450)
        };
        assert_eq!(debt(&one_month), s(48, Some(LoansDue)));
    }

    #[test]
    fn debt_is_none_with_something_left_and_no_completed_month() {
        let input = StatInputs {
            months: 0,
            ..owing(100)
        };
        assert_eq!(debt(&input), None);
    }

    // Savings

    fn saved(money: i64, behind: i64) -> Option<Stat> {
        // Average spending: 900 a month, so three months is 2,700.
        savings(&StatInputs {
            cash_balance: Money(money),
            goals_behind: behind,
            ..base()
        })
    }

    #[test]
    fn savings_is_full_at_three_months_of_spending() {
        assert_eq!(saved(0, 0), s(0, Some(CushionLow)));
        assert_eq!(saved(27, 0), s(1, Some(CushionLow)));
        assert_eq!(saved(1350, 0), s(50, Some(CushionLow)));
        assert_eq!(saved(2699, 0), s(99, Some(CushionLow)));
        assert_eq!(saved(2700, 0), s(100, None));
        assert_eq!(saved(1_000_000, 0), s(100, None));
    }

    #[test]
    fn a_negative_total_balance_is_no_cushion() {
        assert_eq!(saved(-5000, 0), s(0, Some(CushionLow)));
    }

    #[test]
    fn each_goal_behind_takes_15_without_going_below_0() {
        assert_eq!(saved(2700, 1), s(85, Some(GoalBehind)));
        assert_eq!(saved(2700, 2), s(70, Some(GoalBehind)));
        assert_eq!(saved(2700, 6), s(10, Some(GoalBehind)));
        assert_eq!(saved(2700, 7), s(0, Some(GoalBehind)));
        assert_eq!(saved(1350, 4), s(0, Some(GoalBehind)));
        // With no cushion, the cushion costs more than one goal does.
        assert_eq!(saved(0, 1), s(0, Some(CushionLow)));
    }

    #[test]
    fn goals_are_the_reason_only_when_they_cost_more_than_the_cushion_does() {
        // Cushion 85 costs 15, one goal costs 15: a tie names the cushion.
        assert_eq!(saved(2295, 1), s(70, Some(CushionLow)));
        // Cushion 86 costs 14: now the goal.
        assert_eq!(saved(2322, 1), s(71, Some(GoalBehind)));
    }

    #[test]
    fn savings_with_no_spending_is_100_with_money_and_0_without() {
        let quiet = |money| {
            savings(&StatInputs {
                expense: Money(0),
                cash_balance: Money(money),
                ..base()
            })
        };
        assert_eq!(quiet(1), s(100, None));
        assert_eq!(quiet(0), s(0, Some(CushionLow)));
    }

    #[test]
    fn savings_is_none_with_no_completed_month() {
        let input = StatInputs {
            months: 0,
            cash_balance: Money(1000),
            ..base()
        };
        assert_eq!(savings(&input), None);
    }

    // Habit

    fn habit_at(logged: i64, on_time: i64) -> Stat {
        habit(&StatInputs {
            logged_days: logged,
            on_time_weeks: on_time,
            ..base()
        })
    }

    #[test]
    fn habit_is_full_at_25_days_and_4_on_time_weeks() {
        assert_eq!(
            habit_at(0, 0),
            Stat {
                score: 0,
                reason: Some(FewDaysLogged)
            }
        );
        assert_eq!(
            habit_at(25, 4),
            Stat {
                score: 100,
                reason: None
            }
        );
        assert_eq!(habit_at(25, 5).score, 100);
        assert_eq!(
            habit_at(30, 4),
            Stat {
                score: 100,
                reason: None
            }
        );
        // 12 × 70 ÷ 25 = 33; 2 × 30 ÷ 4 = 15.
        assert_eq!(habit_at(12, 2).score, 48);
        // 1 day is 2; 1, 3 weeks are 7, 22.
        assert_eq!(habit_at(1, 0).score, 2);
        assert_eq!(habit_at(0, 1).score, 7);
        assert_eq!(habit_at(0, 3).score, 22);
        assert_eq!(habit_at(24, 0).score, 67);
    }

    #[test]
    fn the_habit_reason_is_days_when_they_cost_at_least_as_much_as_check_ins() {
        // 25 days and no check-ins: days cost 0, check-ins 30.
        assert_eq!(habit_at(25, 0).reason, Some(LateCheckins));
        // 0 days, every check-in on time.
        assert_eq!(habit_at(0, 4).reason, Some(FewDaysLogged));
        // 15 days is 42 (costs 28); 0 weeks costs 30.
        assert_eq!(habit_at(15, 0).reason, Some(LateCheckins));
        // 14 days is 39 (costs 31).
        assert_eq!(habit_at(14, 0).reason, Some(FewDaysLogged));
        // 22 days is 61 (costs 9); 3 weeks is 22 (costs 8): days.
        assert_eq!(habit_at(22, 3).reason, Some(FewDaysLogged));
        // 25 days, 3 weeks: only check-ins are missing.
        assert_eq!(
            habit_at(25, 3),
            Stat {
                score: 92,
                reason: Some(LateCheckins)
            }
        );
    }

    // All six

    #[test]
    fn stats_gathers_all_six_and_habit_always_applies() {
        let none = StatInputs {
            months: 0,
            income: Money(0),
            expense: Money(0),
            loan_total_left: Money(1),
            ..base()
        };
        let all = stats(&none);
        assert_eq!(
            (all.cashflow, all.bills, all.card, all.debt, all.savings),
            (None, None, None, None, None)
        );
        assert_eq!(
            all.habit,
            Stat {
                score: 0,
                reason: Some(FewDaysLogged)
            }
        );

        let full = StatInputs {
            cash_balance: Money(2700),
            logged_days: 25,
            on_time_weeks: 4,
            has_loan: true,
            cards: vec![card_at(1, SpendLevel::None, 5)],
            ..base()
        };
        let hundred = s(100, None);
        assert_eq!(
            stats(&full),
            Stats {
                cashflow: hundred,
                bills: hundred,
                card: hundred,
                debt: hundred,
                savings: hundred,
                habit: hundred.unwrap(),
            }
        );
    }
}
