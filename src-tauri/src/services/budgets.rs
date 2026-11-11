use chrono::{DateTime, Local, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::budget::used_percent;
use crate::domain::money::Money;
use crate::error::AppError;
use crate::repo;
use crate::repo::transactions::Filter;
use crate::services::period::{self, PeriodQuery};
use crate::services::{invalid, not_found, stamp};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum BudgetPeriod {
    Weekly,
    Monthly,
    Yearly,
}

/// What the new and edit budget forms send (BUD-1). The amount is the limit for every period
/// alike: what one period leaves unspent never carries over (BUD-4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BudgetFields {
    /// An expense category, or none for all spending.
    #[ts(type = "number | null")]
    pub category_id: Option<i64>,
    pub period: BudgetPeriod,
    pub amount: Money,
}

/// One stored budget (DATA_MODEL §4.3). The screens look the category up in its list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Budget {
    #[ts(type = "number")]
    pub id: i64,
    #[serde(flatten)]
    pub fields: BudgetFields,
}

/// A budget and how its current period is going (BUD-2): the week, month or year containing
/// today, by the week and month start settings. Each period starts from nothing (BUD-4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BudgetProgress {
    #[serde(flatten)]
    pub budget: Budget,
    pub start: NaiveDate,
    pub end: NaiveDate,
    /// The period's expenses dated today or earlier, in the category or in all of them.
    pub spent: Money,
    /// Below 0 once the limit is passed: the screens say how much over.
    pub remaining: Money,
    /// Whole percent rounded down, so amber means at least 80% and red at least 100% (BUD-3).
    #[ts(type = "number")]
    pub percent: i64,
}

/// Every budget with its progress: all spending first, then in the categories' order, weekly
/// before monthly before yearly.
pub fn list(conn: &Connection, today: NaiveDate) -> Result<Vec<BudgetProgress>, AppError> {
    repo::budgets::all(conn)?
        .into_iter()
        .map(|row| progress(conn, view(row)?, today))
        .collect()
}

/// DATA_MODEL §5.6. Refunds are income, so they never lower `spent`.
fn progress(
    conn: &Connection,
    budget: Budget,
    today: NaiveDate,
) -> Result<BudgetProgress, AppError> {
    let query = match budget.fields.period {
        BudgetPeriod::Weekly => PeriodQuery::Week { anchor: None },
        BudgetPeriod::Monthly => PeriodQuery::Month { anchor: None },
        BudgetPeriod::Yearly => PeriodQuery::Year { anchor: None },
    };
    let current = period::resolve(conn, &query, today)?;
    // The period contains today, and later rows don't count yet (DATA_MODEL §5.2).
    let (from, to) = (current.start().to_string(), today.to_string());
    let filter = Filter {
        kind: Some("expense"),
        category_id: budget.fields.category_id,
        ..Filter::dates(&from, &to)
    };
    let spent = Money(repo::transactions::totals(conn, &filter)?.2);
    let amount = budget.fields.amount;
    Ok(BudgetProgress {
        start: current.start(),
        end: current.end(),
        spent,
        remaining: Money(amount.0.saturating_sub(spent.0)),
        percent: used_percent(spent, amount),
        budget,
    })
}

pub fn create(
    conn: &mut Connection,
    input: BudgetFields,
    now: DateTime<Local>,
) -> Result<Budget, AppError> {
    let tx = conn.transaction()?;
    validate(&tx, &input, None)?;
    let id = repo::budgets::insert(&tx, &values(&input), &stamp(now))?;
    let budget = get(&tx, id)?;
    tx.commit()?;
    Ok(budget)
}

/// Every field can change. The new amount applies to past periods too, since no period carries
/// anything over (DATA_MODEL §4.3).
pub fn update(
    conn: &mut Connection,
    id: i64,
    input: BudgetFields,
    now: DateTime<Local>,
) -> Result<Budget, AppError> {
    let tx = conn.transaction()?;
    let stored = get(&tx, id)?;
    validate(&tx, &input, Some(&stored))?;
    repo::budgets::update(&tx, id, &values(&input), &stamp(now))?;
    let budget = get(&tx, id)?;
    tx.commit()?;
    Ok(budget)
}

/// Nothing points at a budget, so any can go; none is ever archived (DATA_MODEL §4.3).
pub fn delete(conn: &mut Connection, id: i64) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    if repo::budgets::delete(&tx, id)? == 0 {
        return Err(not_found("budget", id));
    }
    tx.commit()?;
    Ok(())
}

/// DATA_MODEL §6. An edit may keep the archived category it already has, like a transaction.
fn validate(conn: &Connection, f: &BudgetFields, stored: Option<&Budget>) -> Result<(), AppError> {
    if f.amount <= Money::ZERO {
        return Err(invalid("amount", "Enter an amount above 0"));
    }
    if f.amount > Money::MAX {
        return Err(invalid("amount", "That amount is too large"));
    }
    if let Some(id) = f.category_id {
        let category = repo::categories::find(conn, id)?
            .ok_or_else(|| invalid("categoryId", "This category no longer exists"))?;
        if category.kind != "expense" {
            return Err(invalid("categoryId", "Choose an expense category"));
        }
        if category.archived && stored.and_then(|s| s.fields.category_id) != Some(id) {
            return Err(invalid("categoryId", "This category is archived"));
        }
    }
    let period = period_text(f.period);
    if repo::budgets::taken(conn, f.category_id, period, stored.map(|s| s.id))? {
        let whose = if f.category_id.is_some() {
            "this category"
        } else {
            "all spending"
        };
        return Err(invalid(
            "period",
            &format!("There's already a {period} budget for {whose}"),
        ));
    }
    Ok(())
}

fn get(conn: &Connection, id: i64) -> Result<Budget, AppError> {
    repo::budgets::find(conn, id)?
        .ok_or_else(|| not_found("budget", id))
        .and_then(view)
}

fn values(f: &BudgetFields) -> repo::budgets::Values<'static> {
    repo::budgets::Values {
        category_id: f.category_id,
        period: period_text(f.period),
        amount: f.amount.0,
    }
}

fn view(row: repo::budgets::Row) -> Result<Budget, AppError> {
    let period = [
        BudgetPeriod::Weekly,
        BudgetPeriod::Monthly,
        BudgetPeriod::Yearly,
    ]
    .into_iter()
    .find(|p| period_text(*p) == row.period)
    .ok_or_else(|| AppError::Database {
        message: format!("Budget {} has an unknown period", row.id),
    })?;
    Ok(Budget {
        id: row.id,
        fields: BudgetFields {
            category_id: row.category_id,
            period,
            amount: Money(row.amount),
        },
    })
}

fn period_text(period: BudgetPeriod) -> &'static str {
    match period {
        BudgetPeriod::Weekly => "weekly",
        BudgetPeriod::Monthly => "monthly",
        BudgetPeriod::Yearly => "yearly",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::accounts::{self, AccountFields, AccountKind, NewAccount};
    use crate::services::transactions::{self, TransactionFields, TransactionKind};
    use crate::services::{categories, settings};
    use crate::test_support::{db, now};
    use std::fmt::Debug;

    // The seed (DATA_MODEL §7): income ids 1–7 (Salary first), expense ids 8–24 (Food & Groceries,
    // Dining Out, …).
    const SALARY: i64 = 1;
    const FOOD: i64 = 8;
    const DINING: i64 = 9;

    fn field_of<T: Debug>(result: Result<T, AppError>) -> String {
        match result {
            Err(AppError::Validation { field, .. }) => field,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    fn fields(category_id: Option<i64>, period: BudgetPeriod, amount: i64) -> BudgetFields {
        BudgetFields {
            category_id,
            period,
            amount: Money(amount),
        }
    }

    fn add(conn: &mut Connection, category_id: Option<i64>, period: BudgetPeriod) -> Budget {
        create(conn, fields(category_id, period, 40_000), now()).unwrap()
    }

    fn shown(conn: &Connection) -> Vec<(Option<i64>, BudgetPeriod)> {
        listed(conn)
            .into_iter()
            .map(|b| (b.fields.category_id, b.fields.period))
            .collect()
    }

    fn listed(conn: &Connection) -> Vec<Budget> {
        list(conn, now().date_naive())
            .unwrap()
            .into_iter()
            .map(|p| p.budget)
            .collect()
    }

    #[test]
    fn create_saves_a_category_or_all_spending_budget() {
        let mut conn = db();
        let food = create(
            &mut conn,
            fields(Some(FOOD), BudgetPeriod::Monthly, 40_000),
            now(),
        )
        .unwrap();
        assert_eq!(
            food.fields,
            fields(Some(FOOD), BudgetPeriod::Monthly, 40_000)
        );
        let all = add(&mut conn, None, BudgetPeriod::Yearly);
        assert_eq!(all.fields.category_id, None);
        assert_eq!(listed(&conn), [all, food]);
    }

    #[test]
    fn the_list_puts_all_spending_first_then_follows_the_categories_and_periods() {
        let mut conn = db();
        add(&mut conn, Some(FOOD), BudgetPeriod::Yearly);
        add(&mut conn, Some(DINING), BudgetPeriod::Weekly);
        add(&mut conn, Some(FOOD), BudgetPeriod::Weekly);
        add(&mut conn, None, BudgetPeriod::Monthly);
        add(&mut conn, Some(FOOD), BudgetPeriod::Monthly);
        categories::reorder(&mut conn, vec![DINING, FOOD], now()).unwrap();
        assert_eq!(
            shown(&conn),
            [
                (None, BudgetPeriod::Monthly),
                (Some(DINING), BudgetPeriod::Weekly),
                (Some(FOOD), BudgetPeriod::Weekly),
                (Some(FOOD), BudgetPeriod::Monthly),
                (Some(FOOD), BudgetPeriod::Yearly),
            ]
        );
    }

    #[test]
    fn the_amount_must_be_above_0_and_at_most_the_max() {
        let mut conn = db();
        for bad in [0, -1, Money::MAX.0 + 1] {
            let result = create(
                &mut conn,
                fields(Some(FOOD), BudgetPeriod::Monthly, bad),
                now(),
            );
            assert_eq!(field_of(result), "amount", "{bad}");
        }
        create(
            &mut conn,
            fields(Some(FOOD), BudgetPeriod::Monthly, Money::MAX.0),
            now(),
        )
        .unwrap();
    }

    #[test]
    fn the_category_must_be_an_existing_expense_category() {
        let mut conn = db();
        for bad in [SALARY, 999] {
            let result = create(
                &mut conn,
                fields(Some(bad), BudgetPeriod::Monthly, 100),
                now(),
            );
            assert_eq!(field_of(result), "categoryId", "{bad}");
        }
    }

    #[test]
    fn each_category_and_all_spending_get_one_budget_per_period() {
        let mut conn = db();
        add(&mut conn, Some(FOOD), BudgetPeriod::Monthly);
        add(&mut conn, None, BudgetPeriod::Monthly);
        for taken in [Some(FOOD), None] {
            let result = create(&mut conn, fields(taken, BudgetPeriod::Monthly, 100), now());
            assert_eq!(field_of(result), "period", "{taken:?}");
        }
        add(&mut conn, Some(FOOD), BudgetPeriod::Weekly);
        add(&mut conn, Some(DINING), BudgetPeriod::Monthly);
    }

    #[test]
    fn edit_changes_every_field_but_cant_take_another_budgets_place() {
        let mut conn = db();
        let food = add(&mut conn, Some(FOOD), BudgetPeriod::Monthly);
        add(&mut conn, None, BudgetPeriod::Weekly);
        // Saving it unchanged isn't a clash with itself.
        update(&mut conn, food.id, food.fields.clone(), now()).unwrap();
        let moved = update(
            &mut conn,
            food.id,
            fields(Some(DINING), BudgetPeriod::Yearly, 999),
            now(),
        )
        .unwrap();
        assert_eq!(
            moved,
            Budget {
                id: food.id,
                fields: fields(Some(DINING), BudgetPeriod::Yearly, 999),
            }
        );
        let result = update(
            &mut conn,
            food.id,
            fields(None, BudgetPeriod::Weekly, 999),
            now(),
        );
        assert_eq!(field_of(result), "period");
        let result = update(
            &mut conn,
            food.id,
            fields(Some(DINING), BudgetPeriod::Yearly, 0),
            now(),
        );
        assert_eq!(field_of(result), "amount");
    }

    #[test]
    fn a_new_budget_cant_use_an_archived_category_but_an_edit_keeps_its_own() {
        let mut conn = db();
        let food = add(&mut conn, Some(FOOD), BudgetPeriod::Monthly);
        categories::archive(&mut conn, FOOD, now()).unwrap();
        let result = create(
            &mut conn,
            fields(Some(FOOD), BudgetPeriod::Weekly, 100),
            now(),
        );
        assert_eq!(field_of(result), "categoryId");
        update(
            &mut conn,
            food.id,
            fields(Some(FOOD), BudgetPeriod::Weekly, 100),
            now(),
        )
        .unwrap();
        categories::archive(&mut conn, DINING, now()).unwrap();
        let result = update(
            &mut conn,
            food.id,
            fields(Some(DINING), BudgetPeriod::Weekly, 100),
            now(),
        );
        assert_eq!(field_of(result), "categoryId");
    }

    #[test]
    fn delete_removes_only_that_budget() {
        let mut conn = db();
        let food = add(&mut conn, Some(FOOD), BudgetPeriod::Monthly);
        add(&mut conn, None, BudgetPeriod::Monthly);
        delete(&mut conn, food.id).unwrap();
        assert_eq!(shown(&conn), [(None, BudgetPeriod::Monthly)]);
    }

    #[test]
    fn a_missing_budget_is_not_found() {
        let mut conn = db();
        let input = fields(None, BudgetPeriod::Monthly, 100);
        assert!(matches!(
            update(&mut conn, 999, input, now()),
            Err(AppError::NotFound { id: 999, .. })
        ));
        assert!(matches!(
            delete(&mut conn, 999),
            Err(AppError::NotFound { id: 999, .. })
        ));
    }

    #[test]
    fn deleting_or_merging_a_category_deletes_its_budgets() {
        let mut conn = db();
        add(&mut conn, Some(FOOD), BudgetPeriod::Monthly);
        add(&mut conn, Some(DINING), BudgetPeriod::Monthly);
        add(&mut conn, Some(DINING), BudgetPeriod::Weekly);
        add(&mut conn, None, BudgetPeriod::Monthly);
        categories::delete(&mut conn, FOOD).unwrap();
        assert_eq!(
            shown(&conn),
            [
                (None, BudgetPeriod::Monthly),
                (Some(DINING), BudgetPeriod::Weekly),
                (Some(DINING), BudgetPeriod::Monthly),
            ]
        );
        // The merged one's budgets go, they don't move, and the target keeps its own.
        add(&mut conn, Some(10), BudgetPeriod::Monthly);
        categories::merge(&mut conn, DINING, 10, now()).unwrap();
        assert_eq!(
            shown(&conn),
            [
                (None, BudgetPeriod::Monthly),
                (Some(10), BudgetPeriod::Monthly)
            ]
        );
    }

    #[test]
    fn a_budget_locks_the_decimals() {
        let mut conn = db();
        assert!(!settings::get(&conn).unwrap().decimals_locked);
        add(&mut conn, None, BudgetPeriod::Monthly);
        assert!(settings::get(&conn).unwrap().decimals_locked);
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("a real test date")
    }

    /// Saves a transaction in the seeded category `category`, from Cash, or a transfer to
    /// `to_account` when it's given.
    fn save(
        conn: &mut Connection,
        kind: TransactionKind,
        category: Option<i64>,
        to_account: Option<i64>,
        amount: i64,
        date: NaiveDate,
    ) {
        let fields = TransactionFields {
            kind,
            amount: Money(amount),
            date: date.to_string(),
            account_id: 1,
            to_account_id: to_account,
            category_id: category,
            payee: None,
            payment_method_id: None,
            note: None,
        };
        transactions::create(conn, fields, now()).unwrap();
    }

    fn spend(conn: &mut Connection, category: i64, amount: i64, date: NaiveDate) {
        save(
            conn,
            TransactionKind::Expense,
            Some(category),
            None,
            amount,
            date,
        );
    }

    /// (start, end, spent, remaining, percent) of each budget, as of `today`.
    fn figures(conn: &Connection, today: NaiveDate) -> Vec<(NaiveDate, NaiveDate, i64, i64, i64)> {
        list(conn, today)
            .unwrap()
            .into_iter()
            .map(|p| (p.start, p.end, p.spent.0, p.remaining.0, p.percent))
            .collect()
    }

    #[test]
    fn progress_counts_the_current_periods_expenses_up_to_today() {
        let mut conn = db();
        let food = fields(Some(FOOD), BudgetPeriod::Monthly, 60_000);
        create(&mut conn, food, now()).unwrap();
        spend(&mut conn, FOOD, 52_040, d(2026, 11, 1));
        // Another category, last month, and a future-dated row don't count (TXN-8).
        spend(&mut conn, DINING, 1_000, d(2026, 11, 5));
        spend(&mut conn, FOOD, 9_999, d(2026, 10, 31));
        spend(&mut conn, FOOD, 9_999, d(2026, 11, 12));
        assert_eq!(
            figures(&conn, d(2026, 11, 11)),
            [(d(2026, 11, 1), d(2026, 11, 30), 52_040, 7_960, 86)]
        );
    }

    #[test]
    fn all_spending_counts_every_category_but_no_income_or_transfer() {
        let mut conn = db();
        create(
            &mut conn,
            fields(None, BudgetPeriod::Monthly, 350_000),
            now(),
        )
        .unwrap();
        spend(&mut conn, FOOD, 200_000, d(2026, 11, 2));
        spend(&mut conn, DINING, 95_040, d(2026, 11, 3));
        // A refund is income: it never lowers what was spent (DATA_MODEL §5.6).
        let refund = Some(SALARY);
        save(
            &mut conn,
            TransactionKind::Income,
            refund,
            None,
            50_000,
            d(2026, 11, 4),
        );
        let bank = NewAccount {
            kind: AccountKind::Bank,
            fields: AccountFields {
                name: "Bank".into(),
                opening_balance: Money(0),
                credit_limit: None,
                statement_day: None,
                due_day: None,
            },
        };
        let bank = accounts::create(&mut conn, bank, now()).unwrap().id;
        save(
            &mut conn,
            TransactionKind::Transfer,
            None,
            Some(bank),
            10_000,
            d(2026, 11, 4),
        );
        assert_eq!(
            figures(&conn, d(2026, 11, 11)),
            [(d(2026, 11, 1), d(2026, 11, 30), 295_040, 54_960, 84)]
        );
    }

    #[test]
    fn past_the_limit_remaining_goes_below_0() {
        let mut conn = db();
        create(
            &mut conn,
            fields(Some(FOOD), BudgetPeriod::Weekly, 15_000),
            now(),
        )
        .unwrap();
        spend(&mut conn, FOOD, 18_000, d(2026, 11, 9));
        // Wednesday Nov 11's week starts on Monday Nov 9.
        assert_eq!(
            figures(&conn, d(2026, 11, 11)),
            [(d(2026, 11, 9), d(2026, 11, 15), 18_000, -3_000, 120)]
        );
    }

    #[test]
    fn nothing_carries_over_to_the_next_period() {
        let mut conn = db();
        create(
            &mut conn,
            fields(Some(FOOD), BudgetPeriod::Weekly, 15_000),
            now(),
        )
        .unwrap();
        spend(&mut conn, FOOD, 5_000, d(2026, 11, 10));
        assert_eq!(
            figures(&conn, d(2026, 11, 11)),
            [(d(2026, 11, 9), d(2026, 11, 15), 5_000, 10_000, 33)]
        );
        // Next week gets its own 15,000, not the 10,000 left over too (BUD-4).
        assert_eq!(
            figures(&conn, d(2026, 11, 16)),
            [(d(2026, 11, 16), d(2026, 11, 22), 0, 15_000, 0)]
        );
    }

    #[test]
    fn periods_follow_the_week_and_month_start_and_years_are_calendar_years() {
        let mut conn = db();
        let mut s = settings::stored(&conn).unwrap();
        s.week_start = settings::WeekStart::Sunday;
        s.month_start_day = 25;
        settings::update(&mut conn, s).unwrap();
        for period in [
            BudgetPeriod::Weekly,
            BudgetPeriod::Monthly,
            BudgetPeriod::Yearly,
        ] {
            create(&mut conn, fields(None, period, 100), now()).unwrap();
        }
        let dates: Vec<_> = figures(&conn, d(2026, 11, 11))
            .into_iter()
            .map(|(start, end, ..)| (start, end))
            .collect();
        assert_eq!(
            dates,
            [
                (d(2026, 11, 8), d(2026, 11, 14)),
                (d(2026, 10, 25), d(2026, 11, 24)),
                (d(2026, 1, 1), d(2026, 12, 31)),
            ]
        );
    }
}
