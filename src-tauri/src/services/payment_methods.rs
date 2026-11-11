use chrono::{DateTime, Local};
use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::error::AppError;
use crate::repo;
use crate::services::{invalid, new_order, stamp};

/// One entry of the editable list behind the optional payment method field (TXN-7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PaymentMethod {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    #[ts(type = "number")]
    pub sort_order: i64,
    pub archived: bool,
    /// Whether any transaction uses it. Only an unused method can be deleted.
    pub in_use: bool,
}

/// Every method, archived included, in display order.
pub fn list(conn: &Connection) -> Result<Vec<PaymentMethod>, AppError> {
    Ok(repo::payment_methods::all(conn)?
        .into_iter()
        .map(view)
        .collect())
}

pub fn create(
    conn: &mut Connection,
    name: String,
    now: DateTime<Local>,
) -> Result<PaymentMethod, AppError> {
    let name = valid_name(&name)?;
    let tx = conn.transaction()?;
    check_name_free(&tx, name, None)?;
    let id = repo::payment_methods::insert(&tx, name, &stamp(now))?;
    let method = get(&tx, id)?;
    tx.commit()?;
    Ok(method)
}

pub fn update(
    conn: &mut Connection,
    id: i64,
    name: String,
    now: DateTime<Local>,
) -> Result<PaymentMethod, AppError> {
    let tx = conn.transaction()?;
    get(&tx, id)?;
    let name = valid_name(&name)?;
    check_name_free(&tx, name, Some(id))?;
    repo::payment_methods::rename(&tx, id, name, &stamp(now))?;
    let method = get(&tx, id)?;
    tx.commit()?;
    Ok(method)
}

/// Gives each method in `ids` its position; the ones left out follow in their old order.
pub fn reorder(conn: &mut Connection, ids: Vec<i64>, now: DateTime<Local>) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let existing = repo::payment_methods::all(&tx)?
        .into_iter()
        .map(|row| row.id);
    let order = new_order(ids, existing)?;
    let now = stamp(now);
    for (position, id) in (0_i64..).zip(order) {
        if repo::payment_methods::set_sort_order(&tx, id, position, &now)? == 0 {
            return Err(not_found(id));
        }
    }
    tx.commit()?;
    Ok(())
}

/// Hides the method from the picker; its transactions keep it. The field is optional, so even
/// the last active method can be archived (TXN-7).
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
    if get(&tx, id)?.archived == archived {
        return Ok(());
    }
    repo::payment_methods::set_archived(&tx, id, archived, &stamp(now))?;
    tx.commit()?;
    Ok(())
}

/// Only a method no transaction or recurring rule uses can go; any other is archived instead
/// (TXN-7).
pub fn delete(conn: &mut Connection, id: i64) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    if get(&tx, id)?.in_use {
        return Err(AppError::Conflict {
            message: "This payment method has transactions or recurring rules, so it can only be archived"
                .into(),
        });
    }
    repo::payment_methods::delete(&tx, id)?;
    tx.commit()?;
    Ok(())
}

fn get(conn: &Connection, id: i64) -> Result<PaymentMethod, AppError> {
    repo::payment_methods::find(conn, id)?
        .map(view)
        .ok_or_else(|| not_found(id))
}

/// The trimmed name, 1–40 characters (DATA_MODEL §6).
fn valid_name(name: &str) -> Result<&str, AppError> {
    let name = name.trim();
    if !(1..=40).contains(&name.chars().count()) {
        return Err(invalid("name", "Enter a name of 1 to 40 characters"));
    }
    Ok(name)
}

fn check_name_free(conn: &Connection, name: &str, except: Option<i64>) -> Result<(), AppError> {
    if repo::payment_methods::name_taken(conn, name, except)? {
        return Err(invalid(
            "name",
            "A payment method with this name already exists",
        ));
    }
    Ok(())
}

fn view(row: repo::payment_methods::Row) -> PaymentMethod {
    PaymentMethod {
        id: row.id,
        name: row.name,
        sort_order: row.sort_order,
        archived: row.archived,
        in_use: row.in_use,
    }
}

fn not_found(id: i64) -> AppError {
    super::not_found("payment method", id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{db, now};
    use std::fmt::Debug;

    // The seed (DATA_MODEL §7): Cash, Debit Card, Credit Card, Bank Transfer, E-wallet, Other.
    const CASH: i64 = 1;
    const DEBIT: i64 = 2;
    const OTHER: i64 = 6;

    fn field_of<T: Debug>(result: Result<T, AppError>) -> String {
        match result {
            Err(AppError::Validation { field, .. }) => field,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    fn names(conn: &Connection) -> Vec<String> {
        list(conn).unwrap().into_iter().map(|m| m.name).collect()
    }

    fn find(conn: &Connection, id: i64) -> PaymentMethod {
        list(conn)
            .unwrap()
            .into_iter()
            .find(|m| m.id == id)
            .expect("the method exists")
    }

    /// Inserts an expense paid with `method` directly: the transaction commands come in ROADMAP 1.5.
    fn txn(conn: &Connection, method: i64) {
        conn.execute(
            "INSERT INTO transactions (kind, amount, date, account_id, category_id, payment_method_id, created_at, updated_at)
             VALUES ('expense', 100, '2026-11-01', 1, (SELECT id FROM categories WHERE kind = 'expense' LIMIT 1), ?1, 'x', 'x')",
            [method],
        )
        .unwrap();
    }

    #[test]
    fn the_seed_is_listed_in_order() {
        let conn = db();
        assert_eq!(
            names(&conn),
            [
                "Cash",
                "Debit Card",
                "Credit Card",
                "Bank Transfer",
                "E-wallet",
                "Other"
            ]
        );
        assert_eq!(
            find(&conn, CASH),
            PaymentMethod {
                id: CASH,
                name: "Cash".into(),
                sort_order: 0,
                archived: false,
                in_use: false,
            }
        );
    }

    #[test]
    fn create_adds_after_the_others() {
        let mut conn = db();
        let gcash = create(&mut conn, " GCash ".into(), now()).unwrap();
        assert_eq!(
            gcash,
            PaymentMethod {
                id: 7,
                name: "GCash".into(),
                sort_order: 6,
                archived: false,
                in_use: false,
            }
        );
        assert_eq!(names(&conn).last().unwrap(), "GCash");
    }

    #[test]
    fn names_are_trimmed_1_to_40_and_unique_ignoring_case() {
        let mut conn = db();
        // 40 characters, but 80 bytes.
        assert!(create(&mut conn, "ñ".repeat(40), now()).is_ok());
        for bad in [
            "".to_string(),
            "   ".into(),
            "a".repeat(41),
            " cash ".into(),
        ] {
            assert_eq!(
                field_of(create(&mut conn, bad.clone(), now())),
                "name",
                "{bad:?}"
            );
        }
        archive(&mut conn, OTHER, now()).unwrap();
        assert_eq!(
            field_of(create(&mut conn, "OTHER".into(), now())),
            "name",
            "archived names still count"
        );
        assert_eq!(
            field_of(update(&mut conn, CASH, "debit card".into(), now())),
            "name"
        );
        assert_eq!(field_of(update(&mut conn, CASH, "".into(), now())), "name");
        // Its own name in a different case is fine.
        assert_eq!(
            update(&mut conn, CASH, "CASH".into(), now()).unwrap().name,
            "CASH"
        );
    }

    #[test]
    fn update_renames_it() {
        let mut conn = db();
        let later = now() + chrono::Duration::days(1);
        let saved = update(&mut conn, OTHER, " Paypal ".into(), later).unwrap();
        assert_eq!(saved.name, "Paypal");
        assert_eq!(find(&conn, OTHER), saved);
        let updated: String = conn
            .query_row(
                "SELECT updated_at FROM payment_methods WHERE id = ?1",
                [OTHER],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(updated, stamp(later));
    }

    #[test]
    fn reorder_sets_the_order_given() {
        let mut conn = db();
        let mut expected = names(&conn);
        reorder(&mut conn, (1..=6).rev().collect(), now()).unwrap();
        expected.reverse();
        assert_eq!(names(&conn), expected);
        assert!(matches!(
            reorder(&mut conn, vec![CASH, 99], now()),
            Err(AppError::NotFound { id: 99, .. })
        ));
        assert_eq!(field_of(reorder(&mut conn, vec![CASH, CASH], now())), "ids");
        assert_eq!(names(&conn), expected, "a rejected reorder changes nothing");
    }

    #[test]
    fn every_method_can_be_archived_and_unarchived() {
        let mut conn = db();
        // The field is optional (TXN-7), so even the last active one can go.
        for id in 1..=6 {
            archive(&mut conn, id, now()).unwrap();
        }
        assert!(list(&conn).unwrap().iter().all(|m| m.archived));
        archive(&mut conn, CASH, now()).unwrap();
        unarchive(&mut conn, CASH, now()).unwrap();
        assert!(!find(&conn, CASH).archived);
        unarchive(&mut conn, CASH, now()).unwrap();
        assert!(!find(&conn, CASH).archived);
    }

    #[test]
    fn only_an_unused_method_can_be_deleted() {
        let mut conn = db();
        txn(&conn, CASH);
        assert!(find(&conn, CASH).in_use);
        assert!(!find(&conn, DEBIT).in_use);
        match delete(&mut conn, CASH) {
            Err(AppError::Conflict { message }) => {
                assert!(message.contains("archived"), "{message}")
            }
            other => panic!("expected a conflict, got {other:?}"),
        }
        delete(&mut conn, DEBIT).unwrap();
        assert_eq!(names(&conn).len(), 5);
        assert!(names(&conn).contains(&"Cash".to_string()));
    }

    #[test]
    fn missing_methods_are_not_found() {
        let mut conn = db();
        let not_found = |result: Result<(), AppError>| {
            assert!(
                matches!(result, Err(AppError::NotFound { ref entity, id: 99 }) if entity == "payment method"),
                "{result:?}"
            );
        };
        not_found(update(&mut conn, 99, "Gone".into(), now()).map(|_| ()));
        not_found(archive(&mut conn, 99, now()));
        not_found(unarchive(&mut conn, 99, now()));
        not_found(delete(&mut conn, 99));
    }
}
