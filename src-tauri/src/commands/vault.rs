use tauri::State;

use crate::error::AppError;
use crate::services::vault::VaultStatus;
use crate::state::AppState;
use crate::{log, services};

/// LVL-1 to LVL-8: the stats, the level, the focuses and the streaks for today.
#[tauri::command(async)]
pub fn get_vault_status(state: State<'_, AppState>) -> Result<VaultStatus, AppError> {
    services::vault::get(&mut *state.conn()?, chrono::Local::now().date_naive())
        .inspect_err(|err| log::record(&state.log_path, err))
}
