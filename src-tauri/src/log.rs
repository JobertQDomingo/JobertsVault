use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

use crate::error::AppError;

const MAX_BYTES: u64 = 1_048_576;

// Commands run on several threads, and two of them rotating at once would lose the older copy.
static WRITING: Mutex<()> = Mutex::new(());

/// Appends a `Database` or `Io` error to the error log. Other kinds are the user's to fix and
/// aren't logged.
pub fn record(path: &Path, err: &AppError) {
    if let (code, Some(message)) = describe(err) {
        append(path, &format!("{code}: {message}"));
    }
}

/// Appends a failure that stopped startup, whatever its kind.
pub fn record_startup(path: &Path, err: &AppError) {
    match describe(err) {
        (code, Some(message)) => append(path, &format!("startup {code}: {message}")),
        (code, None) => append(path, &format!("startup {code}")),
    }
}

/// The error's code, and its message only when it's safe to log. Messages of the other kinds
/// can hold what the user typed: amounts, payees, notes, account or category names.
fn describe(err: &AppError) -> (&'static str, Option<String>) {
    match err {
        AppError::Validation { .. } => ("validation", None),
        AppError::NotFound { .. } => ("not_found", None),
        AppError::Conflict { .. } => ("conflict", None),
        AppError::Database { .. } => ("database", Some(err.to_string())),
        AppError::Io { .. } => ("io", Some(err.to_string())),
    }
}

/// Never fails: a log that can't be written mustn't stop the app (ARCHITECTURE §4.4).
fn append(path: &Path, line: &str) {
    let _writing = WRITING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // If the old copy can't be replaced, skip the entry rather than grow past the cap.
    if fs::metadata(path).is_ok_and(|m| m.len() >= MAX_BYTES)
        && fs::rename(path, path.with_extension("log.1")).is_err()
    {
        return;
    }
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let line = line.replace(['\r', '\n'], " ");
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
            // What a FromSql/ToSql impl (e.g. a Money newtype) could return.
            Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Integer,
                "amount 123456 is out of range".into(),
            )),
            Err(rusqlite::Error::ToSqlConversionFailure(
                "payee Corner Cafe".into(),
            )),
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
        assert_eq!(written.lines().count(), 6, "{written}");
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
    fn startup_failures_never_record_user_text() {
        let dir = TempDir::new("log-startup");
        let log = dir.path().join("test.log");
        record_startup(
            &log,
            &AppError::Conflict {
                message: "BPI Savings is in use".into(),
            },
        );
        record_startup(
            &log,
            &AppError::Database {
                message: "disk I/O error".into(),
            },
        );
        let written = fs::read_to_string(&log).unwrap();
        assert!(
            written.contains(
                "startup conflict
"
            ),
            "{written}"
        );
        assert!(
            written.contains("startup database: disk I/O error"),
            "{written}"
        );
        assert!(!written.contains("BPI Savings"), "{written}");
    }

    #[test]
    fn a_multi_line_message_stays_on_one_line() {
        let dir = TempDir::new("log-lines");
        let log = dir.path().join("test.log");
        record(
            &log,
            &AppError::Database {
                message: "near \"FORM\"
  FROM t
 WHERE x"
                    .into(),
            },
        );
        assert_eq!(fs::read_to_string(&log).unwrap().lines().count(), 1);
    }

    #[test]
    fn a_failed_rotation_stops_the_log_growing() {
        let dir = TempDir::new("log-stuck");
        let log = dir.path().join("test.log");
        fs::create_dir(dir.path().join("test.log.1")).unwrap(); // the rename can't replace a folder
        fs::write(&log, vec![b'x'; 1_048_576]).unwrap();
        record(
            &log,
            &AppError::Io {
                message: "x".into(),
            },
        );
        assert_eq!(fs::metadata(&log).unwrap().len(), 1_048_576);
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
