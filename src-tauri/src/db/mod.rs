mod migrations;
mod seed;

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use chrono::{DateTime, Local};
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
pub const BACKUPS_DIR: &str = if cfg!(debug_assertions) {
    "backups-dev"
} else {
    "backups"
};

/// Where pre-migration backups go, whatever `backup_folder` says (DATA_MODEL §8).
pub fn pre_migration_dir(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join(BACKUPS_DIR).join("pre-migration")
}

/// Whether `conn` holds a sound Jobert's Vault database: SQLite's own check passes, it has a
/// schema version, and the first migration's tables are there (BAK-4).
pub fn is_vault(conn: &Connection) -> Result<bool, AppError> {
    let check: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let tables: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE type = 'table'
         AND name IN ('accounts', 'categories', 'payment_methods', 'transactions', 'settings')",
        [],
        |row| row.get(0),
    )?;
    Ok(check == "ok" && version >= 1 && tables == 5)
}

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

/// Moves every change in the `-wal` file into the database file and empties the `-wal` file.
pub fn checkpoint(conn: &Connection) -> Result<(), AppError> {
    // Gives (busy, wal pages, pages moved); busy is 1 when another connection blocked it.
    let busy: i64 = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))?;
    if busy != 0 {
        return Err(AppError::Database {
            message: "Couldn't write recent changes into the database file".into(),
        });
    }
    Ok(())
}

/// Refuses a database made by a newer version of the app, then runs any pending migrations,
/// backing up an existing database into `backup_dir` first (BAK-5).
pub fn prepare(
    conn: &mut Connection,
    ms: &Migrations,
    backup_dir: &Path,
    now: DateTime<Local>,
) -> Result<(), AppError> {
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
        let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > 0 {
            back_up(conn, backup_dir, version, now)?;
        }
        ms.to_latest(conn)?;
    }
    Ok(())
}

/// Copies the database into `dir` as `<name>-pre-migration-v<version>-<local time>.db`
/// (DATA_MODEL §8). These copies are never deleted automatically.
fn back_up(
    conn: &Connection,
    dir: &Path,
    version: i64,
    now: DateTime<Local>,
) -> Result<(), AppError> {
    let stamp = now.format(crate::domain::backup::STAMP);
    let path = dir.join(format!("{NAME}-pre-migration-v{version}-{stamp}.db"));
    vacuum_into(conn, &path).map_err(|err| AppError::Io {
        message: format!(
            "Couldn't back up the database before updating it, so nothing was changed. {err}"
        ),
    })
}

/// Deletes `path`, which may already be gone.
pub fn remove_if_there(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Err(err) if err.kind() != ErrorKind::NotFound => Err(err),
        _ => Ok(()),
    }
}

/// Writes a consistent copy of the database to `path`. It goes to `<path>.tmp` first and is
/// renamed when complete, so a half-written copy never looks like a backup (ARCHITECTURE §7).
pub fn vacuum_into(conn: &Connection, path: &Path) -> Result<(), AppError> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("db.tmp");
    // Left by a copy that was cut short. VACUUM INTO refuses to overwrite a file.
    remove_if_there(&tmp)?;
    let written = conn
        .execute("VACUUM INTO ?1", [tmp.to_string_lossy()])
        .map_err(AppError::from)
        .and_then(|_| fs::rename(&tmp, path).map_err(AppError::from));
    if written.is_err() {
        // A partial copy can be large, and a retry in a later minute wouldn't find it.
        let _ = fs::remove_file(&tmp);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempDir, now};

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

    use rusqlite_migration::M;

    fn first() -> M<'static> {
        M::up("CREATE TABLE a (x); INSERT INTO a VALUES (1);")
    }

    fn both() -> Migrations<'static> {
        Migrations::new(vec![first(), M::up("CREATE TABLE b (y);")])
    }

    fn version(conn: &Connection) -> i64 {
        conn.pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    fn has_table(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type = 'table' AND name = ?1",
            [name],
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
            == 1
    }

    /// A database file at version 1, as an older app would have left it.
    fn existing(dir: &TempDir) -> Connection {
        let mut conn = open(&dir.path().join("test.db")).unwrap();
        let older = Migrations::new(vec![first()]);
        prepare(&mut conn, &older, &dir.path().join("unused"), now()).unwrap();
        assert_eq!(version(&conn), 1);
        conn
    }

    #[test]
    fn an_existing_database_is_backed_up_before_migrating() {
        let dir = TempDir::new("bak-existing");
        let backups = dir.path().join(BACKUPS_DIR).join("pre-migration");
        let mut conn = existing(&dir);

        prepare(&mut conn, &both(), &backups, now()).unwrap();

        assert_eq!(version(&conn), 2);
        let copy_path = backups.join(format!("{NAME}-pre-migration-v1-2026-11-03_1405.db"));
        let copy = Connection::open(&copy_path).unwrap();
        assert_eq!(version(&copy), 1);
        assert_eq!(
            copy.query_row("SELECT x FROM a", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(
            !has_table(&copy, "b"),
            "the copy is from before the migration"
        );
        assert_eq!(
            std::fs::read_dir(&backups).unwrap().count(),
            1,
            "no .tmp left behind"
        );
    }

    #[test]
    fn a_failed_backup_stops_the_migration() {
        let dir = TempDir::new("bak-failed");
        let blocked = dir.path().join(BACKUPS_DIR);
        std::fs::write(&blocked, b"a file where the folder should be").unwrap();
        let mut conn = existing(&dir);

        let result = prepare(&mut conn, &both(), &blocked.join("pre-migration"), now());

        assert!(
            matches!(&result, Err(AppError::Io { message }) if message.starts_with("Couldn't back up")),
            "{result:?}"
        );
        assert_eq!(version(&conn), 1);
        assert!(!has_table(&conn, "b"), "the migration must not run");
    }

    #[test]
    fn a_new_database_is_not_backed_up() {
        let dir = TempDir::new("bak-new");
        let backups = dir.path().join("pre-migration");
        let mut conn = open(&dir.path().join("test.db")).unwrap();
        prepare(&mut conn, &both(), &backups, now()).unwrap();
        assert_eq!(version(&conn), 2);
        assert!(!backups.exists());
    }

    #[test]
    fn an_up_to_date_database_is_not_backed_up() {
        let dir = TempDir::new("bak-current");
        let backups = dir.path().join("pre-migration");
        let mut conn = open(&dir.path().join("test.db")).unwrap();
        prepare(&mut conn, &both(), &dir.path().join("unused"), now()).unwrap();
        prepare(&mut conn, &both(), &backups, now()).unwrap();
        assert!(!backups.exists());
    }

    #[test]
    fn a_failed_copy_leaves_no_tmp_file() {
        let dir = TempDir::new("bak-cleanup");
        let backups = dir.path().join("pre-migration");
        let name = format!("{NAME}-pre-migration-v1-2026-11-03_1405.db");
        std::fs::create_dir_all(backups.join(&name)).unwrap(); // the rename can't replace a folder
        let mut conn = existing(&dir);

        assert!(prepare(&mut conn, &both(), &backups, now()).is_err());
        assert!(
            !backups.join(format!("{name}.tmp")).exists(),
            "a partial copy was left behind"
        );
    }

    #[test]
    fn a_leftover_tmp_file_does_not_block_the_backup() {
        let dir = TempDir::new("bak-tmp");
        let backups = dir.path().join("pre-migration");
        std::fs::create_dir_all(&backups).unwrap();
        let name = format!("{NAME}-pre-migration-v1-2026-11-03_1405.db");
        std::fs::write(backups.join(format!("{name}.tmp")), b"half a backup").unwrap();
        let mut conn = existing(&dir);

        prepare(&mut conn, &both(), &backups, now()).unwrap();

        let copy = Connection::open(backups.join(&name)).unwrap();
        assert_eq!(version(&copy), 1);
        assert!(!backups.join(format!("{name}.tmp")).exists());
    }
}
