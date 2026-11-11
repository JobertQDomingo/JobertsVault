use chrono::NaiveDate;
use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::domain::money::Money;
use crate::error::AppError;
use crate::repo;
use crate::services::accounts::{self, Account};
use crate::services::budgets::{self, BudgetProgress};
use crate::services::period::{self, PeriodQuery, PeriodView};
use crate::services::reports::{self, Comparison, Timeseries};
use crate::services::transactions::{self, Transaction};

/// Everything the dashboard shows, from one call so its numbers agree (ARCHITECTURE §4.5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Dashboard {
    pub period: PeriodView,
    /// The period's income, expense and their difference, transfers excluded (DSH-2).
    pub income: Money,
    pub expense: Money,
    pub net: Money,
    /// The period's income-vs-expense chart under the stat cards (UI_FLOWS §2, RPT-1).
    pub series: Timeseries,
    /// The cards' % change: the period against the previous one by the comparison setting
    /// (DSH-3, SET-7), as `get_comparison` returns it with no mode.
    pub comparison: Comparison,
    /// The last backup is more than 7 days old, or there's none (BAK-7, UI_FLOWS F9).
    pub backup_due: bool,
    /// How many recurring dates wait in the Due list, for the dashboard's banner (UI_FLOWS F10).
    pub recurring_due: usize,
    /// Every account, archived included, with balances and card summaries as of today (DSH-4, DSH-5).
    pub accounts: Vec<Account>,
    pub net_balance: Money,
    /// The 5 newest transactions, whatever the period (DSH-8).
    pub recent: Vec<Transaction>,
    /// Every budget's own current period, whatever the period shown, as `list_budgets` returns
    /// them (DSH-6).
    pub budgets: Vec<BudgetProgress>,
    pub today: NaiveDate,
}

pub fn get(
    conn: &Connection,
    query: &PeriodQuery,
    today: NaiveDate,
) -> Result<Dashboard, AppError> {
    let period = period::get(conn, query, today)?;
    let series = reports::timeseries(conn, query, None, today)?;
    let comparison = reports::comparison(conn, query, None, None, today)?;
    // The cards' totals are the comparison's current side, so each % is of the amount shown.
    let (income, expense, net) = (
        comparison.income.current,
        comparison.expense.current,
        comparison.net.current,
    );
    let accounts::AccountList {
        accounts,
        net_balance,
    } = accounts::list(conn, today)?;
    let recent = repo::transactions::recent(conn, &today.to_string(), 5)?
        .into_iter()
        .map(transactions::view)
        .collect::<Result<_, _>>()?;
    Ok(Dashboard {
        backup_due: super::backup::is_due(conn, today)?,
        recurring_due: repo::recurring::pending(conn)?.len(),
        budgets: budgets::list(conn, today)?,
        period,
        income,
        expense,
        net,
        series,
        comparison,
        accounts,
        net_balance,
        recent,
        today,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::money::Money;
    use crate::domain::period::ComparisonMode;
    use crate::services::accounts::{self, AccountFields, AccountKind, NewAccount};
    use crate::services::categories::CategoryKind;
    use crate::services::period::{self, PeriodQuery};
    use crate::services::settings;
    use crate::services::transactions::{self, TransactionFields, TransactionKind};
    use crate::test_support::{db, now};
    use chrono::NaiveDate;
    use rusqlite::Connection;

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

    fn account(conn: &mut Connection, kind: AccountKind, name: &str) -> i64 {
        let card = kind == AccountKind::CreditCard;
        accounts::create(
            conn,
            NewAccount {
                kind,
                fields: AccountFields {
                    name: name.into(),
                    opening_balance: Money(0),
                    credit_limit: card.then_some(Money(1_000_000)),
                    statement_day: card.then_some(20),
                    due_day: card.then_some(5),
                },
            },
            now(),
        )
        .unwrap()
        .id
    }

    /// Saves a transaction; `category` names a seeded category, `to` makes it a transfer.
    fn add(
        conn: &mut Connection,
        kind: TransactionKind,
        amount: i64,
        date: NaiveDate,
        account: i64,
        category: Option<&str>,
        to: Option<i64>,
    ) -> i64 {
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
        transactions::create(conn, fields, now()).unwrap().id
    }

    fn spend(conn: &mut Connection, amount: i64, date: NaiveDate, account: i64) -> i64 {
        let food = Some("Food & Groceries");
        add(
            conn,
            TransactionKind::Expense,
            amount,
            date,
            account,
            food,
            None,
        )
    }

    const MONTH: PeriodQuery = PeriodQuery::Month { anchor: None };

    #[test]
    fn totals_cover_the_period_without_transfers() {
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank");
        let card = account(&mut conn, AccountKind::CreditCard, "Card");
        let cash = id_of(&conn, "accounts", "Cash");
        let salary = Some("Salary");
        add(
            &mut conn,
            TransactionKind::Income,
            500_000,
            d(2026, 11, 1),
            bank,
            salary,
            None,
        );
        spend(&mut conn, 20_000, d(2026, 11, 2), card);
        add(
            &mut conn,
            TransactionKind::Transfer,
            20_000,
            d(2026, 11, 2),
            bank,
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
        spend(&mut conn, 1_000, d(2026, 10, 31), cash);

        let dash = get(&conn, &MONTH, d(2026, 11, 3)).unwrap();
        assert_eq!(dash.income, Money(505_000));
        assert_eq!(dash.expense, Money(20_000));
        assert_eq!(dash.net, Money(485_000));
        assert_eq!(
            dash.period,
            period::get(&conn, &MONTH, d(2026, 11, 3)).unwrap()
        );
        assert_eq!(
            dash.series,
            reports::timeseries(&conn, &MONTH, None, d(2026, 11, 3)).unwrap()
        );
        // The chart's bars add up to the cards' totals.
        let income: i64 = dash.series.bars.iter().map(|b| b.income.0).sum();
        let expense: i64 = dash.series.bars.iter().map(|b| b.expense.0).sum();
        assert_eq!((Money(income), Money(expense)), (dash.income, dash.expense));
        assert_eq!(dash.today, d(2026, 11, 3));
    }

    #[test]
    fn the_cards_compare_with_the_previous_period_by_the_setting() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let salary = Some("Salary");
        let income = TransactionKind::Income;
        add(&mut conn, income, 4_000, d(2026, 10, 2), cash, salary, None);
        add(
            &mut conn,
            income,
            6_000,
            d(2026, 10, 20),
            cash,
            salary,
            None,
        );
        add(&mut conn, income, 5_000, d(2026, 11, 2), cash, salary, None);
        spend(&mut conn, 1_000, d(2026, 10, 3), cash);
        spend(&mut conn, 3_000, d(2026, 11, 3), cash);

        // The month so far against Oct 1–3 (the default, DSH-3 and RPT-4a).
        let dash = get(&conn, &MONTH, d(2026, 11, 3)).unwrap();
        let c = &dash.comparison;
        assert_eq!(
            *c,
            reports::comparison(&conn, &MONTH, None, None, d(2026, 11, 3)).unwrap()
        );
        assert_eq!(
            (c.compare_start, c.compare_end),
            (d(2026, 10, 1), d(2026, 10, 3))
        );
        // The cards' totals are the comparison's current side.
        assert_eq!(
            (c.income.current, c.expense.current, c.net.current),
            (dash.income, dash.expense, dash.net)
        );
        let changes = (c.income.change, c.expense.change, c.net.change);
        assert_eq!(changes, (Some(250), Some(2000), Some(-333)));

        // `full` in Settings compares with all of October.
        let mut s = settings::get(&conn).unwrap().settings;
        s.comparison_mode = ComparisonMode::Full;
        settings::update(&mut conn, s).unwrap();
        let c = get(&conn, &MONTH, d(2026, 11, 3)).unwrap().comparison;
        assert_eq!(c.compare_end, d(2026, 10, 31));
        assert_eq!(c.income.change, Some(-500));
    }

    #[test]
    fn rows_after_today_count_nowhere() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        spend(&mut conn, 1_000, d(2026, 11, 2), cash);
        let later = spend(&mut conn, 7_000, d(2026, 11, 3), cash);

        let dash = get(&conn, &MONTH, d(2026, 11, 2)).unwrap();
        assert_eq!(dash.expense, Money(1_000));
        assert!(dash.recent.iter().all(|t| t.id != later));
        assert_eq!(dash.net_balance, Money(-1_000));
    }

    #[test]
    fn a_future_dated_transaction_counts_once_its_date_arrives() {
        // TXN-8: saved through the form's own path, dated after today (Nov 3).
        let mut conn = db();
        let bank = account(&mut conn, AccountKind::Bank, "Bank");
        let card = account(&mut conn, AccountKind::CreditCard, "Card");
        let salary = Some("Salary");
        add(
            &mut conn,
            TransactionKind::Income,
            500_000,
            d(2026, 11, 1),
            bank,
            salary,
            None,
        );
        let purchase = spend(&mut conn, 20_000, d(2026, 11, 10), card);
        let pay = add(
            &mut conn,
            TransactionKind::Income,
            100_000,
            d(2026, 12, 1),
            bank,
            salary,
            None,
        );

        let figures = |today: NaiveDate| {
            let dash = get(&conn, &MONTH, today).unwrap();
            let card = dash.accounts.iter().find(|a| a.id == card).unwrap();
            let owed = card.card.as_ref().unwrap();
            let breakdown =
                reports::breakdown(&conn, &MONTH, CategoryKind::Expense, None, today).unwrap();
            let bars: i64 = dash.series.bars.iter().map(|b| b.expense.0).sum();
            (
                // The dashboard's cards, with the comparison behind their % change.
                (
                    dash.income.0,
                    dash.expense.0,
                    dash.comparison.expense.current.0,
                ),
                // Balances, and the card's amount owed and cycle spending.
                (
                    dash.net_balance.0,
                    card.balance.0,
                    owed.amount_owed.0,
                    owed.cycle_spending.0,
                ),
                // Reports: the chart and the category breakdown.
                (bars, breakdown.total.0),
                dash.recent.iter().map(|t| t.id).collect::<Vec<_>>(),
            )
        };

        let (cards, balances, reports, recent) = figures(d(2026, 11, 3));
        assert_eq!(cards, (500_000, 0, 0));
        assert_eq!(balances, (500_000, 0, 0, 0));
        assert_eq!(reports, (0, 0));
        assert!(!recent.contains(&purchase) && !recent.contains(&pay));

        // On its date it counts everywhere at once; December's pay still waits.
        let (cards, balances, reports, recent) = figures(d(2026, 11, 10));
        assert_eq!(cards, (500_000, 20_000, 20_000));
        assert_eq!(balances, (480_000, -20_000, 20_000, 20_000));
        assert_eq!(reports, (20_000, 20_000));
        assert_eq!(recent[0], purchase);
        assert!(!recent.contains(&pay));
    }

    #[test]
    fn recent_is_the_five_newest_whatever_the_period() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let ids: Vec<i64> = [1, 2, 3, 4, 4, 5, 5]
            .into_iter()
            .map(|day| spend(&mut conn, 100, d(2026, 10, day), cash))
            .collect();

        let empty_day = PeriodQuery::Day {
            anchor: Some(d(2026, 9, 1)),
        };
        let dash = get(&conn, &empty_day, d(2026, 11, 3)).unwrap();
        let recent: Vec<i64> = dash.recent.iter().map(|t| t.id).collect();
        // Newest date first; on the same date, the later entry first.
        assert_eq!(recent, [ids[6], ids[5], ids[4], ids[3], ids[2]]);
        assert_eq!(
            (dash.income, dash.expense, dash.net),
            (Money(0), Money(0), Money(0))
        );
    }

    #[test]
    fn accounts_and_net_balance_match_list_accounts() {
        let mut conn = db();
        let card = account(&mut conn, AccountKind::CreditCard, "Card");
        spend(&mut conn, 64_000, d(2026, 11, 2), card);

        let dash = get(&conn, &MONTH, d(2026, 11, 3)).unwrap();
        let list = accounts::list(&conn, d(2026, 11, 3)).unwrap();
        assert_eq!(dash.accounts, list.accounts);
        assert_eq!(dash.net_balance, Money(-64_000));
        let summary = dash
            .accounts
            .iter()
            .find(|a| a.id == card)
            .unwrap()
            .card
            .as_ref();
        assert_eq!(summary.unwrap().amount_owed, Money(64_000));
    }

    #[test]
    fn it_counts_the_recurring_dates_due() {
        use crate::domain::recurrence::Frequency;
        use crate::services::recurring::{self, RecurringMode, RecurringRuleFields};
        let mut conn = db();
        assert_eq!(get(&conn, &MONTH, d(2026, 11, 3)).unwrap().recurring_due, 0);
        let rule = RecurringRuleFields {
            transaction: TransactionFields {
                kind: TransactionKind::Expense,
                amount: Money(3_240),
                date: "2026-10-24".into(),
                account_id: 1,
                to_account_id: None,
                category_id: Some(id_of(&conn, "categories", "Utilities")),
                payee: None,
                payment_method_id: None,
                note: None,
            },
            frequency: Frequency::Weekly,
            interval: 1,
            end_date: None,
            max_occurrences: None,
            mode: RecurringMode::Confirm,
        };
        // now() is Nov 3, 2026: Oct 24 and 31 are due.
        recurring::create(&mut conn, rule, now()).unwrap();
        assert_eq!(get(&conn, &MONTH, d(2026, 11, 3)).unwrap().recurring_due, 2);
    }

    #[test]
    fn budgets_show_their_current_period_whatever_the_period_shown() {
        use crate::services::budgets::{BudgetFields, BudgetPeriod};
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let food = id_of(&conn, "categories", "Food & Groceries");
        let limit = BudgetFields {
            category_id: Some(food),
            period: BudgetPeriod::Monthly,
            amount: Money(60_000),
        };
        budgets::create(&mut conn, limit, now()).unwrap();
        spend(&mut conn, 52_040, d(2026, 11, 2), cash);
        let last_year = PeriodQuery::Year {
            anchor: Some(d(2025, 6, 1)),
        };
        let today = d(2026, 11, 3);
        let dash = get(&conn, &last_year, today).unwrap();
        assert_eq!(dash.budgets, budgets::list(&conn, today).unwrap());
        assert_eq!(
            (dash.budgets[0].start, dash.budgets[0].spent),
            (d(2026, 11, 1), Money(52_040))
        );
    }
}
