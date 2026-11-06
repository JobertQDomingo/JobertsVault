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
    account_id: Option<i64>,
) -> Result<Timeseries, AppError> {
    services::reports::timeseries(
        &*state.conn()?,
        &query,
        account_id,
        chrono::Local::now().date_naive(),
    )
    .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn get_category_breakdown(
    state: State<'_, AppState>,
    query: PeriodQuery,
    kind: CategoryKind,
    account_id: Option<i64>,
) -> Result<Breakdown, AppError> {
    services::reports::breakdown(
        &*state.conn()?,
        &query,
        kind,
        account_id,
        chrono::Local::now().date_naive(),
    )
    .inspect_err(|err| log::record(&state.log_path, err))
}

/// `mode` none means the comparison setting (SET-7). `account_id` none means every account.
#[tauri::command(async)]
pub fn get_comparison(
    state: State<'_, AppState>,
    query: PeriodQuery,
    mode: Option<ComparisonMode>,
    account_id: Option<i64>,
) -> Result<Comparison, AppError> {
    services::reports::comparison(
        &*state.conn()?,
        &query,
        mode,
        account_id,
        chrono::Local::now().date_naive(),
    )
    .inspect_err(|err| log::record(&state.log_path, err))
}
