use tauri::State;

use crate::error::AppError;
use crate::services::forecast::{Forecast, ForecastGrain};
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub fn get_forecast(
    state: State<'_, AppState>,
    grain: ForecastGrain,
) -> Result<Forecast, AppError> {
    services::forecast::get(&*state.conn()?, grain, chrono::Local::now().date_naive())
        .inspect_err(|err| log::record(&state.log_path, err))
}
