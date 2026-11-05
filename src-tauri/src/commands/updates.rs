use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub async fn check_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, AppError> {
    services::updates::check(&app)
        .await
        .inspect_err(|err| log::record(&state.log_path, err))
}

/// Doesn't return on success: the installer closes the app and opens the new version.
#[tauri::command(async)]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    services::updates::install(&app)
        .await
        .inspect_err(|err| log::record(&state.log_path, err))
}
