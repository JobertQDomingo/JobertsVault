use rusqlite::{Connection, OptionalExtension, params};

/// One `goals` row as stored (DATA_MODEL §4.4).
pub struct Row {
    pub id: i64,
    pub name: String,
    pub target_amount: i64,
    pub target_date: Option<String>,
    pub account_id: i64,
}

/// The columns a create or an edit writes.
pub struct Values<'a> {
    pub name: &'a str,
    pub target_amount: i64,
    pub target_date: Option<&'a str>,
    pub account_id: i64,
}

pub fn any(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row("SELECT EXISTS (SELECT 1 FROM goals)", [], |row| row.get(0))
}

const SELECT: &str = "SELECT id, name, target_amount, target_date, account_id FROM goals";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(0)?,
        name: r.get(1)?,
        target_amount: r.get(2)?,
        target_date: r.get(3)?,
        account_id: r.get(4)?,
    })
}

/// In the order they were created.
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
        "SELECT EXISTS (SELECT 1 FROM goals WHERE name = ?1 AND id IS NOT ?2)",
        params![name, except],
        |r| r.get(0),
    )
}

pub fn insert(conn: &Connection, v: &Values, now: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO goals (name, target_amount, target_date, account_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![v.name, v.target_amount, v.target_date, v.account_id, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, v: &Values, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE goals SET name = ?2, target_amount = ?3, target_date = ?4, account_id = ?5,
                          updated_at = ?6
         WHERE id = ?1",
        params![
            id,
            v.name,
            v.target_amount,
            v.target_date,
            v.account_id,
            now
        ],
    )?;
    Ok(())
}

/// The number of rows deleted: 0 when there's no such goal.
pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM goals WHERE id = ?1", [id])
}
