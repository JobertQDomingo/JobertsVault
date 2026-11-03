mod migrations;
mod seed;

use std::path::Path;

use rusqlite::Connection;
use rusqlite_migration::Migrations;

use crate::error::AppError;

pub use migrations::migrations;

/// Debug builds use their own files, so development never touches real data (ARCHITECTURE §6).
pub const NAME: &str = if cfg!(debug_assertions) {
    "jobertsvault-dev"
} else {
    "jobertsvault"
};

/// Opens the database and applies the connection settings (ARCHITECTURE §4.3).
pub fn open(path: &Path) -> Result<Connection, AppError> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA synchronous = NORMAL;",
    )?;
    Ok(conn)
}

/// Refuses a database made by a newer version of the app, and runs any pending migrations.
pub fn prepare(conn: &mut Connection, ms: &Migrations) -> Result<(), AppError> {
    let pending = ms.pending_migrations(conn)?;
    if pending < 0 {
        return Err(AppError::Database {
            message: concat!(
                "This database was made by a newer version of Jobert's Vault. ",
                "Install the latest version to open it."
            )
            .into(),
        });
    }
    if pending > 0 {
        ms.to_latest(conn)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn open_applies_the_connection_settings() {
        let dir = TempDir::new("db-open");
        let conn = open(&dir.path().join("test.db")).unwrap();
        let pragma = |name: &str| -> String {
            conn.query_row(&format!("PRAGMA {name}"), [], |row| {
                row.get::<_, rusqlite::types::Value>(0)
            })
            .map(|value| match value {
                rusqlite::types::Value::Integer(n) => n.to_string(),
                rusqlite::types::Value::Text(s) => s,
                other => format!("{other:?}"),
            })
            .unwrap()
        };
        assert_eq!(pragma("foreign_keys"), "1");
        assert_eq!(pragma("journal_mode"), "wal");
        assert_eq!(pragma("busy_timeout"), "5000");
        assert_eq!(pragma("synchronous"), "1");
    }

    #[test]
    fn open_waits_for_a_lock_instead_of_failing() {
        let dir = TempDir::new("db-busy");
        let path = dir.path().join("test.db");
        let holder = Connection::open(&path).unwrap();
        // Hold a write lock on a non-empty database while open() runs.
        holder
            .execute_batch("CREATE TABLE t (x); BEGIN EXCLUSIVE")
            .unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            holder.execute_batch("COMMIT").unwrap();
        });
        assert!(
            open(&path).is_ok(),
            "the WAL switch must wait out another writer's lock"
        );
        release.join().unwrap();
    }
}
