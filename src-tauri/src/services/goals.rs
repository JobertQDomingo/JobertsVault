use chrono::{DateTime, Local, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::budget::used_percent;
use crate::domain::goal::{self, OtherGoal, Room, monthly_needed, months_left};
use crate::domain::money::{Decimals, Money};
use crate::error::AppError;
use crate::repo;
use crate::services::forecast::{self, AsGoal};
use crate::services::settings::{self, CurrencyPosition};
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
    /// `YYYY-MM-DD`, required from Phase 4 and not before the earliest realistic date (GOL-6).
    /// None only on goals saved before Phase 4, until they're edited.
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
    /// The day Goals first found its progress at the target (GOL-4), which can be later than the
    /// day the balance got there. It stays if the balance drops; a higher target or another
    /// account clears it. An archived goal isn't checked.
    pub achieved_at: Option<NaiveDate>,
    /// Hidden from the goal cards, under Show archived. Any goal can be archived.
    pub archived: bool,
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
    /// `remaining` over `months_left`, rounded up; none once achieved, when nothing is left to
    /// save, when no month is, or without a target date.
    pub monthly_needed: Option<Money>,
}

/// What the goal dialog asks about as the user types (GOL-5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GoalDraft {
    pub target_amount: Money,
    #[ts(type = "number")]
    pub account_id: i64,
    /// The goal being edited, so it doesn't count against itself; none for a new one.
    #[ts(type = "number | null")]
    pub goal_id: Option<i64>,
}

/// How much could be set aside each month for the goal, and its earliest realistic target date
/// (DATA_MODEL §5.17).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GoalSuggestion {
    pub monthly_room: Money,
    /// The suggested date; today when nothing is left to save. None while something is left and
    /// nothing can be set aside.
    pub earliest_date: Option<NaiveDate>,
}

/// The monthly room and the earliest date for a goal being added or edited; none with no payday
/// rule (GOL-5).
pub fn suggest(
    conn: &Connection,
    draft: GoalDraft,
    today: NaiveDate,
) -> Result<Option<GoalSuggestion>, AppError> {
    check_target(draft.target_amount)?;
    goal_account(conn, draft.account_id, today)?;
    Ok(room(conn, today, &draft)?.map(|r| GoalSuggestion {
        monthly_room: r.monthly_room,
        earliest_date: r.earliest,
    }))
}

/// §5.17, worked out as if the goal is saved: its account isn't counted, and every other active
/// goal that isn't achieved and has a date takes its monthly amount. None with no payday rule.
fn room(conn: &Connection, today: NaiveDate, draft: &GoalDraft) -> Result<Option<Room>, AppError> {
    let as_goal = AsGoal {
        id: draft.goal_id,
        account_id: draft.account_id,
    };
    // No pay windows (no payday rule) gives None below.
    let plan = forecast::gather(conn, today, Some(as_goal))?;
    let accounts = repo::accounts::all(conn, today)?;
    let progress = |id: i64| {
        accounts
            .iter()
            .find(|a| a.id == id)
            .map_or(0, |a| a.balance.max(0))
    };
    let mut others = Vec::new();
    for g in repo::goals::all(conn)? {
        if g.archived || g.achieved_at.is_some() || Some(g.id) == draft.goal_id {
            continue;
        }
        let Some(text) = g.target_date.as_deref() else {
            continue;
        };
        let date = real_date(text).ok_or_else(|| AppError::Database {
            message: format!("Goal {} has an invalid target date", g.id),
        })?;
        let months = months_left(today, date);
        let remaining = Money(
            g.target_amount
                .saturating_sub(progress(g.account_id))
                .max(0),
        );
        if let Some(needed) = monthly_needed(remaining, months) {
            others.push(OtherGoal {
                monthly_needed: needed,
                months_left: months,
            });
        }
    }
    let left = Money(
        draft
            .target_amount
            .0
            .saturating_sub(progress(draft.account_id)),
    );
    Ok(goal::room(
        plan.money_now,
        today,
        &plan.windows,
        &plan.movements,
        &others,
        left,
    ))
}

/// Every goal with its progress, archived ones included, in the order they were created. A goal
/// whose progress has reached its target is marked achieved today, the first time it's seen
/// there (GOL-4).
pub fn list(conn: &mut Connection, today: NaiveDate) -> Result<Vec<GoalProgress>, AppError> {
    let tx = conn.transaction()?;
    let goals = repo::goals::all(&tx)?;
    if goals.is_empty() {
        return Ok(Vec::new());
    }
    let accounts = repo::accounts::all(&tx, today)?;
    let mut list = Vec::with_capacity(goals.len());
    for row in goals {
        let mut goal = view(row)?;
        let balance = accounts
            .iter()
            .find(|a| a.id == goal.fields.account_id)
            .map(|a| a.balance)
            .ok_or_else(|| AppError::Database {
                message: format!("Goal {} has no account", goal.id),
            })?;
        // An archived goal may have been given up, so it isn't marked.
        if !goal.archived && goal.achieved_at.is_none() && balance >= goal.fields.target_amount.0 {
            repo::goals::set_achieved(&tx, goal.id, &today.to_string())?;
            goal.achieved_at = Some(today);
        }
        list.push(progress(goal, balance, today)?);
    }
    tx.commit()?;
    Ok(list)
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
        monthly_needed: months
            .filter(|_| goal.achieved_at.is_none())
            .and_then(|m| monthly_needed(remaining, m)),
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

/// Hides it under Show archived, achieved or not (GOL-4).
pub fn archive(conn: &mut Connection, id: i64, now: DateTime<Local>) -> Result<(), AppError> {
    set_archived(conn, id, true, now)
}

pub fn unarchive(conn: &mut Connection, id: i64, now: DateTime<Local>) -> Result<(), AppError> {
    set_archived(conn, id, false, now)
}

fn set_archived(
    conn: &mut Connection,
    id: i64,
    archived: bool,
    now: DateTime<Local>,
) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    if repo::goals::set_archived(&tx, id, archived, &stamp(now))? == 0 {
        return Err(not_found("goal", id));
    }
    tx.commit()?;
    Ok(())
}

/// Nothing points at a goal, so any can go, archived or not.
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
    check_target(f.target_amount)?;
    // GOL-6: every add and edit needs a date, clearing it included.
    let date = f
        .target_date
        .as_deref()
        .ok_or_else(|| invalid("targetDate", "Enter a target date"))?;
    let date = real_date(date).ok_or_else(|| invalid("targetDate", "Enter a real date"))?;
    let account = goal_account(conn, f.account_id, today)?;
    if account.archived && stored.map(|s| s.fields.account_id) != Some(f.account_id) {
        return Err(invalid("accountId", "This account is archived"));
    }
    // An edit that keeps its date is never judged again, so bills can't make it unsaveable.
    if stored.is_none_or(|s| s.fields.target_date != f.target_date) {
        let draft = GoalDraft {
            target_amount: f.target_amount,
            account_id: f.account_id,
            goal_id: stored.map(|s| s.id),
        };
        check_date(conn, date, &draft, today)?;
    }
    Ok(())
}

/// GOL-6: a payday rule to judge by, then a date no sooner than the earliest (§5.17).
fn check_date(
    conn: &Connection,
    date: NaiveDate,
    draft: &GoalDraft,
    today: NaiveDate,
) -> Result<(), AppError> {
    let room = room(conn, today, draft)?.ok_or_else(|| {
        invalid(
            "targetDate",
            "Mark your pay as a payday first, so the app can work out a realistic date.",
        )
    })?;
    // shortcut: a date too far out for the calendar (a huge target, a tiny room) is reported as
    // no room at all; upgrade if a real goal ever gets there.
    let Some(earliest) = room.earliest else {
        return Err(invalid(
            "targetDate",
            "You can't set anything aside each month right now, so no date is realistic yet. \
             Free up money first.",
        ));
    };
    if date >= earliest {
        return Ok(());
    }
    let s = settings::stored(conn)?;
    let monthly = room.monthly_room.shown(
        Decimals::of(s.currency_decimals),
        &s.currency_symbol,
        s.currency_position == CurrencyPosition::Before,
    );
    Err(invalid(
        "targetDate",
        &format!(
            "That's too soon. The earliest you can reach this is {}, saving about {monthly} a month.",
            earliest.format("%b %-d, %Y")
        ),
    ))
}

fn check_target(amount: Money) -> Result<(), AppError> {
    if amount <= Money::ZERO {
        return Err(invalid("targetAmount", "Enter an amount above 0"));
    }
    if amount > Money::MAX {
        return Err(invalid("targetAmount", "That amount is too large"));
    }
    Ok(())
}

/// The goal's account: one that exists and isn't a credit card (GOL-2).
fn goal_account(
    conn: &Connection,
    id: i64,
    today: NaiveDate,
) -> Result<repo::accounts::Row, AppError> {
    let account = repo::accounts::find(conn, id, today)?
        .ok_or_else(|| invalid("accountId", "This account no longer exists"))?;
    if account.kind == "credit_card" {
        return Err(invalid(
            "accountId",
            "Choose an account that isn't a credit card",
        ));
    }
    Ok(account)
}

fn get(conn: &Connection, id: i64) -> Result<Goal, AppError> {
    repo::goals::find(conn, id)?
        .ok_or_else(|| not_found("goal", id))
        .and_then(view)
}

fn values(f: &GoalFields) -> repo::goals::Values<'_> {
    repo::goals::Values {
        name: &f.name,
        target_amount: f.target_amount.0,
        target_date: f.target_date.as_deref(),
        account_id: f.account_id,
    }
}

fn view(row: repo::goals::Row) -> Result<Goal, AppError> {
    let achieved_at = row
        .achieved_at
        .as_deref()
        .map(|text| {
            real_date(text).ok_or_else(|| AppError::Database {
                message: format!("Goal {} has an invalid achieved date", row.id),
            })
        })
        .transpose()?;
    Ok(Goal {
        id: row.id,
        fields: GoalFields {
            name: row.name,
            target_amount: Money(row.target_amount),
            target_date: row.target_date,
            account_id: row.account_id,
        },
        achieved_at,
        archived: row.archived,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::accounts::{self, AccountFields, AccountKind, NewAccount};
    use crate::services::recurring;
    use crate::services::settings;
    use crate::services::transactions::{self, TransactionFields, TransactionKind};
    use crate::test_support::{db, now, payday};
    use std::fmt::Debug;

    // The seed (DATA_MODEL §7): Cash is account 1, Salary income category 1, Food expense 8.
    const CASH: i64 = 1;
    const SALARY: i64 = 1;
    const FOOD: i64 = 8;

    fn field_of<T: Debug>(result: Result<T, AppError>) -> String {
        refused(result).0
    }

    /// The field and message of a validation error.
    fn refused<T: Debug>(result: Result<T, AppError>) -> (String, String) {
        match result {
            Err(AppError::Validation { field, message }) => (field, message),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    /// Stores a goal as it is, without the checks: for tests about something other than adding
    /// goals, and for goals saved before Phase 4 without a date (GOL-6).
    fn insert(conn: &Connection, f: GoalFields) -> i64 {
        repo::goals::insert(conn, &values(&f), &stamp(now())).unwrap()
    }

    /// A bank account paid 10,000.00 on each 15th, from nothing now: with every goal on another
    /// account, 10,000.00 a month can be set aside (DATA_MODEL §5.17).
    fn paid(conn: &mut Connection) -> i64 {
        let bank = account(conn, "Pay", AccountKind::Bank, 0);
        payday(conn, bank, 1_000_000);
        bank
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

    fn names(conn: &mut Connection) -> Vec<String> {
        list(conn, now().date_naive())
            .unwrap()
            .into_iter()
            .map(|p| p.goal.fields.name)
            .collect()
    }

    #[test]
    fn create_saves_a_goal() {
        let mut conn = db();
        paid(&mut conn);
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
        create(
            &mut conn,
            fields("Rainy day", 100_000, Some("2027-01-01"), CASH),
            now(),
        )
        .unwrap();
        assert_eq!(names(&mut conn), ["Trip", "Rainy day"]);
    }

    #[test]
    fn every_field_is_checked() {
        let mut conn = db();
        let card = account(&mut conn, "Visa", AccountKind::CreditCard, 0);
        // Enough pay that even the largest target is realistic in a few months.
        let bank = account(&mut conn, "Pay", AccountKind::Bank, 0);
        payday(&mut conn, bank, Money::MAX.0);
        let date = Some("2030-01-01");
        create(&mut conn, fields("Trip", 100, date, CASH), now()).unwrap();
        let long = "x".repeat(51);
        let cases = [
            (fields("  ", 100, date, CASH), "name"),
            (fields(&long, 100, date, CASH), "name"),
            (fields("TRIP", 100, date, CASH), "name"),
            (fields("Car", 0, date, CASH), "targetAmount"),
            (fields("Car", -5, date, CASH), "targetAmount"),
            (fields("Car", Money::MAX.0 + 1, date, CASH), "targetAmount"),
            (fields("Car", 100, None, CASH), "targetDate"),
            (fields("Car", 100, Some("2027-02-30"), CASH), "targetDate"),
            (fields("Car", 100, Some("2027-2-3"), CASH), "targetDate"),
            (fields("Car", 100, date, 999), "accountId"),
            (fields("Car", 100, date, card), "accountId"),
        ];
        for (input, field) in cases {
            let shown = format!("{input:?}");
            assert_eq!(field_of(create(&mut conn, input, now())), field, "{shown}");
        }
        create(
            &mut conn,
            fields(&"x".repeat(50), Money::MAX.0, date, CASH),
            now(),
        )
        .unwrap();
    }

    #[test]
    fn edit_changes_every_field_but_cant_take_another_goals_name() {
        let mut conn = db();
        paid(&mut conn);
        let bank = account(&mut conn, "Bank", AccountKind::Bank, 0);
        let date = Some("2027-05-01");
        let trip = create(&mut conn, fields("Trip", 100, date, CASH), now()).unwrap();
        create(&mut conn, fields("Car", 100, date, CASH), now()).unwrap();
        // Saving it unchanged, or in another case, isn't a clash with itself.
        update(&mut conn, trip.id, fields("TRIP", 100, date, CASH), now()).unwrap();
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
                fields: fields("Japan", 900, Some("2027-01-31"), bank),
                achieved_at: None,
                archived: false,
            }
        );
        let result = update(&mut conn, trip.id, fields("car", 900, date, bank), now());
        assert_eq!(field_of(result), "name");
    }

    #[test]
    fn a_new_goal_cant_use_an_archived_account_but_an_edit_keeps_its_own() {
        let mut conn = db();
        paid(&mut conn);
        let bank = account(&mut conn, "Bank", AccountKind::Bank, 0);
        let other = account(&mut conn, "Other", AccountKind::Bank, 0);
        let date = Some("2027-05-01");
        let trip = create(&mut conn, fields("Trip", 100, date, bank), now()).unwrap();
        accounts::archive(&mut conn, bank, false, now()).unwrap();
        accounts::archive(&mut conn, other, false, now()).unwrap();
        assert_eq!(
            field_of(create(&mut conn, fields("Car", 100, date, bank), now())),
            "accountId"
        );
        update(&mut conn, trip.id, fields("Trip", 200, date, bank), now()).unwrap();
        let result = update(&mut conn, trip.id, fields("Trip", 200, date, other), now());
        assert_eq!(field_of(result), "accountId");
    }

    #[test]
    fn delete_removes_only_that_goal_and_missing_ones_are_not_found() {
        let mut conn = db();
        let trip = insert(&conn, fields("Trip", 100, None, CASH));
        insert(&conn, fields("Car", 100, None, CASH));
        delete(&mut conn, trip).unwrap();
        assert_eq!(names(&mut conn), ["Car"]);
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
        let goal = insert(&conn, fields("Trip", 100, None, bank));
        assert!(
            accounts::get(&conn, bank, now().date_naive())
                .unwrap()
                .in_use
        );
        assert!(matches!(
            accounts::delete(&mut conn, bank, now()),
            Err(AppError::Conflict { .. })
        ));
        delete(&mut conn, goal).unwrap();
        accounts::delete(&mut conn, bank, now()).unwrap();
    }

    #[test]
    fn a_goal_locks_the_decimals() {
        let conn = db();
        assert!(!settings::get(&conn).unwrap().decimals_locked);
        insert(&conn, fields("Trip", 100, None, CASH));
        assert!(settings::get(&conn).unwrap().decimals_locked);
    }

    /// (progress, remaining, percent, months left, monthly needed) of one goal.
    type Figures = (i64, i64, i64, Option<i64>, Option<i64>);

    /// Each goal's figures as of `today`.
    fn figures(conn: &mut Connection, today: NaiveDate) -> Vec<Figures> {
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
        insert(&conn, fields("Trip", 500_000, Some("2027-05-12"), savings));
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
            figures(&mut conn, d(2026, 11, 12)),
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
        insert(&conn, fields("Low", 100_000, Some("2027-01-01"), low));
        insert(&conn, fields("High", 500_000, Some("2027-01-01"), high));
        insert(&conn, fields("Almost", 700_001, None, high));
        assert_eq!(
            figures(&mut conn, d(2026, 11, 12)),
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
        insert(&conn, fields("Today", 100, Some("2026-11-12"), CASH));
        insert(&conn, fields("Past", 100, Some("2026-01-01"), CASH));
        assert_eq!(
            figures(&mut conn, d(2026, 11, 12)),
            [(0, 100, 0, Some(0), None), (0, 100, 0, Some(0), None)]
        );
    }

    /// (achieved on, archived) of each goal, as `list` shows them on `today`.
    fn states(conn: &mut Connection, today: NaiveDate) -> Vec<(Option<NaiveDate>, bool)> {
        list(conn, today)
            .unwrap()
            .into_iter()
            .map(|p| (p.goal.achieved_at, p.goal.archived))
            .collect()
    }

    #[test]
    fn a_goal_is_achieved_the_first_day_its_progress_reaches_the_target_and_stays_so() {
        let mut conn = db();
        let bank = account(&mut conn, "Bank", AccountKind::Bank, 40_000);
        insert(&conn, fields("Trip", 50_000, Some("2027-05-12"), bank));
        insert(&conn, fields("Car", 900_000, None, bank));
        save(
            &mut conn,
            TransactionKind::Income,
            bank,
            10_000,
            d(2026, 11, 14),
        );
        assert_eq!(
            states(&mut conn, d(2026, 11, 13)),
            [(None, false), (None, false)]
        );
        assert_eq!(
            states(&mut conn, d(2026, 11, 14)),
            [(Some(d(2026, 11, 14)), false), (None, false)]
        );
        // Spending it later keeps the first day, and an achieved goal asks for no monthly amount.
        save(
            &mut conn,
            TransactionKind::Expense,
            bank,
            20_000,
            d(2026, 11, 15),
        );
        let trip = &list(&mut conn, d(2026, 11, 16)).unwrap()[0];
        assert_eq!(trip.goal.achieved_at, Some(d(2026, 11, 14)));
        assert_eq!((trip.progress.0, trip.percent), (30_000, 60));
        assert_eq!((trip.months_left, trip.monthly_needed), (Some(6), None));
    }

    #[test]
    fn a_new_target_amount_or_account_clears_achieved_but_other_edits_keep_it() {
        let mut conn = db();
        let bank = account(&mut conn, "Bank", AccountKind::Bank, 50_000);
        let other = account(&mut conn, "Other", AccountKind::Bank, 0);
        let trip = insert(&conn, fields("Trip", 50_000, Some("2027-01-01"), bank));
        let today = d(2026, 11, 12);
        assert_eq!(states(&mut conn, today), [(Some(today), false)]);
        // A new name keeps it, even below the target now. Every edit here keeps the date, so none
        // needs the forecast (GOL-6).
        save(
            &mut conn,
            TransactionKind::Expense,
            bank,
            1_000,
            d(2026, 11, 12),
        );
        let renamed = update(
            &mut conn,
            trip,
            fields("Japan", 50_000, Some("2027-01-01"), bank),
            now(),
        )
        .unwrap();
        assert_eq!(renamed.achieved_at, Some(today));
        // A lower target on the same account was reached then too, so it keeps its day.
        let lowered = update(
            &mut conn,
            trip,
            fields("Japan", 40_000, Some("2027-01-01"), bank),
            now(),
        )
        .unwrap();
        assert_eq!(lowered.achieved_at, Some(today));
        // A higher target is checked again: not reached, so not achieved.
        let raised = update(
            &mut conn,
            trip,
            fields("Japan", 60_000, Some("2027-01-01"), bank),
            now(),
        )
        .unwrap();
        assert_eq!(raised.achieved_at, None);
        assert_eq!(states(&mut conn, today), [(None, false)]);
        // Another account, even one that already holds the target, is checked again too.
        update(
            &mut conn,
            trip,
            fields("Japan", 49_000, Some("2027-01-01"), bank),
            now(),
        )
        .unwrap();
        assert_eq!(
            states(&mut conn, d(2026, 11, 13)),
            [(Some(d(2026, 11, 13)), false)]
        );
        let moved = update(
            &mut conn,
            trip,
            fields("Japan", 49_000, Some("2027-01-01"), other),
            now(),
        )
        .unwrap();
        assert_eq!(moved.achieved_at, None);
    }

    #[test]
    fn an_archived_goal_isnt_marked_achieved() {
        let mut conn = db();
        let bank = account(&mut conn, "Bank", AccountKind::Bank, 0);
        let car = insert(&conn, fields("Car", 50_000, None, bank));
        archive(&mut conn, car, now()).unwrap();
        save(
            &mut conn,
            TransactionKind::Income,
            bank,
            60_000,
            d(2026, 11, 12),
        );
        assert_eq!(states(&mut conn, d(2026, 11, 12)), [(None, true)]);
        // Brought back, it's checked again.
        unarchive(&mut conn, car, now()).unwrap();
        assert_eq!(
            states(&mut conn, d(2026, 11, 13)),
            [(Some(d(2026, 11, 13)), false)]
        );
    }

    #[test]
    fn any_goal_can_be_archived_and_unarchived_and_still_shows_progress() {
        let mut conn = db();
        let trip = insert(&conn, fields("Trip", 100, None, CASH));
        insert(&conn, fields("Car", 100, None, CASH));
        archive(&mut conn, trip, now()).unwrap();
        // Twice is fine.
        archive(&mut conn, trip, now()).unwrap();
        assert_eq!(
            states(&mut conn, d(2026, 11, 12)),
            [(None, true), (None, false)]
        );
        // An archived goal can still be deleted.
        unarchive(&mut conn, trip, now()).unwrap();
        assert_eq!(
            states(&mut conn, d(2026, 11, 12)),
            [(None, false), (None, false)]
        );
        archive(&mut conn, trip, now()).unwrap();
        delete(&mut conn, trip).unwrap();
        assert_eq!(names(&mut conn), ["Car"]);
        for result in [
            archive(&mut conn, 999, now()),
            unarchive(&mut conn, 999, now()),
        ] {
            assert!(matches!(result, Err(AppError::NotFound { id: 999, .. })));
        }
    }

    #[test]
    fn a_stored_target_date_that_isnt_real_is_a_database_error() {
        let mut conn = db();
        insert(&conn, fields("Trip", 100, None, CASH));
        conn.execute("UPDATE goals SET target_date = '2027-2-3'", [])
            .unwrap();
        assert!(matches!(
            list(&mut conn, d(2026, 11, 12)),
            Err(AppError::Database { .. })
        ));
    }

    #[test]
    fn goals_on_an_archived_account_still_show_progress() {
        let mut conn = db();
        let savings = account(&mut conn, "Savings", AccountKind::Bank, 30_000);
        insert(&conn, fields("Trip", 60_000, None, savings));
        accounts::archive(&mut conn, savings, true, now()).unwrap();
        assert_eq!(
            figures(&mut conn, d(2026, 11, 12)),
            [(30_000, 30_000, 50, None, None)]
        );
    }

    // GOL-5, GOL-6 (DATA_MODEL §5.17). Tests run on 2026-11-03; `paid` gives a room of 10,000.00.
    const NO_DATE: &str = "Enter a target date";
    const NO_PAYDAY: &str =
        "Mark your pay as a payday first, so the app can work out a realistic date.";
    const NO_ROOM: &str = "You can't set anything aside each month right now, so no date is \
                           realistic yet. Free up money first.";
    const TOO_SOON: &str = "That's too soon. The earliest you can reach this is Feb 3, 2027, \
                            saving about ₱10,000.00 a month.";

    fn draft(target: i64, account_id: i64, goal_id: Option<i64>) -> GoalDraft {
        GoalDraft {
            target_amount: Money(target),
            account_id,
            goal_id,
        }
    }

    /// The monthly room and earliest date `suggest_goal` gives.
    fn suggested(
        conn: &Connection,
        target: i64,
        account_id: i64,
        goal_id: Option<i64>,
    ) -> Option<(i64, Option<NaiveDate>)> {
        suggest(conn, draft(target, account_id, goal_id), now().date_naive())
            .unwrap()
            .map(|s| (s.monthly_room.0, s.earliest_date))
    }

    /// The message of a refusal on `targetDate`.
    fn message<T: Debug>(result: Result<T, AppError>) -> String {
        let (field, message) = refused(result);
        assert_eq!(field, "targetDate", "{message}");
        message
    }

    #[test]
    fn the_suggestion_is_the_monthly_room_and_the_earliest_date() {
        let mut conn = db();
        paid(&mut conn);
        // 30,000.00 at 10,000.00 a month: 3 months.
        assert_eq!(
            suggested(&conn, 3_000_000, CASH, None),
            Some((1_000_000, Some(d(2027, 2, 3))))
        );
    }

    #[test]
    fn a_target_date_is_required_on_add_and_every_edit() {
        let mut conn = db();
        paid(&mut conn);
        let none = |name| fields(name, 100, None, CASH);
        assert_eq!(message(create(&mut conn, none("Trip"), now())), NO_DATE);
        let trip = create(
            &mut conn,
            fields("Trip", 100, Some("2027-05-01"), CASH),
            now(),
        )
        .unwrap();
        // Clearing it is refused, and so is editing a goal from before Phase 4 without one.
        assert_eq!(
            message(update(&mut conn, trip.id, none("Trip"), now())),
            NO_DATE
        );
        let old = insert(&conn, none("Old"));
        assert_eq!(
            message(update(&mut conn, old, none("Old plan"), now())),
            NO_DATE
        );
    }

    #[test]
    fn no_payday_rule_means_no_suggestion_and_no_new_date() {
        let mut conn = db();
        let bank = account(&mut conn, "Bank", AccountKind::Bank, 0);
        assert_eq!(suggested(&conn, 3_000_000, CASH, None), None);
        // A missing date is named first, then the payday.
        let new = |date| fields("Trip", 100, date, CASH);
        assert_eq!(message(create(&mut conn, new(None), now())), NO_DATE);
        assert_eq!(
            message(create(&mut conn, new(Some("2030-01-01")), now())),
            NO_PAYDAY
        );
        let trip = insert(&conn, new(Some("2027-05-01")));
        assert_eq!(
            message(update(&mut conn, trip, new(Some("2027-06-01")), now())),
            NO_PAYDAY
        );
        // An edit that keeps its date can change the name, the target and the account.
        let kept = fields("Japan", 900, Some("2027-05-01"), bank);
        update(&mut conn, trip, kept, now()).unwrap();
        // Giving a goal from before Phase 4 its first date changes its date.
        let old = insert(&conn, fields("Old", 100, None, CASH));
        let dated = fields("Old", 100, Some("2030-01-01"), CASH);
        assert_eq!(message(update(&mut conn, old, dated, now())), NO_PAYDAY);
    }

    #[test]
    fn a_paused_payday_rule_still_counts() {
        let mut conn = db();
        let bank = account(&mut conn, "Pay", AccountKind::Bank, 500_000);
        let rule = payday(&mut conn, bank, 1_000_000);
        recurring::pause(&mut conn, rule, true, now()).unwrap();
        // No pay to come, so 5,000.00 must last 11 saving dates: 454.54 a month.
        assert_eq!(
            suggested(&conn, 100_000, CASH, None),
            Some((45_454, Some(d(2027, 2, 3))))
        );
        let trip = fields("Trip", 100_000, Some("2027-02-03"), CASH);
        create(&mut conn, trip, now()).unwrap();
    }

    #[test]
    fn the_goals_account_isnt_counted_and_pay_into_it_adds_nothing() {
        let mut conn = db();
        let pay = paid(&mut conn);
        let savings = account(&mut conn, "Savings", AccountKind::Bank, 5_000_000);
        // With no goal on it, Savings is spendable: (50,000.00 + 110,000.00) ÷ 11 by Oct 3.
        assert_eq!(
            suggested(&conn, 3_000_000, CASH, None).map(|s| s.0),
            Some(1_454_545)
        );
        // As the goal's account it isn't, and its balance is the progress: 10,000.00 left.
        assert_eq!(
            suggested(&conn, 6_000_000, savings, None),
            Some((1_000_000, Some(d(2026, 12, 3))))
        );
        // Pay into the goal's own account isn't spendable: only Savings' 50,000.00 is.
        assert_eq!(
            suggested(&conn, 3_000_000, pay, None).map(|s| s.0),
            Some(454_545)
        );
    }

    #[test]
    fn editing_a_goal_to_another_account_counts_its_old_one_again() {
        let mut conn = db();
        paid(&mut conn);
        let savings = account(&mut conn, "Savings", AccountKind::Bank, 5_000_000);
        let trip = insert(
            &conn,
            fields("Trip", 3_000_000, Some("2040-01-01"), savings),
        );
        assert_eq!(
            suggested(&conn, 3_000_000, CASH, Some(trip)).map(|s| s.0),
            Some(1_454_545)
        );
    }

    #[test]
    fn a_date_before_the_earliest_is_refused_on_add() {
        let mut conn = db();
        paid(&mut conn);
        let trip = |date| fields("Trip", 3_000_000, Some(date), CASH);
        assert_eq!(
            message(create(&mut conn, trip("2027-02-02"), now())),
            TOO_SOON
        );
        create(&mut conn, trip("2027-02-03"), now()).unwrap();
    }

    #[test]
    fn the_too_soon_message_shows_money_as_the_settings_do() {
        let mut conn = db();
        for (key, value) in [
            ("currency_symbol", "$"),
            ("currency_position", "after"),
            ("currency_decimals", "0"),
        ] {
            repo::settings::set(&conn, key, value).unwrap();
        }
        paid(&mut conn);
        let trip = fields("Trip", 3_000_000, Some("2027-02-02"), CASH);
        assert_eq!(
            message(create(&mut conn, trip, now())),
            "That's too soon. The earliest you can reach this is Feb 3, 2027, saving about \
             1,000,000\u{a0}$ a month."
        );
    }

    #[test]
    fn an_edit_is_checked_only_when_it_changes_the_date() {
        let mut conn = db();
        let pay = paid(&mut conn);
        let japan = |date| fields("Japan", 3_000_000, Some(date), CASH);
        let trip = create(&mut conn, japan("2027-02-03"), now()).unwrap();
        // A 5,000.00 bill on Dec 20 leaves 15,000.00 for two saving dates by Jan 3: 7,500.00.
        save(
            &mut conn,
            TransactionKind::Expense,
            pay,
            500_000,
            d(2026, 12, 20),
        );
        // Keeping the date passes, though it's now too soon.
        update(&mut conn, trip.id, japan("2027-02-03"), now()).unwrap();
        assert_eq!(
            message(update(&mut conn, trip.id, japan("2027-02-04"), now())),
            "That's too soon. The earliest you can reach this is Mar 3, 2027, saving about \
             ₱7,500.00 a month."
        );
        update(&mut conn, trip.id, japan("2027-03-03"), now()).unwrap();
    }

    #[test]
    fn a_room_of_0_refuses_every_date() {
        let mut conn = db();
        let pay = paid(&mut conn);
        // Short by 10,000.00 on Nov 20, before the first saving date.
        save(
            &mut conn,
            TransactionKind::Expense,
            pay,
            2_000_000,
            d(2026, 11, 20),
        );
        assert_eq!(suggested(&conn, 100, CASH, None), Some((0, None)));
        let trip = fields("Trip", 100, Some("2040-01-01"), CASH);
        assert_eq!(message(create(&mut conn, trip, now())), NO_ROOM);
    }

    #[test]
    fn nothing_left_to_save_allows_today_but_not_before() {
        let mut conn = db();
        paid(&mut conn);
        let savings = account(&mut conn, "Savings", AccountKind::Bank, 500_000);
        assert_eq!(
            suggested(&conn, 500_000, savings, None),
            Some((1_000_000, Some(d(2026, 11, 3))))
        );
        let trip = |date| fields("Trip", 500_000, Some(date), savings);
        assert_eq!(
            message(create(&mut conn, trip("2026-11-02"), now())),
            "That's too soon. The earliest you can reach this is Nov 3, 2026, saving about \
             ₱10,000.00 a month."
        );
        create(&mut conn, trip("2026-11-03"), now()).unwrap();
    }

    #[test]
    fn only_active_unachieved_dated_goals_take_room() {
        let mut conn = db();
        paid(&mut conn);
        let other = account(&mut conn, "Other", AccountKind::Bank, 0);
        let feb = Some("2027-02-03");
        let archived = insert(&conn, fields("Archived", 3_000_000, feb, other));
        archive(&mut conn, archived, now()).unwrap();
        let achieved = insert(&conn, fields("Achieved", 3_000_000, feb, other));
        repo::goals::set_achieved(&conn, achieved, "2026-11-01").unwrap();
        insert(&conn, fields("Undated", 3_000_000, None, other));
        insert(
            &conn,
            fields("Passed", 3_000_000, Some("2026-11-01"), other),
        );
        assert_eq!(
            suggested(&conn, 1_000, CASH, None).map(|s| s.0),
            Some(1_000_000)
        );
        // 10,000.00 a month on its 3 saving dates takes all the pay until then.
        let dated = insert(&conn, fields("Dated", 3_000_000, feb, other));
        assert_eq!(suggested(&conn, 1_000, CASH, None).map(|s| s.0), Some(0));
        // The goal being edited doesn't count against itself.
        assert_eq!(
            suggested(&conn, 1_000, CASH, Some(dated)).map(|s| s.0),
            Some(1_000_000)
        );
    }

    #[test]
    fn a_goal_from_before_phase_4_without_a_date_still_lists_archives_and_deletes() {
        let mut conn = db();
        let old = insert(&conn, fields("Old", 100_000, None, CASH));
        assert_eq!(
            figures(&mut conn, d(2026, 11, 12)),
            [(0, 100_000, 0, None, None)]
        );
        archive(&mut conn, old, now()).unwrap();
        unarchive(&mut conn, old, now()).unwrap();
        delete(&mut conn, old).unwrap();
        assert!(names(&mut conn).is_empty());
    }

    #[test]
    fn a_suggestion_checks_its_target_and_account() {
        let mut conn = db();
        paid(&mut conn);
        let card = account(&mut conn, "Visa", AccountKind::CreditCard, 0);
        let today = now().date_naive();
        let ask = |target, account| suggest(&conn, draft(target, account, None), today);
        assert_eq!(field_of(ask(0, CASH)), "targetAmount");
        assert_eq!(field_of(ask(Money::MAX.0 + 1, CASH)), "targetAmount");
        assert_eq!(field_of(ask(100, card)), "accountId");
        assert_eq!(field_of(ask(100, 999)), "accountId");
    }
}
