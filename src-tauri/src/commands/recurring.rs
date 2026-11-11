use tauri::State;

use crate::error::AppError;
use crate::services::recurring::{DueOccurrence, RecurringRule, RecurringRuleFields};
use crate::services::transactions::{Transaction, TransactionFields};
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

/// The Due list (UI_FLOWS F10), oldest first.
#[tauri::command(async)]
pub fn list_due_occurrences(state: State<'_, AppState>) -> Result<Vec<DueOccurrence>, AppError> {
    services::recurring::list_due(&*state.conn()?)
        .inspect_err(|err| log::record(&state.log_path, err))
}

/// Post (no `transaction`) or Edit & post (the edited fields).
#[tauri::command(async)]
pub fn post_occurrence(
    state: State<'_, AppState>,
    id: i64,
    transaction: Option<TransactionFields>,
) -> Result<Transaction, AppError> {
    services::recurring::post(&mut *state.conn()?, id, transaction, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn skip_occurrence(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::recurring::skip(&mut *state.conn()?, id, chrono::Local::now())
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
