use tauri::State;

use crate::error::AppError;
use crate::services::settings::{Settings, SettingsView};
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub fn get_settings(state: State<'_, AppState>) -> Result<SettingsView, AppError> {
    services::settings::get(&*state.conn()?).inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn update_settings(
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<SettingsView, AppError> {
    services::settings::update(&mut *state.conn()?, settings)
        .inspect_err(|err| log::record(&state.log_path, err))
}
