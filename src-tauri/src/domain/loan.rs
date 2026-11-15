use std::collections::HashMap;

use chrono::{Days, NaiveDate};
use serde::Serialize;
use ts_rs::TS;

use crate::domain::budget::used_percent;
use crate::domain::money::Money;
use crate::domain::period::{Period, PeriodKind, PeriodSettings};

/// One loan payment, as the figures read it (DATA_MODEL §4.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Payment {
    pub due_date: NaiveDate,
    /// The scheduled amount.
    pub amount: Money,
    /// Its paid amount: its transaction's when paid (a late fee included), its scheduled amount
    /// when paid before, and none when unpaid.
    pub paid: Option<Money>,
}

/// One loan's figures (LOAN-5, LOAN-6, DATA_MODEL §5.11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoanFigures {
    pub left: Money,
    pub paid: Money,
    /// Whole percent rounded down, so 100 means it's really paid off.
    pub percent_paid: i64,
    /// The index of its earliest unpaid payment.
    pub next: Option<usize>,
    pub interest: Option<Money>,
    pub paid_off: bool,
}

/// One loan, archived or not, as the summary reads it.
#[derive(Clone, Copy, Debug)]
pub struct LoanInput<'a> {
    pub payments: &'a [Payment],
    pub amount_borrowed: Option<Money>,
    pub borrowed_on: Option<NaiveDate>,
}

/// Across every loan, archived ones included (LOAN-5 to LOAN-8, DATA_MODEL §5.11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoanSummary {
    pub total_left: Money,
    pub total_paid: Money,
    /// Whole percent of the totals rounded down, for the top panel's bar.
    #[ts(type = "number")]
    pub percent_paid: i64,
    /// The latest unpaid due date, or today when that has passed. None once nothing is left.
    pub debt_free_on: Option<NaiveDate>,
    #[ts(type = "number | null")]
    pub days_to_go: Option<i64>,
    /// None when no loan has an interest cost.
    pub interest: Option<Money>,
    /// Days since the newest date borrowed. None when no loan has one.
    #[ts(type = "number | null")]
    pub streak_days: Option<i64>,
    /// Money freed up (LOAN-7): this month to the month after the debt-free date.
    pub freed_up: Vec<FreedMonth>,
}

/// One app month of Money freed up (LOAN-7, DATA_MODEL §5.11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FreedMonth {
    pub start: NaiveDate,
    pub end: NaiveDate,
    /// The scheduled amounts due in it.
    pub due: Money,
    /// How much less than this month: below 0 when it's heavier.
    pub freed: Money,
}

/// One loan's figures from its payments, listed by due date, then id (DATA_MODEL §5.11).
pub fn figures(payments: &[Payment], amount_borrowed: Option<Money>) -> LoanFigures {
    let left = total(
        payments
            .iter()
            .filter(|p| p.paid.is_none())
            .map(|p| p.amount),
    );
    let paid = total(payments.iter().filter_map(|p| p.paid));
    // `min_by_key` keeps the first of equal dates, so a tie goes to the lower id.
    let next = (0..payments.len())
        .filter(|&i| payments[i].paid.is_none())
        .min_by_key(|&i| payments[i].due_date);
    let interest = amount_borrowed
        .and_then(|borrowed| total([paid, left].into_iter()).checked_sub(borrowed))
        .filter(|&cost| cost >= Money::ZERO);
    LoanFigures {
        left,
        paid,
        percent_paid: percent_paid(paid, left),
        next,
        interest,
        paid_off: next.is_none(),
    }
}

/// The figures across every loan given, archived ones included. None only for a date past
/// chrono's range.
pub fn summary(
    loans: &[LoanInput],
    today: NaiveDate,
    settings: &PeriodSettings,
) -> Option<LoanSummary> {
    let each: Vec<LoanFigures> = loans
        .iter()
        .map(|l| figures(l.payments, l.amount_borrowed))
        .collect();
    let total_left = total(each.iter().map(|f| f.left));
    let total_paid = total(each.iter().map(|f| f.paid));
    let interest = each
        .iter()
        .any(|f| f.interest.is_some())
        .then(|| total(each.iter().filter_map(|f| f.interest)));
    let debt_free_on = loans
        .iter()
        .flat_map(|l| l.payments)
        .filter(|p| p.paid.is_none())
        .map(|p| p.due_date)
        .max()
        .map(|last| last.max(today));
    let freed_up = match debt_free_on {
        Some(last) => freed_up(loans, today, last, settings)?,
        None => Vec::new(),
    };
    Some(LoanSummary {
        total_left,
        total_paid,
        percent_paid: percent_paid(total_paid, total_left),
        debt_free_on,
        days_to_go: debt_free_on.map(|last| (last - today).num_days()),
        interest,
        streak_days: loans
            .iter()
            .filter_map(|l| l.borrowed_on)
            .max()
            .map(|newest| (today - newest).num_days().max(0)),
        freed_up,
    })
}

/// The app months from today's to the one after `debt_free_on`'s. This month's due counts its
/// paid payments and every unpaid one before it, so it doesn't move as the user pays.
fn freed_up(
    loans: &[LoanInput],
    today: NaiveDate,
    debt_free_on: NaiveDate,
    settings: &PeriodSettings,
) -> Option<Vec<FreedMonth>> {
    let month = |date: NaiveDate| Period::containing(PeriodKind::Month, date, settings);
    let after = |m: &Period| m.end().checked_add_days(Days::new(1)).and_then(month);
    let current = month(today)?;
    let last = after(&month(debt_free_on)?)?;
    // Each payment is put in its month once, so a far-off date doesn't rescan every payment.
    let mut due_by_start: HashMap<NaiveDate, Money> = HashMap::new();
    for p in loans.iter().flat_map(|l| l.payments) {
        let start = if p.paid.is_none() && p.due_date < current.start() {
            current.start()
        } else {
            month(p.due_date)?.start()
        };
        let due = due_by_start.entry(start).or_insert(Money::ZERO);
        *due = total([*due, p.amount].into_iter());
    }
    let base = due_by_start
        .get(&current.start())
        .copied()
        .unwrap_or(Money::ZERO);
    let mut rows = Vec::new();
    let mut m = current;
    loop {
        let due = due_by_start.get(&m.start()).copied().unwrap_or(Money::ZERO);
        rows.push(FreedMonth {
            start: m.start(),
            end: m.end(),
            due,
            freed: Money(base.0.saturating_sub(due.0)),
        });
        if m == last {
            return Some(rows);
        }
        m = after(&m)?;
    }
}

/// "Move money first" (LOAN-4, DATA_MODEL §5.11): `dues` are each banner's account and amount,
/// soonest first. Each gets only its own part of what its account's running total lacks against
/// the balance, so following every banner moves exactly enough. None when it needs nothing.
pub fn move_first(dues: &[(i64, Money)], balance: impl Fn(i64) -> Money) -> Vec<Option<Money>> {
    // Per account: the running total so far, and how much of it the earlier banners asked for.
    let mut told: HashMap<i64, (i64, i64)> = HashMap::new();
    dues.iter()
        .map(|&(account, amount)| {
            let (running, asked) = told.entry(account).or_insert((0, 0));
            *running = running.saturating_add(amount.0);
            let lacks = running.saturating_sub(balance(account).0).max(0);
            let share = lacks - *asked;
            *asked = lacks;
            (share > 0).then_some(Money(share))
        })
        .collect()
}

/// Whole percent rounded down, 100 once nothing is left.
fn percent_paid(paid: Money, left: Money) -> i64 {
    if left == Money::ZERO {
        100
    } else {
        used_percent(paid, total([paid, left].into_iter()))
    }
}

/// Saturates rather than overflowing, which no real set of loans comes near.
fn total(amounts: impl Iterator<Item = Money>) -> Money {
    Money(amounts.fold(0, |sum, m| sum.saturating_add(m.0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("a real test date")
    }

    fn today() -> NaiveDate {
        d(2026, 11, 15)
    }

    fn unpaid(date: NaiveDate, amount: i64) -> Payment {
        Payment {
            due_date: date,
            amount: Money(amount),
            paid: None,
        }
    }

    /// Paid in the app (`paid` may include a late fee) or paid before (`paid` = `amount`).
    fn paid(date: NaiveDate, amount: i64, paid: i64) -> Payment {
        Payment {
            due_date: date,
            amount: Money(amount),
            paid: Some(Money(paid)),
        }
    }

    fn loan(payments: &[Payment]) -> LoanInput<'_> {
        LoanInput {
            payments,
            amount_borrowed: None,
            borrowed_on: None,
        }
    }

    fn settings(month_start_day: u32) -> PeriodSettings {
        PeriodSettings {
            week_start: Weekday::Mon,
            month_start_day,
        }
    }

    fn sum(loans: &[LoanInput]) -> LoanSummary {
        summary(loans, today(), &settings(1)).expect("dates in range")
    }

    /// (start, end, due, freed) of each freed-up month.
    fn freed(loans: &[LoanInput], s: u32) -> Vec<(NaiveDate, NaiveDate, i64, i64)> {
        summary(loans, today(), &settings(s))
            .expect("dates in range")
            .freed_up
            .iter()
            .map(|m| (m.start, m.end, m.due.0, m.freed.0))
            .collect()
    }

    #[test]
    fn a_paid_payment_counts_its_transactions_amount_and_paid_before_its_scheduled_one() {
        let payments = [
            paid(d(2026, 9, 10), 100_000, 100_000),
            // Paid in the app with a ₱50 late fee.
            paid(d(2026, 10, 10), 100_000, 105_000),
            unpaid(d(2026, 11, 10), 100_000),
            unpaid(d(2026, 12, 10), 100_000),
        ];
        let f = figures(&payments, None);
        assert_eq!(f.paid, Money(205_000));
        assert_eq!(f.left, Money(200_000));
        assert!(!f.paid_off);
    }

    #[test]
    fn percent_paid_is_rounded_down_and_100_only_once_nothing_is_left() {
        let mostly = [
            paid(d(2026, 10, 1), 99_900, 99_900),
            unpaid(d(2026, 12, 1), 100),
        ];
        assert_eq!(figures(&mostly, None).percent_paid, 99);
        let none = [unpaid(d(2026, 12, 1), 100_000)];
        assert_eq!(figures(&none, None).percent_paid, 0);
        let all = [paid(d(2026, 10, 1), 100_000, 120_000)];
        let f = figures(&all, None);
        assert_eq!(f.percent_paid, 100);
        assert!(f.paid_off);
        assert_eq!(f.left, Money::ZERO);
    }

    #[test]
    fn next_is_the_earliest_unpaid_payment_with_ties_to_the_earlier_row() {
        let payments = [
            unpaid(d(2026, 11, 10), 100_000),
            unpaid(d(2026, 11, 10), 50_000),
            unpaid(d(2026, 12, 10), 100_000),
        ];
        assert_eq!(figures(&payments, None).next, Some(0));
        // A later one paid early doesn't move it past an earlier one still unpaid.
        let early = [
            unpaid(d(2026, 11, 10), 100_000),
            paid(d(2026, 12, 10), 100_000, 100_000),
        ];
        assert_eq!(figures(&early, None).next, Some(0));
        let first_paid = [
            paid(d(2026, 11, 10), 100_000, 100_000),
            unpaid(d(2026, 12, 10), 100_000),
        ];
        assert_eq!(figures(&first_paid, None).next, Some(1));
        let done = [paid(d(2026, 11, 10), 100_000, 100_000)];
        assert_eq!(figures(&done, None).next, None);
    }

    #[test]
    fn interest_is_every_payment_less_the_amount_borrowed_and_none_below_0() {
        let payments = [
            paid(d(2026, 10, 10), 100_000, 110_000),
            unpaid(d(2026, 11, 10), 100_000),
        ];
        assert_eq!(figures(&payments, None).interest, None);
        // The late fee is part of the cost.
        assert_eq!(
            figures(&payments, Some(Money(180_000))).interest,
            Some(Money(30_000))
        );
        assert_eq!(
            figures(&payments, Some(Money(210_000))).interest,
            Some(Money::ZERO)
        );
        assert_eq!(figures(&payments, Some(Money(210_001))).interest, None);
    }

    #[test]
    fn the_totals_and_interest_count_a_paid_off_loan_too() {
        // An archived loan is paid off: it still counts (DATA_MODEL §9 item 46).
        let done = [paid(d(2026, 8, 1), 200_000, 200_000)];
        let open = [unpaid(d(2026, 12, 1), 100_000)];
        let mut archived = loan(&done);
        archived.amount_borrowed = Some(Money(150_000));
        let mut active = loan(&open);
        active.amount_borrowed = Some(Money(90_000));
        let s = sum(&[archived, active]);
        assert_eq!(s.total_paid, Money(200_000));
        assert_eq!(s.total_left, Money(100_000));
        assert_eq!(s.percent_paid, 66);
        assert_eq!(s.interest, Some(Money(60_000)));
        // Only the loans that have one count.
        assert_eq!(sum(&[archived, loan(&open)]).interest, Some(Money(50_000)));
        assert_eq!(sum(&[loan(&done), loan(&open)]).interest, None);
    }

    #[test]
    fn debt_free_on_is_the_latest_unpaid_date() {
        let a = [
            unpaid(d(2026, 12, 10), 100_000),
            unpaid(d(2027, 8, 10), 100_000),
        ];
        // A paid payment after it doesn't count.
        let b = [
            unpaid(d(2027, 3, 1), 100_000),
            paid(d(2027, 9, 1), 100_000, 100_000),
        ];
        let s = sum(&[loan(&a), loan(&b)]);
        assert_eq!(s.debt_free_on, Some(d(2027, 8, 10)));
        assert_eq!(s.days_to_go, Some(268));
    }

    #[test]
    fn with_nothing_left_there_is_no_debt_free_date_or_freed_up_month() {
        let done = [paid(d(2026, 11, 1), 100_000, 100_000)];
        let s = sum(&[loan(&done)]);
        assert_eq!(s.debt_free_on, None);
        assert_eq!(s.days_to_go, None);
        assert_eq!(s.percent_paid, 100);
        assert!(s.freed_up.is_empty());
        let empty = sum(&[]);
        assert_eq!(
            (empty.debt_free_on, empty.total_left, empty.percent_paid),
            (None, Money::ZERO, 100)
        );
        assert!(empty.freed_up.is_empty());
    }

    #[test]
    fn the_last_payment_due_today_is_debt_free_today_without_being_overdue() {
        let p = [unpaid(today(), 100_000)];
        let s = sum(&[loan(&p)]);
        assert_eq!((s.debt_free_on, s.days_to_go), (Some(today()), Some(0)));
    }

    #[test]
    fn when_everything_left_is_overdue_the_debt_free_date_is_today() {
        let late = [
            unpaid(d(2026, 10, 20), 100_000),
            unpaid(d(2026, 11, 14), 50_000),
        ];
        let s = sum(&[loan(&late)]);
        assert_eq!(s.debt_free_on, Some(today()));
        assert_eq!(s.days_to_go, Some(0));
        // This month carries what's overdue, and the next is free of it.
        assert_eq!(
            freed(&[loan(&late)], 1),
            vec![
                (d(2026, 11, 1), d(2026, 11, 30), 150_000, 0),
                (d(2026, 12, 1), d(2026, 12, 31), 0, 150_000),
            ]
        );
    }

    #[test]
    fn the_streak_counts_from_the_newest_date_borrowed() {
        let p = [unpaid(d(2026, 12, 1), 100_000)];
        let mut older = loan(&p);
        older.borrowed_on = Some(d(2026, 1, 5));
        let mut newer = loan(&p);
        // A loan the user already has: its date borrowed is when they last borrowed.
        newer.borrowed_on = Some(d(2026, 9, 24));
        assert_eq!(sum(&[older, newer, loan(&p)]).streak_days, Some(52));
        let mut same_day = loan(&p);
        same_day.borrowed_on = Some(today());
        assert_eq!(sum(&[same_day]).streak_days, Some(0));
        // A clock set back after saving never makes it negative.
        let mut ahead = loan(&p);
        ahead.borrowed_on = Some(d(2026, 11, 16));
        assert_eq!(sum(&[ahead]).streak_days, Some(0));
    }

    #[test]
    fn with_no_date_borrowed_there_is_no_streak() {
        let p = [unpaid(d(2026, 12, 1), 100_000)];
        assert_eq!(sum(&[loan(&p), loan(&p)]).streak_days, None);
    }

    #[test]
    fn freed_up_runs_by_calendar_month_to_the_month_after_the_debt_free_date() {
        // Today is 2027-11-15 here, so the months reach a leap February.
        let p = [
            unpaid(d(2027, 11, 30), 100_000),
            unpaid(d(2027, 12, 31), 100_000),
            unpaid(d(2028, 1, 31), 60_000),
            unpaid(d(2028, 2, 29), 60_000),
        ];
        let months: Vec<_> = summary(&[loan(&p)], d(2027, 11, 15), &settings(1))
            .expect("dates in range")
            .freed_up
            .iter()
            .map(|m| (m.start, m.end, m.due.0, m.freed.0))
            .collect();
        assert_eq!(
            months,
            vec![
                (d(2027, 11, 1), d(2027, 11, 30), 100_000, 0),
                (d(2027, 12, 1), d(2027, 12, 31), 100_000, 0),
                (d(2028, 1, 1), d(2028, 1, 31), 60_000, 40_000),
                (d(2028, 2, 1), d(2028, 2, 29), 60_000, 40_000),
                (d(2028, 3, 1), d(2028, 3, 31), 0, 100_000),
            ]
        );
    }

    #[test]
    fn this_months_due_counts_its_paid_payments_and_earlier_overdue_ones() {
        let a = [
            // Paid last month: not part of this month.
            paid(d(2026, 10, 5), 100_000, 100_000),
            // Overdue from last month: still due now.
            unpaid(d(2026, 10, 20), 100_000),
            // Paid this month, with a late fee: the scheduled amount counts.
            paid(d(2026, 11, 5), 100_000, 120_000),
            unpaid(d(2026, 12, 5), 100_000),
        ];
        // Another loan's payment due this month but paid before, and one paid early.
        let b = [
            paid(d(2026, 11, 2), 30_000, 30_000),
            paid(d(2026, 12, 2), 30_000, 30_000),
            unpaid(d(2027, 1, 2), 30_000),
        ];
        assert_eq!(
            freed(&[loan(&a), loan(&b)], 1),
            vec![
                (d(2026, 11, 1), d(2026, 11, 30), 230_000, 0),
                (d(2026, 12, 1), d(2026, 12, 31), 130_000, 100_000),
                (d(2027, 1, 1), d(2027, 1, 31), 30_000, 200_000),
                (d(2027, 2, 1), d(2027, 2, 28), 0, 230_000),
            ]
        );
    }

    #[test]
    fn a_heavier_later_month_frees_less_than_0() {
        let p = [
            unpaid(d(2026, 11, 20), 50_000),
            unpaid(d(2026, 12, 20), 80_000),
        ];
        assert_eq!(
            freed(&[loan(&p)], 1),
            vec![
                (d(2026, 11, 1), d(2026, 11, 30), 50_000, 0),
                (d(2026, 12, 1), d(2026, 12, 31), 80_000, -30_000),
                (d(2027, 1, 1), d(2027, 1, 31), 0, 50_000),
            ]
        );
    }

    #[test]
    fn freed_up_follows_the_month_start_day() {
        // With months starting on the 25th, Nov 15 is in Oct 25 – Nov 24.
        let p = [
            unpaid(d(2026, 11, 24), 100_000),
            unpaid(d(2026, 11, 25), 70_000),
            unpaid(d(2026, 12, 24), 20_000),
        ];
        assert_eq!(
            freed(&[loan(&p)], 25),
            vec![
                (d(2026, 10, 25), d(2026, 11, 24), 100_000, 0),
                (d(2026, 11, 25), d(2026, 12, 24), 90_000, 10_000),
                (d(2026, 12, 25), d(2027, 1, 24), 0, 100_000),
            ]
        );
        // On the 25th the next month has started, and the 24th's payment is now overdue in it.
        let first = |today: NaiveDate| {
            summary(&[loan(&p)], today, &settings(25))
                .expect("dates in range")
                .freed_up[0]
        };
        let nov_24 = first(d(2026, 11, 24));
        assert_eq!(
            (nov_24.start, nov_24.due),
            (d(2026, 10, 25), Money(100_000))
        );
        let nov_25 = first(d(2026, 11, 25));
        assert_eq!(
            (nov_25.start, nov_25.end, nov_25.due),
            (d(2026, 11, 25), d(2026, 12, 24), Money(190_000))
        );
    }

    fn shares(balances: &[(i64, i64)], dues: &[(i64, i64)]) -> Vec<Option<i64>> {
        let dues: Vec<(i64, Money)> = dues.iter().map(|&(a, m)| (a, Money(m))).collect();
        let balance = |id| Money(balances.iter().find(|b| b.0 == id).map_or(0, |b| b.1));
        move_first(&dues, balance)
            .into_iter()
            .map(|m| m.map(|m| m.0))
            .collect()
    }

    #[test]
    fn move_first_gives_each_banner_only_its_own_share() {
        // DATA_MODEL §5.11: ₱200 in the account, ₱1,000 then ₱500 due.
        let dues = [(1, 100_000), (1, 50_000)];
        assert_eq!(shares(&[(1, 20_000)], &dues), [Some(80_000), Some(50_000)]);
    }

    #[test]
    fn move_first_is_none_while_the_account_covers_it() {
        let dues = [(1, 100_000), (1, 50_000)];
        assert_eq!(shares(&[(1, 200_000)], &dues), [None, None]);
        assert_eq!(
            shares(&[(1, 150_000)], &dues),
            [None, None],
            "exactly enough"
        );
        assert_eq!(shares(&[(1, 120_000)], &dues), [None, Some(30_000)]);
    }

    #[test]
    fn move_first_keeps_a_running_total_per_account() {
        let dues = [(1, 10_000), (2, 5_000), (1, 30_000)];
        assert_eq!(
            shares(&[(1, 20_000), (2, 0)], &dues),
            [None, Some(5_000), Some(20_000)]
        );
    }

    #[test]
    fn move_first_below_zero_also_covers_what_the_account_lacks() {
        // Moving ₱1,100 pays ₱1,000 without taking the account further below 0.
        assert_eq!(shares(&[(1, -10_000)], &[(1, 100_000)]), [Some(110_000)]);
    }
}
