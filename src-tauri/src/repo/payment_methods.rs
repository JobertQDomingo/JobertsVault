use rusqlite::{Connection, OptionalExtension, params};

/// One `payment_methods` row as stored (DATA_MODEL §3.3).
pub struct Row {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    pub archived: bool,
    pub in_use: bool,
}

const SELECT: &str = "SELECT m.id, m.name, m.sort_order, m.archived,
            EXISTS (SELECT 1 FROM transactions t WHERE t.payment_method_id = m.id)
     FROM payment_methods m";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(0)?,
        name: r.get(1)?,
        sort_order: r.get(2)?,
        archived: r.get(3)?,
        in_use: r.get(4)?,
    })
}

/// Every method, archived included, in display order.
pub fn all(conn: &Connection) -> rusqlite::Result<Vec<Row>> {
    conn.prepare(&format!("{SELECT} ORDER BY m.sort_order, m.id"))?
        .query_map([], row)?
        .collect()
}

pub fn find(conn: &Connection, id: i64) -> rusqlite::Result<Option<Row>> {
    conn.query_row(&format!("{SELECT} WHERE m.id = ?1"), [id], row)
        .optional()
}

/// Whether another method already has `name`. The column is `COLLATE NOCASE`.
pub fn name_taken(conn: &Connection, name: &str, except: Option<i64>) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM payment_methods WHERE name = ?1 AND id IS NOT ?2)",
        params![name, except],
        |r| r.get(0),
    )
}

/// Adds the method after every other one and returns its id.
pub fn insert(conn: &Connection, name: &str, now: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO payment_methods (name, sort_order, created_at, updated_at)
         VALUES (?1, (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM payment_methods), ?2, ?2)",
        params![name, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn rename(conn: &Connection, id: i64, name: &str, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE payment_methods SET name = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, name, now],
    )?;
    Ok(())
}

/// The number of rows changed: 0 when there's no such method.
pub fn set_sort_order(
    conn: &Connection,
    id: i64,
    order: i64,
    now: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE payment_methods SET sort_order = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, order, now],
    )
}

pub fn set_archived(conn: &Connection, id: i64, archived: bool, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE payment_methods SET archived = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, archived, now],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM payment_methods WHERE id = ?1", [id])?;
    Ok(())
}
