use rusqlite::{Connection, OptionalExtension, params};

/// One `budgets` row as stored (DATA_MODEL §4.3). `archived` is never set, so it isn't read.
pub struct Row {
    pub id: i64,
    pub category_id: Option<i64>,
    pub period: String,
    pub amount: i64,
}

/// The columns a create or an edit writes.
pub struct Values<'a> {
    pub category_id: Option<i64>,
    pub period: &'a str,
    pub amount: i64,
}

pub fn any(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row("SELECT EXISTS (SELECT 1 FROM budgets)", [], |row| {
        row.get(0)
    })
}

const SELECT: &str = "SELECT b.id, b.category_id, b.period, b.amount FROM budgets b";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(0)?,
        category_id: r.get(1)?,
        period: r.get(2)?,
        amount: r.get(3)?,
    })
}

/// All spending first, then in the categories' order, each weekly, monthly, then yearly.
pub fn all(conn: &Connection) -> rusqlite::Result<Vec<Row>> {
    conn.prepare(&format!(
        "{SELECT} LEFT JOIN categories c ON c.id = b.category_id
         ORDER BY b.category_id IS NOT NULL, c.sort_order, c.id,
                  CASE b.period WHEN 'weekly' THEN 0 WHEN 'monthly' THEN 1 ELSE 2 END"
    ))?
    .query_map([], row)?
    .collect()
}

pub fn find(conn: &Connection, id: i64) -> rusqlite::Result<Option<Row>> {
    conn.query_row(&format!("{SELECT} WHERE b.id = ?1"), [id], row)
        .optional()
}

/// Whether another budget already covers this category (none: all spending) and period.
pub fn taken(
    conn: &Connection,
    category_id: Option<i64>,
    period: &str,
    except: Option<i64>,
) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM budgets WHERE category_id IS ?1 AND period = ?2 AND id IS NOT ?3)",
        params![category_id, period, except],
        |r| r.get(0),
    )
}

pub fn insert(conn: &Connection, v: &Values, now: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO budgets (category_id, period, amount, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![v.category_id, v.period, v.amount, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, v: &Values, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE budgets SET category_id = ?2, period = ?3, amount = ?4, updated_at = ?5 WHERE id = ?1",
        params![id, v.category_id, v.period, v.amount, now],
    )?;
    Ok(())
}

/// The number of rows deleted: 0 when there's no such budget.
pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM budgets WHERE id = ?1", [id])
}

/// A deleted or merged category takes its budgets with it (DATA_MODEL §4.3).
pub fn delete_for_category(conn: &Connection, category_id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM budgets WHERE category_id = ?1", [category_id])?;
    Ok(())
}
