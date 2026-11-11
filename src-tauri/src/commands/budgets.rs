use tauri::State;

use crate::error::AppError;
use crate::services::budgets::{Budget, BudgetFields, BudgetProgress};
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub fn list_budgets(state: State<'_, AppState>) -> Result<Vec<BudgetProgress>, AppError> {
    services::budgets::list(&*state.conn()?, chrono::Local::now().date_naive())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn create_budget(state: State<'_, AppState>, budget: BudgetFields) -> Result<Budget, AppError> {
    services::budgets::create(&mut *state.conn()?, budget, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn update_budget(
    state: State<'_, AppState>,
    id: i64,
    budget: BudgetFields,
) -> Result<Budget, AppError> {
    services::budgets::update(&mut *state.conn()?, id, budget, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn delete_budget(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::budgets::delete(&mut *state.conn()?, id)
        .inspect_err(|err| log::record(&state.log_path, err))
}
