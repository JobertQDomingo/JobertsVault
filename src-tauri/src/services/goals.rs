use chrono::{DateTime, Local, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::budget::used_percent;
use crate::domain::goal::{monthly_needed, months_left};
use crate::domain::money::Money;
use crate::error::AppError;
use crate::repo;
use crate::services::transactions::real_date;
use crate::services::{invalid, not_found, stamp};

/// What the new and edit goal forms send (GOL-1, GOL-2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GoalFields {
    /// 1–50 characters, unique ignoring case.
    pub name: String,
    pub target_amount: Money,
    /// `YYYY-MM-DD`, or none. It may already have passed.
    pub target_date: Option<String>,
    /// Any account but a credit card. Its balance is the goal's progress: there are no manual
    /// contributions.
    #[ts(type = "number")]
    pub account_id: i64,
}

/// One stored goal (DATA_MODEL §4.4). The screens look the account up in its list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Goal {
    #[ts(type = "number")]
    pub id: i64,
    #[serde(flatten)]
    pub fields: GoalFields,
}

/// A goal and how close its account is to the target as of today (GOL-3, DATA_MODEL §5.7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GoalProgress {
    #[serde(flatten)]
    pub goal: Goal,
    /// The account's balance dated today or earlier, never below 0.
    pub progress: Money,
    /// What's still to save, never below 0.
    pub remaining: Money,
    /// Whole percent rounded down, at most 100, so 100 means the target is really reached.
    #[ts(type = "number")]
    pub percent: i64,
    /// Calendar months to the target date, a part month counting as a whole one; 0 once the
    /// date is today or has passed, and none without a target date.
    #[ts(type = "number | null")]
    pub months_left: Option<i64>,
    /// `remaining` over `months_left`, rounded up; none when nothing is left to save, no month
    /// is, or there's no target date.
    pub monthly_needed: Option<Money>,
}

/// Every goal with its progress, in the order they were created.
pub fn list(conn: &Connection, today: NaiveDate) -> Result<Vec<GoalProgress>, AppError> {
    let goals = repo::goals::all(conn)?;
    if goals.is_empty() {
        return Ok(Vec::new());
    }
    let accounts = repo::accounts::all(conn, today)?;
    goals
        .into_iter()
        .map(|row| {
            let goal = view(row);
            let balance = accounts
                .iter()
                .find(|a| a.id == goal.fields.account_id)
                .map(|a| a.balance)
                .ok_or_else(|| AppError::Database {
                    message: format!("Goal {} has no account", goal.id),
                })?;
            progress(goal, balance, today)
        })
        .collect()
}

/// DATA_MODEL §5.7.
fn progress(goal: Goal, balance: i64, today: NaiveDate) -> Result<GoalProgress, AppError> {
    let target = goal.fields.target_amount;
    let progress = Money(balance.max(0));
    let remaining = Money(target.0.saturating_sub(progress.0).max(0));
    let months = goal
        .fields
        .target_date
        .as_deref()
        .map(|text| {
            real_date(text).ok_or_else(|| AppError::Database {
                message: format!("Goal {} has an invalid target date", goal.id),
            })
        })
        .transpose()?
        .map(|date| months_left(today, date));
    Ok(GoalProgress {
        progress,
        remaining,
        percent: used_percent(progress, target).min(100),
        months_left: months,
        monthly_needed: months.and_then(|m| monthly_needed(remaining, m)),
        goal,
    })
}

pub fn create(
    conn: &mut Connection,
    mut input: GoalFields,
    now: DateTime<Local>,
) -> Result<Goal, AppError> {
    let tx = conn.transaction()?;
    validate(&tx, &mut input, None, now.date_naive())?;
    let id = repo::goals::insert(&tx, &values(&input), &stamp(now))?;
    let goal = get(&tx, id)?;
    tx.commit()?;
    Ok(goal)
}

/// Every field can change.
pub fn update(
    conn: &mut Connection,
    id: i64,
    mut input: GoalFields,
    now: DateTime<Local>,
) -> Result<Goal, AppError> {
    let tx = conn.transaction()?;
    let stored = get(&tx, id)?;
    validate(&tx, &mut input, Some(&stored), now.date_naive())?;
    repo::goals::update(&tx, id, &values(&input), &stamp(now))?;
    let goal = get(&tx, id)?;
    tx.commit()?;
    Ok(goal)
}

/// Nothing points at a goal, so any can go.
pub fn delete(conn: &mut Connection, id: i64) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    if repo::goals::delete(&tx, id)? == 0 {
        return Err(not_found("goal", id));
    }
    tx.commit()?;
    Ok(())
}

/// Trims the name and checks every field (DATA_MODEL §6). An edit may keep the archived account
/// it already has, like a transaction.
fn validate(
    conn: &Connection,
    f: &mut GoalFields,
    stored: Option<&Goal>,
    today: NaiveDate,
) -> Result<(), AppError> {
    f.name = f.name.trim().to_owned();
    if !(1..=50).contains(&f.name.chars().count()) {
        return Err(invalid("name", "Enter a name of 1 to 50 characters"));
    }
    if repo::goals::name_taken(conn, &f.name, stored.map(|s| s.id))? {
        return Err(invalid("name", "A goal with this name already exists"));
    }
    if f.target_amount <= Money::ZERO {
        return Err(invalid("targetAmount", "Enter an amount above 0"));
    }
    if f.target_amount > Money::MAX {
        return Err(invalid("targetAmount", "That amount is too large"));
    }
    if f.target_date
        .as_deref()
        .is_some_and(|d| real_date(d).is_none())
    {
        return Err(invalid("targetDate", "Enter a real date"));
    }
    let account = repo::accounts::find(conn, f.account_id, today)?
        .ok_or_else(|| invalid("accountId", "This account no longer exists"))?;
    if account.kind == "credit_card" {
        return Err(invalid(
            "accountId",
            "Choose an account that isn't a credit card",
        ));
    }
    if account.archived && stored.map(|s| s.fields.account_id) != Some(f.account_id) {
        return Err(invalid("accountId", "This account is archived"));
    }
    Ok(())
}

fn get(conn: &Connection, id: i64) -> Result<Goal, AppError> {
    repo::goals::find(conn, id)?
        .map(view)
        .ok_or_else(|| not_found("goal", id))
}

fn values(f: &GoalFields) -> repo::goals::Values<'_> {
    repo::goals::Values {
        name: &f.name,
        target_amount: f.target_amount.0,
        target_date: f.target_date.as_deref(),
        account_id: f.account_id,
    }
}

fn view(row: repo::goals::Row) -> Goal {
    Goal {
        id: row.id,
        fields: GoalFields {
            name: row.name,
            target_amount: Money(row.target_amount),
            target_date: row.target_date,
            account_id: row.account_id,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::accounts::{self, AccountFields, AccountKind, NewAccount};
    use crate::services::settings;
    use crate::services::transactions::{self, TransactionFields, TransactionKind};
    use crate::test_support::{db, now};
    use std::fmt::Debug;

    // The seed (DATA_MODEL §7): Cash is account 1, Salary income category 1, Food expense 8.
    const CASH: i64 = 1;
    const SALARY: i64 = 1;
    const FOOD: i64 = 8;

    fn field_of<T: Debug>(result: Result<T, AppError>) -> String {
        match result {
            Err(AppError::Validation { field, .. }) => field,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("a real test date")
    }

    fn fields(name: &str, target: i64, date: Option<&str>, account_id: i64) -> GoalFields {
        GoalFields {
            name: name.into(),
            target_amount: Money(target),
            target_date: date.map(Into::into),
            account_id,
        }
    }

    fn account(conn: &mut Connection, name: &str, kind: AccountKind, opening: i64) -> i64 {
        let card = kind == AccountKind::CreditCard;
        let new = NewAccount {
            kind,
            fields: AccountFields {
                name: name.into(),
                opening_balance: Money(opening),
                credit_limit: card.then_some(Money(1_000_000)),
                statement_day: card.then_some(5),
                due_day: card.then_some(25),
            },
        };
        accounts::create(conn, new, now()).unwrap().id
    }

    fn save(
        conn: &mut Connection,
        kind: TransactionKind,
        account_id: i64,
        amount: i64,
        date: NaiveDate,
    ) {
        let category = match kind {
            TransactionKind::Income => Some(SALARY),
            TransactionKind::Expense => Some(FOOD),
            TransactionKind::Transfer => None,
        };
        let fields = TransactionFields {
            kind,
            amount: Money(amount),
            date: date.to_string(),
            account_id,
            to_account_id: None,
            category_id: category,
            payee: None,
            payment_method_id: None,
            note: None,
        };
        transactions::create(conn, fields, now()).unwrap();
    }

    fn names(conn: &Connection) -> Vec<String> {
        list(conn, now().date_naive())
            .unwrap()
            .into_iter()
            .map(|p| p.goal.fields.name)
            .collect()
    }

    #[test]
    fn create_saves_a_goal_with_or_without_a_target_date() {
        let mut conn = db();
        let trip = create(
            &mut conn,
            fields("  Trip ", 500_000, Some("2027-05-01"), CASH),
            now(),
        )
        .unwrap();
        assert_eq!(
            trip.fields,
            fields("Trip", 500_000, Some("2027-05-01"), CASH)
        );
        create(&mut conn, fields("Rainy day", 100_000, None, CASH), now()).unwrap();
        assert_eq!(names(&conn), ["Trip", "Rainy day"]);
    }

    #[test]
    fn every_field_is_checked() {
        let mut conn = db();
        let card = account(&mut conn, "Visa", AccountKind::CreditCard, 0);
        create(&mut conn, fields("Trip", 100, None, CASH), now()).unwrap();
        let long = "x".repeat(51);
        let cases = [
            (fields("  ", 100, None, CASH), "name"),
            (fields(&long, 100, None, CASH), "name"),
            (fields("TRIP", 100, None, CASH), "name"),
            (fields("Car", 0, None, CASH), "targetAmount"),
            (fields("Car", -5, None, CASH), "targetAmount"),
            (fields("Car", Money::MAX.0 + 1, None, CASH), "targetAmount"),
            (fields("Car", 100, Some("2027-02-30"), CASH), "targetDate"),
            (fields("Car", 100, Some("2027-2-3"), CASH), "targetDate"),
            (fields("Car", 100, None, 999), "accountId"),
            (fields("Car", 100, None, card), "accountId"),
        ];
        for (input, field) in cases {
            let shown = format!("{input:?}");
            assert_eq!(field_of(create(&mut conn, input, now())), field, "{shown}");
        }
        create(
            &mut conn,
            fields(&"x".repeat(50), Money::MAX.0, Some("2020-01-01"), CASH),
            now(),
        )
        .unwrap();
    }

    #[test]
    fn edit_changes_every_field_but_cant_take_another_goals_name() {
        let mut conn = db();
        let bank = account(&mut conn, "Bank", AccountKind::Bank, 0);
        let trip = create(&mut conn, fields("Trip", 100, None, CASH), now()).unwrap();
        create(&mut conn, fields("Car", 100, None, CASH), now()).unwrap();
        // Saving it unchanged, or in another case, isn't a clash with itself.
        update(&mut conn, trip.id, fields("TRIP", 100, None, CASH), now()).unwrap();
        let edited = update(
            &mut conn,
            trip.id,
            fields("Japan", 900, Some("2027-01-31"), bank),
            now(),
        )
        .unwrap();
        assert_eq!(
            edited,
            Goal {
                id: trip.id,
                fields: fields("Japan", 900, Some("2027-01-31"), bank)
            }
        );
        let result = update(&mut conn, trip.id, fields("car", 900, None, bank), now());
        assert_eq!(field_of(result), "name");
    }

    #[test]
    fn a_new_goal_cant_use_an_archived_account_but_an_edit_keeps_its_own() {
        let mut conn = db();
        let bank = account(&mut conn, "Bank", AccountKind::Bank, 0);
        let other = account(&mut conn, "Other", AccountKind::Bank, 0);
        let trip = create(&mut conn, fields("Trip", 100, None, bank), now()).unwrap();
        accounts::archive(&mut conn, bank, false, now()).unwrap();
        accounts::archive(&mut conn, other, false, now()).unwrap();
        assert_eq!(
            field_of(create(&mut conn, fields("Car", 100, None, bank), now())),
            "accountId"
        );
        update(&mut conn, trip.id, fields("Trip", 200, None, bank), now()).unwrap();
        let result = update(&mut conn, trip.id, fields("Trip", 200, None, other), now());
        assert_eq!(field_of(result), "accountId");
    }

    #[test]
    fn delete_removes_only_that_goal_and_missing_ones_are_not_found() {
        let mut conn = db();
        let trip = create(&mut conn, fields("Trip", 100, None, CASH), now()).unwrap();
        create(&mut conn, fields("Car", 100, None, CASH), now()).unwrap();
        delete(&mut conn, trip.id).unwrap();
        assert_eq!(names(&conn), ["Car"]);
        assert!(matches!(
            update(&mut conn, 999, fields("X", 1, None, CASH), now()),
            Err(AppError::NotFound { id: 999, .. })
        ));
        assert!(matches!(
            delete(&mut conn, 999),
            Err(AppError::NotFound { id: 999, .. })
        ));
    }

    #[test]
    fn a_goal_puts_its_account_in_use_so_it_can_only_be_archived() {
        let mut conn = db();
        let bank = account(&mut conn, "Bank", AccountKind::Bank, 0);
        let goal = create(&mut conn, fields("Trip", 100, None, bank), now()).unwrap();
        assert!(
            accounts::get(&conn, bank, now().date_naive())
                .unwrap()
                .in_use
        );
        assert!(matches!(
            accounts::delete(&mut conn, bank, now()),
            Err(AppError::Conflict { .. })
        ));
        delete(&mut conn, goal.id).unwrap();
        accounts::delete(&mut conn, bank, now()).unwrap();
    }

    #[test]
    fn a_goal_locks_the_decimals() {
        let mut conn = db();
        assert!(!settings::get(&conn).unwrap().decimals_locked);
        create(&mut conn, fields("Trip", 100, None, CASH), now()).unwrap();
        assert!(settings::get(&conn).unwrap().decimals_locked);
    }

    /// (progress, remaining, percent, months left, monthly needed) of one goal.
    type Figures = (i64, i64, i64, Option<i64>, Option<i64>);

    /// Each goal's figures as of `today`.
    fn figures(conn: &Connection, today: NaiveDate) -> Vec<Figures> {
        list(conn, today)
            .unwrap()
            .into_iter()
            .map(|p| {
                (
                    p.progress.0,
                    p.remaining.0,
                    p.percent,
                    p.months_left,
                    p.monthly_needed.map(|m| m.0),
                )
            })
            .collect()
    }

    #[test]
    fn progress_is_the_accounts_balance_up_to_today() {
        let mut conn = db();
        let savings = account(&mut conn, "Savings", AccountKind::Bank, 100_000);
        create(
            &mut conn,
            fields("Trip", 500_000, Some("2027-05-12"), savings),
            now(),
        )
        .unwrap();
        save(
            &mut conn,
            TransactionKind::Income,
            savings,
            150_000,
            d(2026, 11, 1),
        );
        save(
            &mut conn,
            TransactionKind::Expense,
            savings,
            10_000,
            d(2026, 11, 2),
        );
        // A future-dated row doesn't count yet (TXN-8).
        save(
            &mut conn,
            TransactionKind::Income,
            savings,
            99_999,
            d(2026, 11, 13),
        );
        // 2,600.00 left over 6 months is 433.34 a month (UI_FLOWS §8.4).
        assert_eq!(
            figures(&conn, d(2026, 11, 12)),
            [(240_000, 260_000, 48, Some(6), Some(43_334))]
        );
    }

    #[test]
    fn a_balance_below_0_is_no_progress_and_a_reached_target_is_100() {
        let mut conn = db();
        let low = account(&mut conn, "Low", AccountKind::Bank, 0);
        let high = account(&mut conn, "High", AccountKind::Bank, 700_000);
        save(
            &mut conn,
            TransactionKind::Expense,
            low,
            5_000,
            d(2026, 11, 1),
        );
        create(
            &mut conn,
            fields("Low", 100_000, Some("2027-01-01"), low),
            now(),
        )
        .unwrap();
        create(
            &mut conn,
            fields("High", 500_000, Some("2027-01-01"), high),
            now(),
        )
        .unwrap();
        create(&mut conn, fields("Almost", 700_001, None, high), now()).unwrap();
        assert_eq!(
            figures(&conn, d(2026, 11, 12)),
            [
                (0, 100_000, 0, Some(2), Some(50_000)),
                (700_000, 0, 100, Some(2), None),
                // 99.9998% shows 99, never 100 before it's reached.
                (700_000, 1, 99, None, None),
            ]
        );
    }

    #[test]
    fn a_target_date_today_or_past_has_no_monthly_amount() {
        let mut conn = db();
        create(
            &mut conn,
            fields("Today", 100, Some("2026-11-12"), CASH),
            now(),
        )
        .unwrap();
        create(
            &mut conn,
            fields("Past", 100, Some("2026-01-01"), CASH),
            now(),
        )
        .unwrap();
        assert_eq!(
            figures(&conn, d(2026, 11, 12)),
            [(0, 100, 0, Some(0), None), (0, 100, 0, Some(0), None)]
        );
    }

    #[test]
    fn a_stored_target_date_that_isnt_real_is_a_database_error() {
        let mut conn = db();
        create(&mut conn, fields("Trip", 100, None, CASH), now()).unwrap();
        conn.execute("UPDATE goals SET target_date = '2027-2-3'", [])
            .unwrap();
        assert!(matches!(
            list(&conn, d(2026, 11, 12)),
            Err(AppError::Database { .. })
        ));
    }

    #[test]
    fn goals_on_an_archived_account_still_show_progress() {
        let mut conn = db();
        let savings = account(&mut conn, "Savings", AccountKind::Bank, 30_000);
        create(&mut conn, fields("Trip", 60_000, None, savings), now()).unwrap();
        accounts::archive(&mut conn, savings, true, now()).unwrap();
        assert_eq!(
            figures(&conn, d(2026, 11, 12)),
            [(30_000, 30_000, 50, None, None)]
        );
    }
}
