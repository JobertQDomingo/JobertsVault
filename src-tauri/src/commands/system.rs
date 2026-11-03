use tauri::State;

use crate::error::AppError;
use crate::state::{AppState, StartupStatus};
use crate::{log, services};

#[tauri::command]
pub fn get_startup_status(state: State<'_, AppState>) -> StartupStatus {
    state.startup_status()
}

#[tauri::command]
pub fn open_data_folder(state: State<'_, AppState>) -> Result<(), AppError> {
    services::system::open_folder(&state.data_dir)
        .inspect_err(|err| log::record(&state.log_path, err))
}
