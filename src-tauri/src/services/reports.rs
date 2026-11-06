use chrono::NaiveDate;
use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::domain::money::Money;
use crate::domain::report::{self, Grain};
use crate::error::AppError;
use crate::repo::transactions::{self as repo, Filter};
use crate::services::invalid;
use crate::services::period::{self, PeriodQuery};
use crate::services::transactions::parse_date;

/// One bar of the income-vs-expense chart: its dates and totals, transfers left out (RPT-1, RPT-6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Bar {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub income: Money,
    pub expense: Money,
    pub net: Money,
}

/// A period's income-vs-expense chart, one bar per `grain` (DATA_MODEL §5.9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Timeseries {
    pub grain: Grain,
    pub bars: Vec<Bar>,
    /// Bars after it have nothing yet, so the net line stops there.
    pub today: NaiveDate,
}

pub fn timeseries(
    conn: &Connection,
    query: &PeriodQuery,
    today: NaiveDate,
) -> Result<Timeseries, AppError> {
    let period = period::resolve(conn, query, today)?;
    let (grain, dates) = report::bars(&period, &period::period_settings(conn)?)
        .ok_or_else(|| invalid("period", "That date is outside the supported range"))?;
    let mut bars: Vec<Bar> = dates
        .into_iter()
        .map(|(start, end)| Bar {
            start,
            end,
            income: Money::ZERO,
            expense: Money::ZERO,
            net: Money::ZERO,
        })
        .collect();
    // Rows dated after today count nowhere (DATA_MODEL §5.2).
    let (start, end) = (
        period.start().to_string(),
        period.end().min(today).to_string(),
    );
    for (date, income, expense) in repo::daily_totals(conn, &Filter::dates(&start, &end))? {
        let date =
            parse_date(&date).ok_or_else(|| database("A transaction has an invalid date"))?;
        // The bars cover the period in order, so the first one ending on or after it holds it.
        let i = bars.partition_point(|bar| bar.end < date);
        let bar = bars
            .get_mut(i)
            .ok_or_else(|| database("A transaction is outside the period"))?;
        bar.income = bar
            .income
            .checked_add(Money(income))
            .ok_or_else(too_large)?;
        bar.expense = bar
            .expense
            .checked_add(Money(expense))
            .ok_or_else(too_large)?;
    }
    for bar in &mut bars {
        bar.net = bar.income.checked_sub(bar.expense).ok_or_else(too_large)?;
    }
    Ok(Timeseries { grain, bars, today })
}

fn database(message: &str) -> AppError {
    AppError::Database {
        message: message.into(),
    }
}

fn too_large() -> AppError {
    database("A report total is too large")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::accounts::{self, AccountFields, AccountKind, NewAccount};
    use crate::services::settings::{self, WeekStart};
    use crate::services::transactions::{self, TransactionFields, TransactionKind};
    use crate::test_support::{db, now};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("a real test date")
    }

    fn id_of(conn: &Connection, table: &str, name: &str) -> i64 {
        conn.query_row(
            &format!("SELECT id FROM {table} WHERE name = ?1"),
            [name],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn card(conn: &mut Connection) -> i64 {
        let fields = AccountFields {
            name: "Card".into(),
            opening_balance: Money(0),
            credit_limit: Some(Money(1_000_000)),
            statement_day: Some(20),
            due_day: Some(5),
        };
        let card = NewAccount {
            kind: AccountKind::CreditCard,
            fields,
        };
        accounts::create(conn, card, now()).unwrap().id
    }

    /// Saves an income or expense in the seeded category `category`, or a transfer to `to`.
    fn add(
        conn: &mut Connection,
        kind: TransactionKind,
        amount: i64,
        date: NaiveDate,
        account: i64,
        category: Option<&str>,
        to: Option<i64>,
    ) {
        let category_id = category.map(|name| id_of(conn, "categories", name));
        let fields = TransactionFields {
            kind,
            amount: Money(amount),
            date: date.to_string(),
            account_id: account,
            to_account_id: to,
            category_id,
            payee: None,
            payment_method_id: None,
            note: None,
        };
        transactions::create(conn, fields, now()).unwrap();
    }

    fn spend(conn: &mut Connection, amount: i64, date: NaiveDate) {
        let cash = id_of(conn, "accounts", "Cash");
        let food = Some("Food & Groceries");
        add(
            conn,
            TransactionKind::Expense,
            amount,
            date,
            cash,
            food,
            None,
        );
    }

    /// Each bar as (start, end, income, expense, net).
    fn figures(series: &Timeseries) -> Vec<(NaiveDate, NaiveDate, i64, i64, i64)> {
        series
            .bars
            .iter()
            .map(|b| (b.start, b.end, b.income.0, b.expense.0, b.net.0))
            .collect()
    }

    const MONTH: PeriodQuery = PeriodQuery::Month { anchor: None };

    #[test]
    fn a_month_has_a_bar_a_day_without_transfers_and_with_refunds_as_income() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let card = card(&mut conn);
        let salary = Some("Salary");
        add(
            &mut conn,
            TransactionKind::Income,
            500_000,
            d(2026, 11, 1),
            cash,
            salary,
            None,
        );
        add(
            &mut conn,
            TransactionKind::Expense,
            20_000,
            d(2026, 11, 2),
            card,
            Some("Dining Out"),
            None,
        );
        add(
            &mut conn,
            TransactionKind::Transfer,
            20_000,
            d(2026, 11, 2),
            cash,
            None,
            Some(card),
        );
        add(
            &mut conn,
            TransactionKind::Income,
            5_000,
            d(2026, 11, 3),
            card,
            Some("Refund"),
            None,
        );
        spend(&mut conn, 300, d(2026, 11, 3));
        spend(&mut conn, 1_000, d(2026, 10, 31));

        let series = timeseries(&conn, &MONTH, d(2026, 11, 3)).unwrap();
        assert_eq!((series.grain, series.today), (Grain::Day, d(2026, 11, 3)));
        let bars = figures(&series);
        assert_eq!(bars.len(), 30);
        assert_eq!(
            bars[0],
            (d(2026, 11, 1), d(2026, 11, 1), 500_000, 0, 500_000)
        );
        assert_eq!(
            bars[1],
            (d(2026, 11, 2), d(2026, 11, 2), 0, 20_000, -20_000)
        );
        assert_eq!(bars[2], (d(2026, 11, 3), d(2026, 11, 3), 5_000, 300, 4_700));
        assert!(bars[3..].iter().all(|b| (b.2, b.3, b.4) == (0, 0, 0)));
        assert_eq!(bars[29].1, d(2026, 11, 30));
    }

    #[test]
    fn a_year_adds_up_each_calendar_month() {
        let mut conn = db();
        spend(&mut conn, 100, d(2026, 1, 31));
        spend(&mut conn, 200, d(2026, 2, 1));
        spend(&mut conn, 400, d(2026, 2, 28));
        spend(&mut conn, 800, d(2025, 12, 31));

        let year = PeriodQuery::Year { anchor: None };
        let series = timeseries(&conn, &year, d(2026, 11, 3)).unwrap();
        assert_eq!(series.grain, Grain::Month);
        let expense: Vec<i64> = series.bars.iter().map(|b| b.expense.0).collect();
        assert_eq!(expense, [100, 600, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn rows_after_today_count_nowhere() {
        let mut conn = db();
        spend(&mut conn, 1_000, d(2026, 11, 2));
        spend(&mut conn, 7_000, d(2026, 11, 3));

        let bars = figures(&timeseries(&conn, &MONTH, d(2026, 11, 2)).unwrap());
        assert_eq!((bars[1].3, bars[2].3), (1_000, 0));
    }

    #[test]
    fn custom_weeks_follow_the_week_start_setting() {
        let mut conn = db();
        let mut s = settings::get(&conn).unwrap().settings;
        s.week_start = WeekStart::Sunday;
        settings::update(&mut conn, s).unwrap();
        // Saturday Oct 3 ends the first Sunday week; Sunday Oct 4 starts the next.
        spend(&mut conn, 100, d(2026, 10, 3));
        spend(&mut conn, 200, d(2026, 10, 4));

        let query = PeriodQuery::Custom {
            start: d(2026, 10, 1),
            end: d(2026, 11, 2),
        };
        let series = timeseries(&conn, &query, d(2026, 11, 3)).unwrap();
        assert_eq!(series.grain, Grain::Week);
        let bars = figures(&series);
        assert_eq!(bars[0], (d(2026, 10, 1), d(2026, 10, 3), 0, 100, -100));
        assert_eq!(bars[1], (d(2026, 10, 4), d(2026, 10, 10), 0, 200, -200));
    }

    #[test]
    fn a_backwards_custom_range_is_a_validation_error() {
        let conn = db();
        let query = PeriodQuery::Custom {
            start: d(2026, 10, 2),
            end: d(2026, 10, 1),
        };
        assert!(matches!(
            timeseries(&conn, &query, d(2026, 11, 3)),
            Err(AppError::Validation { field, .. }) if field == "period"
        ));
    }
}
