use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use rusqlite_migration::Migrations;
use serde::Serialize;
use ts_rs::TS;

use crate::error::AppError;
use crate::{db, log};

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
    pub fn open(data_dir: PathBuf, ms: &Migrations) -> Self {
        let db_path = data_dir.join(format!("{}.db", db::NAME));
        let log_path = data_dir.join(format!("{}.log", db::NAME));
        let db = start(&data_dir, &db_path, ms);
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

    pub fn startup_status(&self) -> StartupStatus {
        match &self.db {
            Ok(_) => StartupStatus::Ok,
            Err(err) => StartupStatus::Failed {
                message: err.to_string(),
                database_path: self.db_path.display().to_string(),
            },
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

fn start(data_dir: &Path, db_path: &Path, ms: &Migrations) -> Result<Connection, AppError> {
    fs::create_dir_all(data_dir)?;
    let mut conn = db::open(db_path)?;
    db::prepare(&mut conn, ms)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    const NEWER: &str = "This database was made by a newer version of Jobert's Vault. Install the latest version to open it.";

    #[test]
    #[cfg(debug_assertions)]
    fn debug_builds_use_their_own_files() {
        let dir = TempDir::new("state-names");
        let state = AppState::open(dir.path().to_path_buf(), &Migrations::new(vec![]));
        assert!(state.db_path.ends_with("jobertsvault-dev.db"));
        assert!(state.log_path.ends_with("jobertsvault-dev.log"));
    }

    #[test]
    fn a_fresh_folder_starts_ok() {
        let dir = TempDir::new("state-fresh");
        let data_dir = dir.path().join("com.jobertsvault.app");
        let state = AppState::open(data_dir, &Migrations::new(vec![]));
        assert_eq!(state.startup_status(), StartupStatus::Ok);
        assert!(state.db_path.exists());
    }

    #[test]
    fn a_newer_database_is_refused() {
        let dir = TempDir::new("state-newer");
        let db_path = dir.path().join("jobertsvault-dev.db");
        Connection::open(&db_path)
            .unwrap()
            .pragma_update(None, "user_version", 1)
            .unwrap();

        let state = AppState::open(dir.path().to_path_buf(), &Migrations::new(vec![]));
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
        fs::write(dir.path().join("jobertsvault-dev.db"), vec![0x5a; 4096]).unwrap();
        let state = AppState::open(dir.path().to_path_buf(), &Migrations::new(vec![]));
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
}
