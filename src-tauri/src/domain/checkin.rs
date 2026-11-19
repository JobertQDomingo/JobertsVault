//! Which check-in periods are due (CHK-1, CHK-5, DATA_MODEL §5.19), and the win, personal
//! bests, streaks and history (CHK-6, LVL-7, LVL-8, §5.23).

use chrono::{Datelike, Days, NaiveDate};

use super::focus::{FocusResult, PaymentDates};
use super::money::Money;
use super::period::{Period, PeriodKind, PeriodSettings};
use super::stats::Stats;

/// The periods a check-in finished today would settle. Each is `None` when it isn't due.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DuePeriods {
    pub week: Option<Period>,
    pub month: Option<Period>,
    /// Only with the week or the month, and only in January.
    pub year: Option<Period>,
}

impl DuePeriods {
    /// The period the check-in shows: the month when it's due, else the week. `None` means no
    /// check-in is due.
    pub fn shown(&self) -> Option<Period> {
        self.month.or(self.week)
    }
}

/// Only the newest completed week, month and (in January) year can be due, each when it has no
/// row in `done` (kind and period start) and some transaction is dated on or before its end.
/// Older periods never reviewed are skipped.
pub fn due_periods(
    today: NaiveDate,
    settings: &PeriodSettings,
    done: &[(PeriodKind, NaiveDate)],
    earliest_transaction: Option<NaiveDate>,
) -> DuePeriods {
    let due = |kind| {
        Period::containing(kind, today, settings)
            .and_then(|current| current.previous(settings))
            .filter(|p| !done.contains(&(kind, p.start())))
            .filter(|p| earliest_transaction.is_some_and(|first| first <= p.end()))
    };
    let week = due(PeriodKind::Week);
    let month = due(PeriodKind::Month);
    // The year is never due by itself: it rides on a January check-in.
    let year = (today.month() == 1 && (week.is_some() || month.is_some()))
        .then(|| due(PeriodKind::Year))
        .flatten();
    DuePeriods { week, month, year }
}

/// The six stats, in §5.20's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatName {
    Cashflow,
    Bills,
    Card,
    Debt,
    Savings,
    Habit,
}

const STAT_NAMES: [StatName; 6] = [
    StatName::Cashflow,
    StatName::Bills,
    StatName::Card,
    StatName::Debt,
    StatName::Savings,
    StatName::Habit,
];

/// The six scores in §5.20's order, as a check-in row's `stat_*` columns hold them. `None` is a
/// stat that didn't apply.
pub type Scores = [Option<i64>; 6];

pub fn scores(stats: &Stats) -> Scores {
    let score = |s: Option<super::stats::Stat>| s.map(|s| s.score);
    [
        score(stats.cashflow),
        score(stats.bills),
        score(stats.card),
        score(stats.debt),
        score(stats.savings),
        Some(stats.habit.score),
    ]
}

/// What §5.23's win table needs, for the period the check-in shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinFacts {
    /// The focus the Start step shows was graded `kept`.
    pub focus_kept: bool,
    /// Today's scores.
    pub scores: Scores,
    /// The snapshot on the latest earlier check-in row, none when there's no such row.
    pub previous: Option<Scores>,
    /// Every expense category with an amount (§5.10) in the period or the one before.
    pub categories: Vec<CategoryTotals>,
    /// The period's expense, and the period before's (§5.2).
    pub expense: Money,
    pub expense_before: Money,
    /// The loan payments due in the period, paid-before ones left out (§4.6).
    pub loan_payments: Vec<PaymentDates>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CategoryTotals {
    pub category_id: i64,
    pub amount: Money,
    pub before: Money,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Win {
    FocusKept,
    StatRose { stat: StatName, from: i64, to: i64 },
    CategoryFell { category_id: i64, by: Money },
    SpendingFell,
    LoansPaidOnTime,
}

/// The first win of §5.23's table that applies. With none, there's no win: finishing the
/// check-in never stands in for one (§9 item 126).
pub fn win(facts: &WinFacts) -> Option<Win> {
    if facts.focus_kept {
        return Some(Win::FocusKept);
    }
    stat_rose(facts)
        .or_else(|| category_fell(&facts.categories))
        .or_else(|| {
            (facts.expense_before.0 > 0 && facts.expense < facts.expense_before)
                .then_some(Win::SpendingFell)
        })
        .or_else(|| {
            let on_time = |p: &PaymentDates| p.paid_on.is_some_and(|paid| paid <= p.due_date);
            (!facts.loan_payments.is_empty() && facts.loan_payments.iter().all(on_time))
                .then_some(Win::LoansPaidOnTime)
        })
}

/// The largest rise of 5 or more; equal rises go to the stat first in §5.20's order.
fn stat_rose(facts: &WinFacts) -> Option<Win> {
    let previous = facts.previous?;
    let mut best: Option<Win> = None;
    for (i, stat) in STAT_NAMES.into_iter().enumerate() {
        let (Some(from), Some(to)) = (previous[i], facts.scores[i]) else {
            continue;
        };
        let beats = match best {
            Some(Win::StatRose { from: f, to: t, .. }) => to - from > t - f,
            _ => true,
        };
        if to - from >= 5 && beats {
            best = Some(Win::StatRose { stat, from, to });
        }
    }
    best
}

/// The largest fall in amount; equal falls go to the lower category id.
fn category_fell(categories: &[CategoryTotals]) -> Option<Win> {
    categories
        .iter()
        .filter(|c| c.amount < c.before)
        .max_by_key(|c| (c.before.0 - c.amount.0, std::cmp::Reverse(c.category_id)))
        .map(|c| Win::CategoryFell {
            category_id: c.category_id,
            by: Money(c.before.0 - c.amount.0),
        })
}

/// The periods before `shown` that a personal best compares it with: up to 11, of its kind,
/// latest first, stopping at one that ended before the earliest transaction's date.
pub fn best_periods(
    shown: Period,
    settings: &PeriodSettings,
    earliest_transaction: NaiveDate,
) -> Vec<Period> {
    std::iter::successors(shown.previous(settings), |p| p.previous(settings))
        .take_while(|p| p.end() >= earliest_transaction)
        .take(11)
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeriodTotals {
    pub expense: Money,
    /// Income − expense (§5.2).
    pub net: Money,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonalBest {
    LowestExpense,
    HighestNet,
}

/// `earlier` holds the totals of `best_periods`. With 4 or more periods in all, the shown one
/// sets the lowest expense, or else the highest net, with no tie.
pub fn personal_best(shown: PeriodTotals, earlier: &[PeriodTotals]) -> Option<PersonalBest> {
    if earlier.len() < 3 {
        return None;
    }
    if earlier.iter().all(|p| shown.expense < p.expense) {
        Some(PersonalBest::LowestExpense)
    } else if earlier.iter().all(|p| shown.net > p.net) {
        Some(PersonalBest::HighestNet)
    } else {
        None
    }
}

/// A week's check-in is on time when `completed_on ≤ period_end + 3`: the one rule for the
/// check-in streak and Habit's `on_time_weeks` (§5.20, §5.23).
pub fn on_time(period_end: NaiveDate, completed_on: NaiveDate) -> bool {
    period_end
        .checked_add_days(Days::new(3))
        .is_some_and(|last| completed_on <= last)
}

/// `week_rows` holds `(period_start, completed_on)` of every week row. Counting back from the
/// latest completed week, the weeks in a row finished on time. The latest week with no row
/// yet is passed over until it's too late to be on time.
pub fn checkin_streak(
    today: NaiveDate,
    settings: &PeriodSettings,
    week_rows: &[(NaiveDate, NaiveDate)],
) -> i64 {
    let completed = |w: &Period| {
        week_rows
            .iter()
            .find(|(start, _)| *start == w.start())
            .map(|(_, on)| *on)
    };
    let mut week = Period::containing(PeriodKind::Week, today, settings)
        .and_then(|current| current.previous(settings));
    if let Some(latest) = week
        && completed(&latest).is_none()
        && on_time(latest.end(), today)
    {
        week = latest.previous(settings);
    }
    let mut streak = 0;
    while let Some(w) = week {
        if !completed(&w).is_some_and(|on| on_time(w.end(), on)) {
            break;
        }
        streak += 1;
        week = w.previous(settings);
    }
    streak
}

/// The graded focuses, latest `focus_end` first: those kept in a row, passing over void ones.
pub fn focus_streak(latest_first: impl IntoIterator<Item = FocusResult>) -> i64 {
    latest_first
        .into_iter()
        .filter(|r| *r != FocusResult::Void)
        .take_while(|r| *r == FocusResult::Kept)
        .count() as i64
}

/// The focus record (LVL-8): the 8 latest graded focuses, void ones left out. `latest_first`
/// is sorted by `focus_end`.
pub fn focus_record<T>(
    latest_first: impl IntoIterator<Item = T>,
    result: impl Fn(&T) -> FocusResult,
) -> Vec<T> {
    latest_first
        .into_iter()
        .filter(|f| result(f) != FocusResult::Void)
        .take(8)
        .collect()
}

/// The trend lines' points (LVL-8): one per `completed_on`, the 12 latest, oldest first. A
/// merged check-in's rows hold the same snapshot, so one of them stands for all. The rows may
/// come in any order.
pub fn history<T>(
    rows: impl IntoIterator<Item = T>,
    completed_on: impl Fn(&T) -> NaiveDate,
) -> Vec<T> {
    let mut points: Vec<T> = rows.into_iter().collect();
    points.sort_by_key(|p| std::cmp::Reverse(completed_on(p)));
    points.dedup_by(|a, b| completed_on(a) == completed_on(b));
    points.truncate(12);
    points.reverse();
    points
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn settings(week_start: Weekday, month_start_day: u32) -> PeriodSettings {
        PeriodSettings {
            week_start,
            month_start_day,
        }
    }

    const MONDAY_1ST: PeriodSettings = PeriodSettings {
        week_start: Weekday::Mon,
        month_start_day: 1,
    };

    fn span(p: Option<Period>) -> Option<(NaiveDate, NaiveDate)> {
        p.map(|p| (p.start(), p.end()))
    }

    const LONG_AGO: Option<NaiveDate> = NaiveDate::from_ymd_opt(2025, 1, 1);

    #[test]
    fn the_week_that_ended_most_recently_is_due_with_a_monday_start() {
        // Thu Nov 19: the last full week is Mon Nov 9 to Sun Nov 15.
        let due = due_periods(d(2026, 11, 19), &MONDAY_1ST, &[], LONG_AGO);
        assert_eq!(span(due.week), Some((d(2026, 11, 9), d(2026, 11, 15))));
        assert_eq!(span(due.month), Some((d(2026, 10, 1), d(2026, 10, 31))));
        assert_eq!(due.year, None);
    }

    #[test]
    fn on_the_week_start_the_week_that_ended_yesterday_is_due() {
        let due = due_periods(d(2026, 11, 16), &MONDAY_1ST, &[], LONG_AGO);
        assert_eq!(span(due.week), Some((d(2026, 11, 9), d(2026, 11, 15))));
    }

    #[test]
    fn a_sunday_week_start_moves_the_week() {
        let due = due_periods(d(2026, 11, 19), &settings(Weekday::Sun, 1), &[], LONG_AGO);
        assert_eq!(span(due.week), Some((d(2026, 11, 8), d(2026, 11, 14))));
    }

    #[test]
    fn a_month_starting_on_the_25th_ends_on_the_24th() {
        let s = settings(Weekday::Mon, 25);
        // Nov 19 is in Oct 25 to Nov 24, so Sep 25 to Oct 24 is the latest completed month.
        let due = due_periods(d(2026, 11, 19), &s, &[], LONG_AGO);
        assert_eq!(span(due.month), Some((d(2026, 9, 25), d(2026, 10, 24))));
        // On the 25th, the month that ended yesterday is due.
        let due = due_periods(d(2026, 11, 25), &s, &[], LONG_AGO);
        assert_eq!(span(due.month), Some((d(2026, 10, 25), d(2026, 11, 24))));
    }

    #[test]
    fn a_week_and_a_month_due_together_show_the_month() {
        // Mon Nov 2: the week Oct 26 to Nov 1 and October both just ended.
        let due = due_periods(d(2026, 11, 2), &MONDAY_1ST, &[], LONG_AGO);
        assert_eq!(span(due.week), Some((d(2026, 10, 26), d(2026, 11, 1))));
        assert_eq!(span(due.month), Some((d(2026, 10, 1), d(2026, 10, 31))));
        assert_eq!(span(due.shown()), span(due.month));
    }

    #[test]
    fn only_the_week_due_shows_the_week() {
        let done = [(PeriodKind::Month, d(2026, 10, 1))];
        let due = due_periods(d(2026, 11, 19), &MONDAY_1ST, &done, LONG_AGO);
        assert_eq!(due.month, None);
        assert_eq!(span(due.shown()), Some((d(2026, 11, 9), d(2026, 11, 15))));
    }

    #[test]
    fn the_year_is_due_only_in_january() {
        let due = due_periods(d(2027, 1, 4), &MONDAY_1ST, &[], LONG_AGO);
        assert_eq!(span(due.year), Some((d(2026, 1, 1), d(2026, 12, 31))));
        assert_eq!(span(due.month), Some((d(2026, 12, 1), d(2026, 12, 31))));
        // Jan 31 still counts; Feb 1 doesn't.
        assert!(
            due_periods(d(2027, 1, 31), &MONDAY_1ST, &[], LONG_AGO)
                .year
                .is_some()
        );
        assert_eq!(
            due_periods(d(2027, 2, 1), &MONDAY_1ST, &[], LONG_AGO).year,
            None
        );
        assert_eq!(
            due_periods(d(2026, 12, 31), &MONDAY_1ST, &[], LONG_AGO).year,
            None
        );
    }

    #[test]
    fn the_year_is_never_due_by_itself() {
        // Wed Jan 13: the week of Jan 4 and December are done, the year isn't.
        let done = [
            (PeriodKind::Week, d(2027, 1, 4)),
            (PeriodKind::Month, d(2026, 12, 1)),
        ];
        let due = due_periods(d(2027, 1, 13), &MONDAY_1ST, &done, LONG_AGO);
        assert_eq!(
            due,
            DuePeriods {
                week: None,
                month: None,
                year: None
            }
        );
        assert_eq!(due.shown(), None);
    }

    #[test]
    fn the_year_is_calendar_even_with_a_shifted_month() {
        // Jan 10 with months from the 25th: Nov 25 to Dec 24 is the latest month, 2026 the year.
        let due = due_periods(d(2027, 1, 10), &settings(Weekday::Mon, 25), &[], LONG_AGO);
        assert_eq!(span(due.month), Some((d(2026, 11, 25), d(2026, 12, 24))));
        assert_eq!(span(due.year), Some((d(2026, 1, 1), d(2026, 12, 31))));
    }

    #[test]
    fn a_year_already_checked_in_is_not_due_again() {
        let done = [(PeriodKind::Year, d(2026, 1, 1))];
        let due = due_periods(d(2027, 1, 13), &MONDAY_1ST, &done, LONG_AGO);
        assert!(due.week.is_some());
        assert_eq!(due.year, None);
    }

    #[test]
    fn three_weeks_unopened_leave_only_the_newest_due() {
        // The week of Oct 19 was the last one checked in; Oct 26, Nov 2 and Nov 9 never were.
        let done = [(PeriodKind::Week, d(2026, 10, 19))];
        let due = due_periods(d(2026, 11, 19), &MONDAY_1ST, &done, LONG_AGO);
        assert_eq!(span(due.week), Some((d(2026, 11, 9), d(2026, 11, 15))));
    }

    #[test]
    fn a_period_that_already_has_a_row_is_not_due() {
        let done = [(PeriodKind::Week, d(2026, 11, 9))];
        let due = due_periods(d(2026, 11, 19), &MONDAY_1ST, &done, LONG_AGO);
        assert_eq!(due.week, None);
        assert!(due.month.is_some(), "each kind has its own row");
    }

    #[test]
    fn nothing_is_due_without_a_transaction() {
        let due = due_periods(d(2027, 1, 4), &MONDAY_1ST, &[], None);
        assert_eq!(
            due,
            DuePeriods {
                week: None,
                month: None,
                year: None
            }
        );
    }

    #[test]
    fn a_period_that_ended_before_the_first_transaction_is_not_due() {
        // The first transaction is Wed Nov 11: the week of Nov 9 is due, October isn't.
        let due = due_periods(d(2026, 11, 19), &MONDAY_1ST, &[], Some(d(2026, 11, 11)));
        assert!(due.week.is_some());
        assert_eq!(due.month, None);
        // Dated on the period's last day still counts.
        let due = due_periods(d(2026, 11, 19), &MONDAY_1ST, &[], Some(d(2026, 11, 15)));
        assert!(due.week.is_some());
        // Only a transaction this week: nothing is due yet.
        let due = due_periods(d(2026, 11, 19), &MONDAY_1ST, &[], Some(d(2026, 11, 16)));
        assert_eq!(due.shown(), None);
    }

    // §5.23: the win.

    fn none_apply() -> WinFacts {
        WinFacts {
            focus_kept: false,
            scores: [Some(50); 6],
            previous: Some([Some(50); 6]),
            categories: vec![CategoryTotals {
                category_id: 1,
                amount: Money(500),
                before: Money(400),
            }],
            expense: Money(1_000),
            expense_before: Money(900),
            loan_payments: vec![],
        }
    }

    fn all_apply() -> WinFacts {
        WinFacts {
            focus_kept: true,
            scores: [Some(60), Some(50), Some(50), Some(50), Some(50), Some(50)],
            categories: vec![CategoryTotals {
                category_id: 1,
                amount: Money(300),
                before: Money(400),
            }],
            expense: Money(800),
            loan_payments: vec![PaymentDates {
                due_date: d(2026, 11, 10),
                paid_on: Some(d(2026, 11, 10)),
            }],
            ..none_apply()
        }
    }

    #[test]
    fn with_none_that_applies_there_is_no_win() {
        assert_eq!(win(&none_apply()), None);
    }

    #[test]
    fn the_wins_go_in_the_tables_order() {
        let mut facts = all_apply();
        assert_eq!(win(&facts), Some(Win::FocusKept));
        facts.focus_kept = false;
        assert_eq!(
            win(&facts),
            Some(Win::StatRose {
                stat: StatName::Cashflow,
                from: 50,
                to: 60
            })
        );
        facts.scores = [Some(50); 6];
        assert_eq!(
            win(&facts),
            Some(Win::CategoryFell {
                category_id: 1,
                by: Money(100)
            })
        );
        facts.categories.clear();
        assert_eq!(win(&facts), Some(Win::SpendingFell));
        facts.expense = Money(900);
        assert_eq!(win(&facts), Some(Win::LoansPaidOnTime));
    }

    #[test]
    fn a_stat_must_rise_by_5_and_the_largest_rise_wins() {
        let mut facts = none_apply();
        facts.scores = [Some(54), Some(50), Some(50), Some(50), Some(50), Some(50)];
        assert_eq!(win(&facts), None, "4 isn't enough");
        // Card rose 7, Savings 9, Habit 9: Savings is first of the largest in §5.20's order.
        facts.scores = [Some(55), Some(50), Some(57), Some(50), Some(59), Some(59)];
        assert_eq!(
            win(&facts),
            Some(Win::StatRose {
                stat: StatName::Savings,
                from: 50,
                to: 59
            })
        );
    }

    #[test]
    fn a_stat_that_is_none_now_or_then_never_rose() {
        let mut facts = none_apply();
        facts.scores = [Some(90), None, Some(50), Some(50), Some(50), Some(50)];
        facts.previous = Some([None, Some(10), Some(50), Some(50), Some(50), Some(50)]);
        assert_eq!(win(&facts), None);
        // No earlier row: nothing to rise from.
        facts.previous = None;
        assert_eq!(win(&facts), None);
    }

    #[test]
    fn the_largest_category_fall_wins_with_ties_to_the_lower_id() {
        let mut facts = none_apply();
        let cat = |category_id, amount, before| CategoryTotals {
            category_id,
            amount: Money(amount),
            before: Money(before),
        };
        facts.categories = vec![cat(3, 100, 300), cat(2, 0, 200), cat(5, 50, 400)];
        assert_eq!(
            win(&facts),
            Some(Win::CategoryFell {
                category_id: 5,
                by: Money(350)
            })
        );
        facts.categories = vec![cat(3, 100, 300), cat(2, 0, 200)];
        assert_eq!(
            win(&facts),
            Some(Win::CategoryFell {
                category_id: 2,
                by: Money(200)
            })
        );
    }

    #[test]
    fn spending_fell_needs_spending_before() {
        let mut facts = none_apply();
        facts.categories.clear();
        facts.expense = Money(899);
        assert_eq!(win(&facts), Some(Win::SpendingFell));
        facts.expense = Money(900);
        assert_eq!(win(&facts), None, "equal didn't fall");
        facts.expense = Money(0);
        facts.expense_before = Money(0);
        assert_eq!(win(&facts), None);
    }

    #[test]
    fn loans_paid_on_time_needs_every_payment_paid_by_its_due_date() {
        let pay = |due: u32, paid: Option<u32>| PaymentDates {
            due_date: d(2026, 11, due),
            paid_on: paid.map(|p| d(2026, 11, p)),
        };
        let mut facts = none_apply();
        facts.loan_payments = vec![pay(10, Some(8)), pay(14, Some(14))];
        assert_eq!(win(&facts), Some(Win::LoansPaidOnTime));
        facts.loan_payments = vec![pay(10, Some(8)), pay(14, Some(15))];
        assert_eq!(win(&facts), None, "one paid late");
        facts.loan_payments = vec![pay(10, Some(8)), pay(14, None)];
        assert_eq!(win(&facts), None, "one unpaid");
    }

    // §5.23: personal bests.

    fn totals(expense: i64, net: i64) -> PeriodTotals {
        PeriodTotals {
            expense: Money(expense),
            net: Money(net),
        }
    }

    #[test]
    fn a_personal_best_needs_4_periods() {
        let earlier = [totals(500, 0), totals(600, 0)];
        assert_eq!(personal_best(totals(100, 900), &earlier), None);
        let earlier = [totals(500, 0), totals(600, 0), totals(700, 0)];
        assert_eq!(
            personal_best(totals(100, 900), &earlier),
            Some(PersonalBest::LowestExpense),
            "the lowest expense is named first"
        );
    }

    #[test]
    fn a_tie_sets_no_best() {
        let earlier = [totals(500, 300), totals(600, 0), totals(700, 0)];
        // Tied on expense, so it can still be the highest net.
        assert_eq!(
            personal_best(totals(500, 400), &earlier),
            Some(PersonalBest::HighestNet)
        );
        assert_eq!(personal_best(totals(500, 300), &earlier), None);
    }

    #[test]
    fn best_periods_are_the_11_before_back_to_the_first_transaction() {
        let s = MONDAY_1ST;
        let october = Period::containing(PeriodKind::Month, d(2026, 10, 1), &s).unwrap();
        let all = best_periods(october, &s, d(2020, 1, 1));
        assert_eq!(all.len(), 11);
        assert_eq!(all[0].start(), d(2026, 9, 1));
        assert_eq!(all[10].start(), d(2025, 11, 1));
        // Ending on or after the first transaction's date counts; ending before it doesn't.
        let some = best_periods(october, &s, d(2026, 7, 31));
        assert_eq!(some.len(), 3);
        assert_eq!(some[2].start(), d(2026, 7, 1));
    }

    #[test]
    fn best_periods_follow_a_month_starting_on_the_25th() {
        let s = settings(Weekday::Mon, 25);
        let shown = Period::containing(PeriodKind::Month, d(2026, 10, 1), &s).unwrap();
        // Shown: Sep 25 to Oct 24. A first transaction on Jul 24 keeps Jun 25 to Jul 24.
        let earlier = best_periods(shown, &s, d(2026, 7, 24));
        let spans: Vec<_> = earlier.iter().map(|p| (p.start(), p.end())).collect();
        assert_eq!(
            spans,
            [
                (d(2026, 8, 25), d(2026, 9, 24)),
                (d(2026, 7, 25), d(2026, 8, 24)),
                (d(2026, 6, 25), d(2026, 7, 24)),
            ]
        );
    }

    // §5.23: streaks.

    /// The 2026 week starting on `start_day` (month, day), finished `done_after_end` days after
    /// its last day.
    fn week_row(start_day: (u32, u32), done_after_end: u64) -> (NaiveDate, NaiveDate) {
        let start = d(2026, start_day.0, start_day.1);
        (start, start + Days::new(6 + done_after_end))
    }

    #[test]
    fn the_checkin_streak_counts_on_time_weeks_in_a_row() {
        let rows = [
            week_row((11, 9), 0),
            week_row((11, 2), 3),
            week_row((10, 26), 1),
            week_row((10, 12), 0),
        ];
        // Oct 19 has no row, so the streak stops at 3.
        assert_eq!(checkin_streak(d(2026, 11, 19), &MONDAY_1ST, &rows), 3);
    }

    #[test]
    fn a_late_week_breaks_the_checkin_streak() {
        let rows = [
            week_row((11, 9), 0),
            week_row((11, 2), 4),
            week_row((10, 26), 0),
        ];
        assert_eq!(checkin_streak(d(2026, 11, 19), &MONDAY_1ST, &rows), 1);
    }

    #[test]
    fn the_latest_week_is_passed_over_while_it_can_still_be_on_time() {
        let rows = [week_row((11, 2), 0), week_row((10, 26), 0)];
        // Mon Nov 16 to Wed Nov 18 (Nov 15 + 3): the week of Nov 9 can still be on time.
        assert_eq!(checkin_streak(d(2026, 11, 16), &MONDAY_1ST, &rows), 2);
        assert_eq!(checkin_streak(d(2026, 11, 18), &MONDAY_1ST, &rows), 2);
        // Thu Nov 19: too late, and it's missed.
        assert_eq!(checkin_streak(d(2026, 11, 19), &MONDAY_1ST, &rows), 0);
    }

    #[test]
    fn no_rows_is_no_checkin_streak() {
        assert_eq!(checkin_streak(d(2026, 11, 16), &MONDAY_1ST, &[]), 0);
    }

    #[test]
    fn a_sunday_week_start_moves_the_grace() {
        let s = settings(Weekday::Sun, 1);
        // Weeks Sun to Sat: Nov 8 to 14 is the latest on Tue Nov 17, and on time until Nov 17.
        let rows = [(d(2026, 11, 1), d(2026, 11, 7))];
        assert_eq!(checkin_streak(d(2026, 11, 17), &s, &rows), 1);
        assert_eq!(checkin_streak(d(2026, 11, 18), &s, &rows), 0);
        // A Monday-start row never lines up with a Sunday-start week.
        let monday_rows = [(d(2026, 11, 2), d(2026, 11, 8))];
        assert_eq!(checkin_streak(d(2026, 11, 17), &s, &monday_rows), 0);
    }

    #[test]
    fn on_time_is_by_the_third_day_after_the_end() {
        assert!(on_time(d(2026, 11, 15), d(2026, 11, 18)));
        assert!(!on_time(d(2026, 11, 15), d(2026, 11, 19)));
        assert!(
            !on_time(NaiveDate::MAX, NaiveDate::MAX),
            "no panic past chrono's range"
        );
    }

    #[test]
    fn the_focus_streak_passes_over_void_and_stops_at_missed() {
        use FocusResult::{Kept, Missed, Void};
        assert_eq!(focus_streak([Kept, Void, Kept, Void, Missed, Kept]), 2);
        assert_eq!(focus_streak([Void, Kept, Kept, Kept]), 3);
        assert_eq!(focus_streak([Missed, Kept]), 0);
        assert_eq!(focus_streak([]), 0);
    }

    // §5.23: history.

    #[test]
    fn the_focus_record_is_the_8_latest_without_void() {
        use FocusResult::{Kept, Missed, Void};
        let graded = [
            (1, Kept),
            (2, Void),
            (3, Missed),
            (4, Kept),
            (5, Kept),
            (6, Void),
            (7, Kept),
            (8, Missed),
            (9, Kept),
            (10, Kept),
            (11, Kept),
        ];
        let ids: Vec<i32> = focus_record(graded, |(_, r)| *r)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(ids, [1, 3, 4, 5, 7, 8, 9, 10]);
    }

    #[test]
    fn history_has_one_point_per_completed_on() {
        // A merged week and month finished the same day, apart in the order given.
        let rows = [
            (PeriodKind::Month, d(2026, 11, 2)),
            (PeriodKind::Week, d(2026, 10, 26)),
            (PeriodKind::Week, d(2026, 11, 2)),
        ];
        let points: Vec<NaiveDate> = history(rows, |(_, on)| *on)
            .into_iter()
            .map(|(_, on)| on)
            .collect();
        assert_eq!(points, [d(2026, 10, 26), d(2026, 11, 2)], "oldest first");
    }

    #[test]
    fn history_keeps_the_12_latest() {
        let rows: Vec<NaiveDate> = (0..15)
            .map(|i| d(2026, 11, 30) - Days::new(i * 7))
            .collect();
        let points = history(rows.clone(), |on| *on);
        assert_eq!(points.len(), 12);
        assert_eq!(points[0], rows[11]);
        assert_eq!(points[11], rows[0]);
    }

    #[test]
    fn scores_follow_the_stat_order() {
        use crate::domain::stats::Stat;
        let stat = |score| {
            Some(Stat {
                score,
                reason: None,
            })
        };
        let stats = Stats {
            cashflow: stat(1),
            bills: None,
            card: stat(3),
            debt: stat(4),
            savings: stat(5),
            habit: Stat {
                score: 6,
                reason: None,
            },
        };
        assert_eq!(
            scores(&stats),
            [Some(1), None, Some(3), Some(4), Some(5), Some(6)]
        );
    }
}
