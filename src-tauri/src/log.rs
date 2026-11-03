use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::error::AppError;

const MAX_BYTES: u64 = 1_048_576;

/// Appends a `Database` or `Io` error to the error log. Other errors are the user's to fix, and
/// their messages can hold what they typed, so they're never written.
pub fn record(path: &Path, err: &AppError) {
    if matches!(err, AppError::Database { .. } | AppError::Io { .. }) {
        append(path, &format!("{}: {err}", code(err)));
    }
}

/// Appends a failure that stopped startup, whatever its kind.
pub fn record_startup(path: &Path, err: &AppError) {
    append(path, &format!("startup {}: {err}", code(err)));
}

fn code(err: &AppError) -> &'static str {
    match err {
        AppError::Validation { .. } => "validation",
        AppError::NotFound { .. } => "not_found",
        AppError::Conflict { .. } => "conflict",
        AppError::Database { .. } => "database",
        AppError::Io { .. } => "io",
    }
}

/// Never fails: a log that can't be written mustn't stop the app (ARCHITECTURE §4.4).
fn append(path: &Path, line: &str) {
    if fs::metadata(path).is_ok_and(|m| m.len() >= MAX_BYTES) {
        let _ = fs::rename(path, path.with_extension("log.1"));
    }
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{now} {line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use rusqlite::Connection;
    use std::fs;

    #[test]
    fn the_log_never_records_user_data() {
        let dir = TempDir::new("log-privacy");
        let log = dir.path().join("test.log");
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE t (name TEXT UNIQUE, amount INTEGER CHECK (amount > 0), payee TEXT, note TEXT) STRICT;
             INSERT INTO t VALUES ('BPI Savings', 123456, 'Corner Cafe', 'rent for mom');",
        )
        .unwrap();
        let failures = [
            conn.execute(
                "INSERT INTO t VALUES ('BPI Savings', 1, 'Corner Cafe', 'rent for mom')",
                [],
            ),
            conn.execute(
                "INSERT INTO t VALUES ('Other', -98765, 'Corner Cafe', 'rent for mom')",
                [],
            ),
            conn.execute(
                "INSERT INTO t VALUES ('Other', 'Corner Cafe', 'x', 'rent for mom')",
                [],
            ),
            conn.query_row("SELECT 4242424242", [], |row| row.get::<_, u8>(0))
                .map(|_| 0),
        ];
        for failure in failures {
            record(&log, &AppError::from(failure.unwrap_err()));
        }
        record(
            &log,
            &AppError::Validation {
                field: "name".into(),
                message: "Dining Out is taken".into(),
            },
        );
        record(
            &log,
            &AppError::Conflict {
                message: "Dining Out is in use".into(),
            },
        );

        let written = fs::read_to_string(&log).unwrap();
        assert_eq!(written.lines().count(), 4, "{written}");
        for secret in [
            "BPI Savings",
            "123456",
            "98765",
            "4242424242",
            "Corner Cafe",
            "rent for mom",
            "Dining Out",
        ] {
            assert!(
                !written.contains(secret),
                "{secret} leaked into the log:\n{written}"
            );
        }
    }

    #[test]
    fn startup_failures_are_logged_with_their_code() {
        let dir = TempDir::new("log-startup");
        let log = dir.path().join("test.log");
        record_startup(
            &log,
            &AppError::Conflict {
                message: "x".into(),
            },
        );
        assert!(
            fs::read_to_string(&log)
                .unwrap()
                .contains("startup conflict: x")
        );
    }

    #[test]
    fn the_log_rotates_at_1_mb() {
        let dir = TempDir::new("log-rotate");
        let log = dir.path().join("test.log");
        let old = dir.path().join("test.log.1");
        let err = AppError::Io {
            message: "disk full".into(),
        };
        fs::write(&log, vec![b'x'; 1_048_576]).unwrap();

        record(&log, &err);
        assert_eq!(fs::metadata(&old).unwrap().len(), 1_048_576);
        assert_eq!(fs::read_to_string(&log).unwrap().lines().count(), 1);

        fs::write(&log, vec![b'y'; 1_048_576]).unwrap();
        record(&log, &err);
        assert!(
            fs::read(&old).unwrap().iter().all(|&b| b == b'y'),
            "the older copy is replaced"
        );
    }

    #[test]
    fn a_log_write_failure_is_ignored() {
        let dir = TempDir::new("log-missing");
        let log = dir.path().join("missing").join("dir").join("test.log");
        record(
            &log,
            &AppError::Io {
                message: "x".into(),
            },
        );
        assert!(!log.exists());
    }
}
