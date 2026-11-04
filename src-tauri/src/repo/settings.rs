use rusqlite::{Connection, OptionalExtension};

/// Every stored key and its value.
pub fn all(conn: &Connection) -> rusqlite::Result<Vec<(String, String)>> {
    conn.prepare("SELECT key, value FROM settings")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect()
}

pub fn get(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .optional()
}

pub fn set(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}
