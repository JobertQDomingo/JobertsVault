use rusqlite::{Connection, OptionalExtension, params};

/// One `categories` row as stored (DATA_MODEL §3.2).
pub struct Row {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub color: String,
    pub icon: String,
    pub sort_order: i64,
    pub archived: bool,
    pub transaction_count: i64,
}

/// The columns a create or an edit writes.
pub struct Values<'a> {
    pub name: &'a str,
    pub color: &'a str,
    pub icon: &'a str,
}

const SELECT: &str = "SELECT c.id, c.name, c.kind, c.color, c.icon, c.sort_order, c.archived,
            (SELECT count(*) FROM transactions t WHERE t.category_id = c.id)
     FROM categories c";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(0)?,
        name: r.get(1)?,
        kind: r.get(2)?,
        color: r.get(3)?,
        icon: r.get(4)?,
        sort_order: r.get(5)?,
        archived: r.get(6)?,
        transaction_count: r.get(7)?,
    })
}

/// Every category, archived included, in display order.
pub fn all(conn: &Connection) -> rusqlite::Result<Vec<Row>> {
    conn.prepare(&format!("{SELECT} ORDER BY c.sort_order, c.id"))?
        .query_map([], row)?
        .collect()
}

pub fn find(conn: &Connection, id: i64) -> rusqlite::Result<Option<Row>> {
    conn.query_row(&format!("{SELECT} WHERE c.id = ?1"), [id], row)
        .optional()
}

/// Whether another category of `kind` already has `name`. The column is `COLLATE NOCASE`.
pub fn name_taken(
    conn: &Connection,
    kind: &str,
    name: &str,
    except: Option<i64>,
) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM categories WHERE kind = ?1 AND name = ?2 AND id IS NOT ?3)",
        params![kind, name, except],
        |r| r.get(0),
    )
}

/// Adds the category after every other one of its kind and returns its id.
pub fn insert(conn: &Connection, kind: &str, v: &Values, now: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO categories (name, kind, color, icon, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM categories WHERE kind = ?2), ?5, ?5)",
        params![v.name, kind, v.color, v.icon, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, v: &Values, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE categories SET name = ?2, color = ?3, icon = ?4, updated_at = ?5 WHERE id = ?1",
        params![id, v.name, v.color, v.icon, now],
    )?;
    Ok(())
}

/// The number of rows changed: 0 when there's no such category.
pub fn set_sort_order(
    conn: &Connection,
    id: i64,
    order: i64,
    now: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE categories SET sort_order = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, order, now],
    )
}

pub fn set_archived(conn: &Connection, id: i64, archived: bool, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE categories SET archived = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, archived, now],
    )?;
    Ok(())
}

pub fn active_count(conn: &Connection, kind: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT count(*) FROM categories WHERE kind = ?1 AND archived = 0",
        [kind],
        |r| r.get(0),
    )
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM categories WHERE id = ?1", [id])?;
    Ok(())
}

/// Gives every transaction of `from` the category `into`.
pub fn move_transactions(
    conn: &Connection,
    from: i64,
    into: i64,
    now: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE transactions SET category_id = ?2, updated_at = ?3 WHERE category_id = ?1",
        params![from, into, now],
    )?;
    Ok(())
}
