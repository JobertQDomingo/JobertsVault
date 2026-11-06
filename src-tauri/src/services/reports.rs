use std::collections::BTreeMap;

use chrono::NaiveDate;
use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::domain::money::Money;
use crate::domain::period::{ComparisonMode, Period, comparison_window};
use crate::domain::report::{self, Grain};
use crate::error::AppError;
use crate::repo::transactions::{self as repo, Filter};
use crate::services::categories::{self, CategoryKind};
use crate::services::invalid;
use crate::services::period::{self, PeriodQuery};
use crate::services::settings;
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
    let (start, end) = counted(&period, today);
    let (start, end) = (start.to_string(), end.to_string());
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

/// A category's part of the breakdown (RPT-2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CategoryShare {
    #[ts(type = "number")]
    pub category_id: i64,
    pub amount: Money,
    /// Tenths of a percent of the total: 123 is 12.3%.
    #[ts(type = "number")]
    pub share: i64,
}

/// One slice of the breakdown's donut: a category, or Other (no category) for the rest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Slice {
    #[ts(type = "number | null")]
    pub category_id: Option<i64>,
    pub amount: Money,
    /// Tenths of a percent of the total.
    #[ts(type = "number")]
    pub share: i64,
}

/// A period's expenses or income by category (RPT-2), transfers left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Breakdown {
    /// Which kind this is, so a screen still showing it while the other kind loads labels it right.
    pub kind: CategoryKind,
    pub total: Money,
    /// Every category with something in the period, largest first.
    pub categories: Vec<CategoryShare>,
    /// The donut: the 8 largest categories, then Other with the rest (DESIGN_SYSTEM §3.4).
    pub slices: Vec<Slice>,
}

pub fn breakdown(
    conn: &Connection,
    query: &PeriodQuery,
    kind: CategoryKind,
    today: NaiveDate,
) -> Result<Breakdown, AppError> {
    let (start, end) = counted(&period::resolve(conn, query, today)?, today);
    let ranked: Vec<(i64, Money)> = category_totals(conn, start, end, kind)?
        .into_iter()
        .map(|(id, amount)| (id, Money(amount)))
        .collect();
    let total = ranked
        .iter()
        .try_fold(Money::ZERO, |sum, &(_, amount)| sum.checked_add(amount))
        .ok_or_else(too_large)?;
    let slices = report::slices(&ranked).ok_or_else(too_large)?;
    Ok(Breakdown {
        kind,
        total,
        categories: ranked
            .into_iter()
            .map(|(category_id, amount)| CategoryShare {
                category_id,
                amount,
                share: report::share(amount, total),
            })
            .collect(),
        slices: slices
            .into_iter()
            .map(|(category_id, amount)| Slice {
                category_id,
                amount,
                share: report::share(amount, total),
            })
            .collect(),
    })
}

/// One row of the comparison table: this period, the comparison and the difference (RPT-4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Line {
    pub current: Money,
    pub compare: Money,
    /// `current − compare`.
    pub difference: Money,
    /// Tenths of a percent of the comparison; none when that's 0, shown as "—" (DATA_MODEL §5.4).
    #[ts(type = "number | null")]
    pub change: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CategoryLine {
    #[ts(type = "number")]
    pub category_id: i64,
    #[serde(flatten)]
    #[ts(flatten)]
    pub line: Line,
}

/// A period next to the one before it (RPT-4), compared by the rule in RPT-4a.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Comparison {
    /// Whether today is in the period. One that's over is always compared in full (RPT-4a).
    pub in_progress: bool,
    /// The mode used: `full` for a period that's over, whatever was asked.
    pub mode: ComparisonMode,
    /// The dates counted this period: up to today while it's in progress.
    pub current_start: NaiveDate,
    pub current_end: NaiveDate,
    /// The dates it's compared with (DATA_MODEL §5.4).
    pub compare_start: NaiveDate,
    pub compare_end: NaiveDate,
    pub income: Line,
    pub expense: Line,
    pub net: Line,
    /// Every category with an amount on either side, largest this period first.
    pub income_categories: Vec<CategoryLine>,
    pub expense_categories: Vec<CategoryLine>,
    pub today: NaiveDate,
}

/// `mode` none means the comparison setting (SET-7).
pub fn comparison(
    conn: &Connection,
    query: &PeriodQuery,
    mode: Option<ComparisonMode>,
    today: NaiveDate,
) -> Result<Comparison, AppError> {
    let period = period::resolve(conn, query, today)?;
    let in_progress = (period.start()..=period.end()).contains(&today);
    let mode = match mode {
        _ if !in_progress => ComparisonMode::Full,
        Some(mode) => mode,
        None => settings::stored(conn)?.comparison_mode,
    };
    let window = comparison_window(&period, &period::period_settings(conn)?, today, mode)
        .ok_or_else(|| invalid("period", "That date is outside the supported range"))?;
    let (current_start, current_end) = counted(&period, today);
    let (compare_start, compare_end) = counted(&window, today);
    let both = |kind| -> Result<Vec<CategoryLine>, AppError> {
        lines(
            category_totals(conn, current_start, current_end, kind)?,
            category_totals(conn, compare_start, compare_end, kind)?,
        )
    };
    let (income_categories, expense_categories) =
        (both(CategoryKind::Income)?, both(CategoryKind::Expense)?);
    let (income, expense) = (sum(&income_categories)?, sum(&expense_categories)?);
    let net = line(
        income
            .current
            .checked_sub(expense.current)
            .ok_or_else(too_large)?,
        income
            .compare
            .checked_sub(expense.compare)
            .ok_or_else(too_large)?,
    )?;
    Ok(Comparison {
        in_progress,
        mode,
        current_start,
        current_end,
        compare_start,
        compare_end,
        income,
        expense,
        net,
        income_categories,
        expense_categories,
        today,
    })
}

fn line(current: Money, compare: Money) -> Result<Line, AppError> {
    Ok(Line {
        current,
        compare,
        difference: current.checked_sub(compare).ok_or_else(too_large)?,
        change: report::percent_change(current, compare),
    })
}

/// Each category on either side, largest this period first, then largest before, then by id.
fn lines(
    current: Vec<(i64, i64)>,
    compare: Vec<(i64, i64)>,
) -> Result<Vec<CategoryLine>, AppError> {
    let mut both: BTreeMap<i64, (Money, Money)> = BTreeMap::new();
    for (id, amount) in current {
        both.entry(id).or_insert((Money::ZERO, Money::ZERO)).0 = Money(amount);
    }
    for (id, amount) in compare {
        both.entry(id).or_insert((Money::ZERO, Money::ZERO)).1 = Money(amount);
    }
    let mut lines = both
        .into_iter()
        .map(|(category_id, (current, compare))| {
            Ok(CategoryLine {
                category_id,
                line: line(current, compare)?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    lines.sort_by(|a, b| {
        (b.line.current, b.line.compare, a.category_id).cmp(&(
            a.line.current,
            a.line.compare,
            b.category_id,
        ))
    });
    Ok(lines)
}

/// The categories' lines added up.
fn sum(lines: &[CategoryLine]) -> Result<Line, AppError> {
    let (mut current, mut compare) = (Money::ZERO, Money::ZERO);
    for l in lines {
        current = current.checked_add(l.line.current).ok_or_else(too_large)?;
        compare = compare.checked_add(l.line.compare).ok_or_else(too_large)?;
    }
    line(current, compare)
}

/// Each category's total of `kind` from `start` to `end`, largest first.
fn category_totals(
    conn: &Connection,
    start: NaiveDate,
    end: NaiveDate,
    kind: CategoryKind,
) -> Result<Vec<(i64, i64)>, AppError> {
    let (start, end) = (start.to_string(), end.to_string());
    let filter = Filter {
        kind: Some(categories::kind_text(kind)),
        ..Filter::dates(&start, &end)
    };
    Ok(repo::category_totals(conn, &filter)?)
}

/// The dates of `period` that count: rows dated after today count nowhere (DATA_MODEL §5.2).
fn counted(period: &Period, today: NaiveDate) -> (NaiveDate, NaiveDate) {
    (period.start(), period.end().min(today))
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

    /// Each category's (name, amount, share) in the breakdown.
    fn parts(conn: &Connection, b: &Breakdown) -> Vec<(String, i64, i64)> {
        b.categories
            .iter()
            .map(|c| {
                let name = conn
                    .query_row(
                        "SELECT name FROM categories WHERE id = ?1",
                        [c.category_id],
                        |r| r.get(0),
                    )
                    .unwrap();
                (name, c.amount.0, c.share)
            })
            .collect()
    }

    #[test]
    fn the_breakdown_ranks_expense_categories_without_transfers_or_refunds() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let card = card(&mut conn);
        let day = d(2026, 11, 2);
        spend(&mut conn, 3_000, day);
        spend(&mut conn, 3_000, d(2026, 11, 3));
        add(
            &mut conn,
            TransactionKind::Expense,
            4_000,
            day,
            card,
            Some("Dining Out"),
            None,
        );
        add(
            &mut conn,
            TransactionKind::Expense,
            2_000,
            day,
            cash,
            Some("Transportation"),
            None,
        );
        add(
            &mut conn,
            TransactionKind::Transfer,
            9_000,
            day,
            cash,
            None,
            Some(card),
        );
        add(
            &mut conn,
            TransactionKind::Income,
            1_000,
            day,
            card,
            Some("Refund"),
            None,
        );
        spend(&mut conn, 500, d(2026, 10, 31));

        let b = breakdown(&conn, &MONTH, CategoryKind::Expense, d(2026, 11, 3)).unwrap();
        assert_eq!(b.total, Money(12_000));
        assert_eq!(
            parts(&conn, &b),
            [
                ("Food & Groceries".to_string(), 6_000, 500),
                ("Dining Out".to_string(), 4_000, 333),
                ("Transportation".to_string(), 2_000, 167),
            ]
        );
        assert_eq!(b.slices.len(), 3);
        assert_eq!(b.slices[0].category_id, Some(b.categories[0].category_id));
    }

    #[test]
    fn the_income_breakdown_counts_refunds() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let day = d(2026, 11, 2);
        add(
            &mut conn,
            TransactionKind::Income,
            9_000,
            day,
            cash,
            Some("Salary"),
            None,
        );
        add(
            &mut conn,
            TransactionKind::Income,
            1_000,
            day,
            cash,
            Some("Refund"),
            None,
        );
        spend(&mut conn, 5_000, day);

        let b = breakdown(&conn, &MONTH, CategoryKind::Income, d(2026, 11, 3)).unwrap();
        assert_eq!((b.kind, b.total), (CategoryKind::Income, Money(10_000)));
        assert_eq!(
            parts(&conn, &b),
            [
                ("Salary".to_string(), 9_000, 900),
                ("Refund".to_string(), 1_000, 100)
            ]
        );
    }

    #[test]
    fn past_eight_categories_the_donut_has_an_other_slice() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let names = [
            "Food & Groceries",
            "Dining Out",
            "Transportation",
            "Rent/Housing",
            "Utilities",
            "Phone",
            "Shopping",
            "Health & Medical",
            "Insurance",
            "Entertainment",
        ];
        for (i, name) in names.iter().enumerate() {
            let amount = 1_000 - 50 * i as i64;
            add(
                &mut conn,
                TransactionKind::Expense,
                amount,
                d(2026, 11, 2),
                cash,
                Some(name),
                None,
            );
        }

        let b = breakdown(&conn, &MONTH, CategoryKind::Expense, d(2026, 11, 3)).unwrap();
        assert_eq!(b.categories.len(), 10);
        assert_eq!(b.slices.len(), 9);
        // Insurance (600) and Entertainment (550).
        let other = &b.slices[8];
        assert_eq!((other.category_id, other.amount), (None, Money(1_150)));
        assert_eq!(other.share, report::share(Money(1_150), b.total));
    }

    #[test]
    fn an_empty_breakdown_has_nothing_and_a_total_of_0() {
        let conn = db();
        let b = breakdown(&conn, &MONTH, CategoryKind::Expense, d(2026, 11, 3)).unwrap();
        assert_eq!(
            (b.total, b.categories.len(), b.slices.len()),
            (Money(0), 0, 0)
        );
    }

    #[test]
    fn rows_after_today_are_not_in_the_breakdown() {
        let mut conn = db();
        spend(&mut conn, 1_000, d(2026, 11, 2));
        spend(&mut conn, 7_000, d(2026, 11, 3));
        let b = breakdown(&conn, &MONTH, CategoryKind::Expense, d(2026, 11, 2)).unwrap();
        assert_eq!(b.total, Money(1_000));
    }

    /// (category, current, compare, difference, change) for each category line.
    fn rows(
        conn: &Connection,
        lines: &[CategoryLine],
    ) -> Vec<(String, i64, i64, i64, Option<i64>)> {
        lines
            .iter()
            .map(|c| {
                let name: String = conn
                    .query_row(
                        "SELECT name FROM categories WHERE id = ?1",
                        [c.category_id],
                        |r| r.get(0),
                    )
                    .unwrap();
                let l = &c.line;
                (name, l.current.0, l.compare.0, l.difference.0, l.change)
            })
            .collect()
    }

    fn totals(l: &Line) -> (i64, i64, i64, Option<i64>) {
        (l.current.0, l.compare.0, l.difference.0, l.change)
    }

    /// November 1–3 so far, and October: some on the 1st–3rd, some later.
    fn november_and_october(conn: &mut Connection) {
        let cash = id_of(conn, "accounts", "Cash");
        let card = card(conn);
        let salary = Some("Salary");
        add(
            conn,
            TransactionKind::Income,
            1_000,
            d(2026, 11, 2),
            cash,
            salary,
            None,
        );
        spend(conn, 300, d(2026, 11, 3));
        add(
            conn,
            TransactionKind::Expense,
            50,
            d(2026, 11, 2),
            cash,
            Some("Transportation"),
            None,
        );
        add(
            conn,
            TransactionKind::Transfer,
            9_000,
            d(2026, 11, 2),
            cash,
            None,
            Some(card),
        );
        add(
            conn,
            TransactionKind::Income,
            800,
            d(2026, 10, 2),
            cash,
            salary,
            None,
        );
        spend(conn, 200, d(2026, 10, 2));
        add(
            conn,
            TransactionKind::Expense,
            100,
            d(2026, 10, 3),
            card,
            Some("Dining Out"),
            None,
        );
        spend(conn, 500, d(2026, 10, 20));
    }

    #[test]
    fn a_month_in_progress_is_compared_with_the_same_days_last_month() {
        let mut conn = db();
        november_and_october(&mut conn);
        let c = comparison(&conn, &MONTH, Some(ComparisonMode::ToDate), d(2026, 11, 3)).unwrap();
        assert_eq!((c.in_progress, c.mode), (true, ComparisonMode::ToDate));
        assert_eq!(
            (
                c.current_start,
                c.current_end,
                c.compare_start,
                c.compare_end
            ),
            (
                d(2026, 11, 1),
                d(2026, 11, 3),
                d(2026, 10, 1),
                d(2026, 10, 3)
            )
        );
        assert_eq!(totals(&c.income), (1_000, 800, 200, Some(250)));
        assert_eq!(totals(&c.expense), (350, 300, 50, Some(167)));
        assert_eq!(totals(&c.net), (650, 500, 150, Some(300)));
        assert_eq!(
            rows(&conn, &c.expense_categories),
            [
                ("Food & Groceries".to_string(), 300, 200, 100, Some(500)),
                // Nothing to compare with: "—".
                ("Transportation".to_string(), 50, 0, 50, None),
                ("Dining Out".to_string(), 0, 100, -100, Some(-1000)),
            ]
        );
        assert_eq!(
            rows(&conn, &c.income_categories),
            [("Salary".to_string(), 1_000, 800, 200, Some(250))]
        );
    }

    #[test]
    fn full_compares_the_whole_previous_month() {
        let mut conn = db();
        november_and_october(&mut conn);
        let c = comparison(&conn, &MONTH, Some(ComparisonMode::Full), d(2026, 11, 3)).unwrap();
        assert_eq!(
            (c.compare_start, c.compare_end),
            (d(2026, 10, 1), d(2026, 10, 31))
        );
        assert_eq!(totals(&c.expense), (350, 800, -450, Some(-563)));
    }

    #[test]
    fn without_a_mode_the_setting_decides() {
        let mut conn = db();
        let mut s = settings::get(&conn).unwrap().settings;
        s.comparison_mode = ComparisonMode::Full;
        settings::update(&mut conn, s).unwrap();
        let c = comparison(&conn, &MONTH, None, d(2026, 11, 3)).unwrap();
        assert_eq!(c.compare_end, d(2026, 10, 31));
        let default = comparison(&db(), &MONTH, None, d(2026, 11, 3)).unwrap();
        assert_eq!(default.compare_end, d(2026, 10, 3));
    }

    #[test]
    fn a_month_that_is_over_is_compared_in_full_whatever_the_mode() {
        let mut conn = db();
        november_and_october(&mut conn);
        let october = PeriodQuery::Month {
            anchor: Some(d(2026, 10, 1)),
        };
        let c = comparison(
            &conn,
            &october,
            Some(ComparisonMode::ToDate),
            d(2026, 11, 3),
        )
        .unwrap();
        assert_eq!((c.in_progress, c.mode), (false, ComparisonMode::Full));
        assert_eq!(
            (
                c.current_start,
                c.current_end,
                c.compare_start,
                c.compare_end
            ),
            (
                d(2026, 10, 1),
                d(2026, 10, 31),
                d(2026, 9, 1),
                d(2026, 9, 30)
            )
        );
        // Nothing in September: every change is "—".
        assert_eq!(totals(&c.expense), (800, 0, 800, None));
        assert_eq!(c.net.change, None);
    }

    #[test]
    fn a_comparison_of_nothing_is_all_zero_with_no_change() {
        let c = comparison(&db(), &MONTH, None, d(2026, 11, 3)).unwrap();
        assert_eq!(totals(&c.net), (0, 0, 0, None));
        assert!(c.income_categories.is_empty() && c.expense_categories.is_empty());
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
