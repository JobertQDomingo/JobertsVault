use tauri::State;

use crate::domain::period::ComparisonMode;
use crate::error::AppError;
use crate::services::categories::CategoryKind;
use crate::services::period::PeriodQuery;
use crate::services::reports::{Breakdown, Comparison, Timeseries};
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

#[tauri::command(async)]
pub fn get_category_breakdown(
    state: State<'_, AppState>,
    query: PeriodQuery,
    kind: CategoryKind,
) -> Result<Breakdown, AppError> {
    services::reports::breakdown(
        &*state.conn()?,
        &query,
        kind,
        chrono::Local::now().date_naive(),
    )
    .inspect_err(|err| log::record(&state.log_path, err))
}

/// `mode` none means the comparison setting (SET-7).
#[tauri::command(async)]
pub fn get_comparison(
    state: State<'_, AppState>,
    query: PeriodQuery,
    mode: Option<ComparisonMode>,
) -> Result<Comparison, AppError> {
    services::reports::comparison(
        &*state.conn()?,
        &query,
        mode,
        chrono::Local::now().date_naive(),
    )
    .inspect_err(|err| log::record(&state.log_path, err))
}
