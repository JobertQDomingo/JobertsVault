use tauri::State;

use crate::error::AppError;
use crate::services::recurring::{RecurringRule, RecurringRuleFields};
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub fn list_recurring_rules(state: State<'_, AppState>) -> Result<Vec<RecurringRule>, AppError> {
    services::recurring::list(&*state.conn()?).inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn create_recurring_rule(
    state: State<'_, AppState>,
    rule: RecurringRuleFields,
) -> Result<RecurringRule, AppError> {
    services::recurring::create(&mut *state.conn()?, rule, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn update_recurring_rule(
    state: State<'_, AppState>,
    id: i64,
    rule: RecurringRuleFields,
) -> Result<RecurringRule, AppError> {
    services::recurring::update(&mut *state.conn()?, id, rule, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn pause_recurring_rule(
    state: State<'_, AppState>,
    id: i64,
    paused: bool,
) -> Result<RecurringRule, AppError> {
    services::recurring::pause(&mut *state.conn()?, id, paused, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

/// The frontend calls it when the window regains focus, for a date that changed meanwhile (REC-4).
#[tauri::command(async)]
pub fn process_recurring(state: State<'_, AppState>) -> Result<u32, AppError> {
    services::recurring::process_due(&mut *state.conn()?, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn delete_recurring_rule(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::recurring::delete(&mut *state.conn()?, id)
        .inspect_err(|err| log::record(&state.log_path, err))
}
