//! Pay windows, expected pay, card bills and Safe to spend over the 12-month horizon
//! (DATA_MODEL §5.12–5.15). The service gathers the dates and amounts; everything here is pure.

use chrono::{Datelike, Months, NaiveDate};

use crate::domain::card_cycle::{card_cycle, on_day};
use crate::domain::loan::total;
use crate::domain::money::Money;

/// A pay window, or any other span of days the forecast groups by: both ends included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PayWindow {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
}

/// What a movement comes from, by id: a loan payment, a card (its account), a recurring rule, a
/// future-dated transaction, or a payday rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    Loan(i64),
    Card(i64),
    Recurring(i64),
    Transaction(i64),
    Payday(i64),
}

/// Money due in or out of the counted accounts on a date (§5.15). A date before today counts as
/// today: it's overdue or waiting to be settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Movement {
    pub date: NaiveDate,
    pub amount: Money,
    pub direction: Direction,
    pub source: Source,
}

/// Where an account sits in the forecast: counted (active cash, bank or e-wallet, not a goal's), a
/// credit card (archived ones included), or anything else (§5.15).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Counted,
    Card,
    Other,
}

/// A future-dated transaction's or a recurring date's kind, with where a transfer goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Income,
    Expense,
    Transfer { to: Place },
}

/// What a future-dated transaction or recurring date does: money in or out of the counted
/// accounts, or a charge or refund on a card, which its bills pick up (§5.14).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    In,
    Out,
    Charge,
    Refund,
}

/// A credit card, as its bills start from today (§5.14).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardState {
    pub account_id: i64,
    pub statement_day: u8,
    pub due_day: u8,
    /// −balance today (§5.1): below 0 when the card is in credit, which later charges use up.
    pub owed_now: Money,
    /// For the latest statement (see `first_statement`): what was owed at the end of its closing
    /// date, and the transfers into the card and income on it dated after that, up to today.
    pub owed_at_close: Money,
    pub paid_since_close: Money,
}

/// One pay window (or month) of the forecast.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub start_amount: Money,
    pub money_in: Money,
    pub money_out: Money,
    /// What would be left at its end with nothing else spent.
    pub left: Money,
    /// The lowest running total over its days (from today in the first row).
    pub low: Money,
    /// Its first day with the running total below 0: it runs short (§5.15).
    pub short_on: Option<NaiveDate>,
    /// Soonest first, dated today or later.
    pub movements: Vec<Movement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forecast {
    pub rows: Vec<Row>,
    pub safe_to_spend: Money,
    pub per_day: Money,
    /// The current window's days from today, today included: what `per_day` divides by.
    pub days_left: i64,
    /// `left` of the current window less safe to spend, or 0.
    pub held_back: Money,
    /// The window holding the lowest day, when something is held back.
    pub held_for: Option<PayWindow>,
    /// The first day the running total goes below 0, and by how much its window runs short.
    pub short: Option<(NaiveDate, Money)>,
}

/// The horizon's last day: the day before the same date 12 months on, clamped to the month's end.
pub fn horizon_end(today: NaiveDate) -> Option<NaiveDate> {
    today.checked_add_months(Months::new(12))?.pred_opt()
}

/// The current window, then one from each later payday up to `horizon_end` (§5.12). `paydays` are
/// every payday rule's scheduled dates, in any order; a shared date is one boundary. With no
/// payday rule at all, the service passes no windows to `forecast` instead.
pub fn pay_windows(
    paydays: &[NaiveDate],
    today: NaiveDate,
    horizon_end: NaiveDate,
) -> Vec<PayWindow> {
    let current = paydays
        .iter()
        .copied()
        .filter(|d| *d <= today)
        .max()
        .unwrap_or(today);
    let mut starts: Vec<NaiveDate> = paydays
        .iter()
        .copied()
        .filter(|d| *d > today && *d <= horizon_end)
        .collect();
    starts.sort();
    starts.dedup();
    starts.insert(0, current);
    starts
        .iter()
        .enumerate()
        .map(|(i, &start)| PayWindow {
            start,
            end: starts
                .get(i + 1)
                .and_then(|next| next.pred_opt())
                .unwrap_or(horizon_end),
        })
        .collect()
}

/// The lowest of the latest 3 payslip nets (newest first; older ones are ignored), plus the
/// rule's estimate while there are fewer than 3 (§5.13).
pub fn expected_pay(recent_nets: &[Money], estimate: Money) -> Money {
    let recent = &recent_nets[..recent_nets.len().min(3)];
    let counted = (recent.len() < 3).then_some(estimate);
    recent
        .iter()
        .copied()
        .chain(counted)
        .min()
        .unwrap_or(estimate)
}

/// What a row from account `from` does in the forecast (§5.15). Transfers to or from a card
/// count for nothing: its bills stand in for them, as for transfers between counted accounts.
pub fn effect(flow: Flow, from: Place) -> Option<Effect> {
    match (flow, from) {
        (Flow::Expense, Place::Counted) => Some(Effect::Out),
        (Flow::Income, Place::Counted) => Some(Effect::In),
        (Flow::Expense, Place::Card) => Some(Effect::Charge),
        (Flow::Income, Place::Card) => Some(Effect::Refund),
        (Flow::Transfer { to: Place::Other }, Place::Counted) => Some(Effect::Out),
        (Flow::Transfer { to: Place::Counted }, Place::Other) => Some(Effect::In),
        _ => None,
    }
}

/// The latest statement: its closing date C₀ (today or earlier) and D₀, the first due date after
/// it, which pays it. The service reads `owed_at_close` and `paid_since_close` for C₀.
pub fn first_statement(
    statement_day: u8,
    due_day: u8,
    today: NaiveDate,
) -> Option<(NaiveDate, NaiveDate)> {
    let cycle = card_cycle(statement_day, due_day, today)?;
    let close = if cycle.end == today {
        today
    } else {
        cycle.start.pred_opt()?
    };
    let due = card_cycle(statement_day, due_day, close.succ_opt()?)?.due;
    Some((due, close))
}

/// C(D): the latest closing date before `due`.
fn closing_before(due: NaiveDate, statement_day: u8) -> Option<NaiveDate> {
    let this_month = on_day(due, statement_day)?;
    if this_month < due {
        return Some(this_month);
    }
    on_day(
        due.with_day(1)?.checked_sub_months(Months::new(1))?,
        statement_day,
    )
}

/// A bill on each due date up to `horizon_end`: the statement balance, as if paid in full
/// (§5.14). `charges` are the card's future-dated and recurring expenses, with refunds (income on
/// it) below 0. A bill of 0 is left out.
pub fn card_bills(
    card: &CardState,
    charges: &[(NaiveDate, Money)],
    today: NaiveDate,
    horizon_end: NaiveDate,
) -> Vec<Movement> {
    let mut bills = Vec::new();
    let Some((first_due, first_close)) = first_statement(card.statement_day, card.due_day, today)
    else {
        return bills;
    };
    let Some(first_month) = first_due.with_day(1) else {
        return bills;
    };
    let mut bill_on = |date: NaiveDate, bill: i64| {
        if bill > 0 && date <= horizon_end {
            bills.push(Movement {
                date,
                amount: Money(bill),
                direction: Direction::Out,
                source: Source::Card(card.account_id),
            });
        }
    };
    // The latest statement, due today once its due date has passed (anything overdue is).
    let closed = card.owed_at_close.0.saturating_sub(card.paid_since_close.0);
    let mut billed = closed.clamp(0, card.owed_now.0.max(0));
    bill_on(first_due.max(today), billed);
    let mut paid_close = first_close;
    for k in 1.. {
        // From each month's 1st, so a due day of 31 never drifts to the 30th.
        let due = first_month
            .checked_add_months(Months::new(k))
            .and_then(|month| on_day(month, card.due_day));
        let Some(due) = due.filter(|due| *due <= horizon_end) else {
            break;
        };
        let Some(close) = closing_before(due, card.statement_day) else {
            break;
        };
        // Statement 30, due 31: Jan 31 and Feb 28 both pay Jan 30's statement, billed once.
        if std::mem::replace(&mut paid_close, close) == close {
            continue;
        }
        let charged = total(charges.iter().filter(|(d, _)| *d <= close).map(|(_, m)| *m)).0;
        let bill = card
            .owed_now
            .0
            .saturating_add(charged)
            .saturating_sub(billed)
            .max(0);
        billed = billed.saturating_add(bill);
        bill_on(due, bill);
    }
    bills
}

/// Groups the movements by span, carrying each span's `left` into the next one's start, and walks
/// the running total day by day for each span's `low` and `short_on`. Dates before today count
/// as today, and movements after the last span are left out.
pub fn rows(
    money_now: Money,
    today: NaiveDate,
    spans: &[PayWindow],
    movements: &[Movement],
) -> Vec<Row> {
    let mut start_amount = money_now;
    spans
        .iter()
        .enumerate()
        .map(|(i, span)| {
            let mut ms: Vec<Movement> = movements
                .iter()
                .map(|m| Movement {
                    date: m.date.max(today),
                    ..*m
                })
                .filter(|m| span.start <= m.date && m.date <= span.end)
                .collect();
            ms.sort_by_key(|m| m.date);
            let sum = |direction| {
                total(
                    ms.iter()
                        .filter(|m| m.direction == direction)
                        .map(|m| m.amount),
                )
                .0
            };
            let (money_in, money_out) = (sum(Direction::In), sum(Direction::Out));
            let first_day = if i == 0 { today } else { span.start };
            let mut run = start_amount.0;
            let mut low = i64::MAX;
            let mut short_on = None;
            let mut see = |run: i64, day: NaiveDate| {
                low = low.min(run);
                if run < 0 && short_on.is_none() {
                    short_on = Some(day);
                }
            };
            // A first day with nothing due keeps the total it starts with.
            if ms.first().is_none_or(|m| m.date > first_day) {
                see(run, first_day);
            }
            for day in ms.chunk_by(|a, b| a.date == b.date) {
                run = day.iter().fold(run, |run, m| match m.direction {
                    Direction::In => run.saturating_add(m.amount.0),
                    Direction::Out => run.saturating_sub(m.amount.0),
                });
                see(run, day[0].date);
            }
            let row = Row {
                low: Money(low),
                short_on,
                start: span.start,
                end: span.end,
                start_amount,
                money_in: Money(money_in),
                money_out: Money(money_out),
                left: Money(
                    start_amount
                        .0
                        .saturating_add(money_in)
                        .saturating_sub(money_out),
                ),
                movements: ms,
            };
            start_amount = row.left;
            row
        })
        .collect()
}

/// Safe to spend: the lowest running total, day by day, over every window, never below 0, with
/// what's held back and the first short day (§5.15). `None` with no windows (no payday rule).
pub fn forecast(
    money_now: Money,
    today: NaiveDate,
    windows: &[PayWindow],
    movements: &[Movement],
) -> Option<Forecast> {
    let current = *windows.first()?;
    let rows = rows(money_now, today, windows, movements);
    let lowest_at = lowest_row(&rows);
    let safe = rows[lowest_at].low.0.max(0);
    let held_back = rows[0].left.0.saturating_sub(safe).max(0);
    let days_left = (current.end - today).num_days() + 1;
    Some(Forecast {
        safe_to_spend: Money(safe),
        per_day: Money(safe / days_left.max(1)),
        days_left,
        held_back: Money(held_back),
        held_for: (held_back > 0).then_some(windows[lowest_at]),
        short: rows
            .iter()
            .find_map(|r| r.short_on.map(|day| (day, Money(r.low.0.saturating_neg())))),
        rows,
    })
}

/// The row holding the lowest day: the first one on a tie (0 with no rows).
pub fn lowest_row(rows: &[Row]) -> usize {
    rows.iter()
        .enumerate()
        .fold((0, i64::MAX), |best, (i, r)| {
            if r.low.0 < best.1 { (i, r.low.0) } else { best }
        })
        .0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::recurrence::{Frequency, Schedule};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("a real test date")
    }

    fn w(start: NaiveDate, end: NaiveDate) -> PayWindow {
        PayWindow { start, end }
    }

    fn out(date: NaiveDate, amount: i64) -> Movement {
        Movement {
            date,
            amount: Money(amount),
            direction: Direction::Out,
            source: Source::Recurring(1),
        }
    }

    fn pay(date: NaiveDate, amount: i64) -> Movement {
        Movement {
            date,
            amount: Money(amount),
            direction: Direction::In,
            source: Source::Payday(9),
        }
    }

    fn monthly(start: NaiveDate) -> Schedule {
        Schedule {
            frequency: Frequency::Monthly,
            interval: 1,
            start,
            end: None,
            max_occurrences: None,
        }
    }

    /// Every date of a schedule up to `until`, as the service reads a payday rule's dates.
    fn dates(schedule: &Schedule, until: NaiveDate) -> Vec<NaiveDate> {
        schedule.dates_between(0, 0, NaiveDate::MIN, until, |_| false)
    }

    fn card(owed_now: i64) -> CardState {
        CardState {
            account_id: 7,
            statement_day: 20,
            due_day: 5,
            owed_now: Money(owed_now),
            owed_at_close: Money::ZERO,
            paid_since_close: Money::ZERO,
        }
    }

    fn bills(
        card: &CardState,
        charges: &[(NaiveDate, i64)],
        today: NaiveDate,
        until: NaiveDate,
    ) -> Vec<(NaiveDate, i64)> {
        let charges: Vec<_> = charges.iter().map(|&(date, m)| (date, Money(m))).collect();
        card_bills(card, &charges, today, until)
            .iter()
            .map(|m| {
                assert_eq!((m.direction, m.source), (Direction::Out, Source::Card(7)));
                (m.date, m.amount.0)
            })
            .collect()
    }

    // --- The horizon and pay windows (§5.12) ---

    #[test]
    fn the_horizon_ends_the_day_before_the_same_date_a_year_on() {
        assert_eq!(horizon_end(d(2026, 11, 13)), Some(d(2027, 11, 12)));
        assert_eq!(horizon_end(d(2027, 1, 31)), Some(d(2028, 1, 30)));
        // Feb 29 a year on is Feb 28, so the horizon ends the 27th.
        assert_eq!(horizon_end(d(2028, 2, 29)), Some(d(2029, 2, 27)));
        assert_eq!(horizon_end(NaiveDate::MAX), None);
    }

    #[test]
    fn the_last_window_ends_on_the_horizons_end() {
        let today = d(2027, 1, 31);
        let end = horizon_end(today).expect("in range");
        let paydays = dates(&monthly(d(2027, 1, 31)), end);
        let windows = pay_windows(&paydays, today, end);
        assert_eq!(windows.len(), 12);
        assert_eq!(
            windows[0],
            w(d(2027, 1, 31), d(2027, 2, 27)),
            "Feb 28 is a payday"
        );
        assert_eq!(windows[1], w(d(2027, 2, 28), d(2027, 3, 30)));
        assert_eq!(windows[11], w(d(2027, 12, 31), d(2028, 1, 30)));
    }

    #[test]
    fn a_bill_due_on_a_payday_falls_in_the_window_that_starts_that_day() {
        let today = d(2026, 11, 13);
        let windows = pay_windows(&[d(2026, 11, 15), d(2026, 11, 30)], today, d(2027, 11, 12));
        assert_eq!(
            windows,
            [
                w(today, d(2026, 11, 14)),
                w(d(2026, 11, 15), d(2026, 11, 29)),
                w(d(2026, 11, 30), d(2027, 11, 12)),
            ]
        );
        let f = forecast(
            Money(10_000),
            today,
            &windows,
            &[out(d(2026, 11, 15), 4_000)],
        )
        .expect("windows");
        assert_eq!(f.rows[0].money_out, Money::ZERO);
        assert_eq!(f.rows[1].money_out, Money(4_000));
    }

    #[test]
    fn the_current_window_starts_on_the_latest_payday_on_or_before_today() {
        let today = d(2026, 11, 20);
        let paydays = dates(&monthly(d(2026, 10, 15)), d(2026, 12, 31));
        let windows = pay_windows(&paydays, today, d(2026, 12, 31));
        assert_eq!(windows[0], w(d(2026, 11, 15), d(2026, 12, 14)));
        // A payday today starts the current window today.
        assert_eq!(
            pay_windows(&paydays, d(2026, 11, 15), d(2026, 12, 31))[0].start,
            d(2026, 11, 15)
        );
        // Paydays all ahead: the current window starts today.
        assert_eq!(
            pay_windows(&[d(2026, 12, 1)], today, d(2026, 12, 31))[0],
            w(today, d(2026, 11, 30))
        );
        // Paydays all passed (the rule ended): one window to the horizon's end.
        assert_eq!(
            pay_windows(&[d(2026, 1, 15)], today, d(2026, 12, 31)),
            [w(d(2026, 1, 15), d(2026, 12, 31))]
        );
    }

    #[test]
    fn two_rules_on_the_same_date_make_one_boundary() {
        let today = d(2026, 11, 13);
        let end = d(2027, 1, 31);
        let fifteenth = dates(&monthly(d(2026, 10, 15)), end);
        let thirtieth = dates(&monthly(d(2026, 11, 30)), end);
        let mut both = [thirtieth.clone(), fifteenth.clone(), fifteenth].concat();
        both.push(d(2026, 11, 30));
        assert_eq!(
            pay_windows(&both, today, end),
            [
                w(d(2026, 10, 15), d(2026, 11, 14)),
                w(d(2026, 11, 15), d(2026, 11, 29)),
                w(d(2026, 11, 30), d(2026, 12, 14)),
                w(d(2026, 12, 15), d(2026, 12, 29)),
                w(d(2026, 12, 30), d(2027, 1, 14)),
                w(d(2027, 1, 15), d(2027, 1, 29)),
                w(d(2027, 1, 30), end),
            ]
        );
    }

    #[test]
    fn paydays_past_the_horizon_are_ignored_and_one_on_its_end_is_a_day_long() {
        let today = d(2026, 11, 13);
        let end = d(2026, 12, 15);
        assert_eq!(
            pay_windows(&[d(2026, 12, 15), d(2026, 12, 16)], today, end),
            [w(today, d(2026, 12, 14)), w(end, end)]
        );
    }

    #[test]
    fn no_payday_rule_means_no_forecast() {
        let today = d(2026, 11, 13);
        assert_eq!(forecast(Money(10_000), today, &[], &[out(today, 1)]), None);
        // A rule with no date in reach still has a current window, from today (§5.12).
        assert_eq!(
            pay_windows(&[], today, d(2027, 11, 12)),
            [w(today, d(2027, 11, 12))]
        );
    }

    // --- Expected pay (§5.13) ---

    #[test]
    fn expected_pay_is_the_lowest_of_the_last_3_payslips_and_the_estimate() {
        let estimate = Money(420_000);
        assert_eq!(expected_pay(&[], estimate), estimate);
        assert_eq!(expected_pay(&[Money(450_000)], estimate), estimate);
        assert_eq!(expected_pay(&[Money(400_000)], estimate), Money(400_000));
        assert_eq!(
            expected_pay(&[Money(450_000), Money(430_000)], estimate),
            estimate
        );
        // From the 3rd payslip on, the estimate no longer counts.
        let three = [Money(450_000), Money(430_000), Money(440_000)];
        assert_eq!(expected_pay(&three, estimate), Money(430_000));
        // Only the latest 3 count: an older, lower one is ignored.
        let four = [
            Money(450_000),
            Money(430_000),
            Money(440_000),
            Money(100_000),
        ];
        assert_eq!(expected_pay(&four, estimate), Money(430_000));
    }

    // --- What a future row does (§5.15) ---

    #[test]
    fn transfers_to_or_from_a_card_are_ignored() {
        use Place::*;
        assert_eq!(
            effect(Flow::Transfer { to: Card }, Counted),
            None,
            "paying the bill"
        );
        assert_eq!(
            effect(Flow::Transfer { to: Counted }, Card),
            None,
            "a cash advance"
        );
        assert_eq!(effect(Flow::Transfer { to: Card }, Other), None);
        assert_eq!(effect(Flow::Transfer { to: Other }, Card), None);
    }

    #[test]
    fn rows_count_only_what_moves_counted_money_or_a_cards_bills() {
        use Place::*;
        assert_eq!(effect(Flow::Expense, Counted), Some(Effect::Out));
        assert_eq!(effect(Flow::Income, Counted), Some(Effect::In));
        assert_eq!(effect(Flow::Expense, Card), Some(Effect::Charge));
        assert_eq!(effect(Flow::Income, Card), Some(Effect::Refund));
        assert_eq!(effect(Flow::Expense, Other), None);
        assert_eq!(effect(Flow::Income, Other), None);
        assert_eq!(
            effect(Flow::Transfer { to: Other }, Counted),
            Some(Effect::Out)
        );
        assert_eq!(
            effect(Flow::Transfer { to: Counted }, Other),
            Some(Effect::In)
        );
        assert_eq!(effect(Flow::Transfer { to: Counted }, Counted), None);
        assert_eq!(effect(Flow::Transfer { to: Other }, Other), None);
    }

    // --- Card bills (§5.14) ---

    #[test]
    fn a_card_statement_is_billed_once_on_its_due_date() {
        // Today Nov 25: the Nov 20 statement is due Dec 5. Charges until Dec 20 make the Jan 5 bill.
        let today = d(2026, 11, 25);
        let mut c = card(64_000);
        c.owed_at_close = Money(64_000);
        let charges = [
            (d(2026, 12, 1), 1_599),
            (d(2026, 12, 20), 500),
            (d(2026, 12, 21), 300),
        ];
        assert_eq!(
            bills(&c, &charges, today, d(2027, 2, 28)),
            [
                (d(2026, 12, 5), 64_000),
                (d(2027, 1, 5), 2_099),
                (d(2027, 2, 5), 300)
            ]
        );
    }

    #[test]
    fn two_due_dates_paying_the_same_statement_bill_it_once() {
        // Statement on the 30th, due on the 31st: Jan 31 and Feb 28 both pay the Jan 30 statement,
        // so Feb 28 bills nothing, and what's charged up to Mar 30 is billed Mar 31.
        let today = d(2027, 1, 10);
        let c = CardState {
            statement_day: 30,
            due_day: 31,
            ..card(0)
        };
        let charges = [
            (d(2027, 1, 20), 1_000),
            (d(2027, 2, 10), 200),
            (d(2027, 3, 30), 50),
        ];
        assert_eq!(
            bills(&c, &charges, today, d(2027, 3, 31)),
            [(d(2027, 1, 31), 1_000), (d(2027, 3, 31), 250)]
        );
    }

    #[test]
    fn a_shared_closed_statement_leaves_later_purchases_for_the_next_bill() {
        // Today Jan 31 (due today): the Jan 30 statement closed owing 1,000.00. Feb 28 pays the
        // same statement, so the 500.00 bought since waits for Mar 31.
        let today = d(2027, 1, 31);
        let c = CardState {
            statement_day: 30,
            due_day: 31,
            owed_at_close: Money(100_000),
            ..card(150_000)
        };
        assert_eq!(
            bills(&c, &[], today, d(2027, 3, 31)),
            [(d(2027, 1, 31), 100_000), (d(2027, 3, 31), 50_000)]
        );
    }

    #[test]
    fn a_statement_and_due_day_on_the_same_day_pay_the_month_before() {
        let today = d(2026, 11, 10);
        assert_eq!(
            first_statement(5, 5, today),
            Some((d(2026, 12, 5), d(2026, 11, 5)))
        );
        let c = CardState {
            statement_day: 5,
            due_day: 5,
            owed_at_close: Money(20_000),
            ..card(25_000)
        };
        assert_eq!(
            bills(&c, &[(d(2026, 12, 5), 1_000)], today, d(2027, 1, 31)),
            [(d(2026, 12, 5), 20_000), (d(2027, 1, 5), 6_000)]
        );
    }

    #[test]
    fn a_closed_statement_is_billed_less_what_was_paid_since() {
        // Today Nov 25: the Nov 20 statement closed owing 640.00, and 140.00 is paid since.
        let today = d(2026, 11, 25);
        let mut c = card(70_000);
        c.owed_at_close = Money(64_000);
        c.paid_since_close = Money(14_000);
        assert_eq!(
            bills(&c, &[], today, d(2026, 12, 31)),
            [(d(2026, 12, 5), 50_000)]
        );
        // Paid more than the statement: nothing is due on it.
        c.paid_since_close = Money(70_000);
        assert_eq!(bills(&c, &[], today, d(2026, 12, 31)), []);
        // Never more than what's owed now (a refund since closing).
        c.paid_since_close = Money::ZERO;
        c.owed_now = Money(60_000);
        assert_eq!(
            bills(&c, &[], today, d(2026, 12, 31)),
            [(d(2026, 12, 5), 60_000)]
        );
    }

    #[test]
    fn a_statement_closing_today_has_closed() {
        let today = d(2026, 11, 20);
        assert_eq!(first_statement(20, 5, today), Some((d(2026, 12, 5), today)));
        let mut c = card(64_000);
        c.owed_at_close = Money(64_000);
        // A charge waiting today goes on the next statement.
        assert_eq!(
            bills(&c, &[(today, 1_000)], today, d(2027, 1, 31)),
            [(d(2026, 12, 5), 64_000), (d(2027, 1, 5), 1_000)]
        );
    }

    #[test]
    fn a_statement_not_yet_closed_bills_the_projected_balance() {
        // Today Nov 10: Oct 20's statement was due Nov 5 and is paid; Nov 20's is due Dec 5.
        let today = d(2026, 11, 10);
        let charges = [(d(2026, 11, 8), 500), (d(2026, 11, 15), 1_599)];
        assert_eq!(
            bills(&card(30_000), &charges, today, d(2026, 12, 31)),
            [(d(2026, 12, 5), 32_099)],
            "what's owed now, a pending date (as today) and a future charge"
        );
    }

    #[test]
    fn an_overdue_statement_is_due_today() {
        // Today Nov 10: Oct 20's statement owed 250.00, due Nov 5, and only 50.00 is paid since.
        let today = d(2026, 11, 10);
        assert_eq!(
            first_statement(20, 5, today),
            Some((d(2026, 11, 5), d(2026, 10, 20)))
        );
        let c = CardState {
            owed_at_close: Money(25_000),
            paid_since_close: Money(5_000),
            ..card(30_000)
        };
        assert_eq!(
            bills(&c, &[(d(2026, 11, 15), 2_099)], today, d(2026, 12, 31)),
            [(today, 20_000), (d(2026, 12, 5), 12_099)]
        );
    }

    #[test]
    fn a_statement_due_on_the_31st_after_a_february_close_is_billed_once() {
        // Statement 30, due 31, today Mar 10: Feb 28's statement is due Mar 31, and what's
        // charged up to Mar 30 is billed Apr 30.
        let today = d(2027, 3, 10);
        assert_eq!(
            first_statement(30, 31, today),
            Some((d(2027, 3, 31), d(2027, 2, 28)))
        );
        let c = CardState {
            statement_day: 30,
            due_day: 31,
            owed_at_close: Money(10_000),
            ..card(10_000)
        };
        assert_eq!(
            bills(&c, &[(d(2027, 3, 20), 700)], today, d(2027, 4, 30)),
            [(d(2027, 3, 31), 10_000), (d(2027, 4, 30), 700)]
        );
    }

    #[test]
    fn credit_on_the_card_is_used_up_by_later_charges() {
        let today = d(2026, 11, 10);
        let charges = [(d(2026, 11, 12), 300), (d(2026, 11, 25), 800)];
        assert_eq!(
            bills(&card(-500), &charges, today, d(2027, 1, 31)),
            [(d(2027, 1, 5), 600)]
        );
    }

    #[test]
    fn purchases_after_the_closing_date_roll_into_the_next_bill() {
        let today = d(2026, 11, 10);
        let charges = [(d(2026, 11, 20), 1_000), (d(2026, 11, 21), 2_000)];
        assert_eq!(
            bills(&card(0), &charges, today, d(2027, 1, 31)),
            [(d(2026, 12, 5), 1_000), (d(2027, 1, 5), 2_000)]
        );
    }

    #[test]
    fn a_refund_lowers_the_bill() {
        let today = d(2026, 11, 10);
        let charges = [(d(2026, 11, 12), 5_000), (d(2026, 11, 18), -1_200)];
        assert_eq!(
            bills(&card(10_000), &charges, today, d(2026, 12, 31)),
            [(d(2026, 12, 5), 13_800)]
        );
        // A refund larger than the bill leaves nothing due, and the credit carries on.
        let charges = [(d(2026, 11, 18), -15_000), (d(2026, 12, 1), 8_000)];
        assert_eq!(
            bills(&card(10_000), &charges, today, d(2027, 1, 31)),
            [(d(2027, 1, 5), 3_000)]
        );
    }

    #[test]
    fn nothing_owed_and_nothing_projected_means_no_bills() {
        assert_eq!(bills(&card(0), &[], d(2026, 11, 10), d(2027, 11, 9)), []);
    }

    #[test]
    fn a_due_day_of_31_falls_on_each_months_last_day() {
        let c = CardState {
            statement_day: 15,
            due_day: 31,
            ..card(0)
        };
        let charges = [
            (d(2027, 1, 10), 100),
            (d(2027, 2, 10), 200),
            (d(2027, 3, 10), 300),
        ];
        assert_eq!(
            bills(&c, &charges, d(2027, 1, 1), d(2027, 3, 31)),
            [
                (d(2027, 1, 31), 100),
                (d(2027, 2, 28), 200),
                (d(2027, 3, 31), 300)
            ]
        );
    }

    // --- Safe to spend (§5.15) ---

    #[test]
    fn a_pending_payday_and_an_overdue_payment_count_as_today() {
        let today = d(2026, 11, 16);
        let windows = pay_windows(&[d(2026, 11, 15), d(2026, 11, 30)], today, d(2026, 12, 31));
        let movements = [pay(d(2026, 11, 15), 400_000), out(d(2026, 11, 2), 100_000)];
        let f = forecast(Money(5_000), today, &windows, &movements).expect("windows");
        assert_eq!(
            f.rows[0]
                .movements
                .iter()
                .map(|m| m.date)
                .collect::<Vec<_>>(),
            [today, today]
        );
        assert_eq!(f.rows[0].left, Money(305_000));
        assert_eq!(
            f.safe_to_spend,
            Money(305_000),
            "never 5,000 − 100,000 first: both land today"
        );
    }

    #[test]
    fn pay_and_a_bill_on_the_same_day_net_out_at_its_end() {
        // A bill due today lowers safe to spend; the same day's pay covers it, so nothing is short.
        let today = d(2026, 11, 15);
        let windows = [w(today, d(2026, 11, 29))];
        let movements = [out(today, 300_000), pay(today, 400_000)];
        let f = forecast(Money(100_000), today, &windows, &movements).expect("windows");
        assert_eq!((f.safe_to_spend, f.short), (Money(200_000), None));
        let f = forecast(Money(100_000), today, &windows, &[out(today, 30_000)]).expect("windows");
        assert_eq!(f.safe_to_spend, Money(70_000));
    }

    #[test]
    fn a_bill_before_income_later_in_the_window_lowers_safe_to_spend() {
        let today = d(2026, 11, 15);
        let windows = [
            w(d(2026, 10, 30), d(2026, 11, 29)),
            w(d(2026, 11, 30), d(2026, 12, 29)),
        ];
        let movements = [
            out(d(2026, 11, 20), 400_000),
            pay(d(2026, 11, 25), 300_000),
            pay(d(2026, 11, 30), 500_000),
        ];
        let f = forecast(Money(500_000), today, &windows, &movements).expect("windows");
        assert_eq!(f.rows[0].left, Money(400_000));
        assert_eq!(
            f.safe_to_spend,
            Money(100_000),
            "the Nov 20 dip, not the window's end"
        );
        assert_eq!(
            (f.held_back, f.held_for),
            (Money(300_000), Some(windows[0]))
        );
        assert_eq!(f.short, None);
    }

    #[test]
    fn safe_to_spend_holds_back_money_for_a_later_window_that_would_run_short() {
        let today = d(2026, 11, 1);
        let windows = [
            w(today, d(2026, 11, 14)),
            w(d(2026, 11, 15), d(2026, 11, 29)),
            w(d(2026, 11, 30), d(2026, 12, 14)),
        ];
        let movements = [
            out(d(2026, 11, 5), 100_000),
            pay(d(2026, 11, 15), 200_000),
            out(d(2026, 11, 20), 500_000),
            pay(d(2026, 11, 30), 200_000),
        ];
        let f = forecast(Money(600_000), today, &windows, &movements).expect("windows");
        let lefts: Vec<_> = f.rows.iter().map(|r| r.left.0).collect();
        assert_eq!(lefts, [500_000, 200_000, 400_000]);
        assert_eq!(f.safe_to_spend, Money(200_000));
        assert_eq!(
            (f.held_back, f.held_for),
            (Money(300_000), Some(windows[1]))
        );
    }

    #[test]
    fn held_back_goes_to_the_first_lowest_window_on_a_tie() {
        let today = d(2026, 11, 1);
        let windows = [
            w(today, d(2026, 11, 14)),
            w(d(2026, 11, 15), d(2026, 11, 29)),
            w(d(2026, 11, 30), d(2026, 12, 14)),
        ];
        let movements = [
            pay(d(2026, 11, 15), 100_000),
            out(d(2026, 11, 20), 400_000),
            pay(d(2026, 11, 30), 400_000),
            out(d(2026, 12, 5), 400_000),
        ];
        let f = forecast(Money(500_000), today, &windows, &movements).expect("windows");
        assert_eq!(f.safe_to_spend, Money(200_000));
        assert_eq!(f.held_for, Some(windows[1]));
    }

    #[test]
    fn nothing_is_held_back_when_the_current_window_is_the_lowest_at_its_end() {
        let today = d(2026, 11, 1);
        let windows = [
            w(today, d(2026, 11, 14)),
            w(d(2026, 11, 15), d(2026, 11, 29)),
        ];
        let movements = [out(d(2026, 11, 10), 100_000), pay(d(2026, 11, 15), 200_000)];
        let f = forecast(Money(300_000), today, &windows, &movements).expect("windows");
        assert_eq!(f.safe_to_spend, Money(200_000));
        assert_eq!((f.held_back, f.held_for), (Money::ZERO, None));
    }

    #[test]
    fn a_short_window_reports_its_first_short_day_and_its_lowest_point() {
        let today = d(2026, 11, 1);
        let windows = [
            w(today, d(2026, 11, 14)),
            w(d(2026, 11, 15), d(2026, 11, 29)),
            w(d(2026, 11, 30), d(2026, 12, 14)),
        ];
        let movements = [
            pay(d(2026, 11, 15), 100_000),
            out(d(2026, 11, 18), 250_000),
            out(d(2026, 11, 22), 100_000),
            pay(d(2026, 11, 25), 400_000),
            out(d(2026, 12, 2), 600_000),
        ];
        let f = forecast(Money(100_000), today, &windows, &movements).expect("windows");
        // Nov 18: −50,000, then Nov 22: −150,000, and pay on Nov 25 brings it back. Dec 2 is lower
        // still, but the first short day is Nov 18, short by its window's lowest point.
        assert_eq!(f.short, Some((d(2026, 11, 18), Money(150_000))));
        assert_eq!(f.safe_to_spend, Money::ZERO);
        assert_eq!(f.per_day, Money::ZERO);
        assert_eq!(
            (f.held_back, f.held_for),
            (Money(100_000), Some(windows[2]))
        );
    }

    #[test]
    fn each_row_has_its_lowest_day_and_first_short_day() {
        let today = d(2026, 11, 1);
        let spans = [
            w(d(2026, 10, 30), d(2026, 11, 14)),
            w(d(2026, 11, 15), d(2026, 11, 29)),
        ];
        let movements = [out(d(2026, 11, 2), 150_000), pay(d(2026, 11, 5), 200_000)];
        let rows = rows(Money(100_000), today, &spans, &movements);
        // Below 0 on Nov 2, back above by its end: still short.
        assert_eq!(
            (rows[0].low, rows[0].short_on, rows[0].left),
            (Money(-50_000), Some(d(2026, 11, 2)), Money(150_000))
        );
        // Nothing due keeps the total it starts with.
        assert_eq!((rows[1].low, rows[1].short_on), (Money(150_000), None));
    }

    #[test]
    fn money_now_below_0_is_short_today() {
        let today = d(2026, 11, 1);
        let windows = [w(today, d(2026, 11, 14))];
        let f = forecast(
            Money(-5_000),
            today,
            &windows,
            &[pay(d(2026, 11, 3), 10_000)],
        )
        .expect("windows");
        assert_eq!(f.short, Some((today, Money(5_000))));
        assert_eq!(f.safe_to_spend, Money::ZERO);
    }

    #[test]
    fn per_day_rounds_down() {
        let today = d(2026, 11, 12);
        let windows = [w(d(2026, 11, 1), d(2026, 11, 14))];
        let f = forecast(Money(1_000), today, &windows, &[]).expect("windows");
        assert_eq!(f.per_day, Money(333), "1,000 over Nov 12–14");
        assert_eq!(f.days_left, 3);
        let f = forecast(Money(1_000), d(2026, 11, 14), &windows, &[]).expect("windows");
        assert_eq!(f.per_day, Money(1_000), "the window's last day");
    }

    #[test]
    fn movements_after_the_horizon_are_left_out() {
        let today = d(2026, 11, 1);
        let windows = [w(today, d(2026, 11, 30))];
        let f = forecast(Money(1_000), today, &windows, &[out(d(2026, 12, 1), 5_000)])
            .expect("windows");
        assert_eq!(
            (f.rows[0].movements.len(), f.safe_to_spend),
            (0, Money(1_000))
        );
    }

    /// The Forecast preview in docs/design/ui-flows.html#forecast, built the way the service will.
    #[test]
    fn the_worked_example_adds_up() {
        let today = d(2026, 10, 26);
        let end = horizon_end(today).expect("in range");
        let salary = monthly(d(2026, 11, 25));
        let paydays = dates(&salary, end);
        let windows = pay_windows(&paydays, today, end);
        let expected = expected_pay(
            &[Money(450_000), Money(420_000), Money(435_000)],
            Money(400_000),
        );
        assert_eq!(expected, Money(420_000));

        let mut movements: Vec<Movement> =
            paydays.iter().map(|&date| pay(date, expected.0)).collect();
        // E-wallet loan: 6 payments of ₱1,000 on the 28th.
        let loan = Schedule {
            max_occurrences: Some(6),
            ..monthly(d(2026, 10, 28))
        };
        movements.extend(dates(&loan, end).into_iter().map(|date| Movement {
            source: Source::Loan(1),
            ..out(date, 100_000)
        }));
        // Rent ₱1,200 on the 1st, and insurance ₱6,500 once a year on Jan 10.
        movements.extend(
            dates(&monthly(d(2026, 11, 1)), end)
                .into_iter()
                .map(|date| out(date, 120_000)),
        );
        let insurance = Schedule {
            frequency: Frequency::Yearly,
            ..monthly(d(2027, 1, 10))
        };
        movements.extend(
            dates(&insurance, end)
                .into_iter()
                .map(|date| out(date, 650_000)),
        );
        // Two one-off future-dated expenses.
        movements.push(out(d(2026, 11, 15), 21_171));
        movements.push(out(d(2026, 12, 20), 41_230));
        // A ₱15.99 subscription on the card on the 18th; the Oct 20 statement closed owing ₱15.99.
        let subscription: Vec<_> = dates(&monthly(d(2026, 11, 18)), end)
            .into_iter()
            .map(|date| (date, Money(1_599)))
            .collect();
        let c = CardState {
            owed_at_close: Money(1_599),
            ..card(1_599)
        };
        let card_bills = card_bills(&c, &subscription, today, end);
        assert!(card_bills.iter().all(|b| b.amount == Money(1_599)));
        assert_eq!(card_bills[2].date, d(2027, 1, 5));
        movements.extend(card_bills);

        let f = forecast(Money(888_075), today, &windows, &movements).expect("windows");
        let table: Vec<_> = f.rows[..4]
            .iter()
            .map(|r| {
                (
                    r.start,
                    r.end,
                    r.start_amount.0,
                    r.money_in.0,
                    r.money_out.0,
                    r.left.0,
                )
            })
            .collect();
        assert_eq!(
            table,
            [
                (
                    d(2026, 10, 26),
                    d(2026, 11, 24),
                    888_075,
                    0,
                    242_770,
                    645_305
                ),
                (
                    d(2026, 11, 25),
                    d(2026, 12, 24),
                    645_305,
                    420_000,
                    262_829,
                    802_476
                ),
                (
                    d(2026, 12, 25),
                    d(2027, 1, 24),
                    802_476,
                    420_000,
                    871_599,
                    350_877
                ),
                (
                    d(2027, 1, 25),
                    d(2027, 2, 24),
                    350_877,
                    420_000,
                    221_599,
                    549_278
                ),
            ]
        );
        let dec_25: Vec<_> = f.rows[2]
            .movements
            .iter()
            .map(|m| (m.date, m.amount.0))
            .collect();
        assert_eq!(
            dec_25,
            [
                (d(2026, 12, 25), 420_000),
                (d(2026, 12, 28), 100_000),
                (d(2027, 1, 1), 120_000),
                (d(2027, 1, 5), 1_599),
                (d(2027, 1, 10), 650_000),
            ]
        );
        assert_eq!(f.safe_to_spend, Money(350_877));
        assert_eq!(f.held_back, Money(294_428));
        assert_eq!(f.held_for, Some(w(d(2026, 12, 25), d(2027, 1, 24))));
        assert_eq!(f.per_day, Money(11_695), "30 days left");
        assert_eq!(f.short, None);
        assert_eq!(
            f.rows.last().map(|r| (r.start, r.end)),
            Some((d(2027, 10, 25), d(2027, 10, 25)))
        );
    }
}
