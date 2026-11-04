use tauri::State;

use crate::error::AppError;
use crate::services::transactions::{
    PayBillDefaults, Transaction, TransactionDefaults, TransactionFields, TransactionFilter,
    TransactionList,
};
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub fn list_transactions(
    state: State<'_, AppState>,
    filter: TransactionFilter,
    page: u32,
) -> Result<TransactionList, AppError> {
    services::transactions::list(
        &*state.conn()?,
        filter,
        page,
        chrono::Local::now().date_naive(),
    )
    .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn get_transaction(state: State<'_, AppState>, id: i64) -> Result<Transaction, AppError> {
    services::transactions::get(&*state.conn()?, id)
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn create_transaction(
    state: State<'_, AppState>,
    transaction: TransactionFields,
) -> Result<Transaction, AppError> {
    services::transactions::create(&mut *state.conn()?, transaction, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn update_transaction(
    state: State<'_, AppState>,
    id: i64,
    transaction: TransactionFields,
) -> Result<Transaction, AppError> {
    services::transactions::update(&mut *state.conn()?, id, transaction, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn delete_transaction(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::transactions::delete(&mut *state.conn()?, id)
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn get_transaction_defaults(
    state: State<'_, AppState>,
) -> Result<TransactionDefaults, AppError> {
    services::transactions::defaults(&*state.conn()?, chrono::Local::now().date_naive())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn get_pay_bill_defaults(
    state: State<'_, AppState>,
    card_id: i64,
) -> Result<PayBillDefaults, AppError> {
    services::transactions::pay_bill_defaults(
        &*state.conn()?,
        card_id,
        chrono::Local::now().date_naive(),
    )
    .inspect_err(|err| log::record(&state.log_path, err))
}
