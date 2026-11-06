use chrono::NaiveDate;
use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::domain::money::Money;
use crate::error::AppError;
use crate::repo;
use crate::services::accounts::{self, Account};
use crate::services::period::{self, PeriodQuery, PeriodView};
use crate::services::reports::{self, Timeseries};
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
    /// Every account, archived included, with balances and card summaries as of today (DSH-4, DSH-5).
    pub accounts: Vec<Account>,
    pub net_balance: Money,
    /// The 5 newest transactions, whatever the period (DSH-8).
    pub recent: Vec<Transaction>,
    pub today: NaiveDate,
}

pub fn get(
    conn: &Connection,
    query: &PeriodQuery,
    today: NaiveDate,
) -> Result<Dashboard, AppError> {
    let period = period::get(conn, query, today)?;
    // Rows dated after today count nowhere (DATA_MODEL §5.2).
    let (start, end) = (period.start.to_string(), period.end.min(today).to_string());
    let (_, income, expense) =
        repo::transactions::totals(conn, &repo::transactions::Filter::dates(&start, &end))?;
    let (income, expense) = (Money(income), Money(expense));
    let net = income
        .checked_sub(expense)
        .ok_or_else(|| AppError::Database {
            message: "The net cashflow is too large".into(),
        })?;
    let series = reports::timeseries(conn, query, today)?;
    let accounts::AccountList {
        accounts,
        net_balance,
    } = accounts::list(conn, today)?;
    let recent = repo::transactions::recent(conn, &today.to_string(), 5)?
        .into_iter()
        .map(transactions::view)
        .collect::<Result<_, _>>()?;
    Ok(Dashboard {
        period,
        income,
        expense,
        net,
        series,
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
    use crate::services::accounts::{self, AccountFields, AccountKind, NewAccount};
    use crate::services::period::{self, PeriodQuery};
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
            reports::timeseries(&conn, &MONTH, d(2026, 11, 3)).unwrap()
        );
        assert_eq!(dash.today, d(2026, 11, 3));
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
}
