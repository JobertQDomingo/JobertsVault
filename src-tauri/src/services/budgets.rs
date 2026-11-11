use chrono::{DateTime, Local};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::money::Money;
use crate::error::AppError;
use crate::repo;
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

/// Every budget: all spending first, then in the categories' order, weekly before monthly
/// before yearly.
pub fn list(conn: &Connection) -> Result<Vec<Budget>, AppError> {
    repo::budgets::all(conn)?.into_iter().map(view).collect()
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
        list(conn)
            .unwrap()
            .into_iter()
            .map(|b| (b.fields.category_id, b.fields.period))
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
        assert_eq!(list(&conn).unwrap(), [all, food]);
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
        // Into a category with no budget: the merged one's budgets go, they don't move.
        categories::merge(&mut conn, DINING, 10, now()).unwrap();
        assert_eq!(shown(&conn), [(None, BudgetPeriod::Monthly)]);
    }

    #[test]
    fn a_budget_locks_the_decimals() {
        let mut conn = db();
        assert!(!settings::get(&conn).unwrap().decimals_locked);
        add(&mut conn, None, BudgetPeriod::Monthly);
        assert!(settings::get(&conn).unwrap().decimals_locked);
    }
}
