use tauri::State;

use crate::error::AppError;
use crate::services::payslips::{Payslip, PayslipDefaults, PayslipFields, PayslipNet};
use crate::services::transactions::Transaction;
use crate::state::AppState;
use crate::{log, services};

/// Where the payslip form starts (PAY-2, UI_FLOWS F14).
#[tauri::command(async)]
pub fn get_payslip_defaults(
    state: State<'_, AppState>,
    rule_id: i64,
    date: String,
) -> Result<PayslipDefaults, AppError> {
    let today = chrono::Local::now().date_naive();
    services::payslips::defaults(&*state.conn()?, rule_id, &date, today)
        .inspect_err(|err| log::record(&state.log_path, err))
}

/// The net pay the form shows as the user types (CLAUDE.md rule 2).
#[tauri::command(async)]
pub fn preview_payslip(payslip: PayslipFields) -> PayslipNet {
    services::payslips::preview(&payslip)
}

/// Settles the payday on `date`: one that's due, or one 1–3 days ahead (PAY-4).
#[tauri::command(async)]
pub fn record_payslip(
    state: State<'_, AppState>,
    rule_id: i64,
    date: String,
    payslip: PayslipFields,
) -> Result<Transaction, AppError> {
    services::payslips::record(
        &mut *state.conn()?,
        rule_id,
        &date,
        payslip,
        chrono::Local::now(),
    )
    .inspect_err(|err| log::record(&state.log_path, err))
}

/// The whole payslip, from any of its rows' ids.
#[tauri::command(async)]
pub fn get_payslip(state: State<'_, AppState>, id: i64) -> Result<Payslip, AppError> {
    services::payslips::get(&*state.conn()?, id)
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn update_payslip(
    state: State<'_, AppState>,
    id: i64,
    payslip: PayslipFields,
) -> Result<Payslip, AppError> {
    services::payslips::update(&mut *state.conn()?, id, payslip, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}
