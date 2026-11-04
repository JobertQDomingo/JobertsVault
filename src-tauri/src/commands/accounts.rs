use tauri::State;

use crate::error::AppError;
use crate::services::accounts::{Account, AccountFields, NewAccount};
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub fn list_accounts(state: State<'_, AppState>) -> Result<Vec<Account>, AppError> {
    services::accounts::list(&*state.conn()?).inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn get_account(state: State<'_, AppState>, id: i64) -> Result<Account, AppError> {
    services::accounts::get(&*state.conn()?, id)
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn create_account(
    state: State<'_, AppState>,
    account: NewAccount,
) -> Result<Account, AppError> {
    services::accounts::create(&mut *state.conn()?, account, chrono::Utc::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn update_account(
    state: State<'_, AppState>,
    id: i64,
    account: AccountFields,
) -> Result<Account, AppError> {
    services::accounts::update(&mut *state.conn()?, id, account, chrono::Utc::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn reorder_accounts(state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), AppError> {
    services::accounts::reorder(&mut *state.conn()?, ids, chrono::Utc::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}
