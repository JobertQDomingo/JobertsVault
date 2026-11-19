//! The level ladder, the ring and the next best move (LVL-2, LVL-4, DATA_MODEL §5.21).
//! Pure: the queries hand in the facts, and the stats come from `stats::stats`.

use serde::Serialize;
use ts_rs::TS;

use super::money::Money;
use super::stats::{StatInputs, StatReason, Stats, stats};

/// The facts §5.21 needs beyond the stats' own inputs (`StatInputs`). They must agree with
/// those: `earned` covers the same `months`, `cards` the same active cards, and an overdue
/// move is given exactly when `StatInputs` counts that kind as overdue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelFacts {
    /// Any payday rule, a paused one included (§5.15).
    pub has_payday_rule: bool,
    /// At least one budget.
    pub has_budget: bool,
    /// `paid late(30)` (§5.20).
    pub paid_late_30: i64,
    /// `earned(m)` of §5.20's `months` window, latest first: 0 to 3 entries.
    pub earned: Vec<Money>,
    /// The forecast has a short day (§5.15).
    pub has_short_day: bool,
    /// Every active credit card.
    pub cards: Vec<CardCycles>,
    /// §5.11's loan-free streak in days, none when no loan has a borrowed_on.
    pub loan_free_days: Option<i64>,
    /// The earliest overdue loan payment (by due date, then id).
    pub overdue_payment: Option<OverduePayment>,
    /// The active card's overdue statement with the earliest due date, then the lowest
    /// account id.
    pub overdue_card: Option<OverdueCard>,
}

/// One active card, for `card_under_mid`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardCycles {
    pub account_id: i64,
    /// Its `spend_mid` as it is today.
    pub spend_mid: Money,
    /// Its expenses' total in each of its latest completed cycles (§5.5), up to 3.
    pub completed_cycles: Vec<Money>,
    /// Its spending in the current cycle (§5.18).
    pub current_cycle: Money,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverduePayment {
    pub payment_id: i64,
    pub loan_id: i64,
    pub amount: Money,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverdueCard {
    pub account_id: i64,
    /// The statement amount still to pay (§5.14).
    pub amount: Money,
}

/// §5.21's condition codes, in the table's order. On the wire each is its name there
/// (`logged_20`): serde's `snake_case` puts no `_` before a digit, so the five codes with a
/// number need their own `rename`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ConditionCode {
    #[serde(rename = "logged_20")]
    Logged20,
    PaydayRule,
    Budget,
    NothingOverdue,
    NoneLate,
    EarnedMonth,
    #[serde(rename = "earned_3_months")]
    Earned3Months,
    NoShortDay,
    CardUnderMid,
    TwoWeeksSaved,
    LoansPaid,
    #[serde(rename = "loan_free_90")]
    LoanFree90,
    MonthSaved,
    #[serde(rename = "stats_80")]
    Stats80,
    NoGoalBehind,
    #[serde(rename = "loan_free_365")]
    LoanFree365,
}

/// One condition today. `gap` is never below 0, and is `None` where the table gives none or
/// there's no figure yet. A distance is 0 once met; `earned_3_months`' gap is the months so
/// far and `stats_80`'s the lowest score. `account_id` is the failing card, for `card_under_mid` only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Condition {
    pub code: ConditionCode,
    pub met: bool,
    #[ts(type = "number | null")]
    pub gap: Option<i64>,
    #[ts(type = "number | null")]
    pub account_id: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NextMove {
    /// Move 1, `pay_overdue_loan`.
    PayOverdueLoan(OverduePayment),
    /// Move 2, `pay_overdue_card`.
    PayOverdueCard(OverdueCard),
    /// Move 3: the next level's first unmet condition.
    Condition(Condition),
    /// Move 4: the lowest stat's reason, none when every stat that applies is 100.
    LowestStat(Option<StatReason>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Levels {
    /// 0 to 10.
    pub level: i64,
    /// The next level's conditions in order; level 10's own at level 10.
    pub next_conditions: Vec<Condition>,
    /// The ring: how many of `next_conditions` hold, out of how many.
    pub ring_met: i64,
    pub ring_total: i64,
    pub next_move: NextMove,
}

/// The stats are worked out here from `input`, so `stats_80` and the move can't judge a
/// different input from the rest of the ladder.
pub fn levels(facts: &LevelFacts, input: &StatInputs) -> Levels {
    let stats = &stats(input);
    let ladder = ladder(facts, input, stats);
    let level = ladder
        .iter()
        .take_while(|conds| conds.iter().all(|c| c.met))
        .count();
    let next_conditions = ladder.into_iter().nth(level.min(9)).unwrap_or_default();
    let next_move = if let Some(p) = facts.overdue_payment {
        NextMove::PayOverdueLoan(p)
    } else if let Some(c) = facts.overdue_card {
        NextMove::PayOverdueCard(c)
    } else if let Some(c) = next_conditions.iter().find(|c| !c.met) {
        NextMove::Condition(*c)
    } else {
        NextMove::LowestStat(lowest(stats).reason)
    };
    Levels {
        level: level as i64,
        ring_met: next_conditions.iter().filter(|c| c.met).count() as i64,
        ring_total: next_conditions.len() as i64,
        next_conditions,
        next_move,
    }
}

/// The lowest stat that applies, the first in §5.20's order on a tie. Habit always applies.
fn lowest(stats: &Stats) -> super::stats::Stat {
    [
        stats.cashflow,
        stats.bills,
        stats.card,
        stats.debt,
        stats.savings,
        Some(stats.habit),
    ]
    .into_iter()
    .flatten()
    .min_by_key(|s| s.score)
    .unwrap_or(stats.habit)
}

fn cond(code: ConditionCode, met: bool, gap: Option<i64>) -> Condition {
    Condition {
        code,
        met,
        gap: gap.map(|g| g.max(0)),
        account_id: None,
    }
}

/// Levels 1 to 10, each with its conditions in order.
fn ladder(facts: &LevelFacts, input: &StatInputs, stats: &Stats) -> [Vec<Condition>; 10] {
    use ConditionCode::*;
    let money = input.money();
    let avg = input.avg_spending();
    let earned_in_a_row = facts.earned.iter().take_while(|e| e.0 > 0).count() as i64;
    let loan_free = |days: i64| match facts.loan_free_days {
        // A borrowed_on after today (a clock set back) counts as 0 days.
        Some(d) => (d >= days, Some(days - d.max(0))),
        None => (true, None),
    };
    let saved_gap = |gap: i64| (avg > 0).then_some(gap);
    let (lf90_met, lf90_gap) = loan_free(90);
    let (lf365_met, lf365_gap) = loan_free(365);
    let lowest_score = lowest(stats).score;
    [
        vec![cond(
            Logged20,
            input.logged_days >= 20,
            Some(20 - input.logged_days),
        )],
        vec![
            cond(PaydayRule, facts.has_payday_rule, None),
            cond(Budget, facts.has_budget, None),
        ],
        vec![
            cond(
                NothingOverdue,
                input.overdue_loan_payments + input.overdue_cards == 0,
                None,
            ),
            cond(NoneLate, facts.paid_late_30 == 0, None),
        ],
        vec![cond(
            EarnedMonth,
            facts.earned.first().is_some_and(|e| e.0 > 0),
            None,
        )],
        vec![
            cond(Earned3Months, earned_in_a_row >= 3, Some(earned_in_a_row)),
            cond(NoShortDay, !facts.has_short_day, None),
        ],
        vec![card_under_mid(&facts.cards)],
        vec![cond(
            TwoWeeksSaved,
            // money × 2 ≥ avg, without the multiply that could overflow.
            avg > 0 && money >= (avg + 1) / 2,
            saved_gap((avg + 1) / 2 - money),
        )],
        vec![
            cond(
                LoansPaid,
                input.loan_total_left.0 == 0,
                Some(input.loan_total_left.0),
            ),
            cond(LoanFree90, lf90_met, lf90_gap),
        ],
        vec![cond(
            MonthSaved,
            avg > 0 && money >= avg,
            saved_gap(avg - money),
        )],
        vec![
            cond(Stats80, lowest_score >= 80, Some(lowest_score)),
            cond(NoGoalBehind, input.goals_behind == 0, None),
            cond(LoanFree365, lf365_met, lf365_gap),
        ],
    ]
}

/// Every active card under its `spend_mid` in each completed cycle. The gap is the first
/// failing card's (by account id) current cycle over its `spend_mid`.
fn card_under_mid(cards: &[CardCycles]) -> Condition {
    let failing = cards
        .iter()
        .filter(|c| c.completed_cycles.iter().any(|s| s.0 >= c.spend_mid.0))
        .min_by_key(|c| c.account_id);
    match failing {
        None => cond(ConditionCode::CardUnderMid, true, Some(0)),
        Some(c) => Condition {
            account_id: Some(c.account_id),
            ..cond(
                ConditionCode::CardUnderMid,
                false,
                Some(c.current_cycle.0 - c.spend_mid.0),
            )
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::account::SpendLevel;
    use crate::domain::stats::{CardInput, stats};
    use ConditionCode::*;

    /// Level 10 with every stat at 100: 900 spent a month, 10,000 in the bank, a paid-off loan
    /// borrowed 400 days ago, and one card well under its mid level.
    fn top() -> (LevelFacts, StatInputs) {
        let facts = LevelFacts {
            has_payday_rule: true,
            has_budget: true,
            paid_late_30: 0,
            earned: vec![Money(100), Money(100), Money(100)],
            has_short_day: false,
            cards: vec![CardCycles {
                account_id: 1,
                spend_mid: Money(4000),
                completed_cycles: vec![Money(1000), Money(0), Money(3999)],
                current_cycle: Money(500),
            }],
            loan_free_days: Some(400),
            overdue_payment: None,
            overdue_card: None,
        };
        let input = StatInputs {
            months: 3,
            income: Money(3000),
            expense: Money(2700),
            deductions: Money(0),
            borrowed: Money(0),
            cash_balance: Money(10000),
            logged_days: 25,
            overdue_loan_payments: 0,
            overdue_cards: 0,
            paid_late_90: 0,
            has_loan: true,
            cards: vec![CardInput {
                account_id: 1,
                level: SpendLevel::None,
                utilization_percent: 5,
            }],
            loan_total_left: Money(0),
            loan_due_this_month: Money(0),
            goals_behind: 0,
            on_time_weeks: 4,
        };
        (facts, input)
    }

    fn run(facts: &LevelFacts, input: &StatInputs) -> Levels {
        levels(facts, input)
    }

    /// The level after `change`, and the next level's first unmet condition.
    fn with(change: impl FnOnce(&mut LevelFacts, &mut StatInputs)) -> Levels {
        let (mut facts, mut input) = top();
        change(&mut facts, &mut input);
        run(&facts, &input)
    }

    fn first_unmet(l: &Levels) -> Condition {
        *l.next_conditions
            .iter()
            .find(|c| !c.met)
            .expect("an unmet condition")
    }

    #[test]
    fn the_top_situation_is_level_10_with_a_full_ring_and_no_move() {
        let l = with(|_, _| {});
        assert_eq!(l.level, 10);
        assert_eq!((l.ring_met, l.ring_total), (3, 3));
        let codes: Vec<_> = l.next_conditions.iter().map(|c| c.code).collect();
        assert_eq!(codes, vec![Stats80, NoGoalBehind, LoanFree365]);
        assert_eq!(l.next_move, NextMove::LowestStat(None));
    }

    /// Every condition of the ladder, broken on its own from level 10: the level drops to the
    /// one below it, which is reached, and that condition is the next level's first unmet.
    #[test]
    fn each_condition_unmet_caps_the_level_just_below_its_own() {
        type Change = fn(&mut LevelFacts, &mut StatInputs);
        let cases: [(i64, ConditionCode, Option<i64>, Change); 17] = [
            (0, Logged20, Some(1), |_, i| i.logged_days = 19),
            (1, PaydayRule, None, |f, _| f.has_payday_rule = false),
            (1, Budget, None, |f, _| f.has_budget = false),
            (2, NothingOverdue, None, |_, i| i.overdue_loan_payments = 1),
            (2, NothingOverdue, None, |_, i| i.overdue_cards = 1),
            (2, NoneLate, None, |f, _| f.paid_late_30 = 1),
            (3, EarnedMonth, None, |f, _| f.earned[0] = Money(0)),
            (4, Earned3Months, Some(1), |f, _| f.earned[1] = Money(0)),
            (4, NoShortDay, None, |f, _| f.has_short_day = true),
            (5, CardUnderMid, Some(0), |f, _| {
                f.cards[0].completed_cycles[1] = Money(4000)
            }),
            // (900 + 1) ÷ 2 − 449 = 1.
            (6, TwoWeeksSaved, Some(1), |_, i| {
                i.cash_balance = Money(449)
            }),
            (7, LoansPaid, Some(500), |_, i| {
                i.loan_total_left = Money(500)
            }),
            (7, LoanFree90, Some(1), |f, _| f.loan_free_days = Some(89)),
            (8, MonthSaved, Some(1), |_, i| i.cash_balance = Money(899)),
            // Habit 70: no check-in on time.
            (9, Stats80, Some(70), |_, i| i.on_time_weeks = 0),
            (9, NoGoalBehind, None, |_, i| i.goals_behind = 1),
            (9, LoanFree365, Some(1), |f, _| f.loan_free_days = Some(364)),
        ];
        for (level, code, gap, change) in cases {
            let l = with(change);
            assert_eq!(l.level, level, "{code:?}");
            let c = first_unmet(&l);
            assert_eq!((c.code, c.gap), (code, gap), "{code:?}");
        }
    }

    #[test]
    fn each_threshold_is_reached_exactly_at_its_figure() {
        assert_eq!(with(|_, i| i.logged_days = 20).level, 10);
        // A streak below 0 (a clock set back) is 0 days, so 90 are still to go.
        let l = with(|f, _| f.loan_free_days = Some(-5));
        assert_eq!((l.level, first_unmet(&l).gap), (7, Some(90)));
        // 450 × 2 = 900: two weeks saved, but not a month.
        let l = with(|_, i| i.cash_balance = Money(450));
        assert_eq!(l.level, 8);
        assert_eq!(first_unmet(&l).code, MonthSaved);
        // Exactly a month saved reaches 9; the Savings stat (33) then holds level 10 back.
        let l = with(|_, i| i.cash_balance = Money(900));
        assert_eq!(l.level, 9);
        assert_eq!(first_unmet(&l).code, Stats80);
        assert_eq!(with(|f, _| f.loan_free_days = Some(365)).level, 10);
        // 90 days is debt-free; 365 is still to come.
        let l = with(|f, _| f.loan_free_days = Some(90));
        assert_eq!(l.level, 9);
        assert_eq!(first_unmet(&l).gap, Some(275));
        // Cashflow 50 + 180 × 500 ÷ 3000 = 80 holds; 179 left gives 79, which doesn't.
        assert_eq!(with(|_, i| i.expense = Money(2820)).level, 10);
        let l = with(|_, i| i.expense = Money(2821));
        assert_eq!(l.level, 9);
        assert_eq!(first_unmet(&l), cond(Stats80, false, Some(79)));
    }

    #[test]
    fn one_overdue_payment_drops_a_level_8_situation_to_2() {
        let (mut facts, mut input) = top();
        input.cash_balance = Money(899);
        assert_eq!(run(&facts, &input).level, 8);
        input.overdue_loan_payments = 1;
        let payment = OverduePayment {
            payment_id: 7,
            loan_id: 3,
            amount: Money(1000),
        };
        facts.overdue_payment = Some(payment);
        let l = run(&facts, &input);
        assert_eq!(l.level, 2);
        assert_eq!(first_unmet(&l).code, NothingOverdue);
        assert_eq!((l.ring_met, l.ring_total), (1, 2));
        assert_eq!(l.next_move, NextMove::PayOverdueLoan(payment));
    }

    #[test]
    fn with_no_card_level_6_asks_nothing() {
        let l = with(|f, i| {
            f.cards.clear();
            i.cards.clear();
        });
        assert_eq!(l.level, 10);
        // Reached on its own, with level 7 failing.
        let l = with(|f, i| {
            f.cards.clear();
            i.cards.clear();
            i.cash_balance = Money(0);
        });
        assert_eq!(l.level, 6);
        assert_eq!(first_unmet(&l).code, TwoWeeksSaved);
    }

    #[test]
    fn card_under_mid_names_the_first_failing_card_by_id() {
        let card = |id, cycles: [i64; 3], current| CardCycles {
            account_id: id,
            spend_mid: Money(4000),
            completed_cycles: cycles.map(Money).to_vec(),
            current_cycle: Money(current),
        };
        let cards = [
            card(5, [0, 0, 0], 9000),
            card(3, [4500, 0, 0], 4200),
            card(2, [0, 0, 4000], 1000),
        ];
        // Card 2 is over in an old cycle and under now: its gap is 0, never below.
        let c = card_under_mid(&cards);
        assert_eq!((c.met, c.account_id, c.gap), (false, Some(2), Some(0)));
        let c = card_under_mid(&cards[..2]);
        assert_eq!((c.met, c.account_id, c.gap), (false, Some(3), Some(200)));
        // A card with fewer than 3 completed cycles, or none, is checked on what it has.
        let c = card_under_mid(&[
            card(4, [0, 0, 0], 0),
            CardCycles {
                completed_cycles: vec![],
                ..card(6, [0, 0, 0], 0)
            },
        ]);
        assert!(c.met);
    }

    #[test]
    fn with_no_loan_the_debt_free_conditions_are_met_with_no_gap() {
        let l = with(|f, i| {
            f.loan_free_days = None;
            i.has_loan = false;
        });
        assert_eq!(l.level, 10);
        assert_eq!(l.next_conditions[2], cond(LoanFree365, true, None));
        // Reached on its own, with level 9 failing.
        let l = with(|f, i| {
            f.loan_free_days = None;
            i.has_loan = false;
            i.cash_balance = Money(899);
        });
        assert_eq!(l.level, 8);
        let (facts, input) = top();
        let facts = LevelFacts {
            loan_free_days: None,
            ..facts
        };
        let ladder = ladder(&facts, &input, &stats(&input));
        assert_eq!(ladder[7][1], cond(LoanFree90, true, None));
    }

    #[test]
    fn with_no_completed_month_the_history_levels_wait() {
        let (mut facts, mut input) = top();
        facts.earned.clear();
        input.months = 0;
        let s = stats(&input);
        let l = levels(&facts, &input);
        assert_eq!(l.level, 3);
        assert_eq!(first_unmet(&l).code, EarnedMonth);
        let ladder = ladder(&facts, &input, &s);
        assert_eq!(ladder[4][0], cond(Earned3Months, false, Some(0)));
        // No spending average yet: unmet, with no gap to show.
        assert_eq!(ladder[6][0], cond(TwoWeeksSaved, false, None));
        assert_eq!(ladder[8][0], cond(MonthSaved, false, None));
    }

    #[test]
    fn two_completed_months_stop_at_level_4() {
        let l = with(|f, _| f.earned.truncate(2));
        assert_eq!(l.level, 4);
        assert_eq!(first_unmet(&l).code, Earned3Months);
        assert_eq!(first_unmet(&l).gap, Some(2));
        assert_eq!((l.ring_met, l.ring_total), (1, 2));
    }

    #[test]
    fn level_0_names_logging_and_its_gap() {
        let l = with(|_, i| i.logged_days = 3);
        assert_eq!(l.level, 0);
        assert_eq!((l.ring_met, l.ring_total), (0, 1));
        assert_eq!(
            l.next_move,
            NextMove::Condition(cond(Logged20, false, Some(17)))
        );
    }

    // The next best move, in its order.

    #[test]
    fn move_1_an_overdue_loan_payment_comes_before_a_card() {
        let payment = OverduePayment {
            payment_id: 1,
            loan_id: 1,
            amount: Money(500),
        };
        let card = OverdueCard {
            account_id: 1,
            amount: Money(800),
        };
        let l = with(|f, i| {
            f.overdue_payment = Some(payment);
            f.overdue_card = Some(card);
            i.overdue_loan_payments = 1;
            i.overdue_cards = 1;
        });
        assert_eq!(l.next_move, NextMove::PayOverdueLoan(payment));
    }

    #[test]
    fn move_2_an_overdue_card_comes_before_the_next_condition() {
        let card = OverdueCard {
            account_id: 4,
            amount: Money(800),
        };
        let l = with(|f, i| {
            f.overdue_card = Some(card);
            i.overdue_cards = 1;
            f.has_budget = false;
        });
        assert_eq!(l.level, 1);
        assert_eq!(l.next_move, NextMove::PayOverdueCard(card));
    }

    #[test]
    fn move_3_the_next_levels_first_unmet_condition_with_its_gap() {
        let l = with(|f, _| {
            f.earned[1] = Money(0);
            f.has_short_day = true;
        });
        assert_eq!(l.level, 4);
        assert_eq!(
            l.next_move,
            NextMove::Condition(cond(Earned3Months, false, Some(1)))
        );
        // A later condition when the first holds.
        let l = with(|f, _| f.has_short_day = true);
        assert_eq!(
            l.next_move,
            NextMove::Condition(cond(NoShortDay, false, None))
        );
    }

    #[test]
    fn move_4_at_level_10_the_lowest_stats_reason() {
        // Habit 70 + 3 × 30 ÷ 4 = 92, every other stat 100.
        let l = with(|_, i| i.on_time_weeks = 3);
        assert_eq!(l.level, 10);
        assert_eq!(
            l.next_move,
            NextMove::LowestStat(Some(StatReason::LateCheckins))
        );
        // A tie goes to the first in §5.20's order: Cashflow 50 + 252 × 500 ÷ 3000 = 92
        // before Habit's 92.
        let l = with(|_, i| {
            i.expense = Money(2748);
            i.on_time_weeks = 3;
        });
        assert_eq!(l.level, 10);
        assert_eq!(
            l.next_move,
            NextMove::LowestStat(Some(StatReason::LittleLeft))
        );
    }
}
