use tauri::State;

use crate::domain::loan::LoanPreview;
use crate::error::AppError;
use crate::services::goals::Saved;
use crate::services::loans::{
    Loan, LoanDraft, LoanFields, LoanList, LoanRowAmount, LoanRowsTotal, NewLoan,
};
use crate::services::transactions::{Transaction, TransactionFields};
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub fn list_loans(state: State<'_, AppState>) -> Result<LoanList, AppError> {
    services::loans::list(&*state.conn()?, chrono::Local::now().date_naive())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn create_loan(state: State<'_, AppState>, loan: NewLoan) -> Result<Saved<Loan>, AppError> {
    services::loans::create(&mut *state.conn()?, loan, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn update_loan(
    state: State<'_, AppState>,
    id: i64,
    loan: LoanFields,
) -> Result<Saved<Loan>, AppError> {
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

#[tauri::command(async)]
pub fn mark_loan_payment_paid(
    state: State<'_, AppState>,
    payment_id: i64,
    transaction: TransactionFields,
) -> Result<Saved<Transaction>, AppError> {
    services::loans::mark_paid(
        &mut *state.conn()?,
        payment_id,
        transaction,
        chrono::Local::now(),
    )
    .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn mark_loan_payment_unpaid(
    state: State<'_, AppState>,
    payment_id: i64,
) -> Result<(), AppError> {
    services::loans::mark_unpaid(&mut *state.conn()?, payment_id, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

/// The loan form's running total as the user types (CLAUDE.md rule 2).
#[tauri::command(async)]
pub fn total_loan_payments(rows: Vec<LoanRowAmount>) -> LoanRowsTotal {
    services::loans::total(&rows)
}

/// Before you borrow (LOAN-9) and the new-loan confirm (LOAN-10): saves nothing.
#[tauri::command(async)]
pub fn preview_loan(state: State<'_, AppState>, draft: LoanDraft) -> Result<LoanPreview, AppError> {
    services::loans::preview(&*state.conn()?, draft, chrono::Local::now().date_naive())
        .inspect_err(|err| log::record(&state.log_path, err))
}
