use tauri::State;

use crate::error::AppError;
use crate::services::payment_methods::PaymentMethod;
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub fn list_payment_methods(state: State<'_, AppState>) -> Result<Vec<PaymentMethod>, AppError> {
    services::payment_methods::list(&*state.conn()?)
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn create_payment_method(
    state: State<'_, AppState>,
    name: String,
) -> Result<PaymentMethod, AppError> {
    services::payment_methods::create(&mut *state.conn()?, name, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn update_payment_method(
    state: State<'_, AppState>,
    id: i64,
    name: String,
) -> Result<PaymentMethod, AppError> {
    services::payment_methods::update(&mut *state.conn()?, id, name, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn reorder_payment_methods(state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), AppError> {
    services::payment_methods::reorder(&mut *state.conn()?, ids, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn archive_payment_method(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::payment_methods::archive(&mut *state.conn()?, id, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn unarchive_payment_method(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::payment_methods::unarchive(&mut *state.conn()?, id, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn delete_payment_method(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::payment_methods::delete(&mut *state.conn()?, id)
        .inspect_err(|err| log::record(&state.log_path, err))
}
