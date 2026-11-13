use chrono::{DateTime, Local, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use std::collections::HashSet;

use crate::domain::money::Money;
use crate::error::AppError;
use crate::repo;
use crate::services::transactions::real_date;
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
}

/// What `list_loans` returns. An object, so the figures can join it (ARCHITECTURE §4.5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoanList {
    /// In the order they were added, archived ones included.
    pub loans: Vec<Loan>,
}

/// Every loan, archived ones included, with its payments as of `today`.
pub fn list(conn: &Connection, today: NaiveDate) -> Result<LoanList, AppError> {
    // Both come sorted by loan id, so each loan takes its run of payments.
    let mut payments = repo::loans::payments(conn)?.into_iter().peekable();
    let mut loans = Vec::new();
    for row in repo::loans::all(conn)? {
        let mine = std::iter::from_fn(|| payments.next_if(|p| p.loan_id == row.id)).collect();
        loans.push(view(row, mine, today)?);
    }
    Ok(LoanList { loans })
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
            Ok(LoanPayment {
                id: p.id,
                due_date,
                amount: Money(p.amount),
                status,
                overdue: status == LoanPaymentStatus::Unpaid && due_date < today,
                transaction_id: p.transaction_id,
                paid_amount,
            })
        })
        .collect::<Result<_, AppError>>()?;
    Ok(Loan {
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
        amount_borrowed: row.amount_borrowed.map(Money),
        archived: row.archived,
        payments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::accounts::{self, AccountFields, AccountKind, NewAccount};
    use crate::services::categories::{self, CategoryFields, CategoryKind, NewCategory};
    use crate::services::settings;
    use crate::services::transactions::{self, TransactionFields, TransactionKind};
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

    /// Links the payment to a new expense, as Mark paid will (task 4), and returns its id.
    fn mark_paid(conn: &mut Connection, payment_id: i64, amount: i64) -> i64 {
        let fields = TransactionFields {
            kind: TransactionKind::Expense,
            amount: Money(amount),
            date: "2026-11-03".into(),
            account_id: CASH,
            to_account_id: None,
            category_id: Some(LOAN),
            payee: None,
            payment_method_id: None,
            note: None,
        };
        let txn = transactions::create(conn, fields, now()).unwrap().id;
        conn.execute(
            "UPDATE loan_payments SET transaction_id = ?2 WHERE id = ?1",
            [payment_id, txn],
        )
        .unwrap();
        txn
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
        assert!(loan.payments.iter().all(|p| p.paid_amount.is_none()));
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
        let txn = mark_paid(&mut conn, paid, 105_000);

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
        mark_paid(&mut conn, loan.payments[1].id, 1);
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
        mark_paid(&mut conn, paid_loan.payments[1].id, 1);
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
}
