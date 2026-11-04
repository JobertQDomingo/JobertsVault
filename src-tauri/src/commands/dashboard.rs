use tauri::State;

use crate::error::AppError;
use crate::services::dashboard::Dashboard;
use crate::services::period::PeriodQuery;
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub fn get_dashboard(
    state: State<'_, AppState>,
    query: PeriodQuery,
) -> Result<Dashboard, AppError> {
    services::dashboard::get(&*state.conn()?, &query, chrono::Local::now().date_naive())
        .inspect_err(|err| log::record(&state.log_path, err))
}
