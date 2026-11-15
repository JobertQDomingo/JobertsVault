//! Safe to spend and the 12-month forecast (SAFE-1 to SAFE-7, DATA_MODEL §5.12–5.15): gathers the
//! movements from the database and runs `domain::forecast` on them.

use std::collections::{HashMap, HashSet};

use chrono::{Days, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::forecast::{
    self as f, CardState, Direction, Effect, Flow, Movement, PayWindow, Place, Source,
};
use crate::domain::money::Money;
use crate::domain::period::{Period, PeriodKind};
use crate::error::AppError;
use crate::repo;
use crate::services::period::{self, out_of_range};
use crate::services::{recurring, transactions};

/// The forecast's rows: one per pay window, or one per app month (SAFE-7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ForecastGrain {
    Window,
    Month,
}

/// A span of days, both ends included.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Span {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

/// The first day the running total goes below 0, and by how much its window runs short.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Short {
    pub amount: Money,
    pub date: NaiveDate,
}

/// A payday rule's date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PaydayDate {
    #[ts(type = "number")]
    pub rule_id: i64,
    pub date: NaiveDate,
}

/// What a movement comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum MovementKind {
    Loan,
    Card,
    Recurring,
    Transaction,
    Payday,
}

/// Money due in or out of the counted accounts on a date, today or later (§5.15).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ForecastMovement {
    pub date: NaiveDate,
    pub kind: MovementKind,
    /// The loan payment's, the card's (its account), the rule's or the transaction's id.
    #[ts(type = "number")]
    pub id: i64,
    pub amount: Money,
    /// In, or else out.
    pub money_in: bool,
}

/// A payslip already recorded: it's in `moneyNow`, so it's listed, never added again (§5.15).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Received {
    /// Its gross row's id.
    #[ts(type = "number")]
    pub payslip_id: i64,
    pub date: NaiveDate,
    pub net: Money,
}

/// One pay window, or one month, of the forecast.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ForecastRow {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub start_amount: Money,
    #[serde(rename = "in")]
    #[ts(rename = "in")]
    pub money_in: Money,
    #[serde(rename = "out")]
    #[ts(rename = "out")]
    pub money_out: Money,
    /// What would be left at its end with nothing else spent.
    pub left: Money,
    pub received: Vec<Received>,
    /// Soonest first.
    pub movements: Vec<ForecastMovement>,
}

/// `get_forecast` (ARCHITECTURE §4.5). With no payday rule, everything but `today` and
/// `moneyNow` is none and `rows` is empty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Forecast {
    pub today: NaiveDate,
    /// Σ the counted accounts' balances today (§5.15).
    pub money_now: Money,
    pub safe_to_spend: Option<Money>,
    pub per_day: Option<Money>,
    /// The current pay window's last day.
    pub window_end: Option<NaiveDate>,
    /// The current window's days from today, today included.
    #[ts(type = "number | null")]
    pub days_left: Option<i64>,
    pub held_back: Option<Money>,
    /// The window held back for, when something is.
    pub held_for: Option<Span>,
    pub short: Option<Short>,
    /// The payday that starts the next window.
    pub next_payday: Option<PaydayDate>,
    /// The soonest payday that can be recorded early now (PAY-4).
    pub early_payday: Option<PaydayDate>,
    pub rows: Vec<ForecastRow>,
}

pub fn get(
    conn: &Connection,
    grain: ForecastGrain,
    today: NaiveDate,
) -> Result<Forecast, AppError> {
    let horizon = f::horizon_end(today).ok_or_else(out_of_range)?;
    let accounts = repo::accounts::all(conn, today)?;
    let goal_accounts: HashSet<i64> = repo::goals::all(conn)?
        .into_iter()
        .filter(|g| !g.archived)
        .map(|g| g.account_id)
        .collect();
    let places: HashMap<i64, Place> = accounts
        .iter()
        .map(|a| {
            let place = if a.kind == "credit_card" {
                Place::Card
            } else if !a.archived && !goal_accounts.contains(&a.id) {
                Place::Counted
            } else {
                Place::Other
            };
            (a.id, place)
        })
        .collect();
    let place = |id: i64| places.get(&id).copied().unwrap_or(Place::Other);
    let money_now = Money(
        accounts
            .iter()
            .filter(|a| place(a.id) == Place::Counted)
            .fold(0_i64, |sum, a| sum.saturating_add(a.balance)),
    );

    let mut movements = Vec::new();
    let mut charges: HashMap<i64, Vec<(NaiveDate, Money)>> = HashMap::new();
    // A future-dated row or a rule's date: in or out of the counted accounts, or a card's charge.
    let mut count =
        |kind: &str, amount: i64, date: NaiveDate, from: i64, to: Option<i64>, source| {
            let flow = match kind {
                "income" => Flow::Income,
                "expense" => Flow::Expense,
                _ => Flow::Transfer {
                    to: to.map_or(Place::Other, place),
                },
            };
            let mut out = |direction| {
                movements.push(Movement {
                    date,
                    amount: Money(amount),
                    direction,
                    source,
                })
            };
            match f::effect(flow, place(from)) {
                Some(Effect::In) => out(Direction::In),
                Some(Effect::Out) => out(Direction::Out),
                Some(Effect::Charge) => {
                    charges.entry(from).or_default().push((date, Money(amount)))
                }
                Some(Effect::Refund) => charges
                    .entry(from)
                    .or_default()
                    .push((date, Money(-amount))),
                None => {}
            }
        };

    let today_text = today.to_string();
    for t in repo::transactions::after(conn, &today_text)? {
        let date = stored_date(&t.date)?;
        count(
            &t.kind,
            t.amount,
            date,
            t.account_id,
            t.to_account_id,
            Source::Transaction(t.id),
        );
    }

    let nets = repo::transactions::payslip_nets(conn, &today_text)?;
    let pending = repo::recurring::pending(conn)?;
    let rules = repo::recurring::all(conn)?;
    let mut paydays: Vec<(NaiveDate, i64)> = Vec::new();
    // Paydays 1–3 days ahead with no row yet can be recorded early (PAY-4, DATA_MODEL §4.8).
    let early_until = today
        .checked_add_days(Days::new(3))
        .unwrap_or(NaiveDate::MAX);
    let mut early = Vec::new();
    let mut pay = Vec::new();
    for rule in &rules {
        // Waiting in the Due list, even while paused, then the dates still to come (or not yet
        // processed, which count as today).
        let mut dates = pending
            .iter()
            .filter(|o| o.rule_id == rule.id)
            .map(|o| stored_date(&o.date))
            .collect::<Result<Vec<_>, _>>()?;
        let ahead = if rule.paused {
            Vec::new()
        } else {
            recurring::dates_ahead(conn, rule, NaiveDate::MIN, horizon)?
        };
        dates.extend(&ahead);
        if !rule.is_payday {
            for date in dates {
                count(
                    &rule.kind,
                    rule.amount,
                    date,
                    rule.account_id,
                    rule.to_account_id,
                    Source::Recurring(rule.id),
                );
            }
            continue;
        }
        if !rule.paused {
            // Windows use every scheduled date, those already processed or recorded early too.
            for date in repo::recurring::occurrence_dates_from(conn, rule.id, "")? {
                paydays.push((stored_date(&date)?, rule.id));
            }
            paydays.extend(ahead.iter().map(|d| (*d, rule.id)));
            early.extend(
                ahead
                    .iter()
                    .filter(|d| today < **d && **d <= early_until)
                    .map(|d| (*d, rule.id)),
            );
        }
        if place(rule.account_id) == Place::Counted {
            let recent: Vec<Money> = nets
                .iter()
                .filter(|n| Some(n.category_id) == rule.category_id)
                .take(3)
                .map(|n| Money(n.net))
                .collect();
            let expected = f::expected_pay(&recent, Money(rule.amount));
            pay.extend(dates.into_iter().map(|date| Movement {
                date,
                amount: expected,
                direction: Direction::In,
                source: Source::Payday(rule.id),
            }));
        }
    }
    movements.extend(pay);

    let active_loans: HashSet<i64> = repo::loans::all(conn)?
        .into_iter()
        .filter(|l| !l.archived)
        .map(|l| l.id)
        .collect();
    for p in repo::loans::payments(conn)? {
        if active_loans.contains(&p.loan_id) && p.transaction_id.is_none() && !p.paid_before {
            movements.push(Movement {
                date: stored_date(&p.due_date)?,
                amount: Money(p.amount),
                direction: Direction::Out,
                source: Source::Loan(p.id),
            });
        }
    }

    for card in accounts.iter().filter(|a| a.kind == "credit_card") {
        let (Some(statement_day), Some(due_day)) = (card.statement_day, card.due_day) else {
            continue;
        };
        let (_, close) =
            f::first_statement(statement_day, due_day, today).ok_or_else(out_of_range)?;
        let at_close = repo::accounts::find(conn, card.id, close)?.map_or(0, |a| a.balance);
        let state = CardState {
            account_id: card.id,
            statement_day,
            due_day,
            owed_now: Money(card.balance.saturating_neg()),
            owed_at_close: Money(at_close.saturating_neg().max(0)),
            paid_since_close: Money(repo::accounts::paid_into_card(conn, card.id, close, today)?),
        };
        let charged = charges.get(&card.id).map_or(&[][..], Vec::as_slice);
        movements.extend(f::card_bills(&state, charged, today, horizon));
    }

    let mut forecast = Forecast {
        today,
        money_now,
        safe_to_spend: None,
        per_day: None,
        window_end: None,
        days_left: None,
        held_back: None,
        held_for: None,
        short: None,
        next_payday: None,
        early_payday: None,
        rows: Vec::new(),
    };
    // A paused payday rule is still a payday rule: only its dates are left out (§5.12).
    if !rules.iter().any(|r| r.is_payday) {
        return Ok(forecast);
    }
    let boundaries: Vec<NaiveDate> = paydays.iter().map(|(d, _)| *d).collect();
    let windows = f::pay_windows(&boundaries, today, horizon);
    let summary = f::forecast(money_now, today, &windows, &movements).ok_or_else(out_of_range)?;
    let current = windows[0];
    // The Months view lists the pay received since the current month's real start.
    let (rows, received_from) = match grain {
        ForecastGrain::Window => (summary.rows, current.start),
        ForecastGrain::Month => {
            let (spans, month_start) = months(conn, today, horizon)?;
            (f::rows(money_now, today, &spans, &movements), month_start)
        }
    };
    // Only payslips dated up to today are recorded, so only the first row holds any.
    let mut received = Vec::new();
    for n in nets.iter().rev() {
        let date = stored_date(&n.date)?;
        if date >= received_from {
            received.push(Received {
                payslip_id: n.id,
                date,
                net: Money(n.net),
            });
        }
    }
    let mut received = Some(received);
    forecast.rows = rows
        .into_iter()
        .map(|row| ForecastRow {
            received: received.take().unwrap_or_default(),
            movements: row.movements.iter().map(movement).collect(),
            start: row.start,
            end: row.end,
            start_amount: row.start_amount,
            money_in: row.money_in,
            money_out: row.money_out,
            left: row.left,
        })
        .collect();
    let payday = |(date, rule_id): (NaiveDate, i64)| PaydayDate { rule_id, date };
    forecast.next_payday = paydays
        .into_iter()
        .filter(|(d, _)| *d > today && *d <= horizon)
        .min()
        .map(payday);
    forecast.early_payday = early.into_iter().min().map(payday);
    forecast.safe_to_spend = Some(summary.safe_to_spend);
    forecast.per_day = Some(summary.per_day);
    forecast.window_end = Some(current.end);
    forecast.days_left = Some(summary.days_left);
    forecast.held_back = Some(summary.held_back);
    forecast.held_for = summary.held_for.map(|w| Span {
        start: w.start,
        end: w.end,
    });
    forecast.short = summary.short.map(|(date, amount)| Short { amount, date });
    Ok(forecast)
}

/// The app's months from today to the horizon's end, the first starting today and the last
/// ending on the horizon's end (§5.15), with the first month's real start.
fn months(
    conn: &Connection,
    today: NaiveDate,
    horizon: NaiveDate,
) -> Result<(Vec<PayWindow>, NaiveDate), AppError> {
    let settings = period::period_settings(conn)?;
    let month =
        |day| Period::containing(PeriodKind::Month, day, &settings).ok_or_else(out_of_range);
    let first = month(today)?.start();
    let mut spans = Vec::new();
    let mut day = today;
    while day <= horizon {
        let end = month(day)?.end();
        spans.push(PayWindow {
            start: day,
            end: end.min(horizon),
        });
        day = end.succ_opt().ok_or_else(out_of_range)?;
    }
    Ok((spans, first))
}

fn movement(m: &Movement) -> ForecastMovement {
    let (kind, id) = match m.source {
        Source::Loan(id) => (MovementKind::Loan, id),
        Source::Card(id) => (MovementKind::Card, id),
        Source::Recurring(id) => (MovementKind::Recurring, id),
        Source::Transaction(id) => (MovementKind::Transaction, id),
        Source::Payday(id) => (MovementKind::Payday, id),
    };
    ForecastMovement {
        date: m.date,
        kind,
        id,
        amount: m.amount,
        money_in: m.direction == Direction::In,
    }
}

fn stored_date(text: &str) -> Result<NaiveDate, AppError> {
    transactions::real_date(text).ok_or_else(|| AppError::Database {
        message: "A stored date is invalid".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::recurrence::Frequency;
    use crate::services::accounts::{self, AccountFields, AccountKind, NewAccount};
    use crate::services::goals::{self, GoalFields};
    use crate::services::loans::{self, LoanFields, LoanPaymentFields, NewLoan};
    use crate::services::payslips::{self, Deduction, PayslipFields};
    use crate::services::recurring::{RecurringMode, RecurringRuleFields};
    use crate::services::settings;
    use crate::services::transactions::{TransactionFields, TransactionKind};
    use crate::test_support::{db, now};
    use chrono::{DateTime, Local};

    const TODAY: &str = "2026-11-15";

    fn d(text: &str) -> NaiveDate {
        transactions::real_date(text).unwrap()
    }

    fn on(date: &str) -> DateTime<Local> {
        d(date)
            .and_time(now().time())
            .and_local_timezone(Local)
            .unwrap()
    }

    fn id_of(conn: &Connection, table: &str, name: &str) -> i64 {
        conn.query_row(
            &format!("SELECT id FROM {table} WHERE name = ?1"),
            [name],
            |r| r.get(0),
        )
        .unwrap()
    }

    /// A card closes on the 20th and is due on the 5th.
    fn account(conn: &mut Connection, kind: AccountKind, name: &str, opening: i64) -> i64 {
        let card = kind == AccountKind::CreditCard;
        let fields = AccountFields {
            name: name.into(),
            opening_balance: Money(opening),
            credit_limit: card.then_some(Money(5_000_000)),
            statement_day: card.then_some(20),
            due_day: card.then_some(5),
        };
        accounts::create(conn, NewAccount { kind, fields }, now())
            .unwrap()
            .id
    }

    /// A bank account linked to an active goal.
    fn goal_account(conn: &mut Connection, name: &str, opening: i64) -> i64 {
        let id = account(conn, AccountKind::Bank, name, opening);
        let goal = GoalFields {
            name: format!("{name} goal"),
            target_amount: Money(10_000_000),
            target_date: None,
            account_id: id,
        };
        goals::create(conn, goal, now()).unwrap();
        id
    }

    fn fields(
        conn: &Connection,
        kind: TransactionKind,
        amount: i64,
        date: &str,
        account: i64,
        to: Option<i64>,
    ) -> TransactionFields {
        let category = match kind {
            TransactionKind::Income => Some(id_of(conn, "categories", "Salary")),
            TransactionKind::Expense => Some(id_of(conn, "categories", "Food & Groceries")),
            TransactionKind::Transfer => None,
        };
        TransactionFields {
            kind,
            amount: Money(amount),
            date: date.into(),
            account_id: account,
            to_account_id: to,
            category_id: category,
            payee: None,
            payment_method_id: None,
            note: None,
        }
    }

    fn txn(
        conn: &mut Connection,
        kind: TransactionKind,
        amount: i64,
        date: &str,
        account: i64,
        to: Option<i64>,
    ) -> i64 {
        let f = fields(conn, kind, amount, date, account, to);
        transactions::create(conn, f, now()).unwrap().id
    }

    /// A monthly rule from `start`, created (and processed) on `created`.
    fn rule(
        conn: &mut Connection,
        transaction: TransactionFields,
        mode: RecurringMode,
        is_payday: bool,
        created: &str,
    ) -> i64 {
        let f = RecurringRuleFields {
            transaction,
            frequency: Frequency::Monthly,
            interval: 1,
            end_date: None,
            max_occurrences: None,
            mode,
            is_payday,
        };
        recurring::create(conn, f, on(created)).unwrap().id
    }

    /// Salary each month from `start`, a payday with an estimate of `estimate` net.
    fn payday(
        conn: &mut Connection,
        account: i64,
        start: &str,
        estimate: i64,
        created: &str,
    ) -> i64 {
        let t = fields(
            conn,
            TransactionKind::Income,
            estimate,
            start,
            account,
            None,
        );
        rule(conn, t, RecurringMode::Confirm, true, created)
    }

    /// 4,600.00 gross less 295.00 of deductions: 4,305.00 net.
    fn slip(conn: &Connection, account: i64, date: &str) -> PayslipFields {
        PayslipFields {
            date: date.into(),
            account_id: account,
            gross: Money(460_000),
            deductions: vec![
                Deduction {
                    category_id: id_of(conn, "categories", "SSS"),
                    amount: Money(18_000),
                },
                Deduction {
                    category_id: id_of(conn, "categories", "PhilHealth"),
                    amount: Money(11_500),
                },
            ],
            note: None,
        }
    }

    fn get_on(conn: &Connection, grain: ForecastGrain, today: &str) -> Forecast {
        get(conn, grain, d(today)).unwrap()
    }

    fn windows(conn: &Connection) -> Forecast {
        get_on(conn, ForecastGrain::Window, TODAY)
    }

    /// Every movement of `kind`: date, id and signed amount.
    fn moves(forecast: &Forecast, kind: MovementKind) -> Vec<(NaiveDate, i64, i64)> {
        forecast
            .rows
            .iter()
            .flat_map(|r| &r.movements)
            .filter(|m| m.kind == kind)
            .map(|m| {
                (
                    m.date,
                    m.id,
                    if m.money_in { m.amount.0 } else { -m.amount.0 },
                )
            })
            .collect()
    }

    fn pending_id(conn: &Connection, rule: i64, date: &str) -> i64 {
        conn.query_row(
            "SELECT id FROM recurring_occurrences WHERE rule_id = ?1 AND occurrence_date = ?2",
            rusqlite::params![rule, date],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn no_payday_rule_gives_money_now_only() {
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank", 1_000_000);
        let rent = fields(
            &conn,
            TransactionKind::Expense,
            50_000,
            "2026-11-20",
            bank,
            None,
        );
        rule(&mut conn, rent, RecurringMode::Auto, false, TODAY);

        let forecast = windows(&conn);
        assert_eq!(
            forecast,
            Forecast {
                today: d(TODAY),
                money_now: Money(1_000_000),
                safe_to_spend: None,
                per_day: None,
                window_end: None,
                days_left: None,
                held_back: None,
                held_for: None,
                short: None,
                next_payday: None,
                early_payday: None,
                rows: Vec::new(),
            }
        );
    }

    #[test]
    fn a_paused_or_ended_payday_rule_gives_one_window_to_the_horizon() {
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank", 1_000_000);
        let paused = payday(&mut conn, bank, "2026-11-30", 420_000, TODAY);
        recurring::pause(&mut conn, paused, true, on(TODAY)).unwrap();

        let forecast = windows(&conn);
        assert_eq!(forecast.safe_to_spend, Some(Money(1_000_000)));
        assert_eq!(forecast.rows.len(), 1);
        assert_eq!(forecast.window_end, Some(d("2027-11-14")));
        assert_eq!(forecast.next_payday, None);
        assert!(moves(&forecast, MovementKind::Payday).is_empty());
    }

    #[test]
    fn dates_not_yet_processed_count_once_as_today() {
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank", 1_000_000);
        // Created on Nov 14; the app stays open past midnight, so Nov 15 isn't processed yet.
        let salary = payday(&mut conn, bank, TODAY, 420_000, "2026-11-14");
        let water = fields(&conn, TransactionKind::Expense, 5_000, TODAY, bank, None);
        let water = rule(&mut conn, water, RecurringMode::Auto, false, "2026-11-14");

        let before = windows(&conn);
        assert_eq!(
            before.rows[0].start,
            d(TODAY),
            "today's payday starts the window"
        );
        assert_eq!(
            moves(&before, MovementKind::Payday)[0],
            (d(TODAY), salary, 420_000)
        );
        assert_eq!(
            moves(&before, MovementKind::Recurring)[0],
            (d(TODAY), water, -5_000)
        );

        recurring::process_due(&mut conn, on(TODAY)).unwrap();
        let after = windows(&conn);
        assert_eq!(after.safe_to_spend, before.safe_to_spend);
        assert_eq!(after.rows[0].left, before.rows[0].left);
    }

    #[test]
    fn a_payslip_dated_after_today_counts_as_future_rows() {
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank", 0);
        let salary = payday(&mut conn, bank, "2026-11-30", 420_000, "2026-11-28");
        let later = slip(&conn, bank, "2026-11-30");
        let gross = payslips::record(&mut conn, salary, "2026-11-30", later, on("2026-11-28"))
            .unwrap()
            .id;

        let forecast = get_on(&conn, ForecastGrain::Window, "2026-11-28");
        assert_eq!(forecast.money_now, Money::ZERO);
        assert_eq!(moves(&forecast, MovementKind::Payday)[0].0, d("2026-12-30"));
        let rows = moves(&forecast, MovementKind::Transaction);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], (d("2026-11-30"), gross, 460_000));
        assert_eq!(rows.iter().map(|m| m.2).sum::<i64>(), 430_500);
        assert!(forecast.rows.iter().all(|r| r.received.is_empty()));
        // Not a payslip yet: the next date expects the estimate, not 4,305.00.
        assert_eq!(moves(&forecast, MovementKind::Payday)[0].2, 420_000);
    }

    #[test]
    fn money_now_counts_active_cash_bank_and_ewallet_but_not_a_goal_account() {
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank", 500_000);
        account(&mut conn, AccountKind::Ewallet, "E-wallet", 100_000);
        let old = account(&mut conn, AccountKind::Bank, "Old bank", 70_000);
        accounts::archive(&mut conn, old, true, now()).unwrap();
        let card = account(&mut conn, AccountKind::CreditCard, "Card", 0);
        txn(
            &mut conn,
            TransactionKind::Expense,
            20_000,
            "2026-11-10",
            card,
            None,
        );
        goal_account(&mut conn, "Savings", 300_000);
        // A goal that's archived no longer holds its account back.
        let freed = goal_account(&mut conn, "Freed", 40_000);
        let goal = id_of(&conn, "goals", "Freed goal");
        goals::archive(&mut conn, goal, now()).unwrap();
        payday(&mut conn, bank, "2026-11-30", 420_000, TODAY);
        assert_ne!(freed, bank);

        assert_eq!(windows(&conn).money_now, Money(640_000));
    }

    #[test]
    fn an_unpaid_loan_payment_counts_whatever_its_account() {
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank", 300_000);
        let savings = goal_account(&mut conn, "Savings", 0);
        payday(&mut conn, bank, "2026-11-30", 420_000, TODAY);
        let row = |date: &str| LoanPaymentFields {
            id: None,
            due_date: date.into(),
            amount: Money(100_000),
        };
        // Added Nov 1, so Oct 10 was paid before; Nov 10 is overdue now.
        let category_id = id_of(&conn, "categories", "Loan");
        let loan = loans::create(
            &mut conn,
            NewLoan {
                is_new: false,
                fields: LoanFields {
                    name: "E-wallet loan".into(),
                    account_id: savings,
                    category_id,
                    amount_borrowed: None,
                    borrowed_on: None,
                    payments: ["2026-10-10", "2026-11-10", "2026-12-10", "2027-01-10"]
                        .map(row)
                        .into(),
                },
            },
            on("2026-11-01"),
        )
        .unwrap();
        let [_, nov, dec, jan] = [0, 1, 2, 3].map(|i| loan.payments[i].id);
        // Paid with an expense dated Nov 20: it counts once, as that transaction.
        let paid = fields(
            &conn,
            TransactionKind::Expense,
            100_000,
            "2026-11-20",
            bank,
            None,
        );
        let paid = loans::mark_paid(&mut conn, dec, paid, on(TODAY))
            .unwrap()
            .id;

        let forecast = windows(&conn);
        assert_eq!(
            moves(&forecast, MovementKind::Loan),
            [(d(TODAY), nov, -100_000), (d("2027-01-10"), jan, -100_000)],
            "overdue counts as today; paid and paid-before rows don't count"
        );
        assert_eq!(
            moves(&forecast, MovementKind::Transaction),
            [(d("2026-11-20"), paid, -100_000)]
        );
        // 300,000 less the overdue 100,000 today and the paid 100,000 on Nov 20, before the pay.
        assert_eq!(forecast.money_now, Money(300_000));
        assert_eq!(forecast.safe_to_spend, Some(Money(100_000)));
    }

    #[test]
    fn future_dated_transactions_count_by_where_they_go() {
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank", 1_000_000);
        let wallet = account(&mut conn, AccountKind::Ewallet, "E-wallet", 0);
        let savings = goal_account(&mut conn, "Savings", 1_000_000);
        let card = account(&mut conn, AccountKind::CreditCard, "Card", 0);
        payday(&mut conn, bank, "2026-11-30", 420_000, TODAY);
        let date = "2026-11-20";
        use TransactionKind::{Expense, Income, Transfer};
        let spent = txn(&mut conn, Expense, 10_000, date, bank, None);
        let earned = txn(&mut conn, Income, 20_000, date, wallet, None);
        let saved = txn(&mut conn, Transfer, 30_000, date, bank, Some(savings));
        let drawn = txn(&mut conn, Transfer, 40_000, date, savings, Some(bank));
        txn(&mut conn, Transfer, 50_000, date, bank, Some(wallet));
        txn(&mut conn, Transfer, 60_000, date, bank, Some(card));
        txn(&mut conn, Expense, 70_000, date, savings, None);
        // Dated today, so already in the balances.
        txn(&mut conn, Expense, 80_000, TODAY, bank, None);

        let mut got = moves(&windows(&conn), MovementKind::Transaction);
        got.sort_by_key(|m| m.1);
        let on = d(date);
        assert_eq!(
            got,
            [
                (on, spent, -10_000),
                (on, earned, 20_000),
                (on, saved, -30_000),
                (on, drawn, 40_000),
            ]
        );
    }

    #[test]
    fn recurring_dates_count_pending_as_today_and_skip_paused_rules() {
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank", 1_000_000);
        payday(&mut conn, bank, "2026-11-30", 420_000, TODAY);
        let water = fields(
            &conn,
            TransactionKind::Expense,
            5_000,
            "2026-11-10",
            bank,
            None,
        );
        let water = rule(&mut conn, water, RecurringMode::Confirm, false, TODAY);
        let gym = fields(
            &conn,
            TransactionKind::Expense,
            9_000,
            "2026-11-20",
            bank,
            None,
        );
        let gym = rule(&mut conn, gym, RecurringMode::Auto, false, TODAY);
        recurring::pause(&mut conn, gym, true, on(TODAY)).unwrap();

        let got = moves(&windows(&conn), MovementKind::Recurring);
        assert_eq!(got.len(), 13, "Nov 10 pending, then Dec 10 to Nov 10, 2027");
        assert_eq!(got[0], (d(TODAY), water, -5_000), "pending counts as today");
        assert_eq!(got[1].0, d("2026-12-10"));
        assert_eq!(got[12].0, d("2027-11-10"));
        assert!(
            got.iter().all(|m| m.1 == water),
            "a paused rule has no dates"
        );
    }

    #[test]
    fn paydays_add_expected_pay_unless_posted_or_skipped() {
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank", 0);
        let savings = goal_account(&mut conn, "Savings", 0);
        // Sep 15 recorded, Oct 15 skipped, Nov 15 waiting.
        let salary = payday(&mut conn, bank, "2026-09-15", 450_000, TODAY);
        let sep = slip(&conn, bank, "2026-09-15");
        payslips::record(&mut conn, salary, "2026-09-15", sep, on(TODAY)).unwrap();
        let oct = pending_id(&conn, salary, "2026-10-15");
        recurring::skip(&mut conn, oct, on(TODAY)).unwrap();
        // Pay into a goal's account isn't spendable, but its dates still start windows.
        let bonus = payday(&mut conn, savings, "2026-12-05", 100_000, TODAY);

        let forecast = windows(&conn);
        let got = moves(&forecast, MovementKind::Payday);
        assert_eq!(got.len(), 12, "Nov 15 to Oct 15, 2027");
        assert_eq!(
            got[0],
            (d(TODAY), salary, 430_500),
            "lowest of 4,305.00 and 4,500.00"
        );
        assert_eq!(got[1].0, d("2026-12-15"));
        assert!(got.iter().all(|m| m.1 == salary && m.2 == 430_500));
        assert!(forecast.rows.iter().any(|r| r.start == d("2026-12-05")));
        assert_ne!(bonus, salary);
    }

    #[test]
    fn a_payslip_recorded_early_is_not_counted_twice() {
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank", 0);
        let salary = payday(&mut conn, bank, "2026-11-30", 420_000, "2026-11-27");
        let early = slip(&conn, bank, "2026-11-28");
        let gross = payslips::record(&mut conn, salary, "2026-11-30", early, on("2026-11-28"))
            .unwrap()
            .id;

        let forecast = get_on(&conn, ForecastGrain::Window, "2026-11-28");
        assert_eq!(forecast.money_now, Money(430_500));
        let paydays = moves(&forecast, MovementKind::Payday);
        assert_eq!(
            paydays[0].0,
            d("2026-12-30"),
            "Nov 30 is already in money now"
        );
        assert_eq!(
            forecast.window_end,
            Some(d("2026-11-29")),
            "windows keep the scheduled date"
        );
        assert_eq!(forecast.rows[1].start, d("2026-11-30"));
        assert_eq!(
            forecast.rows[0].received,
            [Received {
                payslip_id: gross,
                date: d("2026-11-28"),
                net: Money(430_500),
            }]
        );
        assert_eq!(forecast.early_payday, None, "already recorded");
        assert_eq!(
            forecast.next_payday,
            Some(PaydayDate {
                rule_id: salary,
                date: d("2026-11-30"),
            })
        );
    }

    #[test]
    fn card_bills_come_from_the_statement_and_future_charges() {
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank", 1_000_000);
        let card = account(&mut conn, AccountKind::CreditCard, "Card", 0);
        payday(&mut conn, bank, "2026-11-30", 420_000, TODAY);
        use TransactionKind::{Expense, Transfer};
        // Oct 20's statement: 50,000, 20,000 of it paid since; due Nov 5, so due now.
        txn(&mut conn, Expense, 50_000, "2026-10-10", card, None);
        txn(&mut conn, Transfer, 20_000, "2026-11-01", bank, Some(card));
        // Nov 20's statement, due Dec 5; then a future purchase on Dec 20's, due Jan 5.
        txn(&mut conn, Expense, 30_000, "2026-11-10", card, None);
        txn(&mut conn, Expense, 10_000, "2026-11-25", card, None);

        let forecast = windows(&conn);
        assert_eq!(
            moves(&forecast, MovementKind::Card),
            [
                (d(TODAY), card, -30_000),
                (d("2026-12-05"), card, -30_000),
                (d("2027-01-05"), card, -10_000),
            ]
        );
        assert!(moves(&forecast, MovementKind::Transaction).is_empty());

        // An archived card is still billed for what it owes.
        accounts::archive(&mut conn, card, true, now()).unwrap();
        assert_eq!(moves(&windows(&conn), MovementKind::Card).len(), 3);
    }

    #[test]
    fn received_pay_is_listed_in_its_window() {
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank", 0);
        let salary = payday(&mut conn, bank, TODAY, 420_000, TODAY);
        let today = slip(&conn, bank, TODAY);
        let gross = payslips::record(&mut conn, salary, TODAY, today, on(TODAY))
            .unwrap()
            .id;

        let forecast = windows(&conn);
        assert_eq!(forecast.rows[0].start, d(TODAY));
        assert_eq!(forecast.rows[0].start_amount, Money(430_500));
        assert_eq!(
            forecast.rows[0].received,
            [Received {
                payslip_id: gross,
                date: d(TODAY),
                net: Money(430_500),
            }]
        );
        assert!(forecast.rows[1..].iter().all(|r| r.received.is_empty()));
        assert_eq!(moves(&forecast, MovementKind::Payday)[0].0, d("2026-12-15"));
    }

    #[test]
    fn the_months_grain_starts_today_and_follows_month_start_day() {
        let mut conn = db();
        let mut s = settings::stored(&conn).unwrap();
        s.month_start_day = 10;
        settings::update(&mut conn, s).unwrap();
        let bank = account(&mut conn, AccountKind::Bank, "Bank", 100_000);
        payday(&mut conn, bank, "2026-11-30", 420_000, TODAY);
        let water = fields(
            &conn,
            TransactionKind::Expense,
            5_000,
            "2026-11-25",
            bank,
            None,
        );
        rule(&mut conn, water, RecurringMode::Auto, false, TODAY);

        let months = get_on(&conn, ForecastGrain::Month, TODAY);
        let spans: Vec<_> = months.rows.iter().map(|r| (r.start, r.end)).collect();
        assert_eq!(spans.len(), 13);
        assert_eq!(spans[0], (d(TODAY), d("2026-12-09")));
        assert_eq!(spans[1], (d("2026-12-10"), d("2027-01-09")));
        assert_eq!(spans[12], (d("2027-11-10"), d("2027-11-14")));
        let windows = windows(&conn);
        assert_eq!(
            months.safe_to_spend, windows.safe_to_spend,
            "always from pay windows"
        );
        assert_eq!(months.rows[12].left, windows.rows.last().unwrap().left);
        assert_eq!(months.rows[0].start_amount, Money(100_000));
    }

    #[test]
    fn next_and_early_payday() {
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank", 280_000);
        let salary = payday(&mut conn, bank, "2026-11-30", 420_000, TODAY);
        let on_30th = Some(PaydayDate {
            rule_id: salary,
            date: d("2026-11-30"),
        });

        let soon = get_on(&conn, ForecastGrain::Window, "2026-11-27");
        assert_eq!(soon.early_payday, on_30th);
        assert_eq!(soon.next_payday, on_30th);
        assert_eq!(soon.window_end, Some(d("2026-11-29")));
        assert_eq!(soon.days_left, Some(3));
        assert_eq!(
            soon.per_day,
            Some(Money(93_333)),
            "280,000 over 3 days, rounded down"
        );

        let later = get_on(&conn, ForecastGrain::Window, "2026-11-20");
        assert_eq!(later.early_payday, None, "more than 3 days ahead");
        assert_eq!(later.next_payday, on_30th);
    }
}
