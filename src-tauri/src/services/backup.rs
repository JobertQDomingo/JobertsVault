use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use chrono::{DateTime, Local, NaiveDate};
use rusqlite::Connection;
use rusqlite_migration::Migrations;
use serde::Serialize;
use ts_rs::TS;

use crate::db;
use crate::domain::backup;
use crate::error::AppError;
use crate::repo;
use crate::services::{failed, invalid, stamp};

const FOLDER: &str = "backup_folder";
const AUTO: &str = "auto_backup_enabled";
const LAST: &str = "last_backup_at";
const LAST_AUTO: &str = "last_auto_backup_date";
/// Settings about this computer's backups, not the data: a restore keeps the current ones
/// (DATA_MODEL §9).
const THIS_COMPUTER: [&str; 4] = [FOLDER, AUTO, LAST, LAST_AUTO];

/// What `backup_now` returns: the name of the file it wrote, for the "Backup saved as …" toast,
/// whose Open folder shows where (UI_FLOWS F9).
#[derive(Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Backup {
    pub file_name: String,
}

/// Where backups go (SET-3): the folder the user chose, else `<app data>\backups` (debug builds:
/// `backups-dev`, ARCHITECTURE §6). Only a chosen folder is stored, so the default follows the
/// app data folder of whichever Windows account opens the data.
pub fn folder(conn: &Connection, data_dir: &Path) -> Result<PathBuf, AppError> {
    Ok(repo::settings::get(conn, FOLDER)?
        .map(PathBuf::from)
        .unwrap_or_else(|| default_folder(data_dir)))
}

fn default_folder(data_dir: &Path) -> PathBuf {
    data_dir.join(db::BACKUPS_DIR)
}

/// Back up now (BAK-1): copies the database, `-wal` included, into the backup folder as
/// `<name>-YYYY-MM-DD_HHMM.db`. These are never deleted automatically (BAK-3).
pub fn back_up_now(
    conn: &Connection,
    data_dir: &Path,
    now: DateTime<Local>,
) -> Result<Backup, AppError> {
    let file_name = back_up(conn, &folder(conn, data_dir)?, db::NAME, now)
        .map_err(failed("Couldn't save the backup."))?;
    record(conn, &[(LAST, &stamp(now))])?;
    Ok(Backup { file_name })
}

/// Saves settings rows in one transaction. `unchecked_transaction` lets a caller holding only a
/// shared `Connection` do so; none of them is already inside one.
fn record(conn: &Connection, rows: &[(&str, &str)]) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;
    for (key, value) in rows {
        repo::settings::set(&tx, key, value)?;
    }
    tx.commit()?;
    Ok(())
}

/// What the Backup tab shows (SET-3, BAK-7).
#[derive(Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BackupStatus {
    /// Where new backups go, and whether that's the default folder.
    pub folder: String,
    pub folder_is_default: bool,
    /// The daily automatic backup (BAK-2) is on.
    pub auto_backup: bool,
    /// The last manual or automatic backup, local time `YYYY-MM-DDTHH:MM`, or none.
    pub last_backup_at: Option<String>,
    /// The warning: the last backup is more than 7 days old, or there's none (BAK-7).
    pub due: bool,
}

pub fn status(
    conn: &Connection,
    data_dir: &Path,
    today: NaiveDate,
) -> Result<BackupStatus, AppError> {
    let last = last_backup(conn)?;
    Ok(BackupStatus {
        folder: folder(conn, data_dir)?.display().to_string(),
        folder_is_default: repo::settings::get(conn, FOLDER)?.is_none(),
        auto_backup: auto_backup_on(conn)?,
        last_backup_at: last.map(|at| at.format("%Y-%m-%dT%H:%M").to_string()),
        due: backup::is_due(last.map(|at| at.date_naive()), today),
    })
}

/// The dashboard's warning (BAK-7, UI_FLOWS F9).
pub fn is_due(conn: &Connection, today: NaiveDate) -> Result<bool, AppError> {
    Ok(backup::is_due(
        last_backup(conn)?.map(|at| at.date_naive()),
        today,
    ))
}

/// `last_backup_at` in local time. A value that isn't a timestamp counts as no backup, so the
/// warning shows rather than hides.
fn last_backup(conn: &Connection) -> Result<Option<DateTime<Local>>, AppError> {
    Ok(repo::settings::get(conn, LAST)?
        .and_then(|text| DateTime::parse_from_rfc3339(&text).ok())
        .map(|at| at.with_timezone(&Local)))
}

fn auto_backup_on(conn: &Connection) -> Result<bool, AppError> {
    Ok(repo::settings::get(conn, AUTO)?.as_deref() != Some("false"))
}

/// Chooses where backups go (SET-3), or with `None` goes back to the default folder. The folder
/// must exist and take a new file. Backups already made stay where they are.
pub fn set_folder(
    conn: &Connection,
    data_dir: &Path,
    chosen: Option<&Path>,
    today: NaiveDate,
) -> Result<BackupStatus, AppError> {
    let custom = match chosen {
        None => None,
        Some(path) => {
            if !path.is_absolute() || !path.is_dir() {
                return Err(invalid("folder", "Choose a folder that exists."));
            }
            if !can_write_in(path) {
                return Err(invalid(
                    "folder",
                    "Jobert's Vault can't save files in this folder. Choose another one.",
                ));
            }
            let default = fs::canonicalize(default_folder(data_dir)).ok();
            (fs::canonicalize(path).ok() != default).then(|| path.display().to_string())
        }
    };
    let tx = conn.unchecked_transaction()?;
    match custom {
        Some(path) => repo::settings::set(&tx, FOLDER, &path)?,
        None => repo::settings::delete(&tx, FOLDER)?,
    }
    tx.commit()?;
    status(conn, data_dir, today)
}

/// Whether a new file can be made in `dir`: makes one and deletes it.
fn can_write_in(dir: &Path) -> bool {
    let probe = dir.join(format!(".{}-write-check.tmp", db::NAME));
    let made = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .is_ok();
    made && fs::remove_file(&probe).is_ok()
}

/// Turns the daily automatic backup on or off (SET-3, BAK-2).
pub fn set_auto_backup(
    conn: &Connection,
    data_dir: &Path,
    on: bool,
    today: NaiveDate,
) -> Result<BackupStatus, AppError> {
    record(conn, &[(AUTO, &on.to_string())])?;
    status(conn, data_dir, today)
}

/// The daily automatic backup (BAK-2), unless it's turned off: the first time the app opens on a
/// local day, it copies the database into `<backup folder>\auto` and records it, then keeps only
/// the newest 30 there (BAK-3). A failed copy records nothing, so the next start tries again.
pub fn back_up_daily(
    conn: &Connection,
    data_dir: &Path,
    now: DateTime<Local>,
) -> Result<(), AppError> {
    let today = now.date_naive().to_string();
    if !auto_backup_on(conn)?
        || repo::settings::get(conn, LAST_AUTO)?.as_deref() == Some(today.as_str())
    {
        return Ok(());
    }
    let dir = folder(conn, data_dir)?.join("auto");
    let made = back_up(conn, &dir, db::NAME, now).map_err(failed("Couldn't save the backup."))?;
    record(conn, &[(LAST_AUTO, &today), (LAST, &stamp(now))])?;
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

/// Copies the database into `dir` as `<prefix>-<now, local time>.db` and returns that name. A
/// second copy in the same minute is `-2`, `-3`, … rather than replacing the first.
fn back_up(
    conn: &Connection,
    dir: &Path,
    prefix: &str,
    now: DateTime<Local>,
) -> Result<String, AppError> {
    let stem = format!("{prefix}-{}", now.format(backup::STAMP));
    let mut name = format!("{stem}.db");
    for n in 2.. {
        if !dir.join(&name).exists() {
            break;
        }
        name = format!("{stem}-{n}.db");
    }
    db::vacuum_into(conn, &dir.join(&name))?;
    Ok(name)
}

/// What `inspect_backup` returns: when the chosen file was last written, in local time as
/// `YYYY-MM-DDTHH:MM`, for the confirm message (UI_FLOWS F9).
#[derive(Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BackupFile {
    pub modified: String,
}

/// Checks a file the user picked to restore, without changing anything (BAK-4).
pub fn inspect(
    data_dir: &Path,
    db_path: &Path,
    chosen: &Path,
    now: DateTime<Local>,
) -> Result<BackupFile, AppError> {
    let scratch = Scratch(data_dir.join(format!("{}-inspect.db.tmp", db::NAME)));
    drop(open_copy(
        chosen,
        db_path,
        &scratch.0,
        &db::migrations(now.to_utc()),
    )?);
    // A copy made by hand can have a -wal written after the file itself.
    let wal = fs::metadata(with_suffix(chosen, "-wal")).and_then(|m| m.modified());
    let modified = fs::metadata(chosen)?
        .modified()?
        .max(wal.unwrap_or(UNIX_EPOCH));
    let modified: DateTime<Local> = modified.into();
    Ok(BackupFile {
        modified: modified.format("%Y-%m-%dT%H:%M").to_string(),
    })
}

/// Restore (BAK-4, ARCHITECTURE §7): replaces the database at `db_path`, open as `conn`, with the
/// backup at `chosen`. If anything fails before the swap, nothing changes.
pub fn restore(
    conn: &mut Connection,
    data_dir: &Path,
    db_path: &Path,
    chosen: &Path,
    now: DateTime<Local>,
) -> Result<(), AppError> {
    let ms = db::migrations(now.to_utc());
    restore_with(conn, data_dir, db_path, chosen, &ms, now)
}

fn restore_with(
    conn: &mut Connection,
    data_dir: &Path,
    db_path: &Path,
    chosen: &Path,
    ms: &Migrations,
    now: DateTime<Local>,
) -> Result<(), AppError> {
    // 1. A checked copy of the backup, brought up to date, that keeps this computer's backup
    // settings. The chosen file itself is never changed, so it already is the copy from before
    // the migration that BAK-5 asks for.
    let scratch = Scratch(data_dir.join(format!("{}-restore.db.tmp", db::NAME)));
    let mut copy = open_copy(chosen, db_path, &scratch.0, ms)?;
    ms.to_latest(&mut copy)?;
    let tx = copy.transaction()?;
    for key in THIS_COMPUTER {
        match repo::settings::get(conn, key)? {
            Some(value) => repo::settings::set(&tx, key, &value)?,
            None => repo::settings::delete(&tx, key)?,
        }
    }
    tx.commit()?;
    // Closed with its -wal checkpointed, so the file alone holds it all.
    db::checkpoint(&copy)?;
    copy.close().map_err(|(_, err)| AppError::from(err))?;

    // 2. A safety copy of the current data, -wal included.
    let prefix = format!("{}-pre-restore", db::NAME);
    back_up(conn, &folder(conn, data_dir)?, &prefix, now).map_err(failed(
        "Couldn't save a safety copy of your current data, so nothing was changed.",
    ))?;

    // 3. The swap, then 4. reopening, with nothing left to migrate.
    swap(conn, db_path, &scratch.0)?;
    let reopened = db::open(db_path).and_then(|mut reopened| {
        db::prepare(&mut reopened, ms, &db::pre_migration_dir(data_dir), now)?;
        Ok(reopened)
    });
    *conn = reopened.map_err(failed(
        "The backup was restored, but it couldn't be opened. Restart Jobert's Vault.",
    ))?;
    Ok(())
}

/// Copies `chosen`, and its `-wal` if it has one, to `scratch` and opens the copy, so the user's
/// file is never changed. Refused unless it's a sound Jobert's Vault database that this version
/// can open.
fn open_copy(
    chosen: &Path,
    db_path: &Path,
    scratch: &Path,
    ms: &Migrations,
) -> Result<Connection, AppError> {
    let refused = |message: &str| invalid("path", message);
    if !chosen.is_absolute() || !chosen.is_file() {
        return Err(refused("Choose a backup file."));
    }
    if fs::canonicalize(chosen)? == fs::canonicalize(db_path)? {
        return Err(refused(
            "That's the database Jobert's Vault is using. Choose a backup file.",
        ));
    }
    for leftover in sidecars(scratch) {
        db::remove_if_there(&leftover)?;
    }
    fs::copy(chosen, scratch)?;
    // A copy made by hand can hold its latest changes in -wal, which SQLite reads on opening.
    let wal = with_suffix(chosen, "-wal");
    if wal.is_file() {
        fs::copy(&wal, with_suffix(scratch, "-wal"))?;
    }
    let not_ours = || refused("This file isn't a Jobert's Vault backup, so nothing was changed.");
    let copy = db::open(scratch).map_err(|_| not_ours())?;
    if !db::is_vault(&copy).unwrap_or(false) {
        return Err(not_ours());
    }
    if ms.pending_migrations(&copy).map_err(|_| not_ours())? < 0 {
        return Err(refused(concat!(
            "This backup was made by a newer version of Jobert's Vault, so nothing was changed. ",
            "Install the latest version to restore it."
        )));
    }
    Ok(copy)
}

/// Closes `conn` and puts `copy` in place of the database file. A `-wal` that still holds
/// changes is never deleted: unless the checkpoint emptied it, the original is reopened and
/// nothing changes. On success `conn` is a placeholder until the caller reopens the database.
fn swap(conn: &mut Connection, db_path: &Path, copy: &Path) -> Result<(), AppError> {
    let unchanged = failed("Couldn't replace the database, so nothing was changed.");
    db::checkpoint(conn).map_err(&unchanged)?;
    let live = std::mem::replace(conn, Connection::open_in_memory()?);
    if let Err((live, err)) = live.close() {
        *conn = live;
        return Err(unchanged(err.into()));
    }
    let wal = with_suffix(db_path, "-wal");
    let replaced = if fs::metadata(&wal).is_ok_and(|m| m.len() > 0) {
        Err(AppError::Io {
            message: "Recent changes are still waiting to be written.".into(),
        })
    } else {
        fs::rename(copy, db_path).map_err(AppError::from)
    };
    if let Err(err) = replaced {
        *conn = db::open(db_path)?;
        return Err(unchanged(err));
    }
    // Both are empty now; SQLite makes new ones for the restored file.
    let _ = fs::remove_file(&wal);
    let _ = fs::remove_file(with_suffix(db_path, "-shm"));
    Ok(())
}

/// A scratch copy in the data folder, removed with its `-wal` and `-shm` when dropped.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        for file in sidecars(&self.0) {
            let _ = fs::remove_file(file);
        }
    }
}

fn sidecars(path: &Path) -> [PathBuf; 3] {
    [
        path.to_path_buf(),
        with_suffix(path, "-wal"),
        with_suffix(path, "-shm"),
    ]
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    name.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempDir, now};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A database file in `dir` with every migration applied, as the app opens it.
    /// The default backup folder in `dir`.
    fn backups(dir: &TempDir) -> PathBuf {
        dir.path().join(db::BACKUPS_DIR)
    }

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
        assert_eq!(account_names(&backups(&dir).join(name)), ["Cash", "Bank"]);
        assert_eq!(
            fs::read_dir(backups(&dir)).unwrap().count(),
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
            account_names(&backups(&dir).join(backup.file_name)),
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
        let read = |backup: &Backup| account_names(&backups(&dir).join(&backup.file_name));
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
        let backups = backups(&dir);
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
        fs::write(backups(&dir), b"a file where the folder should be").unwrap();

        let result = back_up_now(&conn, dir.path(), now());

        assert!(
            matches!(&result, Err(AppError::Io { message }) if message.starts_with("Couldn't save the backup")),
            "{result:?}"
        );
        assert!(backups(&dir).is_file());
    }

    fn auto_files(dir: &TempDir) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(backups(dir).join("auto"))
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
        let conn = database(&dir);
        let stamp = |name: &str| format!("{}-{name}.db", db::NAME);

        back_up_daily(&conn, dir.path(), now()).unwrap();
        assert_eq!(auto_files(&dir), [stamp("2026-11-03_1405")]);
        assert_eq!(last_auto(&conn).as_deref(), Some("2026-11-03"));

        let later = now() + chrono::Duration::hours(9); // 23:05, the same day
        back_up_daily(&conn, dir.path(), later).unwrap();
        assert_eq!(auto_files(&dir).len(), 1, "once a day");

        let tomorrow = now() + chrono::Duration::hours(11);
        back_up_daily(&conn, dir.path(), tomorrow).unwrap();
        assert_eq!(
            auto_files(&dir),
            [stamp("2026-11-03_1405"), stamp("2026-11-04_0105")]
        );
        assert_eq!(last_auto(&conn).as_deref(), Some("2026-11-04"));
        assert_eq!(
            account_names(&backups(&dir).join("auto").join(stamp("2026-11-04_0105"))),
            ["Cash"]
        );
    }

    #[test]
    fn automatic_backups_keep_exactly_the_newest_30() {
        let dir = TempDir::new("bak3-prune");
        let conn = database(&dir);
        let auto = backups(&dir).join("auto");
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

        back_up_daily(&conn, dir.path(), now()).unwrap();

        let files = auto_files(&dir);
        let kept: Vec<&String> = files
            .iter()
            .filter(|n| n.ends_with("_0900.db") || n.ends_with("_1405.db"))
            .collect();
        assert_eq!(kept.len(), 30);
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
            backups(&dir).join(manual.file_name).exists(),
            "manual backups are never pruned"
        );
    }

    #[test]
    #[cfg(windows)]
    fn a_locked_old_backup_does_not_stop_the_others_being_deleted() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = TempDir::new("bak3-locked");
        let conn = database(&dir);
        let auto = backups(&dir).join("auto");
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

        let result = back_up_daily(&conn, dir.path(), now());

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
        let conn = database(&dir);
        let auto = backups(&dir).join("auto");
        fs::create_dir_all(backups(&dir)).unwrap();
        fs::write(&auto, b"a file where the folder should be").unwrap();

        let result = back_up_daily(&conn, dir.path(), now());

        assert!(
            matches!(&result, Err(AppError::Io { message }) if message.starts_with("Couldn't save the backup")),
            "{result:?}"
        );
        assert_eq!(last_auto(&conn), None, "nothing is recorded");
        fs::remove_file(&auto).unwrap();
        back_up_daily(&conn, dir.path(), now()).unwrap();
        assert_eq!(auto_files(&dir).len(), 1);
    }

    // Restore (BAK-4)

    fn db_path(dir: &TempDir) -> PathBuf {
        dir.path().join(format!("{}.db", db::NAME))
    }

    fn names_in(conn: &Connection) -> Vec<String> {
        conn.prepare("SELECT name FROM accounts ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    /// A backup holding Cash and Bank, then a change after it that only the `-wal` holds.
    fn backed_up_then_changed(dir: &TempDir) -> (Connection, PathBuf) {
        let conn = database(dir);
        add_account(&conn, "Bank");
        let backup = back_up_now(&conn, dir.path(), now()).unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .unwrap();
        add_account(&conn, "After the backup");
        (conn, backups(dir).join(backup.file_name))
    }

    fn pre_restore(dir: &TempDir) -> PathBuf {
        backups(dir).join(format!("{}-pre-restore-2026-11-03_1405.db", db::NAME))
    }

    /// Scratch copies left in the data folder.
    fn leftovers(dir: &TempDir) -> Vec<String> {
        fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.contains(".tmp"))
            .collect()
    }

    const BEFORE: [&str; 3] = ["Cash", "Bank", "After the backup"];

    #[test]
    fn restore_swaps_in_the_backup_after_a_safety_copy() {
        let dir = TempDir::new("bak4-restore");
        let (mut conn, backup) = backed_up_then_changed(&dir);
        let plain = dir.path().join("plain-copy");
        fs::copy(db_path(&dir), &plain).unwrap();
        assert_eq!(
            account_names(&plain),
            ["Cash", "Bank"],
            "the newest row is only in -wal"
        );
        let backup_bytes = fs::read(&backup).unwrap();

        restore(&mut conn, dir.path(), &db_path(&dir), &backup, now()).unwrap();

        assert_eq!(names_in(&conn), ["Cash", "Bank"]);
        assert_eq!(
            account_names(&pre_restore(&dir)),
            BEFORE,
            "the safety copy has the -wal's rows"
        );
        assert_eq!(
            fs::read(&backup).unwrap(),
            backup_bytes,
            "the backup itself is untouched"
        );
        assert!(leftovers(&dir).is_empty(), "{:?}", leftovers(&dir));
        // The reopened connection is the real file again, with the app's connection settings.
        let foreign_keys: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(foreign_keys, 1);
        add_account(&conn, "Added after");
        drop(conn);
        assert_eq!(
            account_names(&db_path(&dir)),
            ["Cash", "Bank", "Added after"]
        );
    }

    #[test]
    fn a_file_that_is_not_a_vault_backup_is_refused_and_nothing_changes() {
        let dir = TempDir::new("bak4-refused");
        let (mut conn, backup) = backed_up_then_changed(&dir);
        let other = dir.path().join("other");
        fs::create_dir_all(&other).unwrap();
        let file = |name: &str| other.join(name);
        fs::write(file("garbage.db"), vec![0x5a; 8192]).unwrap();
        fs::write(file("empty.db"), b"").unwrap();
        Connection::open(file("foreign.db"))
            .unwrap()
            .execute_batch("CREATE TABLE notes (x); PRAGMA user_version = 3;")
            .unwrap();
        fs::copy(&backup, file("unversioned.db")).unwrap();
        Connection::open(file("unversioned.db"))
            .unwrap()
            .pragma_update(None, "user_version", 0)
            .unwrap();
        // Page 1 (the schema) intact, every page after it overwritten.
        let mut bytes = fs::read(&backup).unwrap();
        bytes[4096..].fill(0xff);
        fs::write(file("corrupt.db"), bytes).unwrap();
        fs::copy(&backup, file("newer.db")).unwrap();
        Connection::open(file("newer.db"))
            .unwrap()
            .pragma_update(None, "user_version", 99)
            .unwrap();

        let not_ours = "isn't a Jobert's Vault backup, so nothing was changed";
        let cases: Vec<(PathBuf, &str)> = vec![
            (file("garbage.db"), not_ours),
            (file("empty.db"), not_ours),
            (file("foreign.db"), not_ours),
            (file("unversioned.db"), not_ours),
            (file("corrupt.db"), not_ours),
            (
                file("newer.db"),
                "made by a newer version of Jobert's Vault, so nothing was changed",
            ),
            (db_path(&dir), "That's the database Jobert's Vault is using"),
            (PathBuf::from("relative.db"), "Choose a backup file"),
            (file("missing.db"), "Choose a backup file"),
            (other.clone(), "Choose a backup file"),
        ];
        for (path, why) in cases {
            let refused = |result: &Result<(), AppError>| {
                matches!(result, Err(AppError::Validation { field, message })
                    if field == "path" && message.contains(why))
            };
            let inspected = inspect(dir.path(), &db_path(&dir), &path, now()).map(|_| ());
            assert!(refused(&inspected), "inspect {path:?}: {inspected:?}");
            let restored = restore(&mut conn, dir.path(), &db_path(&dir), &path, now());
            assert!(refused(&restored), "restore {path:?}: {restored:?}");
            assert_eq!(names_in(&conn), BEFORE);
        }
        assert!(
            !pre_restore(&dir).exists(),
            "no safety copy for a refused file"
        );
        assert!(leftovers(&dir).is_empty(), "{:?}", leftovers(&dir));
    }

    #[test]
    fn an_older_backup_is_brought_up_to_date_before_the_swap() {
        use rusqlite_migration::M;
        let dir = TempDir::new("bak4-older");
        let (mut conn, backup) = backed_up_then_changed(&dir);
        let next_version = Migrations::new(vec![
            M::up(include_str!("../../migrations/0001_initial.sql")),
            M::up("CREATE TABLE added_later (x);"),
        ]);

        restore_with(
            &mut conn,
            dir.path(),
            &db_path(&dir),
            &backup,
            &next_version,
            now(),
        )
        .unwrap();

        assert_eq!(names_in(&conn), ["Cash", "Bank"]);
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);
        assert!(
            !db::pre_migration_dir(dir.path()).exists(),
            "the chosen file, unchanged, is the copy from before the migration"
        );
        assert_eq!(account_names(&backup), ["Cash", "Bank"]);
    }

    #[test]
    fn a_restore_keeps_this_computers_backup_settings() {
        let dir = TempDir::new("bak4-keys");
        let mut conn = database(&dir);
        let set = |conn: &Connection, key: &str, value: &str| {
            repo::settings::set(conn, key, value).unwrap()
        };
        // The backup comes from another computer, whose backups went to its own folder.
        let old_pc = dir.path().join("Old PC");
        fs::create_dir_all(&old_pc).unwrap();
        set(&conn, "currency_symbol", "$");
        set(&conn, "backup_folder", &old_pc.display().to_string());
        set(&conn, LAST_AUTO, "2026-01-01");
        let backup = old_pc.join(back_up_now(&conn, dir.path(), now()).unwrap().file_name);
        set(&conn, "currency_symbol", "\u{20B1}");
        repo::settings::delete(&conn, "backup_folder").unwrap();
        set(&conn, "auto_backup_enabled", "false");
        set(&conn, "last_backup_at", "2026-11-03T06:05:00Z");
        set(&conn, LAST_AUTO, "2026-11-03");

        restore(&mut conn, dir.path(), &db_path(&dir), &backup, now()).unwrap();

        let get = |key: &str| repo::settings::get(&conn, key).unwrap();
        assert_eq!(
            get("currency_symbol").as_deref(),
            Some("$"),
            "the data's own settings"
        );
        assert_eq!(get("backup_folder"), None);
        assert_eq!(get("auto_backup_enabled").as_deref(), Some("false"));
        assert_eq!(
            get("last_backup_at").as_deref(),
            Some("2026-11-03T06:05:00Z")
        );
        assert_eq!(get(LAST_AUTO).as_deref(), Some("2026-11-03"));
    }

    #[test]
    fn a_failed_safety_copy_stops_the_restore() {
        let dir = TempDir::new("bak4-safety");
        let (mut conn, backup) = backed_up_then_changed(&dir);
        let moved = dir.path().join("chosen.db");
        fs::rename(&backup, &moved).unwrap();
        fs::remove_dir_all(backups(&dir)).unwrap();
        fs::write(backups(&dir), b"a file where the folder should be").unwrap();

        let result = restore(&mut conn, dir.path(), &db_path(&dir), &moved, now());

        assert!(
            matches!(&result, Err(AppError::Io { message })
                if message.starts_with("Couldn't save a safety copy of your current data, so nothing was changed.")),
            "{result:?}"
        );
        assert_eq!(names_in(&conn), BEFORE);
        assert!(leftovers(&dir).is_empty(), "{:?}", leftovers(&dir));
    }

    #[test]
    fn a_copy_made_by_hand_is_restored_with_its_wal() {
        let dir = TempDir::new("bak4-hand");
        let mut conn = database(&dir);
        // Another install's files, copied while it ran: its newest row is only in -wal.
        let other = TempDir::new("bak4-hand-other");
        let running = database(&other);
        running
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .unwrap();
        add_account(&running, "Only in the wal");
        let copied = dir.path().join("copied");
        fs::create_dir_all(&copied).unwrap();
        let chosen = copied.join("jobertsvault.db");
        for suffix in ["", "-wal"] {
            fs::copy(
                with_suffix(&db_path(&other), suffix),
                with_suffix(&chosen, suffix),
            )
            .unwrap();
        }
        drop(running);
        // The -wal was written after the file, so the confirm gives its time.
        let at = |hour| {
            chrono::NaiveDate::from_ymd_opt(2026, 10, 20)
                .unwrap()
                .and_hms_opt(hour, 30, 0)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap()
        };
        for (suffix, hour) in [("", 9), ("-wal", 18)] {
            fs::File::options()
                .write(true)
                .open(with_suffix(&chosen, suffix))
                .unwrap()
                .set_modified(at(hour).into())
                .unwrap();
        }
        let file = inspect(dir.path(), &db_path(&dir), &chosen, now()).unwrap();
        assert_eq!(file.modified, "2026-10-20T18:30");
        let wal_bytes = fs::read(with_suffix(&chosen, "-wal")).unwrap();

        restore(&mut conn, dir.path(), &db_path(&dir), &chosen, now()).unwrap();

        assert_eq!(names_in(&conn), ["Cash", "Only in the wal"]);
        assert_eq!(
            fs::read(with_suffix(&chosen, "-wal")).unwrap(),
            wal_bytes,
            "left as it was"
        );
    }

    #[test]
    fn a_wal_that_still_holds_changes_is_never_deleted() {
        let dir = TempDir::new("bak4-busy");
        let (mut conn, backup) = backed_up_then_changed(&dir);
        conn.busy_timeout(std::time::Duration::from_millis(50))
            .unwrap();
        // A reader part-way through a read keeps the checkpoint from emptying the -wal.
        let reader = Connection::open(db_path(&dir)).unwrap();
        reader.execute_batch("BEGIN").unwrap();
        let _: i64 = reader
            .query_row("SELECT count(*) FROM accounts", [], |row| row.get(0))
            .unwrap();
        let wal = with_suffix(&db_path(&dir), "-wal");
        assert!(fs::metadata(&wal).unwrap().len() > 0);

        let result = restore(&mut conn, dir.path(), &db_path(&dir), &backup, now());

        assert!(
            matches!(&result, Err(AppError::Io { message })
                if message.starts_with("Couldn't replace the database, so nothing was changed.")),
            "{result:?}"
        );
        assert!(
            fs::metadata(&wal).unwrap().len() > 0,
            "the -wal is still there"
        );
        assert_eq!(names_in(&conn), BEFORE);
        drop(reader);
        add_account(&conn, "Still works");
    }

    #[test]
    fn inspect_says_when_the_backup_was_written_and_changes_nothing() {
        let dir = TempDir::new("bak4-inspect");
        let (conn, backup) = backed_up_then_changed(&dir);
        let written = chrono::NaiveDate::from_ymd_opt(2026, 10, 20)
            .unwrap()
            .and_hms_opt(9, 30, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();
        fs::File::options()
            .write(true)
            .open(&backup)
            .unwrap()
            .set_modified(written.into())
            .unwrap();
        let bytes = fs::read(&backup).unwrap();

        let file = inspect(dir.path(), &db_path(&dir), &backup, now()).unwrap();

        assert_eq!(file.modified, "2026-10-20T09:30");
        assert_eq!(fs::read(&backup).unwrap(), bytes);
        assert_eq!(names_in(&conn), BEFORE);
        assert!(leftovers(&dir).is_empty(), "{:?}", leftovers(&dir));
    }

    // Backup settings (SET-3, BAK-7)

    fn today() -> NaiveDate {
        now().date_naive()
    }

    #[test]
    fn backups_go_to_the_chosen_folder_until_the_default_is_chosen_again() {
        let dir = TempDir::new("set3-folder");
        let conn = database(&dir);
        let chosen = dir.path().join("D drive");
        fs::create_dir_all(&chosen).unwrap();

        let shown = set_folder(&conn, dir.path(), Some(&chosen), today()).unwrap();
        assert_eq!(shown.folder, chosen.display().to_string());
        assert!(!shown.folder_is_default);
        let manual = back_up_now(&conn, dir.path(), now()).unwrap();
        assert!(chosen.join(&manual.file_name).exists());
        back_up_daily(&conn, dir.path(), now()).unwrap();
        assert_eq!(fs::read_dir(chosen.join("auto")).unwrap().count(), 1);
        assert!(
            !backups(&dir).exists(),
            "nothing went to the default folder"
        );
        assert!(
            fs::read_dir(&chosen).unwrap().all(|e| !e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")),
            "the write check leaves nothing behind"
        );

        let back = set_folder(&conn, dir.path(), None, today()).unwrap();
        assert_eq!(back.folder, backups(&dir).display().to_string());
        assert!(back.folder_is_default);
        assert_eq!(repo::settings::get(&conn, FOLDER).unwrap(), None);
    }

    #[test]
    fn choosing_the_default_folder_stores_nothing() {
        let dir = TempDir::new("set3-default");
        let conn = database(&dir);
        fs::create_dir_all(backups(&dir)).unwrap();
        let shown = set_folder(&conn, dir.path(), Some(&backups(&dir)), today()).unwrap();
        assert!(shown.folder_is_default);
        assert_eq!(repo::settings::get(&conn, FOLDER).unwrap(), None);
    }

    #[test]
    fn a_folder_that_is_missing_relative_or_a_file_is_refused() {
        let dir = TempDir::new("set3-refused");
        let conn = database(&dir);
        let file = dir.path().join("a file");
        fs::write(&file, b"x").unwrap();
        for bad in [dir.path().join("missing"), PathBuf::from("relative"), file] {
            let result = set_folder(&conn, dir.path(), Some(&bad), today());
            assert!(
                matches!(&result, Err(AppError::Validation { field, .. }) if field == "folder"),
                "{bad:?}: {result:?}"
            );
        }
        assert_eq!(repo::settings::get(&conn, FOLDER).unwrap(), None);
    }

    #[test]
    fn manual_and_automatic_backups_both_count_as_the_last_backup() {
        let dir = TempDir::new("bak7-last");
        let conn = database(&dir);
        let first = status(&conn, dir.path(), today()).unwrap();
        assert_eq!(first.last_backup_at, None);
        assert!(first.due, "never backed up");

        back_up_now(&conn, dir.path(), now()).unwrap();
        let after = status(&conn, dir.path(), today()).unwrap();
        assert_eq!(after.last_backup_at.as_deref(), Some("2026-11-03T14:05"));
        assert!(!after.due);
        let eight_days_on = today() + chrono::Days::new(8);
        assert!(status(&conn, dir.path(), eight_days_on).unwrap().due);
        assert!(is_due(&conn, eight_days_on).unwrap());
        assert!(!is_due(&conn, today() + chrono::Days::new(7)).unwrap());

        let later = now() + chrono::Duration::days(8);
        back_up_daily(&conn, dir.path(), later).unwrap();
        let auto = status(&conn, dir.path(), later.date_naive()).unwrap();
        assert_eq!(auto.last_backup_at.as_deref(), Some("2026-11-11T14:05"));
        assert!(!auto.due);
    }

    #[test]
    fn automatic_backups_can_be_turned_off() {
        let dir = TempDir::new("set3-auto-off");
        let conn = database(&dir);
        assert!(status(&conn, dir.path(), today()).unwrap().auto_backup);

        let off = set_auto_backup(&conn, dir.path(), false, today()).unwrap();
        assert!(!off.auto_backup);
        back_up_daily(&conn, dir.path(), now()).unwrap();
        assert!(
            !backups(&dir).exists(),
            "no automatic backup while it's off"
        );
        assert_eq!(last_auto(&conn), None);

        set_auto_backup(&conn, dir.path(), true, today()).unwrap();
        back_up_daily(&conn, dir.path(), now()).unwrap();
        assert_eq!(auto_files(&dir).len(), 1);
    }

    #[test]
    fn the_restores_safety_copy_goes_to_the_chosen_folder() {
        let dir = TempDir::new("set3-restore");
        let (mut conn, backup) = backed_up_then_changed(&dir);
        let chosen = dir.path().join("chosen");
        fs::create_dir_all(&chosen).unwrap();
        set_folder(&conn, dir.path(), Some(&chosen), today()).unwrap();

        restore(&mut conn, dir.path(), &db_path(&dir), &backup, now()).unwrap();

        let safety = chosen.join(format!("{}-pre-restore-2026-11-03_1405.db", db::NAME));
        assert_eq!(account_names(&safety), BEFORE);
        assert_eq!(
            folder(&conn, dir.path()).unwrap(),
            chosen,
            "the restored data keeps this computer's folder"
        );
    }
}
