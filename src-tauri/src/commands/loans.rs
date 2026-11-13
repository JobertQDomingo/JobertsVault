use tauri::State;

use crate::error::AppError;
use crate::services::loans::{Loan, LoanFields, LoanList, NewLoan};
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub fn list_loans(state: State<'_, AppState>) -> Result<LoanList, AppError> {
    services::loans::list(&*state.conn()?, chrono::Local::now().date_naive())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn create_loan(state: State<'_, AppState>, loan: NewLoan) -> Result<Loan, AppError> {
    services::loans::create(&mut *state.conn()?, loan, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn update_loan(
    state: State<'_, AppState>,
    id: i64,
    loan: LoanFields,
) -> Result<Loan, AppError> {
    services::loans::update(&mut *state.conn()?, id, loan, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn archive_loan(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::loans::archive(&mut *state.conn()?, id, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn unarchive_loan(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::loans::unarchive(&mut *state.conn()?, id, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn delete_loan(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::loans::delete(&mut *state.conn()?, id)
        .inspect_err(|err| log::record(&state.log_path, err))
}
