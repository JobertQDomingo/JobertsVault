use tauri::State;

use crate::error::AppError;
use crate::services::period::PeriodQuery;
use crate::services::reports::Timeseries;
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub fn get_timeseries(
    state: State<'_, AppState>,
    query: PeriodQuery,
) -> Result<Timeseries, AppError> {
    services::reports::timeseries(&*state.conn()?, &query, chrono::Local::now().date_naive())
        .inspect_err(|err| log::record(&state.log_path, err))
}
