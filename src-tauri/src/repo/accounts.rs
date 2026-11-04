use rusqlite::{Connection, OptionalExtension, params};

/// One `accounts` row as stored (DATA_MODEL §3.1).
pub struct Row {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub opening_balance: i64,
    pub credit_limit: Option<i64>,
    pub statement_day: Option<u8>,
    pub due_day: Option<u8>,
    pub sort_order: i64,
    pub archived: bool,
}

/// The columns a create or an edit writes.
pub struct Values<'a> {
    pub name: &'a str,
    pub opening_balance: i64,
    pub credit_limit: Option<i64>,
    pub statement_day: Option<u8>,
    pub due_day: Option<u8>,
}

const SELECT: &str = "SELECT id, name, kind, opening_balance, credit_limit, statement_day, due_day, sort_order, archived FROM accounts";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(0)?,
        name: r.get(1)?,
        kind: r.get(2)?,
        opening_balance: r.get(3)?,
        credit_limit: r.get(4)?,
        statement_day: r.get(5)?,
        due_day: r.get(6)?,
        sort_order: r.get(7)?,
        archived: r.get(8)?,
    })
}

/// Every account, archived included, in display order.
pub fn all(conn: &Connection) -> rusqlite::Result<Vec<Row>> {
    conn.prepare(&format!("{SELECT} ORDER BY sort_order, id"))?
        .query_map([], row)?
        .collect()
}

pub fn find(conn: &Connection, id: i64) -> rusqlite::Result<Option<Row>> {
    conn.query_row(&format!("{SELECT} WHERE id = ?1"), [id], row)
        .optional()
}

/// Whether another account already has `name`. The column is `COLLATE NOCASE`, so case is ignored.
pub fn name_taken(conn: &Connection, name: &str, except: Option<i64>) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM accounts WHERE name = ?1 AND id IS NOT ?2)",
        params![name, except],
        |r| r.get(0),
    )
}

/// Adds the account after every other one and returns its id.
pub fn insert(conn: &Connection, kind: &str, v: &Values, now: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO accounts (name, kind, opening_balance, credit_limit, statement_day, due_day, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM accounts), ?7, ?7)",
        params![v.name, kind, v.opening_balance, v.credit_limit, v.statement_day, v.due_day, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, v: &Values, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE accounts SET name = ?2, opening_balance = ?3, credit_limit = ?4, statement_day = ?5,
         due_day = ?6, updated_at = ?7 WHERE id = ?1",
        params![id, v.name, v.opening_balance, v.credit_limit, v.statement_day, v.due_day, now],
    )?;
    Ok(())
}

/// The number of rows changed: 0 when there's no such account.
pub fn set_sort_order(
    conn: &Connection,
    id: i64,
    order: i64,
    now: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE accounts SET sort_order = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, order, now],
    )
}

/// Whether any account, archived or not, stores an amount: an opening balance or a credit limit.
pub fn any_with_amount(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM accounts WHERE opening_balance <> 0 OR credit_limit IS NOT NULL)",
        [],
        |row| row.get(0),
    )
}
