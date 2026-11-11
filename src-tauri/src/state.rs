use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use chrono::{DateTime, Local};
use rusqlite::Connection;
use rusqlite_migration::Migrations;
use serde::Serialize;
use ts_rs::TS;

use crate::error::AppError;
use crate::{db, log, services};

/// What the frontend checks before anything else (ARCHITECTURE §6).
#[derive(Debug, PartialEq, Serialize, TS)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum StartupStatus {
    Ok,
    Failed {
        message: String,
        database_path: String,
    },
}

/// What every command shares. If startup failed, `db` holds the reason instead of the
/// connection, and every data command returns it (ARCHITECTURE §6).
pub struct AppState {
    pub db: Result<Mutex<Connection>, AppError>,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub log_path: PathBuf,
}

impl AppState {
    /// Runs the startup sequence (ARCHITECTURE §6) in `data_dir`. A failure is logged and kept
    /// in place of the connection, so the app still opens and shows it.
    pub fn open(data_dir: PathBuf, ms: &Migrations, now: DateTime<Local>) -> Self {
        let db_path = data_dir.join(format!("{}.db", db::NAME));
        let log_path = data_dir.join(format!("{}.log", db::NAME));
        let db = start(&data_dir, &db_path, ms, now);
        if let Err(err) = &db {
            log::record_startup(&log_path, err);
        }
        Self {
            db: db.map(Mutex::new),
            data_dir,
            db_path,
            log_path,
        }
    }

    /// Startup steps 7 and 8 (BAK-2, REC-4), run on their own thread so the window never waits
    /// for a slow backup folder; commands wait for the connection instead, so no screen reads the
    /// data from before them. `locked` runs once this holds the connection (or found none). A
    /// failure is logged, never a reason not to start.
    pub fn back_up_and_process(&self, now: DateTime<Local>, locked: impl FnOnce()) {
        let conn = self.conn();
        locked();
        if self.db.is_err() {
            return;
        }
        let result = conn.and_then(|mut conn| {
            if let Err(err) = services::backup::back_up_daily(&conn, &self.data_dir, now) {
                log::record(&self.log_path, &err);
            }
            services::recurring::process_due(&mut conn, now).map(|_| ())
        });
        if let Err(err) = result {
            log::record(&self.log_path, &err);
        }
    }

    pub fn startup_status(&self) -> StartupStatus {
        match &self.db {
            Ok(_) => StartupStatus::Ok,
            Err(err) => StartupStatus::Failed {
                message: err.to_string(),
                database_path: self.db_path.display().to_string(),
            },
        }
    }

    /// Runs when the app quits. WAL mode keeps recent changes in the `-wal` file, and the
    /// process exits without closing the connection, so this moves them into the database
    /// file: a copy of that file alone is then a complete backup (PRD §11).
    pub fn close(&self) {
        if self.db.is_ok()
            && let Err(err) = self.conn().and_then(|conn| db::checkpoint(&conn))
        {
            log::record(&self.log_path, &err);
        }
    }

    pub fn conn(&self) -> Result<MutexGuard<'_, Connection>, AppError> {
        match &self.db {
            Ok(db) => db.lock().map_err(|_| AppError::Database {
                message: "The database lock was poisoned".into(),
            }),
            Err(err) => Err(err.clone()),
        }
    }
}

fn start(
    data_dir: &Path,
    db_path: &Path,
    ms: &Migrations,
    now: DateTime<Local>,
) -> Result<Connection, AppError> {
    fs::create_dir_all(data_dir)?;
    let mut conn = db::open(db_path)?;
    db::prepare(&mut conn, ms, &db::pre_migration_dir(data_dir), now)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempDir, now};

    const NEWER: &str = "This database was made by a newer version of Jobert's Vault. Install the latest version to open it.";

    #[test]
    #[cfg(debug_assertions)]
    fn debug_builds_use_their_own_files() {
        let dir = TempDir::new("state-names");
        let state = AppState::open(dir.path().to_path_buf(), &Migrations::new(vec![]), now());
        assert!(state.db_path.ends_with("jobertsvault-dev.db"));
        assert!(state.log_path.ends_with("jobertsvault-dev.log"));
    }

    #[test]
    fn after_close_the_database_file_alone_holds_everything() {
        let dir = TempDir::new("state-close");
        let state = AppState::open(
            dir.path().to_path_buf(),
            &db::migrations(now().to_utc()),
            now(),
        );
        let copy_has_accounts = || {
            let copy = dir.path().join("copy.db");
            let _ = fs::remove_file(&copy);
            fs::copy(&state.db_path, &copy).unwrap();
            Connection::open(&copy)
                .unwrap()
                .query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get::<_, i64>(0))
                .is_ok_and(|n| n > 0)
        };
        // While it runs, the changes sit in the -wal file.
        assert!(!copy_has_accounts());
        state.close();
        assert!(copy_has_accounts());
    }

    #[test]
    fn a_fresh_folder_starts_ok() {
        let dir = TempDir::new("state-fresh");
        let data_dir = dir.path().join("com.jobertsvault.app");
        let state = AppState::open(data_dir, &Migrations::new(vec![]), now());
        assert_eq!(state.startup_status(), StartupStatus::Ok);
        assert!(state.db_path.exists());
    }

    #[test]
    fn a_newer_database_is_refused() {
        let dir = TempDir::new("state-newer");
        let db_path = dir.path().join(format!("{}.db", db::NAME));
        Connection::open(&db_path)
            .unwrap()
            .pragma_update(None, "user_version", 99) // far past any real migration
            .unwrap();

        let state = AppState::open(
            dir.path().to_path_buf(),
            &db::migrations(now().to_utc()),
            now(),
        );
        assert_eq!(
            state.startup_status(),
            StartupStatus::Failed {
                message: NEWER.into(),
                database_path: db_path.display().to_string(),
            }
        );
        assert!(
            fs::read_to_string(&state.log_path)
                .unwrap()
                .contains("startup database:")
        );
    }

    #[test]
    fn a_corrupt_file_is_reported() {
        let dir = TempDir::new("state-corrupt");
        fs::write(
            dir.path().join(format!("{}.db", db::NAME)),
            vec![0x5a; 4096],
        )
        .unwrap();
        let state = AppState::open(dir.path().to_path_buf(), &Migrations::new(vec![]), now());
        assert!(matches!(
            state.startup_status(),
            StartupStatus::Failed { .. }
        ));
    }

    fn state(db: Result<Mutex<Connection>, AppError>) -> AppState {
        AppState {
            db,
            data_dir: PathBuf::new(),
            db_path: PathBuf::new(),
            log_path: PathBuf::new(),
        }
    }

    #[test]
    fn a_failed_startup_returns_its_error_from_every_data_command() {
        let failed = state(Err(AppError::Io {
            message: "disk gone".into(),
        }));
        assert!(matches!(failed.conn(), Err(AppError::Io { message }) if message == "disk gone"));
    }

    #[test]
    fn conn_locks_the_connection() {
        let ok = state(Ok(Mutex::new(Connection::open_in_memory().unwrap())));
        let one: i64 = ok
            .conn()
            .unwrap()
            .query_row("SELECT 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(one, 1);
    }

    #[test]
    fn the_real_migrations_apply_and_the_app_reopens() {
        let dir = TempDir::new("state-real");
        for _launch in 0..2 {
            let state = AppState::open(
                dir.path().to_path_buf(),
                &db::migrations(now().to_utc()),
                now(),
            );
            assert_eq!(state.startup_status(), StartupStatus::Ok);
            let version: i64 = state
                .conn()
                .unwrap()
                .query_row("PRAGMA user_version", [], |row| row.get(0))
                .unwrap();
            assert_eq!(version, 2);
        }
    }

    /// A monthly rule from Oct 3 that the app hasn't processed yet, as after a month closed.
    fn add_rule(state: &AppState) {
        state
            .conn()
            .unwrap()
            .execute_batch(
                "INSERT INTO recurring_rules (kind, amount, account_id, category_id, frequency,
                                              start_date, mode, created_at, updated_at)
                 VALUES ('expense', 100, 1, 8, 'monthly', '2026-10-03', 'auto', 'x', 'x')",
            )
            .unwrap();
    }

    fn posted(state: &AppState) -> i64 {
        state
            .conn()
            .unwrap()
            .query_row("SELECT count(*) FROM transactions", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn startup_makes_the_daily_backup_and_processes_recurring_rules() {
        let dir = TempDir::new("state-daily");
        let state = AppState::open(
            dir.path().to_path_buf(),
            &db::migrations(now().to_utc()),
            now(),
        );
        assert_eq!(state.startup_status(), StartupStatus::Ok);
        add_rule(&state);
        state.back_up_and_process(now(), || {});
        let auto = dir.path().join(db::BACKUPS_DIR).join("auto");
        assert_eq!(fs::read_dir(auto).unwrap().count(), 1);
        assert_eq!(posted(&state), 2, "Oct 3 and Nov 3");
    }

    #[test]
    fn a_failed_daily_backup_does_not_stop_startup() {
        let dir = TempDir::new("state-daily-failed");
        fs::write(
            dir.path().join(db::BACKUPS_DIR),
            b"a file where the folder should be",
        )
        .unwrap();
        let state = AppState::open(
            dir.path().to_path_buf(),
            &db::migrations(now().to_utc()),
            now(),
        );
        assert_eq!(state.startup_status(), StartupStatus::Ok);
        add_rule(&state);
        state.back_up_and_process(now(), || {});
        assert!(
            fs::read_to_string(&state.log_path)
                .unwrap()
                .contains("io: Couldn't save the backup")
        );
        assert_eq!(posted(&state), 2, "the rules are still processed");
    }

    #[test]
    fn startup_status_reports_a_failed_backup() {
        use rusqlite_migration::M;
        let dir = TempDir::new("state-backup");
        let first = M::up("CREATE TABLE a (x);");
        let db_path = dir.path().join(format!("{}.db", db::NAME));
        let mut conn = db::open(&db_path).unwrap();
        Migrations::new(vec![first.clone()])
            .to_latest(&mut conn)
            .unwrap();
        drop(conn);
        fs::write(
            dir.path().join(db::BACKUPS_DIR),
            b"a file where the folder should be",
        )
        .unwrap();

        let both = Migrations::new(vec![first, M::up("CREATE TABLE b (y);")]);
        let state = AppState::open(dir.path().to_path_buf(), &both, now());

        let StartupStatus::Failed { message, .. } = state.startup_status() else {
            panic!("startup should have failed");
        };
        assert!(message.starts_with("Couldn't back up"), "{message}");
        assert!(
            fs::read_to_string(&state.log_path)
                .unwrap()
                .contains("startup io: Couldn't back up")
        );
        let version: i64 = Connection::open(&db_path)
            .unwrap()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1, "the migration must not run");
    }
}
