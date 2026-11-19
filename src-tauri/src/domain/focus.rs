//! Focus offers, grading and a cap's state (CHK-7, CHK-8, DATA_MODEL §5.22). Pure: the
//! queries hand in the figures. "Everyday spending" (the expenses left after loan payments,
//! recurring dates and payslip deductions) is worked out by the query, never here.

use chrono::NaiveDate;

use super::account::{SpendLevel, SpendLevels, spend_level};
use super::money::Money;
use super::stats::{CardInput, Stat, Stats, one_card};

/// The facts §5.22's offer table needs, for the period the focus would run in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferFacts {
    /// The focus would run for a week; else for a month.
    pub for_week: bool,
    /// The earliest unpaid payment of an active loan due in the focus period.
    pub loan_payment_id: Option<i64>,
    /// Every active credit card.
    pub cards: Vec<CardOffer>,
    /// Any payday rule.
    pub has_payday_rule: bool,
    /// §5.15's `per day`.
    pub per_day: Money,
    /// Every active expense category.
    pub categories: Vec<CategorySpending>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardOffer {
    pub account_id: i64,
    pub levels: SpendLevels,
    /// Its cycle spending today (§5.18).
    pub spent: Money,
    /// As in `CardInput`, for its Card score.
    pub utilization_percent: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CategorySpending {
    pub category_id: i64,
    /// Its everyday spending in the period the check-in reviews.
    pub reviewed: Money,
    /// Its everyday spending in the period before that one.
    pub before: Money,
}

/// One focus, as offered and as stored in `checkins`' focus columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    LoanOnTime {
        payment_id: i64,
    },
    CardLevel {
        account_id: i64,
        target: Money,
    },
    SpendingCap {
        target: Money,
    },
    CategoryCap {
        category_id: i64,
        target: Money,
    },
    /// `target` is days.
    Logging {
        target: i64,
    },
}

/// Up to 3 offers, by their stat's score today, lowest first, ties in §5.22's table order.
/// Logging always applies, so there's always at least one.
pub fn offers(facts: &OfferFacts, stats: &Stats) -> Vec<Focus> {
    // A stat that's none ranks as 100.
    let score = |s: Option<Stat>| s.map_or(100, |s| s.score);
    let mut ranked: Vec<(i64, Focus)> = [
        loan_on_time(facts).map(|f| (score(stats.bills), f)),
        card_level(&facts.cards).map(|f| (score(stats.card), f)),
        spending_cap(facts).map(|f| (score(stats.cashflow), f)),
        category_cap(&facts.categories).map(|f| (score(stats.cashflow), f)),
        Some((
            stats.habit.score,
            Focus::Logging {
                target: if facts.for_week { 5 } else { 20 },
            },
        )),
    ]
    .into_iter()
    .flatten()
    .collect();
    // Stable, so equal scores keep the table's order.
    ranked.sort_by_key(|(s, _)| *s);
    ranked.into_iter().take(3).map(|(_, f)| f).collect()
}

fn loan_on_time(facts: &OfferFacts) -> Option<Focus> {
    facts
        .loan_payment_id
        .map(|payment_id| Focus::LoanOnTime { payment_id })
}

/// The card below hard with the lowest Card score (ties by account id), aiming at the lowest
/// of its levels still above its spending.
fn card_level(cards: &[CardOffer]) -> Option<Focus> {
    let (card, level) = cards
        .iter()
        .map(|c| (c, spend_level(c.spent, c.levels)))
        .filter(|(_, level)| *level != SpendLevel::Hard)
        .min_by_key(|(c, level)| {
            let input = CardInput {
                account_id: c.account_id,
                level: *level,
                utilization_percent: c.utilization_percent,
            };
            (one_card(&input).score, c.account_id)
        })?;
    let target = match level {
        SpendLevel::None => card.levels.soft,
        SpendLevel::Soft => card.levels.mid,
        _ => card.levels.hard,
    };
    Some(Focus::CardLevel {
        account_id: card.account_id,
        target,
    })
}

fn spending_cap(facts: &OfferFacts) -> Option<Focus> {
    (facts.for_week && facts.has_payday_rule && facts.per_day.0 > 0).then(|| Focus::SpendingCap {
        target: Money(facts.per_day.0.saturating_mul(7)),
    })
}

/// The largest rise in amount (ties by category id) over an earlier period above 0.
fn category_cap(categories: &[CategorySpending]) -> Option<Focus> {
    categories
        .iter()
        .filter(|c| c.before.0 > 0 && c.reviewed > c.before)
        .max_by_key(|c| (c.reviewed.0 - c.before.0, std::cmp::Reverse(c.category_id)))
        .map(|c| Focus::CategoryCap {
            category_id: c.category_id,
            target: c.before,
        })
}

/// A focus's target with what the queries measured from `focus_start` to `focus_end`
/// (§5.22's grading table). Every kind's `actual` is what `focus_actual` stores; a
/// `loan_on_time` focus stores none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    CategoryCap {
        target: Money,
        actual: Money,
    },
    SpendingCap {
        target: Money,
        actual: Money,
    },
    CardLevel {
        target: Money,
        actual: Money,
    },
    /// `None` when the payment was removed.
    LoanOnTime {
        payment: Option<PaymentDates>,
    },
    /// Days with at least one transaction.
    Logging {
        target: i64,
        actual: i64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaymentDates {
    pub due_date: NaiveDate,
    /// The date of its expense, none while it's unpaid.
    pub paid_on: Option<NaiveDate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusResult {
    Kept,
    Missed,
    /// Neither kept nor missed.
    Void,
}

pub fn grade(ended: &Ended) -> FocusResult {
    let kept = match *ended {
        Ended::CategoryCap { target, actual } | Ended::SpendingCap { target, actual } => {
            actual <= target
        }
        // Reaching a spending level counts as reaching it (§5.18).
        Ended::CardLevel { target, actual } => actual < target,
        Ended::LoanOnTime { payment: None } => return FocusResult::Void,
        Ended::LoanOnTime { payment: Some(p) } => p.paid_on.is_some_and(|paid| paid <= p.due_date),
        Ended::Logging { target, actual } => actual >= target,
    };
    if kept {
        FocusResult::Kept
    } else {
        FocusResult::Missed
    }
}

/// Sets a cap's bar color: amber when near, red when over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapState {
    Ok,
    Near,
    Over,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapProgress {
    /// `actual × 100 ÷ target`, rounded down.
    pub percent: i64,
    pub state: CapState,
}

/// An active cap's progress, with `actual` counted to today. `target` is above 0 (§4.9).
pub fn cap(actual: Money, target: Money) -> CapProgress {
    // In i128: the product can pass i64 for large amounts.
    let (a, t) = (i128::from(actual.0), i128::from(target.0));
    let state = if a > t {
        CapState::Over
    } else if a * 100 >= t * 80 {
        CapState::Near
    } else {
        CapState::Ok
    };
    CapProgress {
        percent: i64::try_from(a * 100 / t).unwrap_or(i64::MAX),
        state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn s(score: i64) -> Option<Stat> {
        Some(Stat {
            score,
            reason: None,
        })
    }

    /// Every stat that applies at 100.
    fn all_100() -> Stats {
        Stats {
            cashflow: s(100),
            bills: s(100),
            card: s(100),
            debt: s(100),
            savings: s(100),
            habit: Stat {
                score: 100,
                reason: None,
            },
        }
    }

    /// Nothing but logging applies.
    fn none() -> OfferFacts {
        OfferFacts {
            for_week: true,
            loan_payment_id: None,
            cards: vec![],
            has_payday_rule: false,
            per_day: Money(0),
            categories: vec![],
        }
    }

    const LEVELS: SpendLevels = SpendLevels {
        soft: Money(300_000),
        mid: Money(400_000),
        hard: Money(500_000),
    };

    fn card(account_id: i64, spent: i64, utilization_percent: i64) -> CardOffer {
        CardOffer {
            account_id,
            levels: LEVELS,
            spent: Money(spent),
            utilization_percent,
        }
    }

    fn cat(category_id: i64, reviewed: i64, before: i64) -> CategorySpending {
        CategorySpending {
            category_id,
            reviewed: Money(reviewed),
            before: Money(before),
        }
    }

    /// Every kind applies.
    fn all() -> OfferFacts {
        OfferFacts {
            for_week: true,
            loan_payment_id: Some(7),
            cards: vec![card(3, 0, 0)],
            has_payday_rule: true,
            per_day: Money(50_000),
            categories: vec![cat(4, 2_000, 1_000)],
        }
    }

    // Offers

    #[test]
    fn logging_alone_is_offered_when_nothing_else_applies() {
        assert_eq!(offers(&none(), &all_100()), [Focus::Logging { target: 5 }]);
        let month = OfferFacts {
            for_week: false,
            ..none()
        };
        assert_eq!(offers(&month, &all_100()), [Focus::Logging { target: 20 }]);
    }

    #[test]
    fn loan_on_time_is_offered_for_a_payment_due_in_the_period() {
        let facts = OfferFacts {
            loan_payment_id: Some(7),
            ..none()
        };
        assert_eq!(
            offers(&facts, &all_100())[0],
            Focus::LoanOnTime { payment_id: 7 }
        );
        assert_eq!(offers(&none(), &all_100()).len(), 1);
    }

    #[test]
    fn card_level_aims_at_the_lowest_level_above_the_spending() {
        let at = |spent| {
            let facts = OfferFacts {
                cards: vec![card(3, spent, 0)],
                ..none()
            };
            offers(&facts, &all_100())[0]
        };
        let target = |t| Focus::CardLevel {
            account_id: 3,
            target: Money(t),
        };
        assert_eq!(at(0), target(300_000));
        assert_eq!(at(299_999), target(300_000));
        // Reaching a level means the next one is the target.
        assert_eq!(at(300_000), target(400_000));
        assert_eq!(at(400_000), target(500_000));
        assert_eq!(at(499_999), target(500_000));
    }

    #[test]
    fn card_level_is_not_offered_for_a_card_at_hard() {
        let facts = OfferFacts {
            cards: vec![card(3, 500_000, 0)],
            ..none()
        };
        assert_eq!(offers(&facts, &all_100()), [Focus::Logging { target: 5 }]);
    }

    #[test]
    fn card_level_picks_the_card_with_the_lowest_score_below_hard_then_the_lowest_id() {
        let facts = OfferFacts {
            cards: vec![
                card(1, 500_000, 95), // hard: left out, though its score is lowest
                card(5, 300_000, 50), // soft: 70 × .6 + 50 × .4 = 62
                card(2, 300_000, 50), // the same score, a lower id
                card(4, 0, 0),        // 100
            ],
            ..none()
        };
        assert_eq!(
            offers(&facts, &all_100())[0],
            Focus::CardLevel {
                account_id: 2,
                target: Money(400_000)
            }
        );
    }

    #[test]
    fn spending_cap_needs_a_week_a_payday_rule_and_per_day_above_0() {
        let facts = OfferFacts {
            has_payday_rule: true,
            per_day: Money(50_000),
            ..none()
        };
        assert_eq!(
            offers(&facts, &all_100())[0],
            Focus::SpendingCap {
                target: Money(350_000)
            }
        );
        for not in [
            OfferFacts {
                for_week: false,
                ..facts.clone()
            },
            OfferFacts {
                has_payday_rule: false,
                ..facts.clone()
            },
            OfferFacts {
                per_day: Money(0),
                ..facts.clone()
            },
        ] {
            assert!(
                !offers(&not, &all_100())
                    .iter()
                    .any(|f| matches!(f, Focus::SpendingCap { .. }))
            );
        }
    }

    #[test]
    fn category_cap_takes_the_largest_rise_over_an_earlier_amount_above_0() {
        let facts = OfferFacts {
            categories: vec![
                cat(1, 9_000, 0),     // nothing before: left out
                cat(2, 1_000, 2_000), // fell
                cat(3, 3_000, 1_000), // rose 2,000
                cat(4, 2_500, 500),   // rose 2,000 too, a higher id
                cat(5, 1_500, 1_000), // rose 500
            ],
            ..none()
        };
        assert_eq!(
            offers(&facts, &all_100())[0],
            Focus::CategoryCap {
                category_id: 3,
                target: Money(1_000)
            }
        );
    }

    #[test]
    fn category_cap_is_not_offered_without_a_rise() {
        let facts = OfferFacts {
            categories: vec![cat(1, 9_000, 0), cat(2, 1_000, 1_000), cat(3, 500, 1_000)],
            ..none()
        };
        assert_eq!(offers(&facts, &all_100()), [Focus::Logging { target: 5 }]);
    }

    fn kinds(list: &[Focus]) -> Vec<&'static str> {
        list.iter()
            .map(|f| match f {
                Focus::LoanOnTime { .. } => "loan",
                Focus::CardLevel { .. } => "card",
                Focus::SpendingCap { .. } => "spending",
                Focus::CategoryCap { .. } => "category",
                Focus::Logging { .. } => "logging",
            })
            .collect()
    }

    #[test]
    fn ties_keep_the_table_order_and_only_3_are_offered() {
        assert_eq!(
            kinds(&offers(&all(), &all_100())),
            ["loan", "card", "spending"]
        );
    }

    #[test]
    fn offers_go_lowest_stat_first() {
        let stats = Stats {
            habit: Stat {
                score: 10,
                reason: None,
            },
            cashflow: s(40),
            card: s(60),
            bills: s(80),
            ..all_100()
        };
        assert_eq!(
            kinds(&offers(&all(), &stats)),
            ["logging", "spending", "category"]
        );
    }

    #[test]
    fn a_stat_that_is_none_ranks_as_100() {
        let stats = Stats {
            bills: None,
            card: None,
            cashflow: s(99),
            ..all_100()
        };
        // Spending and category (99) first, then loan (none = 100) before card and logging.
        assert_eq!(
            kinds(&offers(&all(), &stats)),
            ["spending", "category", "loan"]
        );
    }

    // Grading

    #[test]
    fn caps_are_kept_up_to_and_at_the_target() {
        let t = Money(150_000);
        for (actual, result) in [(150_000, FocusResult::Kept), (150_001, FocusResult::Missed)] {
            let actual = Money(actual);
            assert_eq!(grade(&Ended::CategoryCap { target: t, actual }), result);
            assert_eq!(grade(&Ended::SpendingCap { target: t, actual }), result);
        }
    }

    #[test]
    fn a_card_level_is_missed_on_reaching_it() {
        let t = Money(300_000);
        let at = |a| {
            grade(&Ended::CardLevel {
                target: t,
                actual: Money(a),
            })
        };
        assert_eq!(at(299_999), FocusResult::Kept);
        assert_eq!(at(300_000), FocusResult::Missed);
    }

    #[test]
    fn a_loan_payment_is_kept_when_paid_on_or_before_its_due_date() {
        let at = |paid_on| {
            grade(&Ended::LoanOnTime {
                payment: Some(PaymentDates {
                    due_date: d(2026, 11, 15),
                    paid_on,
                }),
            })
        };
        assert_eq!(at(Some(d(2026, 11, 15))), FocusResult::Kept);
        assert_eq!(at(Some(d(2026, 11, 16))), FocusResult::Missed);
        assert_eq!(at(None), FocusResult::Missed);
    }

    #[test]
    fn a_loan_focus_whose_payment_was_removed_is_void() {
        assert_eq!(
            grade(&Ended::LoanOnTime { payment: None }),
            FocusResult::Void
        );
    }

    #[test]
    fn logging_is_kept_on_reaching_its_days() {
        let at = |actual| grade(&Ended::Logging { target: 5, actual });
        assert_eq!(at(5), FocusResult::Kept);
        assert_eq!(at(4), FocusResult::Missed);
    }

    // State

    #[test]
    fn a_cap_is_near_from_80_percent_and_over_past_the_target() {
        let at = |a| cap(Money(a), Money(100_000));
        let p = |percent, state| CapProgress { percent, state };
        assert_eq!(at(0), p(0, CapState::Ok));
        assert_eq!(at(79_999), p(79, CapState::Ok));
        assert_eq!(at(80_000), p(80, CapState::Near));
        assert_eq!(at(100_000), p(100, CapState::Near));
        assert_eq!(at(100_001), p(100, CapState::Over));
        assert_eq!(at(250_000), p(250, CapState::Over));
    }

    #[test]
    fn near_is_exact_where_80_percent_isnt_a_whole_amount() {
        // 80% of 333 is 266.4: 266 is short of it, 267 reaches it.
        assert_eq!(cap(Money(266), Money(333)).state, CapState::Ok);
        assert_eq!(cap(Money(267), Money(333)).state, CapState::Near);
    }

    #[test]
    fn large_amounts_dont_overflow() {
        let max = cap(Money(i64::MAX), Money(1));
        assert_eq!((max.percent, max.state), (i64::MAX, CapState::Over));
        assert_eq!(cap(Money(i64::MAX), Money(i64::MAX)).state, CapState::Near);
        let facts = OfferFacts {
            has_payday_rule: true,
            per_day: Money(i64::MAX),
            ..none()
        };
        assert_eq!(
            offers(&facts, &all_100())[0],
            Focus::SpendingCap {
                target: Money(i64::MAX)
            }
        );
    }
}
