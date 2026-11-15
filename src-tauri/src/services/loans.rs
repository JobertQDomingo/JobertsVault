use chrono::{DateTime, Local, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use std::collections::{HashMap, HashSet};

use crate::domain::loan::{self, LoanSummary};
use crate::domain::money::Money;
use crate::error::AppError;
use crate::repo;
use crate::services::period;
use crate::services::transactions::{
    self, Transaction, TransactionFields, TransactionKind, real_date,
};
use crate::services::{invalid, not_found, stamp};

/// What the add loan form sends (LOAN-1). Whether it's new can't change afterwards, so the edit
/// form sends only the fields.
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NewLoan {
    /// A new loan, or one the user already has.
    pub is_new: bool,
    #[serde(flatten)]
    pub fields: LoanFields,
}

/// What the add and edit loan forms send (LOAN-1, LOAN-2, DATA_MODEL §4.5–4.6).
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoanFields {
    /// 1–50 characters, unique ignoring case.
    pub name: String,
    /// A cash, bank or e-wallet account, never a credit card.
    #[ts(type = "number")]
    pub account_id: i64,
    /// An expense category: Loan to start with.
    #[ts(type = "number")]
    pub category_id: i64,
    pub amount_borrowed: Option<Money>,
    /// `YYYY-MM-DD`, not after today. Required for a new loan.
    pub borrowed_on: Option<String>,
    /// On an edit, these replace every payment without a transaction, and a paid one is kept as
    /// it is. An error on a row names it by its place in this list, from 1.
    pub payments: Vec<LoanPaymentFields>,
}

/// One typed payment row.
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoanPaymentFields {
    /// The stored payment this row edits, or none for a new row.
    #[ts(type = "number | null")]
    pub id: Option<i64>,
    /// `YYYY-MM-DD`.
    pub due_date: String,
    pub amount: Money,
}

/// Read from the row, never stored (DATA_MODEL §4.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LoanPaymentStatus {
    Unpaid,
    /// Marked paid in the app: it has a transaction.
    Paid,
    /// On a loan the user already has, dated before the day it was added. No transaction.
    PaidBefore,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoanPayment {
    #[ts(type = "number")]
    pub id: i64,
    pub due_date: NaiveDate,
    /// The scheduled amount.
    pub amount: Money,
    pub status: LoanPaymentStatus,
    /// Unpaid and dated before today.
    pub overdue: bool,
    #[ts(type = "number | null")]
    pub transaction_id: Option<i64>,
    /// Its transaction's amount when paid (a late fee included), its scheduled amount when paid
    /// before, and none when unpaid.
    pub paid_amount: Option<Money>,
    /// Its transaction's date when paid ("Paid Nov 20"), and none otherwise.
    pub paid_on: Option<NaiveDate>,
}

/// One stored loan with its payments, by due date, then id. The screens look the account and
/// category up in their lists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Loan {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    #[ts(type = "number")]
    pub account_id: i64,
    #[ts(type = "number")]
    pub category_id: i64,
    pub is_new: bool,
    pub amount_borrowed: Option<Money>,
    pub borrowed_on: Option<NaiveDate>,
    /// The day it was added: on a loan the user already has, payments before it are paid before.
    pub added_on: NaiveDate,
    /// Only a paid-off loan is archived.
    pub archived: bool,
    pub payments: Vec<LoanPayment>,
    // Its figures (LOAN-5, LOAN-6, DATA_MODEL §5.11).
    /// Σ of its unpaid payments.
    pub left: Money,
    /// Σ of its paid amounts.
    pub paid: Money,
    /// Whole percent rounded down, so 100 means it's really paid off.
    #[ts(type = "number")]
    pub percent_paid: i64,
    /// Its earliest unpaid payment.
    pub next: Option<LoanPayment>,
    /// Every payment less the amount borrowed. None without one, or when that's below 0.
    pub interest: Option<Money>,
    pub paid_off: bool,
}

/// What `list_loans` returns (ARCHITECTURE §4.5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoanList {
    /// In the order they were added, archived ones included.
    pub loans: Vec<Loan>,
    /// Across every loan, archived ones included (LOAN-5 to LOAN-8).
    pub summary: LoanSummary,
}

/// Every loan, archived ones included, with its payments and figures as of `today`, and the
/// summary in the app's months (DATA_MODEL §5.3, §5.11).
pub fn list(conn: &Connection, today: NaiveDate) -> Result<LoanList, AppError> {
    let mut payments: HashMap<i64, Vec<repo::loans::PaymentRow>> = HashMap::new();
    for p in repo::loans::payments(conn)? {
        payments.entry(p.loan_id).or_default().push(p);
    }
    let loans: Vec<Loan> = repo::loans::all(conn)?
        .into_iter()
        .map(|row| {
            let mine = payments.remove(&row.id).unwrap_or_default();
            view(row, mine, today)
        })
        .collect::<Result<_, _>>()?;
    let inputs: Vec<Vec<loan::Payment>> = loans.iter().map(|l| inputs(&l.payments)).collect();
    let all: Vec<loan::LoanInput> = loans
        .iter()
        .zip(&inputs)
        .map(|(l, payments)| loan::LoanInput {
            payments,
            amount_borrowed: l.amount_borrowed,
            borrowed_on: l.borrowed_on,
        })
        .collect();
    let summary = loan::summary(&all, today, &period::period_settings(conn)?)
        .ok_or_else(period::out_of_range)?;
    Ok(LoanList { loans, summary })
}

fn inputs(payments: &[LoanPayment]) -> Vec<loan::Payment> {
    payments
        .iter()
        .map(|p| loan::Payment {
            due_date: p.due_date,
            amount: p.amount,
            // By status, so a paid row never counts as left.
            paid: (p.status != LoanPaymentStatus::Unpaid)
                .then(|| p.paid_amount.unwrap_or(p.amount)),
        })
        .collect()
}

/// Adds the loan and its payments as one change. It's added today, so on a loan the user
/// already has, the rows dated before today are paid before (DATA_MODEL §4.6).
pub fn create(
    conn: &mut Connection,
    input: NewLoan,
    now: DateTime<Local>,
) -> Result<Loan, AppError> {
    let NewLoan { is_new, mut fields } = input;
    let today = now.date_naive();
    let tx = conn.transaction()?;
    let borrowed_on = validate(&tx, &mut fields, is_new, None, today)?;
    check_payments(&fields.payments, &[], is_new, borrowed_on)?;
    let added_on = today.to_string();
    let stamp = stamp(now);
    let id = repo::loans::insert(&tx, &values(&fields), is_new, &added_on, &stamp)?;
    save_payments(&tx, id, &fields.payments, is_new, &added_on, &stamp)?;
    let loan = get(&tx, id, today)?;
    tx.commit()?;
    Ok(loan)
}

/// Every field can change, and the rows given replace the payments without a transaction. A
/// paid one is never changed (LOAN-2, DATA_MODEL §4.6). Leaving an unpaid payment unarchives
/// the loan, so an archived loan never hides money still owed (§4.5).
pub fn update(
    conn: &mut Connection,
    id: i64,
    mut fields: LoanFields,
    now: DateTime<Local>,
) -> Result<Loan, AppError> {
    let today = now.date_naive();
    let tx = conn.transaction()?;
    let stored = repo::loans::find(&tx, id)?.ok_or_else(|| not_found("loan", id))?;
    let borrowed_on = validate(&tx, &mut fields, stored.is_new, Some(&stored), today)?;
    let payments = repo::loans::payments_of(&tx, id)?;
    check_payments(&fields.payments, &payments, stored.is_new, borrowed_on)?;
    let stamp = stamp(now);
    repo::loans::update(&tx, id, &values(&fields), &stamp)?;
    for p in payments.iter().filter(|p| p.transaction_id.is_none()) {
        if !fields.payments.iter().any(|r| r.id == Some(p.id)) {
            repo::loans::delete_payment(&tx, p.id)?;
        }
    }
    save_payments(
        &tx,
        id,
        &fields.payments,
        stored.is_new,
        &stored.added_on,
        &stamp,
    )?;
    repo::loans::unarchive_if_unpaid(&tx, id, &stamp)?;
    let loan = get(&tx, id, today)?;
    tx.commit()?;
    Ok(loan)
}

/// Only a paid-off loan, so an archived loan never hides money still owed (LOAN-2).
pub fn archive(conn: &mut Connection, id: i64, now: DateTime<Local>) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let loan = repo::loans::find(&tx, id)?.ok_or_else(|| not_found("loan", id))?;
    if loan.archived {
        return Ok(());
    }
    let unpaid = repo::loans::payments_of(&tx, id)?
        .iter()
        .any(|p| p.transaction_id.is_none() && !p.paid_before);
    if unpaid {
        return Err(AppError::Conflict {
            message: "Pay or remove its unpaid payments first".into(),
        });
    }
    repo::loans::set_archived(&tx, id, true, &stamp(now))?;
    tx.commit()?;
    Ok(())
}

pub fn unarchive(conn: &mut Connection, id: i64, now: DateTime<Local>) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let loan = repo::loans::find(&tx, id)?.ok_or_else(|| not_found("loan", id))?;
    if loan.archived {
        repo::loans::set_archived(&tx, id, false, &stamp(now))?;
    }
    tx.commit()?;
    Ok(())
}

/// Only a loan with no payment marked paid: its payments go with it. Any other is archived once
/// it's paid off (LOAN-2).
pub fn delete(conn: &mut Connection, id: i64) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    repo::loans::find(&tx, id)?.ok_or_else(|| not_found("loan", id))?;
    let paid = repo::loans::payments_of(&tx, id)?
        .iter()
        .any(|p| p.transaction_id.is_some());
    if paid {
        return Err(AppError::Conflict {
            message: "A payment of this loan is marked paid, so it can only be archived once it's paid off".into(),
        });
    }
    repo::loans::delete(&tx, id)?;
    tx.commit()?;
    Ok(())
}

/// Mark paid (LOAN-3): records the expense the user confirmed and links it, in one change. Only
/// an unpaid payment. The expense is checked like a new one, but may keep the loan's account and
/// category though they've been archived since. It leaves `last_used_account_id` alone
/// (DATA_MODEL §4.6).
pub fn mark_paid(
    conn: &mut Connection,
    payment_id: i64,
    mut fields: TransactionFields,
    now: DateTime<Local>,
) -> Result<Transaction, AppError> {
    let tx = conn.transaction()?;
    let payment = repo::loans::find_payment(&tx, payment_id)?
        .ok_or_else(|| not_found("loan payment", payment_id))?;
    if payment.transaction_id.is_some() || payment.paid_before {
        return Err(AppError::Conflict {
            message: "This payment is already paid".into(),
        });
    }
    if fields.kind != TransactionKind::Expense {
        return Err(invalid("kind", "A loan payment is an expense"));
    }
    let loan = repo::loans::find(&tx, payment.loan_id)?
        .ok_or_else(|| not_found("loan", payment.loan_id))?;
    // The loan's picks, in the shape the transaction rules keep archived records from.
    let template = Transaction {
        id: 0,
        kind: TransactionKind::Expense,
        amount: Money(payment.amount),
        // The rules don't read the template's date.
        date: now.date_naive(),
        account_id: loan.account_id,
        to_account_id: None,
        category_id: Some(loan.category_id),
        payee: None,
        payment_method_id: None,
        note: None,
        loan_payment_id: None,
        payslip_id: None,
    };
    transactions::validate(&tx, &mut fields, Some(&template), now.date_naive())?;
    let stamp = stamp(now);
    let txn = repo::transactions::insert(&tx, &transactions::values(&fields), &stamp)?;
    repo::loans::set_payment_transaction(&tx, payment_id, txn, &stamp)?;
    let saved = transactions::get(&tx, txn)?;
    tx.commit()?;
    Ok(saved)
}

/// Mark unpaid: deletes the payment's expense, which makes it unpaid and unarchives its loan
/// (DATA_MODEL §4.5–4.6). Only a payment marked paid.
pub fn mark_unpaid(
    conn: &mut Connection,
    payment_id: i64,
    now: DateTime<Local>,
) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let payment = repo::loans::find_payment(&tx, payment_id)?
        .ok_or_else(|| not_found("loan payment", payment_id))?;
    let txn = payment.transaction_id.ok_or_else(|| AppError::Conflict {
        message: "This payment isn't marked paid".into(),
    })?;
    transactions::delete_in(&tx, txn, &stamp(now))?;
    tx.commit()?;
    Ok(())
}

/// Trims the name and checks every field but the payments (DATA_MODEL §6), and returns the date
/// borrowed. An edit may keep the archived account or category it already has.
fn validate(
    conn: &Connection,
    f: &mut LoanFields,
    is_new: bool,
    stored: Option<&repo::loans::Row>,
    today: NaiveDate,
) -> Result<Option<NaiveDate>, AppError> {
    f.name = f.name.trim().to_owned();
    if !(1..=50).contains(&f.name.chars().count()) {
        return Err(invalid("name", "Enter a name of 1 to 50 characters"));
    }
    if repo::loans::name_taken(conn, &f.name, stored.map(|s| s.id))? {
        return Err(invalid("name", "A loan with this name already exists"));
    }
    let account = repo::accounts::find(conn, f.account_id, today)?
        .ok_or_else(|| invalid("accountId", "This account no longer exists"))?;
    if account.kind == "credit_card" {
        return Err(invalid(
            "accountId",
            "Choose a cash, bank or e-wallet account",
        ));
    }
    if account.archived && stored.map(|s| s.account_id) != Some(f.account_id) {
        return Err(invalid("accountId", "This account is archived"));
    }
    let category = repo::categories::find(conn, f.category_id)?
        .ok_or_else(|| invalid("categoryId", "This category no longer exists"))?;
    if category.kind != "expense" {
        return Err(invalid("categoryId", "Choose an expense category"));
    }
    if category.archived && stored.map(|s| s.category_id) != Some(f.category_id) {
        return Err(invalid("categoryId", "This category is archived"));
    }
    if let Some(amount) = f.amount_borrowed {
        if amount <= Money::ZERO {
            return Err(invalid("amountBorrowed", "Enter an amount above 0"));
        }
        if amount > Money::MAX {
            return Err(invalid("amountBorrowed", "That amount is too large"));
        }
    }
    let borrowed_on = match f.borrowed_on.as_deref() {
        None if is_new => return Err(invalid("borrowedOn", "Enter the date borrowed")),
        None => None,
        Some(text) => {
            Some(real_date(text).ok_or_else(|| invalid("borrowedOn", "Enter a real date"))?)
        }
    };
    if borrowed_on.is_some_and(|date| date > today) {
        return Err(invalid(
            "borrowedOn",
            "The date borrowed can't be after today",
        ));
    }
    Ok(borrowed_on)
}

/// The rows given, against the loan's `stored` payments (none for a new loan). A row with an id
/// edits one of its payments without a transaction; a paid one can't be changed (`Conflict`).
fn check_payments(
    rows: &[LoanPaymentFields],
    stored: &[repo::loans::PaymentRow],
    is_new: bool,
    borrowed_on: Option<NaiveDate>,
) -> Result<(), AppError> {
    let paid: Vec<&repo::loans::PaymentRow> = stored
        .iter()
        .filter(|p| p.transaction_id.is_some())
        .collect();
    if !(1..=600).contains(&(paid.len() + rows.len())) {
        return Err(invalid("payments", "Enter 1 to 600 payments"));
    }
    // A new loan's payments aren't dated before its date borrowed (§6).
    let too_early = |date: NaiveDate| is_new && borrowed_on.is_some_and(|b| date < b);
    let mut seen = HashSet::new();
    for (n, r) in (1..).zip(rows) {
        if let Some(id) = r.id {
            if paid.iter().any(|p| p.id == id) {
                return Err(AppError::Conflict {
                    message: format!("Payment {n} is marked paid. Mark it unpaid to change it."),
                });
            }
            if !stored.iter().any(|p| p.id == id) || !seen.insert(id) {
                return Err(invalid(
                    "payments",
                    &format!("Payment {n} isn't one of this loan's payments"),
                ));
            }
        }
        let date = real_date(&r.due_date)
            .ok_or_else(|| invalid("payments", &format!("Payment {n}: enter a real date")))?;
        if r.amount <= Money::ZERO {
            return Err(invalid(
                "payments",
                &format!("Payment {n}: enter an amount above 0"),
            ));
        }
        if r.amount > Money::MAX {
            return Err(invalid(
                "payments",
                &format!("Payment {n}: that amount is too large"),
            ));
        }
        if too_early(date) {
            return Err(invalid(
                "payments",
                &format!("Payment {n} is dated before the date borrowed"),
            ));
        }
    }
    // A paid row isn't in the list, so it's named by its date.
    for p in paid {
        if real_date(&p.due_date).is_some_and(too_early) {
            return Err(invalid(
                "payments",
                &format!(
                    "The payment marked paid on {} is before the date borrowed. Change the date borrowed.",
                    p.due_date
                ),
            ));
        }
    }
    Ok(())
}

/// Updates the rows with an id and adds the others, in the order given, so rows on one date
/// keep it. On a loan the user already has, a row dated before `added_on` is paid before.
fn save_payments(
    conn: &Connection,
    loan_id: i64,
    rows: &[LoanPaymentFields],
    is_new: bool,
    added_on: &str,
    now: &str,
) -> Result<(), AppError> {
    for r in rows {
        // Both are checked YYYY-MM-DD dates, which sort as text.
        let paid_before = !is_new && r.due_date.as_str() < added_on;
        match r.id {
            Some(id) => {
                repo::loans::update_payment(conn, id, &r.due_date, r.amount.0, paid_before, now)?
            }
            None => repo::loans::insert_payment(
                conn,
                loan_id,
                &r.due_date,
                r.amount.0,
                paid_before,
                now,
            )?,
        }
    }
    Ok(())
}

fn values(f: &LoanFields) -> repo::loans::Values<'_> {
    repo::loans::Values {
        name: &f.name,
        account_id: f.account_id,
        category_id: f.category_id,
        amount_borrowed: f.amount_borrowed.map(|m| m.0),
        borrowed_on: f.borrowed_on.as_deref(),
    }
}

fn get(conn: &Connection, id: i64, today: NaiveDate) -> Result<Loan, AppError> {
    let row = repo::loans::find(conn, id)?.ok_or_else(|| not_found("loan", id))?;
    let payments = repo::loans::payments_of(conn, id)?;
    view(row, payments, today)
}

/// A stored date the column's CHECK let through but that isn't real is a Database error.
fn stored_date(text: &str, what: &str, loan_id: i64) -> Result<NaiveDate, AppError> {
    real_date(text).ok_or_else(|| AppError::Database {
        message: format!("Loan {loan_id} has an invalid {what}"),
    })
}

fn view(
    row: repo::loans::Row,
    payments: Vec<repo::loans::PaymentRow>,
    today: NaiveDate,
) -> Result<Loan, AppError> {
    let payments = payments
        .into_iter()
        .map(|p| {
            let due_date = stored_date(&p.due_date, "payment date", row.id)?;
            let status = match (p.transaction_id, p.paid_before) {
                (Some(_), _) => LoanPaymentStatus::Paid,
                (None, true) => LoanPaymentStatus::PaidBefore,
                (None, false) => LoanPaymentStatus::Unpaid,
            };
            let paid_amount = match status {
                LoanPaymentStatus::Paid => p.paid_amount.map(Money),
                LoanPaymentStatus::PaidBefore => Some(Money(p.amount)),
                LoanPaymentStatus::Unpaid => None,
            };
            let paid_on = match (status, &p.paid_on) {
                (LoanPaymentStatus::Paid, Some(date)) => {
                    Some(stored_date(date, "paid date", row.id)?)
                }
                _ => None,
            };
            Ok(LoanPayment {
                id: p.id,
                due_date,
                amount: Money(p.amount),
                status,
                overdue: status == LoanPaymentStatus::Unpaid && due_date < today,
                transaction_id: p.transaction_id,
                paid_amount,
                paid_on,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let amount_borrowed = row.amount_borrowed.map(Money);
    let f = loan::figures(&inputs(&payments), amount_borrowed);
    Ok(Loan {
        left: f.left,
        paid: f.paid,
        percent_paid: f.percent_paid,
        next: f.next.and_then(|i| payments.get(i)).cloned(),
        interest: f.interest,
        paid_off: f.paid_off,
        borrowed_on: row
            .borrowed_on
            .as_deref()
            .map(|text| stored_date(text, "date borrowed", row.id))
            .transpose()?,
        added_on: stored_date(&row.added_on, "date added", row.id)?,
        id: row.id,
        name: row.name,
        account_id: row.account_id,
        category_id: row.category_id,
        is_new: row.is_new,
        amount_borrowed,
        archived: row.archived,
        payments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::accounts::{self, AccountFields, AccountKind, NewAccount};
    use crate::services::budgets::{self, BudgetFields, BudgetPeriod};
    use crate::services::categories::{self, CategoryFields, CategoryKind, NewCategory};
    use crate::services::period::PeriodQuery;
    use crate::services::transactions::TransactionFilter;
    use crate::services::{dashboard, goals, payment_methods, reports, settings};
    use crate::test_support::{db, now};
    use LoanPaymentStatus::{Paid, PaidBefore, Unpaid};
    use chrono::TimeZone;
    use std::fmt::Debug;

    // The seed (DATA_MODEL §7): Cash is account 1, Salary income category 1, Food expense 8, and
    // Loan 25, the first category 0003 adds. `now()` is 2026-11-03.
    const CASH: i64 = 1;
    const SALARY: i64 = 1;
    const FOOD: i64 = 8;
    const LOAN: i64 = 25;

    fn field_of<T: Debug>(result: Result<T, AppError>) -> String {
        match result {
            Err(AppError::Validation { field, .. }) => field,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    fn message_of<T: Debug>(result: Result<T, AppError>) -> String {
        match result {
            Err(AppError::Validation { message, .. }) => message,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    fn conflict<T: Debug>(result: Result<T, AppError>) -> String {
        match result {
            Err(AppError::Conflict { message }) => message,
            other => panic!("expected a conflict, got {other:?}"),
        }
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("a real test date")
    }

    fn on(day: NaiveDate) -> DateTime<Local> {
        Local
            .from_local_datetime(&day.and_hms_opt(9, 0, 0).unwrap())
            .unwrap()
    }

    fn today() -> NaiveDate {
        d(2026, 11, 3)
    }

    fn row(date: &str, amount: i64) -> LoanPaymentFields {
        LoanPaymentFields {
            id: None,
            due_date: date.into(),
            amount: Money(amount),
        }
    }

    fn kept(id: i64, date: &str, amount: i64) -> LoanPaymentFields {
        LoanPaymentFields {
            id: Some(id),
            ..row(date, amount)
        }
    }

    /// Paid from Cash into Loan, with no amount or date borrowed.
    fn fields(name: &str, payments: Vec<LoanPaymentFields>) -> LoanFields {
        LoanFields {
            name: name.into(),
            account_id: CASH,
            category_id: LOAN,
            amount_borrowed: None,
            borrowed_on: None,
            payments,
        }
    }

    /// Borrowed on 2026-11-01.
    fn new_fields(name: &str, payments: Vec<LoanPaymentFields>) -> LoanFields {
        LoanFields {
            amount_borrowed: Some(Money(500_000)),
            borrowed_on: Some("2026-11-01".into()),
            ..fields(name, payments)
        }
    }

    fn existing(name: &str, payments: Vec<LoanPaymentFields>) -> NewLoan {
        NewLoan {
            is_new: false,
            fields: fields(name, payments),
        }
    }

    fn new_loan(name: &str, payments: Vec<LoanPaymentFields>) -> NewLoan {
        NewLoan {
            is_new: true,
            fields: new_fields(name, payments),
        }
    }

    fn account(conn: &mut Connection, name: &str, kind: AccountKind) -> i64 {
        let card = kind == AccountKind::CreditCard;
        let new = NewAccount {
            kind,
            fields: AccountFields {
                name: name.into(),
                opening_balance: Money(0),
                credit_limit: card.then_some(Money(1_000_000)),
                statement_day: card.then_some(5),
                due_day: card.then_some(25),
            },
        };
        accounts::create(conn, new, now()).unwrap().id
    }

    fn category(conn: &mut Connection, name: &str, kind: CategoryKind) -> i64 {
        let new = NewCategory {
            kind,
            fields: CategoryFields {
                name: name.into(),
                color: "cat-1".into(),
                icon: "coins".into(),
            },
        };
        categories::create(conn, new, now()).unwrap().id
    }

    /// The expense Mark paid starts with (DATA_MODEL §4.6): today, from Cash into Loan.
    fn expense(amount: i64) -> TransactionFields {
        TransactionFields {
            kind: TransactionKind::Expense,
            amount: Money(amount),
            date: "2026-11-03".into(),
            account_id: CASH,
            to_account_id: None,
            category_id: Some(LOAN),
            payee: Some("E-wallet loan".into()),
            payment_method_id: None,
            note: None,
        }
    }

    /// Marks the payment paid with `amount` and returns its expense's id.
    fn pay(conn: &mut Connection, payment_id: i64, amount: i64) -> i64 {
        mark_paid(conn, payment_id, expense(amount), now())
            .unwrap()
            .id
    }

    fn payment(conn: &Connection, loan_id: i64, payment_id: i64) -> LoanPayment {
        find(conn, loan_id)
            .payments
            .into_iter()
            .find(|p| p.id == payment_id)
            .unwrap()
    }

    fn transaction_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT count(*) FROM transactions", [], |r| r.get(0))
            .unwrap()
    }

    fn statuses(loan: &Loan) -> Vec<(NaiveDate, LoanPaymentStatus, bool)> {
        loan.payments
            .iter()
            .map(|p| (p.due_date, p.status, p.overdue))
            .collect()
    }

    fn find(conn: &Connection, id: i64) -> Loan {
        list(conn, today())
            .unwrap()
            .loans
            .into_iter()
            .find(|l| l.id == id)
            .unwrap()
    }

    #[test]
    fn a_new_loan_reads_what_the_form_sends() {
        let sent = r#"{"isNew":true,"name":"E-wallet loan","accountId":1,"categoryId":25,
            "amountBorrowed":null,"borrowedOn":"2026-11-01",
            "payments":[{"id":null,"dueDate":"2026-12-01","amount":100000}]}"#;
        let loan: NewLoan = serde_json::from_str(sent).unwrap();
        assert!(loan.is_new);
        assert_eq!(loan.fields.payments[0].amount, Money(100_000));
    }

    #[test]
    fn the_seed_has_the_loan_category() {
        let conn = db();
        let name: String = conn
            .query_row(
                "SELECT name FROM categories WHERE id = ?1 AND kind = 'expense'",
                [LOAN],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(name, "Loan");
    }

    #[test]
    fn a_new_loan_saves_every_row_unpaid_even_one_already_past() {
        let mut conn = db();
        let input = new_loan(
            "  E-wallet loan  ",
            vec![row("2026-12-01", 100_000), row("2026-11-02", 100_000)],
        );
        let loan = create(&mut conn, input, now()).unwrap();
        assert_eq!(loan.name, "E-wallet loan");
        assert!(loan.is_new);
        assert_eq!((loan.account_id, loan.category_id), (CASH, LOAN));
        assert_eq!(loan.amount_borrowed, Some(Money(500_000)));
        assert_eq!(loan.borrowed_on, Some(d(2026, 11, 1)));
        assert_eq!(loan.added_on, today());
        assert!(!loan.archived);
        // By due date, and the one before today is overdue, never paid before.
        assert_eq!(
            statuses(&loan),
            [
                (d(2026, 11, 2), Unpaid, true),
                (d(2026, 12, 1), Unpaid, false)
            ]
        );
        assert!(
            loan.payments
                .iter()
                .all(|p| p.paid_amount.is_none() && p.paid_on.is_none())
        );
        assert_eq!(list(&conn, today()).unwrap().loans, [loan]);
    }

    #[test]
    fn a_loan_i_already_have_saves_rows_before_the_day_its_added_as_paid_before() {
        let mut conn = db();
        let input = existing(
            "E-wallet loan",
            vec![
                row("2026-10-03", 100_000),
                row("2026-11-03", 100_000),
                row("2026-11-02", 99_999),
                row("2026-12-03", 100_000),
            ],
        );
        let loan = create(&mut conn, input, now()).unwrap();
        assert!(!loan.is_new);
        assert_eq!(loan.borrowed_on, None);
        // A row on the day it's added is unpaid.
        assert_eq!(
            statuses(&loan),
            [
                (d(2026, 10, 3), PaidBefore, false),
                (d(2026, 11, 2), PaidBefore, false),
                (d(2026, 11, 3), Unpaid, false),
                (d(2026, 12, 3), Unpaid, false),
            ]
        );
        // Paid before counts its scheduled amount and has no transaction.
        assert_eq!(loan.payments[1].paid_amount, Some(Money(99_999)));
        assert!(loan.payments.iter().all(|p| p.transaction_id.is_none()));
        // No transaction, so past cashflow doesn't change.
        let count: i64 = conn
            .query_row("SELECT count(*) FROM transactions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn paid_before_goes_by_the_day_the_loan_was_added_not_today() {
        let mut conn = db();
        let input = existing("E-wallet loan", vec![row("2026-11-01", 100_000)]);
        let loan = create(&mut conn, input, now()).unwrap();
        let first = loan.payments[0].id;

        // A week later: a row dated after the day it was added is unpaid, and overdue.
        let later = on(d(2026, 11, 10));
        let edit = fields(
            "E-wallet loan",
            vec![
                kept(first, "2026-11-01", 100_000),
                row("2026-11-05", 100_000),
            ],
        );
        let loan = update(&mut conn, loan.id, edit, later).unwrap();
        assert_eq!(loan.added_on, today());
        assert_eq!(
            statuses(&loan),
            [
                (d(2026, 11, 1), PaidBefore, false),
                (d(2026, 11, 5), Unpaid, true)
            ]
        );

        // Moving a row across that day sets its flag again, both ways.
        let second = loan.payments[1].id;
        let edit = fields(
            "E-wallet loan",
            vec![
                kept(first, "2026-11-04", 100_000),
                kept(second, "2026-11-02", 100_000),
            ],
        );
        let loan = update(&mut conn, loan.id, edit, later).unwrap();
        assert_eq!(
            statuses(&loan),
            [
                (d(2026, 11, 2), PaidBefore, false),
                (d(2026, 11, 4), Unpaid, true)
            ]
        );
    }

    #[test]
    fn every_field_is_checked() {
        let mut conn = db();
        let card = account(&mut conn, "Card", AccountKind::CreditCard);
        let gone_account = account(&mut conn, "Old wallet", AccountKind::Ewallet);
        accounts::archive(&mut conn, gone_account, false, now()).unwrap();
        let gone_category = category(&mut conn, "Old loans", CategoryKind::Expense);
        categories::archive(&mut conn, gone_category, now()).unwrap();
        let bank = create(
            &mut conn,
            existing("Bank loan", vec![row("2026-12-01", 1)]),
            now(),
        );
        let theirs = bank.unwrap().payments[0].id;

        type Change = Box<dyn Fn(&mut LoanFields)>;
        let cases: Vec<(&str, Change)> = vec![
            ("name", Box::new(|f| f.name = "   ".into())),
            ("name", Box::new(|f| f.name = "x".repeat(51))),
            ("name", Box::new(|f| f.name = " BANK LOAN ".into())),
            ("accountId", Box::new(|f| f.account_id = 999)),
            ("accountId", Box::new(move |f| f.account_id = card)),
            ("accountId", Box::new(move |f| f.account_id = gone_account)),
            ("categoryId", Box::new(|f| f.category_id = 999)),
            ("categoryId", Box::new(|f| f.category_id = SALARY)),
            (
                "categoryId",
                Box::new(move |f| f.category_id = gone_category),
            ),
            (
                "amountBorrowed",
                Box::new(|f| f.amount_borrowed = Some(Money(0))),
            ),
            (
                "amountBorrowed",
                Box::new(|f| f.amount_borrowed = Some(Money(Money::MAX.0 + 1))),
            ),
            ("borrowedOn", Box::new(|f| f.borrowed_on = None)),
            (
                "borrowedOn",
                Box::new(|f| f.borrowed_on = Some("2026-02-30".into())),
            ),
            (
                "borrowedOn",
                Box::new(|f| f.borrowed_on = Some("2026-11-04".into())),
            ),
            ("payments", Box::new(|f| f.payments = vec![])),
            (
                "payments",
                Box::new(|f| f.payments = vec![row("2026-12-01", 1); 601]),
            ),
            (
                "payments",
                Box::new(|f| f.payments = vec![row("2026-13-01", 1)]),
            ),
            (
                "payments",
                Box::new(|f| f.payments = vec![row("2026-12-01", 0)]),
            ),
            (
                "payments",
                Box::new(|f| f.payments = vec![row("2026-12-01", Money::MAX.0 + 1)]),
            ),
            // A new loan's payment can't come before the day it was borrowed.
            (
                "payments",
                Box::new(|f| f.payments = vec![row("2026-10-31", 1)]),
            ),
            // A row id only edits one of this loan's own payments.
            (
                "payments",
                Box::new(move |f| f.payments = vec![kept(theirs, "2026-12-01", 1)]),
            ),
        ];
        for (field, change) in &cases {
            let mut input = new_loan("E-wallet loan", vec![row("2026-12-01", 100_000)]);
            change(&mut input.fields);
            assert_eq!(field_of(create(&mut conn, input, now())), *field);
        }
        // A row's error says which one, by its place in the list.
        let input = new_loan(
            "E-wallet loan",
            vec![row("2026-12-01", 1), row("2026-12-41", 1)],
        );
        assert!(message_of(create(&mut conn, input, now())).starts_with("Payment 2"));

        // One the user already has: a date borrowed is optional, but follows the same rule.
        let mut input = existing("Shop loan", vec![row("2026-12-01", 1)]);
        input.fields.borrowed_on = Some("2026-11-04".into());
        assert_eq!(field_of(create(&mut conn, input, now())), "borrowedOn");

        // The limits themselves are fine, and an existing loan's rows may come before its date.
        let mut input = new_loan(&"x".repeat(50), vec![]);
        input.fields.amount_borrowed = Some(Money::MAX);
        input.fields.borrowed_on = Some("2026-11-03".into());
        input.fields.payments = vec![row("2026-11-03", Money::MAX.0); 600];
        assert!(create(&mut conn, input, now()).is_ok());
        let mut input = existing("Shop loan", vec![row("2026-01-31", 1)]);
        input.fields.borrowed_on = Some("2026-02-01".into());
        assert!(create(&mut conn, input, now()).is_ok());
    }

    #[test]
    fn an_edit_replaces_the_unpaid_rows_keeping_their_ids() {
        let mut conn = db();
        let input = new_loan(
            "E-wallet loan",
            vec![
                row("2026-12-01", 100_000),
                row("2027-01-01", 100_000),
                row("2027-02-01", 100_000),
            ],
        );
        let loan = create(&mut conn, input, now()).unwrap();
        let ids: Vec<i64> = loan.payments.iter().map(|p| p.id).collect();
        let wallet = account(&mut conn, "Wallet", AccountKind::Ewallet);
        let mut edit = new_fields(
            "Shop loan",
            vec![
                kept(ids[2], "2027-03-01", 90_000),
                row("2027-04-01", 80_000),
                kept(ids[0], "2026-12-01", 100_000),
            ],
        );
        edit.account_id = wallet;
        edit.amount_borrowed = None;
        edit.borrowed_on = Some("2026-10-15".into());
        let saved = update(&mut conn, loan.id, edit, now()).unwrap();
        assert_eq!(saved.name, "Shop loan");
        assert_eq!(saved.account_id, wallet);
        assert_eq!(saved.amount_borrowed, None);
        assert_eq!(saved.borrowed_on, Some(d(2026, 10, 15)));
        assert!(saved.is_new, "an edit keeps it a new loan");
        let rows: Vec<(i64, NaiveDate, Money)> = saved
            .payments
            .iter()
            .map(|p| (p.id, p.due_date, p.amount))
            .collect();
        assert_eq!(rows.len(), 3, "the row left out is removed");
        assert_eq!(rows[0], (ids[0], d(2026, 12, 1), Money(100_000)));
        assert_eq!(rows[1], (ids[2], d(2027, 3, 1), Money(90_000)));
        assert_eq!((rows[2].1, rows[2].2), (d(2027, 4, 1), Money(80_000)));
        assert!(!ids.contains(&rows[2].0));
    }

    #[test]
    fn an_edit_never_changes_a_paid_row() {
        let mut conn = db();
        let input = new_loan(
            "E-wallet loan",
            vec![row("2026-11-02", 100_000), row("2026-12-01", 100_000)],
        );
        let loan = create(&mut conn, input, now()).unwrap();
        let (paid, unpaid) = (loan.payments[0].id, loan.payments[1].id);
        // A late fee: the paid amount is the transaction's.
        let txn = pay(&mut conn, paid, 105_000);

        // Giving the paid row's id is refused.
        let edit = new_fields("E-wallet loan", vec![kept(paid, "2026-11-20", 1)]);
        conflict(update(&mut conn, loan.id, edit, now()));

        // Leaving it out keeps it as it is, and every unpaid row may go.
        let edit = new_fields("E-wallet loan", vec![]);
        let saved = update(&mut conn, loan.id, edit, now()).unwrap();
        assert_eq!(saved.payments.len(), 1);
        let p = &saved.payments[0];
        assert_eq!(
            (p.id, p.due_date, p.amount),
            (paid, d(2026, 11, 2), Money(100_000))
        );
        assert_eq!((p.status, p.overdue), (Paid, false));
        assert_eq!(p.transaction_id, Some(txn));
        assert_eq!(p.paid_amount, Some(Money(105_000)));
        assert!(!saved.payments.iter().any(|p| p.id == unpaid));

        // A paid row still can't come before a new loan's date borrowed, and it's named by date.
        let mut edit = new_fields("E-wallet loan", vec![]);
        edit.borrowed_on = Some("2026-11-03".into());
        let message = message_of(update(&mut conn, loan.id, edit, now()));
        assert!(message.contains("2026-11-02"), "{message}");

        // Paid rows count toward the 600.
        let edit = new_fields("E-wallet loan", vec![row("2026-12-01", 1); 600]);
        assert_eq!(
            field_of(update(&mut conn, loan.id, edit, now())),
            "payments"
        );
        let edit = new_fields("E-wallet loan", vec![row("2026-12-01", 1); 599]);
        assert!(update(&mut conn, loan.id, edit, now()).is_ok());
    }

    #[test]
    fn a_loan_with_nothing_paid_still_needs_a_row() {
        let mut conn = db();
        let input = existing("E-wallet loan", vec![row("2026-10-01", 1)]);
        let loan = create(&mut conn, input, now()).unwrap();
        let edit = fields("E-wallet loan", vec![]);
        assert_eq!(
            field_of(update(&mut conn, loan.id, edit, now())),
            "payments"
        );
    }

    #[test]
    fn an_edit_only_takes_the_loans_own_unpaid_rows_once_and_a_free_name() {
        let mut conn = db();
        let input = existing("E-wallet loan", vec![row("2026-12-01", 1)]);
        let loan = create(&mut conn, input, now()).unwrap();
        let other = create(
            &mut conn,
            existing("Bank loan", vec![row("2026-12-01", 1)]),
            now(),
        );
        let theirs = other.unwrap().payments[0].id;

        let edit = fields("E-wallet loan", vec![kept(theirs, "2026-12-01", 1)]);
        assert_eq!(
            field_of(update(&mut conn, loan.id, edit, now())),
            "payments"
        );

        let mine = loan.payments[0].id;
        let twice = vec![kept(mine, "2026-12-01", 1), kept(mine, "2027-01-01", 1)];
        let edit = fields("E-wallet loan", twice);
        assert_eq!(
            field_of(update(&mut conn, loan.id, edit, now())),
            "payments"
        );

        let edit = fields("Bank loan", vec![row("2026-12-01", 1)]);
        assert_eq!(field_of(update(&mut conn, loan.id, edit, now())), "name");
        // Nothing was half-saved by the refused edits.
        assert_eq!(find(&conn, loan.id), loan);

        // Its own name in another case is fine.
        let edit = fields("E-WALLET LOAN", vec![row("2026-12-01", 1)]);
        assert!(update(&mut conn, loan.id, edit, now()).is_ok());

        let edit = fields("E-wallet loan", vec![row("2026-12-01", 1)]);
        assert!(matches!(
            update(&mut conn, 999, edit, now()),
            Err(AppError::NotFound { .. })
        ));
    }

    #[test]
    fn an_edit_keeps_its_archived_account_and_category_but_cant_pick_others() {
        let mut conn = db();
        let wallet = account(&mut conn, "Wallet", AccountKind::Ewallet);
        let other_wallet = account(&mut conn, "Other wallet", AccountKind::Ewallet);
        let mine = category(&mut conn, "Lender", CategoryKind::Expense);
        let mut input = existing("E-wallet loan", vec![row("2026-12-01", 1)]);
        input.fields.account_id = wallet;
        input.fields.category_id = mine;
        let loan = create(&mut conn, input.clone(), now()).unwrap();
        accounts::archive(&mut conn, wallet, false, now()).unwrap();
        accounts::archive(&mut conn, other_wallet, false, now()).unwrap();
        categories::archive(&mut conn, mine, now()).unwrap();
        categories::archive(&mut conn, FOOD, now()).unwrap();

        let mut edit = input.fields;
        edit.payments = vec![kept(loan.payments[0].id, "2026-12-02", 1)];
        assert!(update(&mut conn, loan.id, edit.clone(), now()).is_ok());

        let mut other = edit.clone();
        other.account_id = other_wallet;
        assert_eq!(
            field_of(update(&mut conn, loan.id, other, now())),
            "accountId"
        );
        let mut other = edit;
        other.category_id = FOOD;
        assert_eq!(
            field_of(update(&mut conn, loan.id, other, now())),
            "categoryId"
        );
    }

    #[test]
    fn only_a_paid_off_loan_can_be_archived_and_any_can_be_unarchived() {
        let mut conn = db();
        let input = existing(
            "E-wallet loan",
            vec![row("2026-10-01", 1), row("2026-12-01", 1)],
        );
        let loan = create(&mut conn, input, now()).unwrap();
        assert_eq!(
            conflict(archive(&mut conn, loan.id, now())),
            "Pay or remove its unpaid payments first"
        );
        assert!(!find(&conn, loan.id).archived);
        pay(&mut conn, loan.payments[1].id, 1);
        archive(&mut conn, loan.id, now()).unwrap();
        archive(&mut conn, loan.id, now()).unwrap();
        assert!(find(&conn, loan.id).archived);
        unarchive(&mut conn, loan.id, now()).unwrap();
        unarchive(&mut conn, loan.id, now()).unwrap();
        assert!(!find(&conn, loan.id).archived);
        for result in [
            archive(&mut conn, 999, now()),
            unarchive(&mut conn, 999, now()),
            delete(&mut conn, 999),
        ] {
            assert!(matches!(result, Err(AppError::NotFound { .. })));
        }
    }

    #[test]
    fn the_list_gives_each_loans_figures_and_a_summary_with_archived_loans_counted() {
        let mut conn = db();
        let mut s = settings::stored(&conn).unwrap();
        s.month_start_day = 25;
        settings::update(&mut conn, s).unwrap();
        let mut old = existing(
            "E-wallet loan",
            vec![row("2026-10-01", 100_000), row("2026-11-10", 100_000)],
        );
        old.fields.amount_borrowed = Some(Money(200_000));
        let old = create(&mut conn, old, now()).unwrap();
        // Paid with a ₱50 late fee, then archived once paid off.
        pay(&mut conn, old.payments[1].id, 105_000);
        archive(&mut conn, old.id, now()).unwrap();
        let bank = new_loan(
            "Bank loan",
            vec![row("2026-12-01", 300_000), row("2027-01-01", 300_000)],
        );
        let bank = create(&mut conn, bank, now()).unwrap();

        let LoanList { loans, summary } = list(&conn, today()).unwrap();
        let figures = |l: &Loan| (l.left, l.paid, l.percent_paid, l.interest, l.paid_off);
        assert!(loans[0].archived);
        assert_eq!(
            figures(&loans[0]),
            (Money::ZERO, Money(205_000), 100, Some(Money(5_000)), true)
        );
        assert_eq!(loans[0].next, None);
        assert_eq!(
            figures(&loans[1]),
            (Money(600_000), Money::ZERO, 0, Some(Money(100_000)), false)
        );
        assert_eq!(loans[1].next.as_ref(), Some(&bank.payments[0]));
        // A save returns the figures too.
        assert_eq!(bank.left, Money(600_000));

        assert_eq!(summary.total_left, Money(600_000));
        assert_eq!(summary.total_paid, Money(205_000));
        assert_eq!(summary.percent_paid, 25);
        assert_eq!(summary.debt_free_on, Some(d(2027, 1, 1)));
        assert_eq!(summary.days_to_go, Some(59));
        assert_eq!(summary.interest, Some(Money(105_000)));
        // The Bank loan was borrowed on Nov 1.
        assert_eq!(summary.streak_days, Some(2));
        // The app's months start on the 25th.
        let months: Vec<_> = summary
            .freed_up
            .iter()
            .map(|m| (m.start, m.end, m.due.0, m.freed.0))
            .collect();
        assert_eq!(
            months,
            [
                (d(2026, 10, 25), d(2026, 11, 24), 100_000, 0),
                (d(2026, 11, 25), d(2026, 12, 24), 300_000, -200_000),
                (d(2026, 12, 25), d(2027, 1, 24), 300_000, -200_000),
                (d(2027, 1, 25), d(2027, 2, 24), 0, 100_000),
            ]
        );
    }

    #[test]
    fn an_edit_that_leaves_an_unpaid_row_unarchives_the_loan() {
        let mut conn = db();
        let input = existing("E-wallet loan", vec![row("2026-10-01", 1)]);
        let loan = create(&mut conn, input, now()).unwrap();
        archive(&mut conn, loan.id, now()).unwrap();
        let first = loan.payments[0].id;

        // An edit that leaves it paid off keeps it archived.
        let edit = fields("Old loan", vec![kept(first, "2026-10-01", 1)]);
        assert!(update(&mut conn, loan.id, edit, now()).unwrap().archived);

        // Adding a row.
        let edit = fields(
            "Old loan",
            vec![kept(first, "2026-10-01", 1), row("2026-12-01", 1)],
        );
        assert!(!update(&mut conn, loan.id, edit, now()).unwrap().archived);

        // Moving a paid-before row to on or after the day the loan was added.
        let edit = fields("Old loan", vec![kept(first, "2026-10-01", 1)]);
        update(&mut conn, loan.id, edit, now()).unwrap();
        archive(&mut conn, loan.id, now()).unwrap();
        let edit = fields("Old loan", vec![kept(first, "2026-11-03", 1)]);
        let saved = update(&mut conn, loan.id, edit, now()).unwrap();
        assert_eq!(statuses(&saved), [(today(), Unpaid, false)]);
        assert!(!saved.archived);
    }

    #[test]
    fn a_loan_with_a_payment_marked_paid_cant_be_deleted() {
        let mut conn = db();
        let input = existing(
            "E-wallet loan",
            vec![row("2026-10-01", 1), row("2026-12-01", 1)],
        );
        let paid_loan = create(&mut conn, input, now()).unwrap();
        pay(&mut conn, paid_loan.payments[1].id, 1);
        conflict(delete(&mut conn, paid_loan.id));

        // Paid before and unpaid rows don't stop it, and they go with it.
        let input = existing(
            "Bank loan",
            vec![row("2026-10-01", 1), row("2026-12-01", 1)],
        );
        let other = create(&mut conn, input, now()).unwrap();
        delete(&mut conn, other.id).unwrap();
        let rows: i64 = conn
            .query_row(
                "SELECT count(*) FROM loan_payments WHERE loan_id = ?1",
                [other.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rows, 0);
        let left = list(&conn, today()).unwrap().loans;
        assert_eq!(
            left.iter().map(|l| l.id).collect::<Vec<_>>(),
            [paid_loan.id]
        );
    }

    #[test]
    fn the_list_is_in_the_order_added_with_payments_by_date_then_id() {
        let mut conn = db();
        let zeta = create(
            &mut conn,
            existing("Zeta loan", vec![row("2026-12-01", 1)]),
            now(),
        );
        let input = existing(
            "Alpha loan",
            vec![
                row("2026-12-15", 2),
                row("2026-12-01", 3),
                row("2026-12-01", 4),
            ],
        );
        let alpha = create(&mut conn, input, now()).unwrap();
        let loans = list(&conn, d(2026, 12, 2)).unwrap().loans;
        assert_eq!(
            loans.iter().map(|l| l.id).collect::<Vec<_>>(),
            [zeta.unwrap().id, alpha.id]
        );
        // Two rows on one date keep the order they were typed in; overdue follows today.
        let rows: Vec<(i64, bool)> = loans[1]
            .payments
            .iter()
            .map(|p| (p.amount.0, p.overdue))
            .collect();
        assert_eq!(rows, [(3, true), (4, true), (2, false)]);
    }

    #[test]
    fn a_loan_puts_its_account_in_use() {
        let mut conn = db();
        let wallet = account(&mut conn, "Wallet", AccountKind::Ewallet);
        let mut input = existing("E-wallet loan", vec![row("2026-12-01", 1)]);
        input.fields.account_id = wallet;
        create(&mut conn, input, now()).unwrap();
        assert!(accounts::get(&conn, wallet, today()).unwrap().in_use);
        assert!(conflict(accounts::delete(&mut conn, wallet, now())).contains("loans"));
        // It can still be archived, and still not deleted.
        accounts::archive(&mut conn, wallet, false, now()).unwrap();
        assert!(conflict(accounts::delete(&mut conn, wallet, now())).contains("loans"));
    }

    #[test]
    fn a_loan_puts_its_category_in_use_and_a_merge_moves_it() {
        let mut conn = db();
        let mine = category(&mut conn, "Lender", CategoryKind::Expense);
        let mut input = existing("E-wallet loan", vec![row("2026-12-01", 1)]);
        input.fields.category_id = mine;
        let loan = create(&mut conn, input, now()).unwrap();
        let in_use = |conn: &Connection, id: i64| {
            categories::list(conn)
                .unwrap()
                .categories
                .into_iter()
                .find(|c| c.id == id)
                .unwrap()
                .in_use
        };
        assert!(in_use(&conn, mine));
        assert!(!in_use(&conn, LOAN));
        assert_eq!(
            conflict(categories::delete(&mut conn, mine)),
            "A loan uses this category, so it can only be archived or merged"
        );

        categories::merge(&mut conn, mine, LOAN, now()).unwrap();
        assert_eq!(find(&conn, loan.id).category_id, LOAN);
        assert!(in_use(&conn, LOAN));
    }

    #[test]
    fn a_loan_locks_the_decimals() {
        let mut conn = db();
        assert!(!settings::get(&conn).unwrap().decimals_locked);
        let input = existing("E-wallet loan", vec![row("2026-10-01", 1)]);
        let loan = create(&mut conn, input, now()).unwrap();
        assert!(settings::get(&conn).unwrap().decimals_locked);
        delete(&mut conn, loan.id).unwrap();
        assert!(!settings::get(&conn).unwrap().decimals_locked);
    }

    /// Every figure a screen shows about money this month, as JSON. The dashboard's by name: its
    /// loan banners and Safe to spend are meant to count unpaid payments (tasks 5–6).
    fn figures(conn: &mut Connection) -> serde_json::Value {
        let month = PeriodQuery::Month { anchor: None };
        let filter = TransactionFilter {
            period: month,
            kind: None,
            account_id: None,
            category_id: None,
            payment_method_id: None,
            search: None,
        };
        let board = dashboard::get(conn, &month, today()).unwrap();
        serde_json::json!({
            "accounts": accounts::list(conn, today()).unwrap(),
            "transactions": transactions::list(conn, filter, 0, today()).unwrap(),
            "budgets": budgets::list(conn, today()).unwrap(),
            "goals": goals::list(conn, today()).unwrap(),
            "timeseries": reports::timeseries(conn, &month, None, today()).unwrap(),
            "breakdown": reports::breakdown(conn, &month, CategoryKind::Expense, None, today()).unwrap(),
            "comparison": reports::comparison(conn, &month, None, None, today()).unwrap(),
            "dashboard": [
                serde_json::json!([board.income, board.expense, board.net, board.net_balance]),
                serde_json::json!(board.series),
                serde_json::json!(board.comparison),
                serde_json::json!(board.accounts),
                serde_json::json!(board.recent),
                serde_json::json!(board.budgets),
            ],
        })
    }

    #[test]
    fn an_unpaid_loan_payment_changes_no_balance_or_total() {
        let mut conn = db();
        // Cash already in use, so the loan putting it in use changes nothing on its own.
        let mut pay_day = expense(300_000);
        pay_day.kind = TransactionKind::Income;
        pay_day.category_id = Some(SALARY);
        transactions::create(&mut conn, pay_day, now()).unwrap();
        let budget = BudgetFields {
            category_id: Some(LOAN),
            period: BudgetPeriod::Monthly,
            amount: Money(500_000),
        };
        budgets::create(&mut conn, budget, now()).unwrap();
        let goal = goals::GoalFields {
            name: "Emergency fund".into(),
            target_amount: Money(1_000_000),
            target_date: None,
            account_id: CASH,
        };
        goals::create(&mut conn, goal, now()).unwrap();
        let before = figures(&mut conn);

        // One overdue and one due today: neither is a transaction until it's marked paid.
        let input = new_loan(
            "E-wallet loan",
            vec![row("2026-11-02", 100_000), row("2026-11-03", 100_000)],
        );
        let loan = create(&mut conn, input, now()).unwrap();
        assert_eq!(figures(&mut conn), before);

        pay(&mut conn, loan.payments[0].id, 100_000);
        assert_ne!(figures(&mut conn), before);
        let cash = accounts::get(&conn, CASH, today()).unwrap();
        assert_eq!(cash.balance, Money(200_000));
        let spent = budgets::list(&conn, today()).unwrap()[0].spent;
        assert_eq!(spent, Money(100_000));
    }

    #[test]
    fn mark_paid_records_the_expense_sent_and_links_it() {
        let mut conn = db();
        let input = new_loan(
            "E-wallet loan",
            vec![row("2026-11-02", 100_000), row("2026-12-01", 100_000)],
        );
        let loan = create(&mut conn, input, now()).unwrap();
        let (first, second) = (loan.payments[0].id, loan.payments[1].id);
        assert!(loan.payments[0].overdue);

        let mut sent = expense(100_000);
        sent.note = Some("  November  ".into());
        let txn = mark_paid(&mut conn, first, sent, now()).unwrap();
        assert_eq!(
            (
                txn.kind,
                txn.amount,
                txn.date,
                txn.account_id,
                txn.category_id
            ),
            (
                TransactionKind::Expense,
                Money(100_000),
                today(),
                CASH,
                Some(LOAN)
            )
        );
        assert_eq!(txn.payee.as_deref(), Some("E-wallet loan"));
        // Trimmed like any expense.
        assert_eq!(txn.note.as_deref(), Some("November"));
        assert_eq!(txn.loan_payment_id, Some(first));
        assert_eq!(transactions::get(&conn, txn.id).unwrap(), txn);
        let month = TransactionFilter {
            period: PeriodQuery::Month { anchor: None },
            kind: None,
            account_id: None,
            category_id: None,
            payment_method_id: None,
            search: None,
        };
        let listed = transactions::list(&conn, month, 0, today()).unwrap().items;
        assert_eq!(listed, std::slice::from_ref(&txn));

        // Paid, so no longer overdue, and the other payment is untouched.
        let p = payment(&conn, loan.id, first);
        assert_eq!(
            (p.status, p.overdue, p.transaction_id, p.paid_amount),
            (Paid, false, Some(txn.id), Some(Money(100_000)))
        );
        assert_eq!(payment(&conn, loan.id, second).status, Unpaid);

        // Any other transaction pays no loan.
        let other = transactions::create(&mut conn, expense(1), now()).unwrap();
        assert_eq!(other.loan_payment_id, None);
    }

    #[test]
    fn a_payment_can_be_paid_before_its_date_and_with_a_late_fee() {
        let mut conn = db();
        let input = new_loan(
            "E-wallet loan",
            vec![row("2026-11-02", 100_000), row("2026-12-01", 100_000)],
        );
        let loan = create(&mut conn, input, now()).unwrap();
        let (late, early) = (loan.payments[0].id, loan.payments[1].id);

        pay(&mut conn, early, 100_000);
        let p = payment(&conn, loan.id, early);
        assert_eq!((p.due_date, p.status), (d(2026, 12, 1), Paid));

        // The scheduled amount stays, and the paid amount is what was paid.
        pay(&mut conn, late, 105_000);
        let p = payment(&conn, loan.id, late);
        assert_eq!(
            (p.amount, p.paid_amount, p.overdue),
            (Money(100_000), Some(Money(105_000)), false)
        );
    }

    #[test]
    fn only_an_unpaid_payment_can_be_marked_paid_and_only_a_paid_one_unpaid() {
        let mut conn = db();
        let input = existing(
            "E-wallet loan",
            vec![row("2026-10-01", 100_000), row("2026-12-01", 100_000)],
        );
        let loan = create(&mut conn, input, now()).unwrap();
        let (before, unpaid) = (loan.payments[0].id, loan.payments[1].id);
        assert_eq!(loan.payments[0].status, PaidBefore);

        let not_paid = "This payment isn't marked paid";
        let already = "This payment is already paid";
        assert_eq!(conflict(mark_unpaid(&mut conn, unpaid, now())), not_paid);
        assert_eq!(conflict(mark_unpaid(&mut conn, before, now())), not_paid);
        assert_eq!(
            conflict(mark_paid(&mut conn, before, expense(1), now())),
            already
        );
        pay(&mut conn, unpaid, 100_000);
        assert_eq!(
            conflict(mark_paid(&mut conn, unpaid, expense(1), now())),
            already
        );
        assert!(matches!(
            mark_paid(&mut conn, 999, expense(1), now()),
            Err(AppError::NotFound { .. })
        ));
        assert!(matches!(
            mark_unpaid(&mut conn, 999, now()),
            Err(AppError::NotFound { .. })
        ));
        // The refused ones saved nothing.
        assert_eq!(transaction_count(&conn), 1);
    }

    #[test]
    fn mark_paid_checks_the_expense_like_a_new_one_but_keeps_the_loans_archived_picks() {
        let mut conn = db();
        let wallet = account(&mut conn, "Wallet", AccountKind::Ewallet);
        let lender = category(&mut conn, "Lender", CategoryKind::Expense);
        let gone_account = account(&mut conn, "Old wallet", AccountKind::Ewallet);
        let gone_category = category(&mut conn, "Old loans", CategoryKind::Expense);
        let mut input = existing("E-wallet loan", vec![row("2026-12-01", 100_000)]);
        input.fields.account_id = wallet;
        input.fields.category_id = lender;
        let loan = create(&mut conn, input, now()).unwrap();
        let id = loan.payments[0].id;
        for archived in [wallet, gone_account] {
            accounts::archive(&mut conn, archived, false, now()).unwrap();
        }
        for archived in [lender, gone_category] {
            categories::archive(&mut conn, archived, now()).unwrap();
        }
        let gone_method = 1;
        payment_methods::archive(&mut conn, gone_method, now()).unwrap();

        type Change = Box<dyn Fn(&mut TransactionFields)>;
        let cases: Vec<(&str, Change)> = vec![
            (
                "kind",
                Box::new(|f| {
                    f.kind = TransactionKind::Income;
                    f.category_id = Some(SALARY);
                }),
            ),
            (
                "kind",
                Box::new(|f| {
                    f.kind = TransactionKind::Transfer;
                    f.to_account_id = Some(CASH);
                    f.category_id = None;
                    f.payee = None;
                }),
            ),
            ("amount", Box::new(|f| f.amount = Money(0))),
            ("date", Box::new(|f| f.date = "2026-02-30".into())),
            ("accountId", Box::new(|f| f.account_id = 999)),
            ("accountId", Box::new(move |f| f.account_id = gone_account)),
            ("categoryId", Box::new(|f| f.category_id = Some(SALARY))),
            ("categoryId", Box::new(|f| f.category_id = None)),
            (
                "categoryId",
                Box::new(move |f| f.category_id = Some(gone_category)),
            ),
            ("payee", Box::new(|f| f.payee = Some("x".repeat(101)))),
            (
                "paymentMethodId",
                Box::new(move |f| f.payment_method_id = Some(gone_method)),
            ),
        ];
        for (field, change) in &cases {
            let mut sent = expense(100_000);
            sent.account_id = wallet;
            sent.category_id = Some(lender);
            change(&mut sent);
            assert_eq!(field_of(mark_paid(&mut conn, id, sent, now())), *field);
        }
        assert_eq!(payment(&conn, loan.id, id).status, Unpaid);
        assert_eq!(transaction_count(&conn), 0);

        // The loan's own account and category, though archived since.
        let mut sent = expense(100_000);
        sent.account_id = wallet;
        sent.category_id = Some(lender);
        let txn = mark_paid(&mut conn, id, sent, now()).unwrap();
        assert_eq!((txn.account_id, txn.category_id), (wallet, Some(lender)));

        // Like any expense, it may come from another active account and category.
        let input = existing("Bank loan", vec![row("2026-12-01", 100_000)]);
        let other = create(&mut conn, input, now()).unwrap();
        let mut sent = expense(100_000);
        sent.category_id = Some(FOOD);
        let txn = mark_paid(&mut conn, other.payments[0].id, sent, now()).unwrap();
        assert_eq!((txn.account_id, txn.category_id), (CASH, Some(FOOD)));
    }

    #[test]
    fn mark_paid_leaves_the_last_used_account_alone() {
        let mut conn = db();
        let wallet = account(&mut conn, "Wallet", AccountKind::Ewallet);
        let mut spend = expense(1);
        spend.account_id = wallet;
        transactions::create(&mut conn, spend, now()).unwrap();
        let input = existing("E-wallet loan", vec![row("2026-12-01", 100_000)]);
        let loan = create(&mut conn, input, now()).unwrap();
        pay(&mut conn, loan.payments[0].id, 100_000);
        let defaults = transactions::defaults(&conn, today()).unwrap();
        assert_eq!(defaults.account_id, Some(wallet));
    }

    #[test]
    fn deleting_a_loan_payments_expense_makes_it_unpaid_again() {
        let mut conn = db();
        let input = new_loan("E-wallet loan", vec![row("2026-11-02", 100_000)]);
        let loan = create(&mut conn, input, now()).unwrap();
        let id = loan.payments[0].id;
        let txn = pay(&mut conn, id, 105_000);

        transactions::delete(&mut conn, txn, now()).unwrap();
        let p = payment(&conn, loan.id, id);
        assert_eq!(
            (p.status, p.overdue, p.transaction_id, p.paid_amount),
            (Unpaid, true, None, None)
        );
        // It can be paid again.
        pay(&mut conn, id, 100_000);
        assert_eq!(payment(&conn, loan.id, id).status, Paid);
    }

    #[test]
    fn mark_unpaid_deletes_the_expense() {
        let mut conn = db();
        let input = new_loan("E-wallet loan", vec![row("2026-11-02", 100_000)]);
        let loan = create(&mut conn, input, now()).unwrap();
        let id = loan.payments[0].id;
        let txn = pay(&mut conn, id, 100_000);

        mark_unpaid(&mut conn, id, now()).unwrap();
        assert!(matches!(
            transactions::get(&conn, txn),
            Err(AppError::NotFound { .. })
        ));
        let p = payment(&conn, loan.id, id);
        assert_eq!((p.status, p.overdue), (Unpaid, true));
        assert_eq!(
            accounts::get(&conn, CASH, today()).unwrap().balance,
            Money(0)
        );
    }

    #[test]
    fn an_archived_loan_is_unarchived_when_a_payment_becomes_unpaid() {
        let mut conn = db();
        let input = existing(
            "E-wallet loan",
            vec![row("2026-10-01", 100_000), row("2026-12-01", 100_000)],
        );
        let loan = create(&mut conn, input, now()).unwrap();
        let id = loan.payments[1].id;

        // By Mark unpaid.
        pay(&mut conn, id, 100_000);
        archive(&mut conn, loan.id, now()).unwrap();
        mark_unpaid(&mut conn, id, now()).unwrap();
        assert!(!find(&conn, loan.id).archived);

        // By deleting its expense from the transaction list.
        let txn = pay(&mut conn, id, 100_000);
        archive(&mut conn, loan.id, now()).unwrap();
        transactions::delete(&mut conn, txn, now()).unwrap();
        assert!(!find(&conn, loan.id).archived);

        // Deleting any other transaction leaves an archived loan as it is.
        pay(&mut conn, id, 100_000);
        archive(&mut conn, loan.id, now()).unwrap();
        let other = transactions::create(&mut conn, expense(1), now()).unwrap();
        transactions::delete(&mut conn, other.id, now()).unwrap();
        assert!(find(&conn, loan.id).archived);
    }

    #[test]
    fn a_loan_payments_expense_can_be_edited_but_stays_an_expense() {
        let mut conn = db();
        let input = new_loan("E-wallet loan", vec![row("2026-11-02", 100_000)]);
        let loan = create(&mut conn, input, now()).unwrap();
        let id = loan.payments[0].id;
        let txn = pay(&mut conn, id, 100_000);

        // The link stays, and the paid amount follows the edit.
        let wallet = account(&mut conn, "Wallet", AccountKind::Ewallet);
        let mut edit = expense(105_000);
        edit.category_id = Some(FOOD);
        edit.account_id = wallet;
        edit.date = "2026-11-01".into();
        let saved = transactions::update(&mut conn, txn, edit, now()).unwrap();
        assert_eq!(saved.loan_payment_id, Some(id));
        let p = payment(&conn, loan.id, id);
        assert_eq!((p.status, p.paid_amount), (Paid, Some(Money(105_000))));
        // "Paid Nov 1": the paid date is the expense's date, and follows its edit.
        assert_eq!(p.paid_on, Some(d(2026, 11, 1)));

        let refused = "This pays a loan. Mark it unpaid to change it.";
        let mut income = expense(100_000);
        income.kind = TransactionKind::Income;
        income.category_id = Some(SALARY);
        assert_eq!(
            conflict(transactions::update(&mut conn, txn, income, now())),
            refused
        );
        let mut transfer = expense(100_000);
        transfer.kind = TransactionKind::Transfer;
        transfer.to_account_id = Some(wallet);
        // Refused before the fields are checked, so a category left on it doesn't matter.
        assert_eq!(
            conflict(transactions::update(&mut conn, txn, transfer, now())),
            refused
        );
        let stored = transactions::get(&conn, txn).unwrap();
        assert_eq!(
            (stored.kind, stored.amount),
            (TransactionKind::Expense, Money(105_000))
        );
    }
}
