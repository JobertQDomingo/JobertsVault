use tauri::State;

use crate::error::AppError;
use crate::services::period::{PeriodQuery, PeriodView};
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub fn get_period(state: State<'_, AppState>, query: PeriodQuery) -> Result<PeriodView, AppError> {
    services::period::get(&*state.conn()?, &query, chrono::Local::now().date_naive())
        .inspect_err(|err| log::record(&state.log_path, err))
}
