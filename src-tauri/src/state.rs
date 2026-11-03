use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::error::AppError;

/// What every command shares. If startup failed, `db` holds the reason instead of the
/// connection, and every data command returns it (ARCHITECTURE §6).
pub struct AppState {
    pub db: Result<Mutex<Connection>, AppError>,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub log_path: PathBuf,
}

impl AppState {
    pub fn conn(&self) -> Result<MutexGuard<'_, Connection>, AppError> {
        match &self.db {
            Ok(db) => db.lock().map_err(|_| AppError::Database {
                message: "The database lock was poisoned".into(),
            }),
            Err(err) => Err(err.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
