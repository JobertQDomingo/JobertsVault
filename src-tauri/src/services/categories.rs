use chrono::{DateTime, Local};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::category::{ICONS, PALETTE, least_used_color};
use crate::error::AppError;
use crate::repo;
use crate::services::{invalid, new_order, stamp};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CategoryKind {
    Income,
    Expense,
}

/// What the new and edit category forms send. The kind can't change once created, because the
/// category's transactions must keep matching it (CAT-3).
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CategoryFields {
    pub name: String,
    /// A palette key, `cat-1` … `cat-12` (DESIGN_SYSTEM §3.4).
    pub color: String,
    /// A name from `src/lib/categoryIcons.ts` (DESIGN_SYSTEM §6).
    pub icon: String,
}

#[derive(Clone, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NewCategory {
    pub kind: CategoryKind,
    #[serde(flatten)]
    pub fields: CategoryFields,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Category {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub kind: CategoryKind,
    pub color: String,
    pub icon: String,
    #[ts(type = "number")]
    pub sort_order: i64,
    pub archived: bool,
    /// Merging moves them all (CAT-4).
    #[ts(type = "number")]
    pub transaction_count: i64,
    /// A transaction or recurring rule uses it, so it can't be deleted (CAT-4).
    pub in_use: bool,
}

/// The color a new category of each kind starts with (DATA_MODEL §7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DefaultColors {
    pub income: String,
    pub expense: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CategoryList {
    pub categories: Vec<Category>,
    pub default_colors: DefaultColors,
}

/// Every category, archived included, in display order.
pub fn list(conn: &Connection) -> Result<CategoryList, AppError> {
    let categories = repo::categories::all(conn)?
        .into_iter()
        .map(view)
        .collect::<Result<Vec<_>, _>>()?;
    let default_color = |kind| {
        least_used_color(
            categories
                .iter()
                .filter(|c| c.kind == kind && !c.archived)
                .map(|c| c.color.as_str()),
        )
        .to_owned()
    };
    let default_colors = DefaultColors {
        income: default_color(CategoryKind::Income),
        expense: default_color(CategoryKind::Expense),
    };
    Ok(CategoryList {
        categories,
        default_colors,
    })
}

pub fn create(
    conn: &mut Connection,
    mut input: NewCategory,
    now: DateTime<Local>,
) -> Result<Category, AppError> {
    validate(&mut input.fields)?;
    let kind = kind_text(input.kind);
    let tx = conn.transaction()?;
    check_name_free(&tx, kind, &input.fields.name, None)?;
    let id = repo::categories::insert(&tx, kind, &values(&input.fields), &stamp(now))?;
    let category = get(&tx, id)?;
    tx.commit()?;
    Ok(category)
}

pub fn update(
    conn: &mut Connection,
    id: i64,
    mut input: CategoryFields,
    now: DateTime<Local>,
) -> Result<Category, AppError> {
    let tx = conn.transaction()?;
    let kind = stored(&tx, id)?.kind;
    validate(&mut input)?;
    check_name_free(&tx, &kind, &input.name, Some(id))?;
    repo::categories::update(&tx, id, &values(&input), &stamp(now))?;
    let category = get(&tx, id)?;
    tx.commit()?;
    Ok(category)
}

/// Gives each category in `ids` its position. The settings tab drags one kind's list at a time,
/// and the other kind keeps its order after them.
pub fn reorder(conn: &mut Connection, ids: Vec<i64>, now: DateTime<Local>) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let existing = repo::categories::all(&tx)?.into_iter().map(|row| row.id);
    let order = new_order(ids, existing)?;
    let now = stamp(now);
    for (position, id) in (0_i64..).zip(order) {
        if repo::categories::set_sort_order(&tx, id, position, &now)? == 0 {
            return Err(not_found(id));
        }
    }
    tx.commit()?;
    Ok(())
}

/// Hides the category from pickers; its transactions keep it (CAT-2).
pub fn archive(conn: &mut Connection, id: i64, now: DateTime<Local>) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let row = stored(&tx, id)?;
    if row.archived {
        return Ok(());
    }
    check_not_last_active(&tx, &row.kind)?;
    repo::categories::set_archived(&tx, id, true, &stamp(now))?;
    tx.commit()?;
    Ok(())
}

pub fn unarchive(conn: &mut Connection, id: i64, now: DateTime<Local>) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    if !stored(&tx, id)?.archived {
        return Ok(());
    }
    repo::categories::set_archived(&tx, id, false, &stamp(now))?;
    tx.commit()?;
    Ok(())
}

/// Only a category no transaction or recurring rule uses can go; any other is archived or merged
/// (CAT-4). Its budgets go with it (DATA_MODEL §4.3).
pub fn delete(conn: &mut Connection, id: i64) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let row = stored(&tx, id)?;
    if row.transaction_count > 0 {
        return Err(AppError::Conflict {
            message: "This category has transactions, so it can only be archived or merged".into(),
        });
    }
    if row.in_rule {
        return Err(AppError::Conflict {
            message: "A recurring rule uses this category, so it can only be archived or merged"
                .into(),
        });
    }
    if !row.archived {
        check_not_last_active(&tx, &row.kind)?;
    }
    repo::budgets::delete_for_category(&tx, id)?;
    repo::categories::delete(&tx, id)?;
    tx.commit()?;
    Ok(())
}

/// Moves every transaction and recurring rule of `from_id` to the active category `into_id` of
/// the same kind, then deletes `from_id` and its budgets, as one change (DATA_MODEL §3.2, §4.3,
/// UI_FLOWS F7).
pub fn merge(
    conn: &mut Connection,
    from_id: i64,
    into_id: i64,
    now: DateTime<Local>,
) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let from = stored(&tx, from_id)?;
    let into = stored(&tx, into_id)?;
    if from_id == into_id {
        return Err(invalid("intoId", "Choose a different category"));
    }
    if into.kind != from.kind {
        return Err(invalid(
            "intoId",
            &format!("Choose an {} category", from.kind),
        ));
    }
    if into.archived {
        return Err(invalid("intoId", "Choose an active category"));
    }
    repo::categories::move_uses(&tx, from_id, into_id, &stamp(now))?;
    repo::budgets::delete_for_category(&tx, from_id)?;
    repo::categories::delete(&tx, from_id)?;
    tx.commit()?;
    Ok(())
}

fn get(conn: &Connection, id: i64) -> Result<Category, AppError> {
    view(stored(conn, id)?)
}

fn stored(conn: &Connection, id: i64) -> Result<repo::categories::Row, AppError> {
    repo::categories::find(conn, id)?.ok_or_else(|| not_found(id))
}

/// Called before an active category is archived or deleted (CAT-2).
fn check_not_last_active(conn: &Connection, kind: &str) -> Result<(), AppError> {
    if repo::categories::active_count(conn, kind)? <= 1 {
        return Err(AppError::Conflict {
            message: format!("Keep at least one active {kind} category"),
        });
    }
    Ok(())
}

/// Trims the name and checks every field (DATA_MODEL §6).
fn validate(fields: &mut CategoryFields) -> Result<(), AppError> {
    fields.name = fields.name.trim().to_owned();
    if !(1..=40).contains(&fields.name.chars().count()) {
        return Err(invalid("name", "Enter a name of 1 to 40 characters"));
    }
    if !PALETTE.contains(&fields.color.as_str()) {
        return Err(invalid("color", "Choose one of the 12 colors"));
    }
    if !ICONS.contains(&fields.icon.as_str()) {
        return Err(invalid("icon", "Choose an icon from the list"));
    }
    Ok(())
}

fn values(fields: &CategoryFields) -> repo::categories::Values<'_> {
    repo::categories::Values {
        name: &fields.name,
        color: &fields.color,
        icon: &fields.icon,
    }
}

fn check_name_free(
    conn: &Connection,
    kind: &str,
    name: &str,
    except: Option<i64>,
) -> Result<(), AppError> {
    if repo::categories::name_taken(conn, kind, name, except)? {
        return Err(invalid(
            "name",
            &format!("An {kind} category with this name already exists"),
        ));
    }
    Ok(())
}

fn view(row: repo::categories::Row) -> Result<Category, AppError> {
    let kind = [CategoryKind::Income, CategoryKind::Expense]
        .into_iter()
        .find(|k| kind_text(*k) == row.kind)
        .ok_or_else(|| AppError::Database {
            message: format!("Category {} has an unknown kind", row.id),
        })?;
    Ok(Category {
        id: row.id,
        name: row.name,
        kind,
        color: row.color,
        icon: row.icon,
        sort_order: row.sort_order,
        archived: row.archived,
        transaction_count: row.transaction_count,
        in_use: row.transaction_count > 0 || row.in_rule,
    })
}

pub(crate) fn kind_text(kind: CategoryKind) -> &'static str {
    match kind {
        CategoryKind::Income => "income",
        CategoryKind::Expense => "expense",
    }
}

fn not_found(id: i64) -> AppError {
    super::not_found("category", id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{db, now};
    use std::fmt::Debug;

    // The seed (DATA_MODEL §7) gives income ids 1–7 (Salary, Bonus, …) and expense ids 8–24
    // (Food & Groceries, Dining Out, …).
    const SALARY: i64 = 1;
    const BONUS: i64 = 2;
    const FOOD: i64 = 8;
    const DINING: i64 = 9;

    fn field_of<T: Debug>(result: Result<T, AppError>) -> String {
        match result {
            Err(AppError::Validation { field, .. }) => field,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    fn conflict<T: Debug>(result: Result<T, AppError>) -> String {
        match result {
            Err(AppError::Conflict { message }) => message,
            other => panic!("expected a conflict, got {other:?}"),
        }
    }

    fn fields(name: &str, color: &str, icon: &str) -> CategoryFields {
        CategoryFields {
            name: name.into(),
            color: color.into(),
            icon: icon.into(),
        }
    }

    fn new(kind: CategoryKind, name: &str) -> NewCategory {
        NewCategory {
            kind,
            fields: fields(name, "cat-1", "coffee"),
        }
    }

    fn names(conn: &Connection, kind: CategoryKind) -> Vec<String> {
        list(conn)
            .unwrap()
            .categories
            .into_iter()
            .filter(|c| c.kind == kind)
            .map(|c| c.name)
            .collect()
    }

    fn find(conn: &Connection, id: i64) -> Category {
        list(conn)
            .unwrap()
            .categories
            .into_iter()
            .find(|c| c.id == id)
            .expect("the category exists")
    }

    /// Inserts a transaction directly: the transaction commands come in ROADMAP 1.5.
    fn txn(conn: &Connection, category: i64) {
        conn.execute(
            "INSERT INTO transactions (kind, amount, date, account_id, category_id, created_at, updated_at)
             VALUES ((SELECT kind FROM categories WHERE id = ?1), 100, '2026-11-01', 1, ?1, 'x', 'x')",
            [category],
        )
        .unwrap();
    }

    #[test]
    fn the_seed_is_listed_with_default_colors() {
        let conn = db();
        let all = list(&conn).unwrap();
        assert_eq!(all.categories.len(), 24);
        assert_eq!(
            all.categories[0],
            Category {
                id: SALARY,
                name: "Salary".into(),
                kind: CategoryKind::Income,
                color: "cat-12".into(),
                icon: "briefcase".into(),
                sort_order: 0,
                archived: false,
                transaction_count: 0,
                in_use: false,
            }
        );
        assert_eq!(
            names(&conn, CategoryKind::Expense)[..3],
            ["Food & Groceries", "Dining Out", "Transportation"]
        );
        // Income leaves cat-1 unused; every expense color is used and cat-2 is the first used once.
        assert_eq!(
            all.default_colors,
            DefaultColors {
                income: "cat-1".into(),
                expense: "cat-2".into(),
            }
        );
    }

    #[test]
    fn the_default_color_ignores_archived_categories() {
        let mut conn = db();
        archive(&mut conn, DINING, now()).unwrap();
        assert_eq!(list(&conn).unwrap().default_colors.expense, "cat-6");
    }

    #[test]
    fn create_adds_after_the_others_of_its_kind() {
        let mut conn = db();
        let snacks = create(
            &mut conn,
            NewCategory {
                kind: CategoryKind::Expense,
                fields: fields(" Snacks ", "cat-6", "coffee"),
            },
            now(),
        )
        .unwrap();
        assert_eq!(
            snacks,
            Category {
                id: 25,
                name: "Snacks".into(),
                kind: CategoryKind::Expense,
                color: "cat-6".into(),
                icon: "coffee".into(),
                sort_order: 17,
                archived: false,
                transaction_count: 0,
                in_use: false,
            }
        );
        assert_eq!(find(&conn, snacks.id), snacks);
        let tips = create(&mut conn, new(CategoryKind::Income, "Tips"), now()).unwrap();
        assert_eq!(tips.sort_order, 7);
        assert_eq!(names(&conn, CategoryKind::Income).last().unwrap(), "Tips");
    }

    #[test]
    fn names_are_trimmed_1_to_40_and_unique_within_a_kind() {
        let mut conn = db();
        // 40 characters, but 80 bytes.
        assert!(
            create(
                &mut conn,
                new(CategoryKind::Expense, &"ñ".repeat(40)),
                now()
            )
            .is_ok()
        );
        for bad in ["".to_string(), "   ".into(), "a".repeat(41)] {
            assert_eq!(
                field_of(create(&mut conn, new(CategoryKind::Expense, &bad), now())),
                "name",
                "{bad:?}"
            );
        }
        // The other kind may use the same name.
        assert!(create(&mut conn, new(CategoryKind::Expense, "Salary"), now()).is_ok());
        assert_eq!(
            field_of(create(
                &mut conn,
                new(CategoryKind::Income, " salary "),
                now()
            )),
            "name"
        );
        archive(&mut conn, BONUS, now()).unwrap();
        assert_eq!(
            field_of(create(&mut conn, new(CategoryKind::Income, "BONUS"), now())),
            "name",
            "archived names still count"
        );
        assert_eq!(
            field_of(update(
                &mut conn,
                SALARY,
                fields("bonus", "cat-1", "coins"),
                now()
            )),
            "name"
        );
        // Its own name in a different case is fine.
        let renamed = update(&mut conn, SALARY, fields("SALARY", "cat-1", "coins"), now()).unwrap();
        assert_eq!(renamed.name, "SALARY");
    }

    #[test]
    fn color_and_icon_must_be_from_the_lists() {
        let mut conn = db();
        let with = |color: &str, icon: &str| NewCategory {
            kind: CategoryKind::Expense,
            fields: fields("Snacks", color, icon),
        };
        for bad in ["#ff0000", "cat-0", "cat-13", ""] {
            assert_eq!(
                field_of(create(&mut conn, with(bad, "coffee"), now())),
                "color",
                "{bad:?}"
            );
            assert_eq!(
                field_of(update(
                    &mut conn,
                    FOOD,
                    fields("Food", bad, "coffee"),
                    now()
                )),
                "color"
            );
        }
        for bad in ["rocket", "", "Coffee"] {
            assert_eq!(
                field_of(create(&mut conn, with("cat-1", bad), now())),
                "icon",
                "{bad:?}"
            );
            assert_eq!(
                field_of(update(&mut conn, FOOD, fields("Food", "cat-1", bad), now())),
                "icon"
            );
        }
        assert!(create(&mut conn, with("cat-12", "wifi"), now()).is_ok());
    }

    #[test]
    fn update_changes_the_name_color_and_icon_but_not_the_kind() {
        let mut conn = db();
        let later = now() + chrono::Duration::days(1);
        let saved = update(&mut conn, SALARY, fields(" Pay ", "cat-5", "coins"), later).unwrap();
        assert_eq!(
            (
                saved.name.as_str(),
                saved.color.as_str(),
                saved.icon.as_str(),
                saved.kind
            ),
            ("Pay", "cat-5", "coins", CategoryKind::Income)
        );
        assert_eq!(find(&conn, SALARY), saved);
        let updated: String = conn
            .query_row(
                "SELECT updated_at FROM categories WHERE id = ?1",
                [SALARY],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(updated, stamp(later));
    }

    #[test]
    fn reordering_one_kind_keeps_the_other_in_order() {
        let mut conn = db();
        let expense_before = names(&conn, CategoryKind::Expense);
        let mut income = names(&conn, CategoryKind::Income);
        reorder(&mut conn, (1..=7).rev().collect(), now()).unwrap();
        income.reverse();
        assert_eq!(names(&conn, CategoryKind::Income), income);
        assert_eq!(names(&conn, CategoryKind::Expense), expense_before);

        assert!(matches!(
            reorder(&mut conn, vec![SALARY, 99], now()),
            Err(AppError::NotFound { id: 99, .. })
        ));
        assert_eq!(
            field_of(reorder(&mut conn, vec![SALARY, SALARY], now())),
            "ids"
        );
        assert_eq!(
            names(&conn, CategoryKind::Income),
            income,
            "a rejected reorder changes nothing"
        );
    }

    #[test]
    fn archive_and_unarchive() {
        let mut conn = db();
        archive(&mut conn, FOOD, now()).unwrap();
        assert!(find(&conn, FOOD).archived);
        archive(&mut conn, FOOD, now()).unwrap();
        assert!(
            find(&conn, FOOD).archived,
            "archiving twice changes nothing"
        );
        unarchive(&mut conn, FOOD, now()).unwrap();
        assert!(!find(&conn, FOOD).archived);
        unarchive(&mut conn, FOOD, now()).unwrap();
        assert!(!find(&conn, FOOD).archived);
    }

    #[test]
    fn the_last_active_category_of_a_kind_cannot_be_archived() {
        let mut conn = db();
        for id in 2..=7 {
            archive(&mut conn, id, now()).unwrap();
        }
        assert_eq!(
            conflict(archive(&mut conn, SALARY, now())),
            "Keep at least one active income category"
        );
        assert!(!find(&conn, SALARY).archived);
        // Expense categories are counted separately.
        archive(&mut conn, FOOD, now()).unwrap();
    }

    #[test]
    fn missing_categories_are_not_found() {
        let mut conn = db();
        let not_found = |result: Result<(), AppError>| {
            assert!(
                matches!(result, Err(AppError::NotFound { ref entity, id: 99 }) if entity == "category"),
                "{result:?}"
            );
        };
        not_found(update(&mut conn, 99, fields("Gone", "cat-1", "coffee"), now()).map(|_| ()));
        not_found(archive(&mut conn, 99, now()));
        not_found(unarchive(&mut conn, 99, now()));
    }

    #[test]
    fn transaction_count_counts_its_transactions() {
        let conn = db();
        txn(&conn, FOOD);
        txn(&conn, FOOD);
        txn(&conn, SALARY);
        assert_eq!(find(&conn, FOOD).transaction_count, 2);
        assert_eq!(find(&conn, SALARY).transaction_count, 1);
        assert_eq!(find(&conn, DINING).transaction_count, 0);
    }

    fn gone(conn: &Connection, id: i64) -> bool {
        list(conn).unwrap().categories.iter().all(|c| c.id != id)
    }

    #[test]
    fn an_unused_category_can_be_deleted() {
        let mut conn = db();
        delete(&mut conn, FOOD).unwrap();
        assert!(gone(&conn, FOOD));
    }

    #[test]
    fn a_category_with_transactions_can_only_be_archived_or_merged() {
        let mut conn = db();
        txn(&conn, FOOD);
        let message = conflict(delete(&mut conn, FOOD));
        assert!(message.contains("merged"), "{message}");
        assert!(!gone(&conn, FOOD));
    }

    #[test]
    fn the_last_active_category_of_a_kind_cannot_be_deleted() {
        let mut conn = db();
        for id in 3..=7 {
            delete(&mut conn, id).unwrap();
        }
        archive(&mut conn, BONUS, now()).unwrap();
        assert_eq!(
            conflict(delete(&mut conn, SALARY)),
            "Keep at least one active income category"
        );
        // An archived one can still go, since Salary stays active.
        delete(&mut conn, BONUS).unwrap();
        assert!(!gone(&conn, SALARY));
    }

    fn stamps_of(conn: &Connection, category: i64) -> Vec<String> {
        conn.prepare("SELECT updated_at FROM transactions WHERE category_id = ?1 ORDER BY id")
            .unwrap()
            .query_map([category], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn merge_moves_every_transaction_and_deletes_the_source() {
        let mut conn = db();
        txn(&conn, DINING);
        for _ in 0..3 {
            txn(&conn, FOOD);
        }
        let later = now() + chrono::Duration::days(1);
        merge(&mut conn, FOOD, DINING, later).unwrap();
        assert!(gone(&conn, FOOD));
        assert_eq!(find(&conn, DINING).transaction_count, 4);
        let moved = stamp(later);
        assert_eq!(
            stamps_of(&conn, DINING),
            ["x", moved.as_str(), moved.as_str(), moved.as_str()],
            "only the moved rows change"
        );
    }

    #[test]
    fn merge_rejects_itself_other_kinds_and_archived_targets() {
        let mut conn = db();
        txn(&conn, FOOD);
        archive(&mut conn, DINING, now()).unwrap();
        for into in [FOOD, SALARY, DINING] {
            assert_eq!(
                field_of(merge(&mut conn, FOOD, into, now())),
                "intoId",
                "{into}"
            );
        }
        assert_eq!(find(&conn, FOOD).transaction_count, 1, "nothing moved");
    }

    #[test]
    fn an_archived_category_can_be_merged() {
        let mut conn = db();
        txn(&conn, FOOD);
        archive(&mut conn, FOOD, now()).unwrap();
        merge(&mut conn, FOOD, DINING, now()).unwrap();
        assert!(gone(&conn, FOOD));
        assert_eq!(find(&conn, DINING).transaction_count, 1);
    }

    #[test]
    fn delete_and_merge_of_a_missing_category_are_not_found() {
        let mut conn = db();
        let not_found = |result: Result<(), AppError>| {
            assert!(
                matches!(result, Err(AppError::NotFound { id: 99, .. })),
                "{result:?}"
            );
        };
        not_found(delete(&mut conn, 99));
        not_found(merge(&mut conn, 99, DINING, now()));
        not_found(merge(&mut conn, FOOD, 99, now()));
    }
}
