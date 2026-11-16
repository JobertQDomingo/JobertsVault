use chrono::NaiveDate;
use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::domain::card_cycle;
use crate::domain::loan;
use crate::domain::money::Money;
use crate::error::AppError;
use crate::repo;
use crate::services::accounts::{self, Account};
use crate::services::budgets::{self, BudgetProgress};
use crate::services::forecast::{self, Forecast, ForecastGrain};
use crate::services::loans;
use crate::services::period::{self, PeriodQuery, PeriodView};
use crate::services::recurring::{self, UpcomingDate};
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
    /// The 10 newest transactions, whatever the period. The dashboard shows as many as fit, and
    /// at least 5 (DSH-8).
    pub recent: Vec<Transaction>,
    /// Every budget's own current period, whatever the period shown, as `list_budgets` returns
    /// them (DSH-6).
    pub budgets: Vec<BudgetProgress>,
    /// Every date a recurring rule falls on after today and up to a week on, soonest first, as
    /// Coming up shows them (DSH-7).
    pub upcoming: Vec<UpcomingDate>,
    /// Active cards whose payment is due within 5 days with something owed, soonest first, for
    /// their banners (CC-7).
    pub cards_due: Vec<CardDue>,
    /// Active loans whose next unpaid payment is due within 5 days or overdue, soonest first, for
    /// their banners (LOAN-4).
    pub loans_due: Vec<LoanDue>,
    /// What's left across every loan, under the net balance. None once nothing is left (LOAN-11).
    pub loans_left: Option<LoansLeft>,
    /// Safe to spend for today, whatever the period, as `get_forecast` returns it with no rows
    /// (SAFE-6).
    pub safe_to_spend: Forecast,
    pub today: NaiveDate,
}

/// A loan's reminder: its next unpaid payment (LOAN-4, DATA_MODEL §5.11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoanDue {
    #[ts(type = "number")]
    pub loan_id: i64,
    pub name: String,
    /// The account its payments come from.
    #[ts(type = "number")]
    pub account_id: i64,
    #[ts(type = "number")]
    pub payment_id: i64,
    pub amount: Money,
    pub due_date: NaiveDate,
    /// 0 when it's due today, below 0 when it's overdue.
    #[ts(type = "number")]
    pub days: i64,
    /// The loan's other overdue payments.
    #[ts(type = "number")]
    pub more_overdue: usize,
    /// This banner's part of what its account lacks, or none (DATA_MODEL §5.11).
    pub move_first: Option<Money>,
}

/// The dashboard line under the net balance (LOAN-11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoansLeft {
    pub total_left: Money,
    pub debt_free_on: NaiveDate,
    /// Everything left is overdue, so the line says so instead of a date already passed.
    pub overdue: bool,
}

/// A card's due-date reminder (CC-7, DATA_MODEL §5.5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CardDue {
    #[ts(type = "number")]
    pub account_id: i64,
    pub amount_owed: Money,
    /// 0 when the payment is due today.
    #[ts(type = "number")]
    pub days: i64,
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
    // Archived accounts take no new transfers, so their Pay bill couldn't work.
    let mut cards_due: Vec<_> = accounts
        .iter()
        .filter(|a| !a.archived)
        .filter_map(|a| {
            let card = a.card.as_ref()?;
            Some(CardDue {
                account_id: a.id,
                amount_owed: card.amount_owed,
                days: card_cycle::reminder_days(card.due_date, card.amount_owed, today)?,
            })
        })
        .collect();
    cards_due.sort_by_key(|c| c.days);
    let loans::LoanList { loans, summary } = loans::list(conn, today)?;
    let all_overdue = loans
        .iter()
        .flat_map(|l| &l.payments)
        .filter(|p| p.status == loans::LoanPaymentStatus::Unpaid)
        .all(|p| p.overdue);
    let mut loans_due: Vec<LoanDue> = loans
        .into_iter()
        // Defensive: an archived loan is paid off, so it has no next payment anyway.
        .filter(|l| !l.archived)
        .filter_map(|l| {
            let next = l.next?;
            let days = (next.due_date - today).num_days();
            (days <= 5).then(|| LoanDue {
                loan_id: l.id,
                account_id: l.account_id,
                payment_id: next.id,
                amount: next.amount,
                due_date: next.due_date,
                days,
                more_overdue: l
                    .payments
                    .iter()
                    .filter(|p| p.overdue)
                    .count()
                    .saturating_sub(1),
                move_first: None,
                name: l.name,
            })
        })
        .collect();
    loans_due.sort_by_key(|l| (l.due_date, l.loan_id));
    let dues: Vec<_> = loans_due.iter().map(|l| (l.account_id, l.amount)).collect();
    let balance = |id| {
        accounts
            .iter()
            .find(|a| a.id == id)
            .map_or(Money::ZERO, |a| a.balance)
    };
    for (due, share) in loans_due.iter_mut().zip(loan::move_first(&dues, balance)) {
        due.move_first = share;
    }
    // None exactly when no payment is unpaid.
    let loans_left = summary.debt_free_on.map(|debt_free_on| LoansLeft {
        total_left: summary.total_left,
        debt_free_on,
        overdue: all_overdue,
    });
    let recent = repo::transactions::recent(conn, &today.to_string(), 10)?
        .into_iter()
        .map(transactions::view)
        .collect::<Result<_, _>>()?;
    Ok(Dashboard {
        backup_due: super::backup::is_due(conn, today)?,
        recurring_due: repo::recurring::pending(conn)?.len(),
        budgets: budgets::list(conn, today)?,
        upcoming: recurring::upcoming(conn, today)?,
        cards_due,
        loans_due,
        loans_left,
        safe_to_spend: Forecast {
            rows: Vec::new(),
            ..forecast::get(conn, ForecastGrain::Window, today)?
        },
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
    use crate::services::loans::{self, Loan};
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
                    spend_soft: None,
                    spend_mid: None,
                    spend_hard: None,
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
        transactions::create(conn, fields, now()).unwrap().saved.id
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
    fn recent_is_the_ten_newest_whatever_the_period() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let ids: Vec<i64> = [1, 2, 3, 4, 4, 5, 5, 6, 7, 8, 9, 10]
            .into_iter()
            .map(|day| spend(&mut conn, 100, d(2026, 10, day), cash))
            .collect();

        let empty_day = PeriodQuery::Day {
            anchor: Some(d(2026, 9, 1)),
        };
        let dash = get(&conn, &empty_day, d(2026, 11, 3)).unwrap();
        let recent: Vec<i64> = dash.recent.iter().map(|t| t.id).collect();
        // Newest date first; on the same date, the later entry first.
        assert_eq!(
            recent,
            [
                ids[11], ids[10], ids[9], ids[8], ids[7], ids[6], ids[5], ids[4], ids[3], ids[2]
            ]
        );
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
            is_payday: false,
        };
        // now() is Nov 3, 2026: Oct 24 and 31 are due.
        recurring::create(&mut conn, rule, now()).unwrap();
        let dash = get(&conn, &MONTH, d(2026, 11, 3)).unwrap();
        assert_eq!(dash.recurring_due, 2);
        // Nov 7 is coming up; Nov 14 is more than a week away (DSH-7).
        let dates: Vec<_> = dash
            .upcoming
            .iter()
            .map(|u| u.transaction.date.as_str())
            .collect();
        assert_eq!(dates, ["2026-11-07"]);
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

    #[test]
    fn active_cards_due_within_5_days_with_something_owed_get_a_reminder() {
        let mut conn = db();
        // The helper's cards are due on the 5th.
        let soon = account(&mut conn, AccountKind::CreditCard, "Visa");
        account(&mut conn, AccountKind::CreditCard, "Paid off");
        let shelved = account(&mut conn, AccountKind::CreditCard, "Shelved");
        spend(&mut conn, 64_000, d(2026, 10, 25), soon);
        spend(&mut conn, 10_000, d(2026, 10, 25), shelved);
        accounts::archive(&mut conn, shelved, true, now()).unwrap();

        let due = |today| get(&conn, &MONTH, today).unwrap().cards_due;
        assert_eq!(due(d(2026, 10, 30)), [], "6 days away");
        assert_eq!(
            due(d(2026, 11, 2)),
            [CardDue {
                account_id: soon,
                amount_owed: Money(64_000),
                days: 3,
            }]
        );
        assert_eq!(due(d(2026, 11, 5))[0].days, 0, "due today");
    }

    #[test]
    fn cards_due_come_soonest_first() {
        let mut conn = db();
        let later = account(&mut conn, AccountKind::CreditCard, "Due 5th");
        let sooner = account(&mut conn, AccountKind::CreditCard, "Due 3rd");
        accounts::update(
            &mut conn,
            sooner,
            AccountFields {
                name: "Due 3rd".into(),
                opening_balance: Money(0),
                credit_limit: Some(Money(1_000_000)),
                statement_day: Some(20),
                due_day: Some(3),
                spend_soft: None,
                spend_mid: None,
                spend_hard: None,
            },
            now(),
        )
        .unwrap();
        spend(&mut conn, 1_000, d(2026, 10, 25), later);
        spend(&mut conn, 1_000, d(2026, 10, 25), sooner);

        let ids: Vec<_> = get(&conn, &MONTH, d(2026, 11, 1))
            .unwrap()
            .cards_due
            .iter()
            .map(|c| (c.account_id, c.days))
            .collect();
        assert_eq!(ids, [(sooner, 2), (later, 4)]);
    }

    /// A new loan borrowed on Nov 1, paid from `account` into Loan, with these payments.
    fn loan(conn: &mut Connection, name: &str, account: i64, payments: &[(&str, i64)]) -> Loan {
        use crate::services::loans::{LoanFields, LoanPaymentFields, NewLoan};
        let fields = LoanFields {
            name: name.into(),
            account_id: account,
            category_id: id_of(conn, "categories", "Loan"),
            amount_borrowed: None,
            borrowed_on: Some("2026-11-01".into()),
            payments: payments
                .iter()
                .map(|&(date, amount)| LoanPaymentFields {
                    id: None,
                    due_date: date.into(),
                    amount: Money(amount),
                })
                .collect(),
        };
        let new = NewLoan {
            is_new: true,
            fields,
        };
        loans::create(conn, new, now()).unwrap().saved
    }

    fn due_dates(dash: &Dashboard) -> Vec<(i64, NaiveDate, i64)> {
        dash.loans_due
            .iter()
            .map(|l| (l.loan_id, l.due_date, l.days))
            .collect()
    }

    #[test]
    fn a_loan_payment_due_within_5_days_or_overdue_gets_a_banner() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let soon = loan(&mut conn, "Soon", cash, &[("2026-11-25", 100_000)]);
        let later = loan(&mut conn, "Later", cash, &[("2026-11-26", 100_000)]);
        let today = d(2026, 11, 20);
        let dash = get(&conn, &MONTH, today).unwrap();
        assert_eq!(
            due_dates(&dash),
            [(soon.id, d(2026, 11, 25), 5)],
            "6 days out"
        );

        // Overdue days are below 0, and it stays until it's paid.
        let dash = get(&conn, &MONTH, d(2026, 11, 28)).unwrap();
        assert_eq!(
            due_dates(&dash),
            [
                (soon.id, d(2026, 11, 25), -3),
                (later.id, d(2026, 11, 26), -2)
            ]
        );
        let first = &dash.loans_due[0];
        assert_eq!(
            (
                first.payment_id,
                first.amount,
                first.account_id,
                first.name.as_str()
            ),
            (soon.payments[0].id, Money(100_000), cash, "Soon")
        );
    }

    #[test]
    fn a_loan_gets_one_banner_for_its_next_payment_saying_how_many_more_are_overdue() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let dates = [
            ("2026-11-10", 100_000),
            ("2026-11-20", 100_000),
            ("2026-11-30", 100_000),
            ("2026-12-10", 100_000),
        ];
        let l = loan(&mut conn, "E-wallet loan", cash, &dates);
        let banners = |today| get(&conn, &MONTH, today).unwrap().loans_due;
        let one = banners(d(2026, 12, 5));
        assert_eq!(one.len(), 1);
        assert_eq!(
            (one[0].payment_id, one[0].more_overdue),
            (l.payments[0].id, 2)
        );
        // Due today isn't overdue yet, and a payment not yet due has no others overdue.
        assert_eq!(banners(d(2026, 11, 10))[0].more_overdue, 0);
        assert_eq!(banners(d(2026, 11, 7))[0].more_overdue, 0);
    }

    #[test]
    fn archived_loans_get_no_banner() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let l = loan(&mut conn, "Shelved", cash, &[("2026-11-05", 100_000)]);
        // Only a paid-off loan can be archived, so this one is archived behind the service's back.
        conn.execute("UPDATE loans SET archived = 1 WHERE id = ?1", [l.id])
            .unwrap();
        assert_eq!(get(&conn, &MONTH, d(2026, 11, 5)).unwrap().loans_due, []);
    }

    #[test]
    fn loan_banners_come_soonest_first_and_each_says_its_own_part_to_move() {
        let mut conn = db();
        let wallet = account(&mut conn, AccountKind::Ewallet, "E-wallet");
        let salary = Some("Salary");
        add(
            &mut conn,
            TransactionKind::Income,
            20_000,
            d(2026, 11, 2),
            wallet,
            salary,
            None,
        );
        let second = loan(&mut conn, "Phone loan", wallet, &[("2026-11-24", 50_000)]);
        let first = loan(
            &mut conn,
            "E-wallet loan",
            wallet,
            &[("2026-11-22", 100_000)],
        );
        let dash = get(&conn, &MONTH, d(2026, 11, 20)).unwrap();
        let banners: Vec<_> = dash
            .loans_due
            .iter()
            .map(|l| (l.loan_id, l.move_first))
            .collect();
        // ₱200 in the account, ₱1,000 then ₱500 due (DATA_MODEL §5.11).
        assert_eq!(
            banners,
            [
                (first.id, Some(Money(80_000))),
                (second.id, Some(Money(50_000)))
            ]
        );
    }

    #[test]
    fn loans_left_shows_what_is_left_and_the_debt_free_date() {
        use crate::services::loans::mark_paid;
        let mut conn = db();
        assert_eq!(get(&conn, &MONTH, d(2026, 11, 3)).unwrap().loans_left, None);
        let cash = id_of(&conn, "accounts", "Cash");
        let dates = [("2026-12-01", 100_000), ("2027-01-01", 40_000)];
        let l = loan(&mut conn, "E-wallet loan", cash, &dates);
        let left = |conn: &Connection| get(conn, &MONTH, d(2026, 11, 3)).unwrap().loans_left;
        assert_eq!(
            left(&conn),
            Some(LoansLeft {
                total_left: Money(140_000),
                debt_free_on: d(2027, 1, 1),
                overdue: false,
            })
        );
        let loan_category = id_of(&conn, "categories", "Loan");
        for (p, amount) in l.payments.iter().zip([100_000, 40_000]) {
            let fields = TransactionFields {
                kind: TransactionKind::Expense,
                amount: Money(amount),
                date: "2026-11-03".into(),
                account_id: cash,
                to_account_id: None,
                category_id: Some(loan_category),
                payee: None,
                payment_method_id: None,
                note: None,
            };
            mark_paid(&mut conn, p.id, fields, now()).unwrap();
        }
        assert_eq!(left(&conn), None, "hidden once nothing is left");
    }

    #[test]
    fn move_first_counts_only_the_payments_the_banners_show() {
        let mut conn = db();
        let wallet = account(&mut conn, AccountKind::Ewallet, "E-wallet");
        let dates = [
            ("2026-11-05", 100_000),
            ("2026-11-06", 100_000),
            ("2026-11-07", 100_000),
        ];
        loan(&mut conn, "Behind", wallet, &dates);
        loan(&mut conn, "Far off", wallet, &[("2026-11-16", 100_000)]);
        let dash = get(&conn, &MONTH, d(2026, 11, 10)).unwrap();
        let banners: Vec<_> = dash
            .loans_due
            .iter()
            .map(|l| (l.more_overdue, l.move_first))
            .collect();
        // One banner for the earliest; the other overdue ones and the one 6 days out add nothing.
        assert_eq!(banners, [(2, Some(Money(100_000)))]);
    }

    #[test]
    fn overdue_banners_come_before_due_ones_and_a_tie_goes_by_the_order_added() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let a = loan(&mut conn, "A", cash, &[("2026-11-12", 1_000)]);
        let b = loan(&mut conn, "B", cash, &[("2026-11-12", 1_000)]);
        let late = loan(&mut conn, "Late", cash, &[("2026-11-08", 1_000)]);
        let dash = get(&conn, &MONTH, d(2026, 11, 10)).unwrap();
        let ids: Vec<_> = dash.loans_due.iter().map(|l| l.loan_id).collect();
        assert_eq!(ids, [late.id, a.id, b.id]);
    }

    #[test]
    fn loans_left_says_when_everything_left_is_overdue() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        loan(&mut conn, "Behind", cash, &[("2026-11-10", 100_000)]);
        let left = |today| get(&conn, &MONTH, today).unwrap().loans_left.unwrap();
        // Due today isn't overdue yet: the last payment is today.
        assert_eq!(
            left(d(2026, 11, 10)),
            LoansLeft {
                total_left: Money(100_000),
                debt_free_on: d(2026, 11, 10),
                overdue: false,
            }
        );
        // Once it's passed, the debt-free date is today and the line says it's overdue instead.
        assert_eq!(
            left(d(2026, 11, 20)),
            LoansLeft {
                total_left: Money(100_000),
                debt_free_on: d(2026, 11, 20),
                overdue: true,
            }
        );
    }

    #[test]
    fn the_dashboard_has_safe_to_spend_without_rows() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        loan(&mut conn, "Behind", cash, &[("2026-11-20", 100_000)]);
        let today = d(2026, 11, 10);
        let none = get(&conn, &MONTH, today).unwrap().safe_to_spend;
        assert_eq!(none.safe_to_spend, None, "no payday rule yet");

        let salary = crate::services::recurring::RecurringRuleFields {
            transaction: TransactionFields {
                kind: TransactionKind::Income,
                amount: Money(420_000),
                date: "2026-11-15".into(),
                account_id: cash,
                to_account_id: None,
                category_id: Some(id_of(&conn, "categories", "Salary")),
                payee: None,
                payment_method_id: None,
                note: None,
            },
            frequency: crate::domain::recurrence::Frequency::Monthly,
            interval: 1,
            end_date: None,
            max_occurrences: None,
            mode: crate::services::recurring::RecurringMode::Confirm,
            is_payday: true,
        };
        crate::services::recurring::create(&mut conn, salary, now()).unwrap();
        let shown = get(&conn, &MONTH, today).unwrap().safe_to_spend;
        let full = forecast::get(&conn, ForecastGrain::Window, today).unwrap();
        assert!(!full.rows.is_empty());
        assert_eq!(
            shown,
            Forecast {
                rows: Vec::new(),
                ..full
            }
        );
    }
}
