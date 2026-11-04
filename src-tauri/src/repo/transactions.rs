use rusqlite::Connection;

/// Whether any transaction exists at all.
pub fn any(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row("SELECT EXISTS (SELECT 1 FROM transactions)", [], |row| {
        row.get(0)
    })
}
