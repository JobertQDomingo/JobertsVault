use tauri::{AppHandle, Manager, State};

use crate::error::AppError;
use crate::services::backup::Backup;
use crate::state::AppState;
use crate::{log, services};

/// Runs `job` on a blocking thread: a backup can take seconds (ARCHITECTURE §4.3).
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
        message: format!("The backup stopped unexpectedly. {err}"),
    })?
}

#[tauri::command(async)]
pub async fn backup_now(app: AppHandle) -> Result<Backup, AppError> {
    blocking(app, |state| {
        services::backup::back_up_now(&*state.conn()?, &state.data_dir, chrono::Local::now())
    })
    .await
}

/// Opens Explorer at the backup folder. The path never comes from the frontend.
#[tauri::command(async)]
pub fn open_backup_folder(state: State<'_, AppState>) -> Result<(), AppError> {
    services::system::open_folder(&services::backup::folder(&state.data_dir))
        .inspect_err(|err| log::record(&state.log_path, err))
}
