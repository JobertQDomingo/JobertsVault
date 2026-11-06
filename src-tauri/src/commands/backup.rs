use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::error::AppError;
use crate::services::backup::{Backup, BackupFile, BackupStatus};
use crate::services::export::{CsvExport, DateRange};
use crate::state::AppState;
use crate::{log, services};

/// Runs `job` on a blocking thread: a backup, a restore or an export can take seconds
/// (ARCHITECTURE §4.3).
async fn blocking<T: Send + 'static>(
    app: AppHandle,
    job: impl FnOnce(&AppState) -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        job(&state).inspect_err(|err| log::record(&state.log_path, err))
    })
    .await
    .map_err(|err| AppError::Io {
        message: format!("It stopped unexpectedly, so it may not have finished. {err}"),
    })?
}

#[tauri::command(async)]
pub async fn backup_now(app: AppHandle) -> Result<Backup, AppError> {
    blocking(app, |state| {
        services::backup::back_up_now(&*state.conn()?, &state.data_dir, chrono::Local::now())
    })
    .await
}

/// Checks a file picked to restore and says when it was written, changing nothing (BAK-4).
#[tauri::command(async)]
pub async fn inspect_backup(app: AppHandle, path: PathBuf) -> Result<BackupFile, AppError> {
    blocking(app, move |state| {
        services::backup::inspect(&state.data_dir, &state.db_path, &path, chrono::Local::now())
    })
    .await
}

/// Replaces all the data with the backup at `path` (BAK-4). The connection stays locked
/// throughout, so no other command reads or writes in between.
#[tauri::command(async)]
pub async fn restore_backup(app: AppHandle, path: PathBuf) -> Result<(), AppError> {
    blocking(app, move |state| {
        services::backup::restore(
            &mut *state.conn()?,
            &state.data_dir,
            &state.db_path,
            &path,
            chrono::Local::now(),
        )
    })
    .await
}

/// Writes the transactions from `range` to the CSV file at `path` (BAK-6).
#[tauri::command(async)]
pub async fn export_csv(
    app: AppHandle,
    range: DateRange,
    path: PathBuf,
) -> Result<CsvExport, AppError> {
    blocking(app, move |state| {
        // The connection is locked only while the rows are read, not while the file is written.
        let file = services::export::prepare(&*state.conn()?, &range, &path)?;
        file.save()
    })
    .await
}

/// Opens Explorer at the backup folder. The path never comes from the frontend.
#[tauri::command(async)]
pub fn open_backup_folder(state: State<'_, AppState>) -> Result<(), AppError> {
    let folder = services::backup::folder(&*state.conn()?, &state.data_dir);
    folder
        .and_then(|folder| services::system::open_folder(&folder))
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn get_backup_status(state: State<'_, AppState>) -> Result<BackupStatus, AppError> {
    services::backup::status(&*state.conn()?, &state.data_dir, today())
        .inspect_err(|err| log::record(&state.log_path, err))
}

/// Chooses the backup folder (SET-3); none goes back to the default one.
#[tauri::command(async)]
pub fn set_backup_folder(
    state: State<'_, AppState>,
    path: Option<PathBuf>,
) -> Result<BackupStatus, AppError> {
    services::backup::set_folder(&*state.conn()?, &state.data_dir, path.as_deref(), today())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn set_auto_backup(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<BackupStatus, AppError> {
    services::backup::set_auto_backup(&*state.conn()?, &state.data_dir, enabled, today())
        .inspect_err(|err| log::record(&state.log_path, err))
}

fn today() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}
