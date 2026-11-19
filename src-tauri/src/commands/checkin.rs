use tauri::State;

use crate::error::AppError;
use crate::services::checkin::{Checkin, Written};
use crate::services::vault::FocusFields;
use crate::state::AppState;
use crate::{log, services};

/// CHK-1 to CHK-7: the whole check-in, or none when nothing is due.
#[tauri::command(async)]
pub fn get_checkin(state: State<'_, AppState>) -> Result<Option<Checkin>, AppError> {
    services::checkin::get(&mut *state.conn()?, chrono::Local::now().date_naive())
        .inspect_err(|err| log::record(&state.log_path, err))
}

/// Finish: `focus` must be one of the offers `get_checkin` would make now.
#[tauri::command(async)]
pub fn complete_checkin(
    state: State<'_, AppState>,
    focus: FocusFields,
) -> Result<Vec<Written>, AppError> {
    services::checkin::complete(&mut *state.conn()?, focus, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}
