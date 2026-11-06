use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::db;
use crate::domain::backup;
use crate::error::AppError;
use crate::repo;

const LAST_AUTO: &str = "last_auto_backup_date";

/// What `backup_now` returns: the name of the file it wrote, for the "Backup saved as …" toast,
/// whose Open folder shows where (UI_FLOWS F9).
#[derive(Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Backup {
    pub file_name: String,
}

/// Where backups go: `<app data>\backups` (debug builds: `backups-dev`, ARCHITECTURE §6).
pub fn folder(data_dir: &Path) -> PathBuf {
    data_dir.join(db::BACKUPS_DIR)
}

/// Back up now (BAK-1): copies the database, `-wal` included, into the backup folder as
/// `<name>-YYYY-MM-DD_HHMM.db`. These are never deleted automatically (BAK-3).
pub fn back_up_now(
    conn: &Connection,
    data_dir: &Path,
    now: DateTime<Local>,
) -> Result<Backup, AppError> {
    let file_name = back_up(conn, &folder(data_dir), now)?;
    Ok(Backup { file_name })
}

/// The daily automatic backup (BAK-2): the first time the app opens on a local day, it copies the
/// database into `<backup folder>\auto` and records the date, then keeps only the newest 30
/// there (BAK-3). A failed copy records nothing, so the next start tries again.
pub fn back_up_daily(
    conn: &mut Connection,
    data_dir: &Path,
    now: DateTime<Local>,
) -> Result<(), AppError> {
    let today = now.date_naive().to_string();
    if repo::settings::get(conn, LAST_AUTO)?.as_deref() == Some(today.as_str()) {
        return Ok(());
    }
    let dir = folder(data_dir).join("auto");
    let made = back_up(conn, &dir, now)?;
    let tx = conn.transaction()?;
    repo::settings::set(&tx, LAST_AUTO, &today)?;
    tx.commit()?;
    prune(&dir, &made).map_err(|err| AppError::Io {
        message: format!("Couldn't delete the oldest automatic backups. {err}"),
    })
}

/// Deletes all but the newest 30 automatic backups in `dir`, `just_made` among those kept. Other
/// files there are left alone. A file that can't be deleted doesn't stop the rest; the first
/// error is returned.
fn prune(dir: &Path, just_made: &str) -> std::io::Result<()> {
    let names: Vec<String> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    backup::to_prune(db::NAME, &names, just_made)
        .into_iter()
        .map(|name| fs::remove_file(dir.join(name)))
        .fold(Ok(()), Result::and)
}

/// Copies the database into `dir` under a name stamped with `now`, local time, and returns that
/// name. A second copy in the same minute is `-2`, `-3`, … rather than replacing the first.
fn back_up(conn: &Connection, dir: &Path, now: DateTime<Local>) -> Result<String, AppError> {
    let stem = format!("{}-{}", db::NAME, now.format(backup::STAMP));
    let mut name = format!("{stem}.db");
    for n in 2.. {
        if !dir.join(&name).exists() {
            break;
        }
        name = format!("{stem}-{n}.db");
    }
    db::vacuum_into(conn, &dir.join(&name)).map_err(|err| AppError::Io {
        message: format!("Couldn't save the backup. {err}"),
    })?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempDir, now};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A database file in `dir` with every migration applied, as the app opens it.
    fn database(dir: &TempDir) -> Connection {
        let mut conn = db::open(&dir.path().join(format!("{}.db", db::NAME))).unwrap();
        let ms = db::migrations(now().to_utc());
        db::prepare(&mut conn, &ms, &dir.path().join("unused"), now()).unwrap();
        conn
    }

    fn account_names(path: &Path) -> Vec<String> {
        Connection::open(path)
            .unwrap()
            .prepare("SELECT name FROM accounts ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn add_account(conn: &Connection, name: &str) {
        conn.execute(
            "INSERT INTO accounts (name, kind, created_at, updated_at) VALUES (?1, 'bank', 'x', 'x')",
            [name],
        )
        .unwrap();
    }

    #[test]
    fn back_up_now_copies_everything_into_the_backup_folder() {
        let dir = TempDir::new("bak1-copy");
        let conn = database(&dir);
        add_account(&conn, "Bank");

        let backup = back_up_now(&conn, dir.path(), now()).unwrap();

        let name = format!("{}-2026-11-03_1405.db", db::NAME);
        assert_eq!(backup.file_name, name);
        assert_eq!(
            account_names(&folder(dir.path()).join(name)),
            ["Cash", "Bank"]
        );
        assert_eq!(
            fs::read_dir(folder(dir.path())).unwrap().count(),
            1,
            "no .tmp left behind"
        );
    }

    #[test]
    fn a_backup_includes_changes_still_in_the_wal() {
        let dir = TempDir::new("bak1-wal");
        let conn = database(&dir);
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .unwrap();
        add_account(&conn, "Only in the wal");
        let live = dir.path().join(format!("{}.db", db::NAME));
        let copy = dir.path().join("plain-copy.db");
        fs::copy(&live, &copy).unwrap();
        assert_eq!(account_names(&copy), ["Cash"], "the row is only in -wal");

        let backup = back_up_now(&conn, dir.path(), now()).unwrap();

        assert_eq!(
            account_names(&folder(dir.path()).join(backup.file_name)),
            ["Cash", "Only in the wal"]
        );
    }

    #[test]
    fn a_second_backup_in_the_same_minute_keeps_the_first() {
        let dir = TempDir::new("bak1-twice");
        let conn = database(&dir);
        let first = back_up_now(&conn, dir.path(), now()).unwrap();
        add_account(&conn, "Bank");

        let second = back_up_now(&conn, dir.path(), now()).unwrap();
        let third = back_up_now(&conn, dir.path(), now()).unwrap();

        let stem = format!("{}-2026-11-03_1405", db::NAME);
        assert_eq!(first.file_name, format!("{stem}.db"));
        assert_eq!(second.file_name, format!("{stem}-2.db"));
        assert_eq!(third.file_name, format!("{stem}-3.db"));
        let read = |backup: &Backup| account_names(&folder(dir.path()).join(&backup.file_name));
        assert_eq!(read(&first), ["Cash"]);
        assert_eq!(read(&second), ["Cash", "Bank"]);
    }

    #[test]
    fn a_backup_cut_off_partway_is_never_renamed_into_place() {
        let dir = TempDir::new("bak1-partial");
        let conn = database(&dir);
        // About 6 MB, more than SQLite's page cache, so VACUUM INTO writes the file as it goes.
        conn.execute_batch(
            "CREATE TABLE filler (x BLOB);
             WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 1500)
             INSERT INTO filler SELECT randomblob(4000) FROM n;",
        )
        .unwrap();
        let backups = folder(dir.path());
        let target = backups.join(format!("{}-2026-11-03_1405.db", db::NAME));
        let tmp = target.with_extension("db.tmp");
        // Stops the copy once part of it is on disk, as a full disk or a crash would.
        let cut = Arc::new(AtomicBool::new(false));
        let cut_in_handler = Arc::clone(&cut);
        let tmp_in_handler = tmp.clone();
        conn.progress_handler(
            100,
            Some(move || {
                let partial = fs::metadata(&tmp_in_handler).is_ok_and(|m| m.len() > 0);
                if partial {
                    cut_in_handler.store(true, Ordering::SeqCst);
                }
                partial
            }),
        )
        .unwrap();

        let result = back_up_now(&conn, dir.path(), now());

        assert!(cut.load(Ordering::SeqCst), "the copy must have started");
        assert!(
            matches!(&result, Err(AppError::Io { message }) if message.starts_with("Couldn't save the backup")),
            "{result:?}"
        );
        assert!(
            !target.exists(),
            "a partial copy must not look like a backup"
        );
        assert!(!tmp.exists(), "the partial copy is removed");

        conn.progress_handler(0, None::<fn() -> bool>).unwrap();
        assert!(back_up_now(&conn, dir.path(), now()).is_ok());
        assert!(target.exists());
    }

    #[test]
    fn a_backup_that_cannot_start_says_why_and_writes_nothing() {
        let dir = TempDir::new("bak1-failed");
        let conn = database(&dir);
        fs::write(folder(dir.path()), b"a file where the folder should be").unwrap();

        let result = back_up_now(&conn, dir.path(), now());

        assert!(
            matches!(&result, Err(AppError::Io { message }) if message.starts_with("Couldn't save the backup")),
            "{result:?}"
        );
        assert!(folder(dir.path()).is_file());
    }

    fn auto_files(dir: &TempDir) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(folder(dir.path()).join("auto"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    fn last_auto(conn: &Connection) -> Option<String> {
        repo::settings::get(conn, LAST_AUTO).unwrap()
    }

    #[test]
    fn the_first_start_of_each_day_makes_one_automatic_backup() {
        let dir = TempDir::new("bak2-daily");
        let mut conn = database(&dir);
        let stamp = |name: &str| format!("{}-{name}.db", db::NAME);

        back_up_daily(&mut conn, dir.path(), now()).unwrap();
        assert_eq!(auto_files(&dir), [stamp("2026-11-03_1405")]);
        assert_eq!(last_auto(&conn).as_deref(), Some("2026-11-03"));

        let later = now() + chrono::Duration::hours(9); // 23:05, the same day
        back_up_daily(&mut conn, dir.path(), later).unwrap();
        assert_eq!(auto_files(&dir).len(), 1, "once a day");

        let tomorrow = now() + chrono::Duration::hours(11);
        back_up_daily(&mut conn, dir.path(), tomorrow).unwrap();
        assert_eq!(
            auto_files(&dir),
            [stamp("2026-11-03_1405"), stamp("2026-11-04_0105")]
        );
        assert_eq!(last_auto(&conn).as_deref(), Some("2026-11-04"));
        assert_eq!(
            account_names(
                &folder(dir.path())
                    .join("auto")
                    .join(stamp("2026-11-04_0105"))
            ),
            ["Cash"]
        );
    }

    #[test]
    fn automatic_backups_keep_exactly_the_newest_30() {
        let dir = TempDir::new("bak3-prune");
        let mut conn = database(&dir);
        let auto = folder(dir.path()).join("auto");
        fs::create_dir_all(&auto).unwrap();
        // 31 older daily backups, Oct 3 to Nov 2.
        let day = |n: u64| {
            (now() - chrono::Days::new(n))
                .format("%Y-%m-%d")
                .to_string()
        };
        for n in 1..=31 {
            fs::write(
                auto.join(format!("{}-{}_0900.db", db::NAME, day(n))),
                b"old",
            )
            .unwrap();
        }
        fs::write(auto.join("notes.txt"), b"mine").unwrap();
        fs::create_dir_all(auto.join(format!("{}-2020-01-01_0000.db", db::NAME))).unwrap();
        let manual = back_up_now(&conn, dir.path(), now() - chrono::Days::new(90)).unwrap();

        back_up_daily(&mut conn, dir.path(), now()).unwrap();

        let files = auto_files(&dir);
        let backups: Vec<&String> = files
            .iter()
            .filter(|n| n.ends_with("_0900.db") || n.ends_with("_1405.db"))
            .collect();
        assert_eq!(backups.len(), 30);
        assert!(
            files.contains(&format!("{}-2026-11-03_1405.db", db::NAME)),
            "today's is kept"
        );
        assert!(files.contains(&format!("{}-{}_0900.db", db::NAME, day(29))));
        assert!(
            !files.contains(&format!("{}-{}_0900.db", db::NAME, day(30))),
            "the two oldest go"
        );
        assert!(!files.contains(&format!("{}-{}_0900.db", db::NAME, day(31))));
        assert!(files.contains(&"notes.txt".to_string()), "other files stay");
        assert!(
            auto.join(format!("{}-2020-01-01_0000.db", db::NAME))
                .is_dir(),
            "folders stay"
        );
        assert!(
            folder(dir.path()).join(manual.file_name).exists(),
            "manual backups are never pruned"
        );
    }

    #[test]
    #[cfg(windows)]
    fn a_locked_old_backup_does_not_stop_the_others_being_deleted() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = TempDir::new("bak3-locked");
        let mut conn = database(&dir);
        let auto = folder(dir.path()).join("auto");
        fs::create_dir_all(&auto).unwrap();
        let old = |n: u64| {
            let day = (now() - chrono::Days::new(n)).format("%Y-%m-%d");
            auto.join(format!("{}-{day}_0900.db", db::NAME))
        };
        for n in 1..=31 {
            fs::write(old(n), b"old").unwrap();
        }
        // The first of the two due to go, held open with no sharing as antivirus or a sync tool
        // can.
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(old(30))
            .unwrap();

        let result = back_up_daily(&mut conn, dir.path(), now());

        assert!(
            matches!(&result, Err(AppError::Io { message }) if message.starts_with("Couldn't delete the oldest")),
            "{result:?}"
        );
        assert!(old(30).exists(), "the locked one stays");
        assert!(!old(31).exists(), "the other one due still goes");
        assert_eq!(
            last_auto(&conn).as_deref(),
            Some("2026-11-03"),
            "the backup itself worked"
        );
        drop(lock);
    }

    #[test]
    fn a_failed_automatic_backup_is_tried_again_at_the_next_start() {
        let dir = TempDir::new("bak2-failed");
        let mut conn = database(&dir);
        let auto = folder(dir.path()).join("auto");
        fs::create_dir_all(folder(dir.path())).unwrap();
        fs::write(&auto, b"a file where the folder should be").unwrap();

        let result = back_up_daily(&mut conn, dir.path(), now());

        assert!(
            matches!(&result, Err(AppError::Io { message }) if message.starts_with("Couldn't save the backup")),
            "{result:?}"
        );
        assert_eq!(last_auto(&conn), None, "nothing is recorded");
        fs::remove_file(&auto).unwrap();
        back_up_daily(&mut conn, dir.path(), now()).unwrap();
        assert_eq!(auto_files(&dir).len(), 1);
    }
}
