use rusqlite::{Connection, OptionalExtension, params};

/// One `loans` row as stored (DATA_MODEL §4.5).
pub struct Row {
    pub id: i64,
    pub name: String,
    pub account_id: i64,
    pub category_id: i64,
    pub is_new: bool,
    pub amount_borrowed: Option<i64>,
    pub borrowed_on: Option<String>,
    pub added_on: String,
    pub archived: bool,
}

/// One `loan_payments` row as stored (DATA_MODEL §4.6), with its transaction's amount once paid.
pub struct PaymentRow {
    pub id: i64,
    pub loan_id: i64,
    pub due_date: String,
    pub amount: i64,
    pub paid_before: bool,
    pub transaction_id: Option<i64>,
    pub paid_amount: Option<i64>,
}

/// The columns a create or an edit writes. `is_new` and `added_on` are written once, on insert.
pub struct Values<'a> {
    pub name: &'a str,
    pub account_id: i64,
    pub category_id: i64,
    pub amount_borrowed: Option<i64>,
    pub borrowed_on: Option<&'a str>,
}

/// A loan always has a payment, so this covers every loan amount the decimals lock counts.
pub fn any(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row("SELECT EXISTS (SELECT 1 FROM loans)", [], |r| r.get(0))
}

const SELECT: &str = "SELECT id, name, account_id, category_id, is_new, amount_borrowed,
            borrowed_on, added_on, archived FROM loans";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(0)?,
        name: r.get(1)?,
        account_id: r.get(2)?,
        category_id: r.get(3)?,
        is_new: r.get(4)?,
        amount_borrowed: r.get(5)?,
        borrowed_on: r.get(6)?,
        added_on: r.get(7)?,
        archived: r.get(8)?,
    })
}

/// In the order they were added.
pub fn all(conn: &Connection) -> rusqlite::Result<Vec<Row>> {
    conn.prepare(&format!("{SELECT} ORDER BY id"))?
        .query_map([], row)?
        .collect()
}

pub fn find(conn: &Connection, id: i64) -> rusqlite::Result<Option<Row>> {
    conn.query_row(&format!("{SELECT} WHERE id = ?1"), [id], row)
        .optional()
}

/// Ignoring case, as the column's NOCASE does.
pub fn name_taken(conn: &Connection, name: &str, except: Option<i64>) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM loans WHERE name = ?1 AND id IS NOT ?2)",
        params![name, except],
        |r| r.get(0),
    )
}

pub fn insert(
    conn: &Connection,
    v: &Values,
    is_new: bool,
    added_on: &str,
    now: &str,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO loans (name, account_id, category_id, is_new, amount_borrowed, borrowed_on,
                            added_on, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![
            v.name,
            v.account_id,
            v.category_id,
            is_new,
            v.amount_borrowed,
            v.borrowed_on,
            added_on,
            now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, v: &Values, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE loans SET name = ?2, account_id = ?3, category_id = ?4, amount_borrowed = ?5,
                          borrowed_on = ?6, updated_at = ?7
         WHERE id = ?1",
        params![
            id,
            v.name,
            v.account_id,
            v.category_id,
            v.amount_borrowed,
            v.borrowed_on,
            now
        ],
    )?;
    Ok(())
}

pub fn set_archived(conn: &Connection, id: i64, archived: bool, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE loans SET archived = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, archived, now],
    )?;
    Ok(())
}

/// An archived loan never hides money still owed (DATA_MODEL §4.5).
pub fn unarchive_if_unpaid(conn: &Connection, id: i64, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE loans SET archived = 0, updated_at = ?2
         WHERE id = ?1 AND archived = 1 AND EXISTS (
             SELECT 1 FROM loan_payments p
             WHERE p.loan_id = ?1 AND p.transaction_id IS NULL AND p.paid_before = 0)",
        params![id, now],
    )?;
    Ok(())
}

/// Its payments go with it (`ON DELETE CASCADE`).
pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM loans WHERE id = ?1", [id])?;
    Ok(())
}

const SELECT_PAYMENTS: &str =
    "SELECT p.id, p.loan_id, p.due_date, p.amount, p.paid_before, p.transaction_id, t.amount
     FROM loan_payments p LEFT JOIN transactions t ON t.id = p.transaction_id";

fn payment(r: &rusqlite::Row) -> rusqlite::Result<PaymentRow> {
    Ok(PaymentRow {
        id: r.get(0)?,
        loan_id: r.get(1)?,
        due_date: r.get(2)?,
        amount: r.get(3)?,
        paid_before: r.get(4)?,
        transaction_id: r.get(5)?,
        paid_amount: r.get(6)?,
    })
}

/// Every loan's, by loan, then `due_date`, then `id` (DATA_MODEL §4.6).
pub fn payments(conn: &Connection) -> rusqlite::Result<Vec<PaymentRow>> {
    conn.prepare(&format!(
        "{SELECT_PAYMENTS} ORDER BY p.loan_id, p.due_date, p.id"
    ))?
    .query_map([], payment)?
    .collect()
}

pub fn payments_of(conn: &Connection, loan_id: i64) -> rusqlite::Result<Vec<PaymentRow>> {
    conn.prepare(&format!(
        "{SELECT_PAYMENTS} WHERE p.loan_id = ?1 ORDER BY p.due_date, p.id"
    ))?
    .query_map([loan_id], payment)?
    .collect()
}

pub fn insert_payment(
    conn: &Connection,
    loan_id: i64,
    due_date: &str,
    amount: i64,
    paid_before: bool,
    now: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO loan_payments (loan_id, due_date, amount, paid_before, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![loan_id, due_date, amount, paid_before, now],
    )?;
    Ok(())
}

/// Only a payment without a transaction: a paid one is never changed by a save (§4.6).
pub fn update_payment(
    conn: &Connection,
    id: i64,
    due_date: &str,
    amount: i64,
    paid_before: bool,
    now: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE loan_payments SET due_date = ?2, amount = ?3, paid_before = ?4, updated_at = ?5
         WHERE id = ?1 AND transaction_id IS NULL",
        params![id, due_date, amount, paid_before, now],
    )?;
    Ok(())
}

pub fn delete_payment(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM loan_payments WHERE id = ?1 AND transaction_id IS NULL",
        [id],
    )?;
    Ok(())
}
