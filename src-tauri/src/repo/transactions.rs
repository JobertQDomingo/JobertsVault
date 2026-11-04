use rusqlite::{Connection, OptionalExtension, params};

/// Whether any transaction exists at all.
pub fn any(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row("SELECT EXISTS (SELECT 1 FROM transactions)", [], |row| {
        row.get(0)
    })
}

/// One `transactions` row as stored (DATA_MODEL §3.4).
pub struct Row {
    pub id: i64,
    pub kind: String,
    pub amount: i64,
    pub date: String,
    pub account_id: i64,
    pub to_account_id: Option<i64>,
    pub category_id: Option<i64>,
    pub payee: Option<String>,
    pub payment_method_id: Option<i64>,
    pub note: Option<String>,
}

/// The columns a create or an edit writes.
pub struct Values<'a> {
    pub kind: &'a str,
    pub amount: i64,
    pub date: &'a str,
    pub account_id: i64,
    pub to_account_id: Option<i64>,
    pub category_id: Option<i64>,
    pub payee: Option<&'a str>,
    pub payment_method_id: Option<i64>,
    pub note: Option<&'a str>,
}

const SELECT: &str = "SELECT id, kind, amount, date, account_id, to_account_id, category_id,
            payee, payment_method_id, note
     FROM transactions";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(0)?,
        kind: r.get(1)?,
        amount: r.get(2)?,
        date: r.get(3)?,
        account_id: r.get(4)?,
        to_account_id: r.get(5)?,
        category_id: r.get(6)?,
        payee: r.get(7)?,
        payment_method_id: r.get(8)?,
        note: r.get(9)?,
    })
}

pub fn find(conn: &Connection, id: i64) -> rusqlite::Result<Option<Row>> {
    conn.query_row(&format!("{SELECT} WHERE id = ?1"), [id], row)
        .optional()
}

pub fn insert(conn: &Connection, v: &Values, now: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO transactions (kind, amount, date, account_id, to_account_id, category_id,
                                   payee, payment_method_id, note, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![
            v.kind,
            v.amount,
            v.date,
            v.account_id,
            v.to_account_id,
            v.category_id,
            v.payee,
            v.payment_method_id,
            v.note,
            now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, v: &Values, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE transactions SET kind = ?2, amount = ?3, date = ?4, account_id = ?5,
                to_account_id = ?6, category_id = ?7, payee = ?8, payment_method_id = ?9,
                note = ?10, updated_at = ?11
         WHERE id = ?1",
        params![
            id,
            v.kind,
            v.amount,
            v.date,
            v.account_id,
            v.to_account_id,
            v.category_id,
            v.payee,
            v.payment_method_id,
            v.note,
            now
        ],
    )?;
    Ok(())
}

/// The source account of the latest transaction whose source is an active non-card account.
pub fn latest_non_card_source(conn: &Connection) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        "SELECT t.account_id FROM transactions t JOIN accounts a ON a.id = t.account_id
         WHERE a.archived = 0 AND a.kind <> 'credit_card'
         ORDER BY t.date DESC, t.id DESC LIMIT 1",
        [],
        |r| r.get(0),
    )
    .optional()
}

/// The number of rows deleted: 0 when there's no such transaction.
pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM transactions WHERE id = ?1", [id])
}
