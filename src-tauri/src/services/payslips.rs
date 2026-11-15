use std::collections::HashSet;

use chrono::{DateTime, Local, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::money::Money;
use crate::error::AppError;
use crate::repo;
use crate::services::recurring;
use crate::services::transactions::{self, Transaction};
use crate::services::{invalid, not_found, stamp};

/// One deduction line: an expense on the payslip (PAY-2, DATA_MODEL §4.7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Deduction {
    #[ts(type = "number")]
    pub category_id: i64,
    pub amount: Money,
}

/// What the payslip form sends (UI_FLOWS F14).
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PayslipFields {
    /// `YYYY-MM-DD`: the day the pay arrived, on every row.
    pub date: String,
    #[ts(type = "number")]
    pub account_id: i64,
    pub gross: Money,
    pub deductions: Vec<Deduction>,
    pub note: Option<String>,
}

/// A stored payslip: its gross row's id and category, and its rows as the form shows them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Payslip {
    /// The gross row's id, which every row's `payslipId` holds.
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub category_id: i64,
    pub date: NaiveDate,
    #[ts(type = "number")]
    pub account_id: i64,
    pub gross: Money,
    pub deductions: Vec<Deduction>,
    pub note: Option<String>,
}

/// Where a new payslip's form starts (PAY-2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PayslipDefaults {
    #[ts(type = "number")]
    pub account_id: i64,
    #[ts(type = "number")]
    pub category_id: i64,
    /// The payday, or today when it's recorded early.
    pub date: NaiveDate,
    pub gross: Money,
    pub deductions: Vec<Deduction>,
}

/// The net the form shows, so the frontend never adds up money (CLAUDE.md rule 2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PayslipNet {
    pub net: Money,
}

/// The rule's last payslip, else the last one in its category, else the rule's amount and no
/// lines (PAY-2). A line whose category is archived now is left out: a new payslip can't use it.
pub fn defaults(
    conn: &Connection,
    rule_id: i64,
    date: &str,
    today: NaiveDate,
) -> Result<PayslipDefaults, AppError> {
    let rule = payday(conn, rule_id)?;
    let category_id = gross_category(&rule)?;
    let date = transactions::real_date(date).ok_or_else(|| invalid("date", "Enter a real date"))?;
    let last = match repo::transactions::last_payslip_of_rule(conn, rule_id)? {
        Some(id) => Some(id),
        None => repo::transactions::last_payslip_in_category(conn, category_id)?,
    };
    let (gross, deductions) = match last {
        Some(id) => {
            let last = get(conn, id)?;
            let mut lines = Vec::new();
            for line in last.deductions {
                if repo::categories::find(conn, line.category_id)?.is_some_and(|c| !c.archived) {
                    lines.push(line);
                }
            }
            (last.gross, lines)
        }
        None => (Money(rule.amount), Vec::new()),
    };
    Ok(PayslipDefaults {
        account_id: rule.account_id,
        category_id,
        date: date.min(today),
        gross,
        deductions,
    })
}

/// gross − Σ deductions, whatever was typed: the form shows it in `--danger` when it isn't above 0.
pub fn preview(f: &PayslipFields) -> PayslipNet {
    let deducted = f
        .deductions
        .iter()
        .fold(0_i64, |sum, d| sum.saturating_add(d.amount.0));
    PayslipNet {
        net: Money(f.gross.0.saturating_sub(deducted)),
    }
}

/// Saves the payslip and settles the payday on `date` (PAY-2, PAY-4, DATA_MODEL §4.7–4.8): a date
/// in the Due list, or one 1–3 days ahead, which gets its occurrence row now so processing never
/// creates it again. Returns the gross row.
pub fn record(
    conn: &mut Connection,
    rule_id: i64,
    date: &str,
    mut fields: PayslipFields,
    now: DateTime<Local>,
) -> Result<Transaction, AppError> {
    let today = now.date_naive();
    let tx = conn.transaction()?;
    let rule = payday(&tx, rule_id)?;
    // A payday that arrived while the dialog was open is queued first, so it's due, not early.
    recurring::catch_up(&tx, rule, now)?;
    let rule = recurring::stored(&tx, rule_id)?;
    validate(&tx, &mut fields, None, today)?;
    let pending = match repo::recurring::occurrence_on(&tx, rule_id, date)? {
        Some(o) if o.status == "pending" => Some(o.id),
        Some(_) => return Err(conflict("This payday was already recorded or skipped")),
        None if recurring::early_dates(&tx, &rule, today)?
            .iter()
            .any(|d| d.to_string() == date) =>
        {
            None
        }
        None => return Err(conflict("This payday can't be recorded yet")),
    };
    let stamp = stamp(now);
    let gross = repo::transactions::insert(
        &tx,
        &repo::transactions::Values {
            kind: "income",
            amount: fields.gross.0,
            date: &fields.date,
            account_id: fields.account_id,
            to_account_id: None,
            category_id: Some(gross_category(&rule)?),
            payee: rule.payee.as_deref(),
            payment_method_id: None,
            note: fields.note.as_deref(),
        },
        &stamp,
    )?;
    repo::transactions::set_payslip(&tx, gross, gross)?;
    insert_deductions(&tx, gross, &fields, &stamp)?;
    let occurrence = match pending {
        Some(id) => id,
        None => repo::recurring::insert_occurrence(&tx, rule_id, date, "posted", &stamp)?
            .ok_or_else(|| conflict("This payday was already recorded or skipped"))?,
    };
    repo::recurring::settle_occurrence(&tx, occurrence, "posted", Some(gross), &stamp)?;
    // Moves the rule's next date past a payday recorded early (DATA_MODEL §4.8).
    recurring::catch_up(&tx, recurring::stored(&tx, rule_id)?, now)?;
    let saved = transactions::get(&tx, gross)?;
    tx.commit()?;
    Ok(saved)
}

/// The whole payslip, from any of its rows' ids.
pub fn get(conn: &Connection, id: i64) -> Result<Payslip, AppError> {
    let gross_id = repo::transactions::find(conn, id)?
        .and_then(|row| row.payslip_id)
        .ok_or_else(|| not_found("payslip", id))?;
    let mut rows = repo::transactions::payslip_rows(conn, gross_id)?.into_iter();
    let gross = rows
        .next()
        .filter(|row| row.id == gross_id)
        .ok_or_else(|| not_found("payslip", id))?;
    let bad = || AppError::Database {
        message: format!("Payslip {gross_id} has a row with no category"),
    };
    Ok(Payslip {
        id: gross.id,
        category_id: gross.category_id.ok_or_else(bad)?,
        date: transactions::real_date(&gross.date).ok_or_else(|| AppError::Database {
            message: format!("Payslip {gross_id} has an invalid date"),
        })?,
        account_id: gross.account_id,
        gross: Money(gross.amount),
        deductions: rows
            .map(|row| {
                Ok(Deduction {
                    category_id: row.category_id.ok_or_else(bad)?,
                    amount: Money(row.amount),
                })
            })
            .collect::<Result<_, AppError>>()?,
        note: gross.note,
    })
}

/// Edits the gross row in place, so its id and its payday's link stay, and replaces the
/// deductions; the date and account move on every row (DATA_MODEL §4.7). `id` is any of its rows.
pub fn update(
    conn: &mut Connection,
    id: i64,
    mut fields: PayslipFields,
    now: DateTime<Local>,
) -> Result<Payslip, AppError> {
    let tx = conn.transaction()?;
    let stored = get(&tx, id)?;
    validate(&tx, &mut fields, Some(&stored), now.date_naive())?;
    let payee = repo::transactions::find(&tx, stored.id)?.and_then(|row| row.payee);
    let stamp = stamp(now);
    repo::transactions::update(
        &tx,
        stored.id,
        &repo::transactions::Values {
            kind: "income",
            amount: fields.gross.0,
            date: &fields.date,
            account_id: fields.account_id,
            to_account_id: None,
            category_id: Some(stored.category_id),
            payee: payee.as_deref(),
            payment_method_id: None,
            note: fields.note.as_deref(),
        },
        &stamp,
    )?;
    repo::transactions::delete_deductions(&tx, stored.id)?;
    insert_deductions(&tx, stored.id, &fields, &stamp)?;
    let saved = get(&tx, stored.id)?;
    tx.commit()?;
    Ok(saved)
}

fn insert_deductions(
    conn: &Connection,
    gross: i64,
    f: &PayslipFields,
    stamp: &str,
) -> Result<(), AppError> {
    for line in &f.deductions {
        let id = repo::transactions::insert(
            conn,
            &repo::transactions::Values {
                kind: "expense",
                amount: line.amount.0,
                date: &f.date,
                account_id: f.account_id,
                to_account_id: None,
                category_id: Some(line.category_id),
                payee: None,
                payment_method_id: None,
                note: None,
            },
            stamp,
        )?;
        repo::transactions::set_payslip(conn, id, gross)?;
    }
    Ok(())
}

/// Every payslip rule in DATA_MODEL §6. `stored` is the payslip being edited: it may keep the
/// archived account or deduction categories it already uses.
fn validate(
    conn: &Connection,
    f: &mut PayslipFields,
    stored: Option<&Payslip>,
    today: NaiveDate,
) -> Result<(), AppError> {
    transactions::real_date(&f.date).ok_or_else(|| invalid("date", "Enter a real date"))?;
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
    if f.gross <= Money::ZERO {
        return Err(invalid("gross", "Enter an amount above 0"));
    }
    if f.gross > Money::MAX {
        return Err(invalid("gross", "That amount is too large"));
    }
    if f.deductions.len() > 30 {
        return Err(invalid("deductions", "Enter up to 30 deductions"));
    }
    let kept: HashSet<i64> = stored
        .map(|s| s.deductions.iter().map(|d| d.category_id).collect())
        .unwrap_or_default();
    let mut deducted = Money::ZERO;
    for (i, line) in f.deductions.iter().enumerate() {
        let category = format!("deductions.{i}.categoryId");
        let found = repo::categories::find(conn, line.category_id)?
            .ok_or_else(|| invalid(&category, "Choose a category"))?;
        if found.kind != "expense" {
            return Err(invalid(&category, "Choose an expense category"));
        }
        if found.archived && !kept.contains(&line.category_id) {
            return Err(invalid(&category, "This category is archived"));
        }
        let amount = format!("deductions.{i}.amount");
        if line.amount <= Money::ZERO {
            return Err(invalid(&amount, "Enter an amount above 0"));
        }
        if line.amount > Money::MAX {
            return Err(invalid(&amount, "That amount is too large"));
        }
        deducted = deducted
            .checked_add(line.amount)
            .ok_or_else(|| invalid(&amount, "That amount is too large"))?;
    }
    if deducted >= f.gross {
        return Err(invalid(
            "deductions",
            "Deductions can't add up to the gross pay or more",
        ));
    }
    f.note = transactions::clean(f.note.take());
    if f.note.as_ref().is_some_and(|n| n.chars().count() > 500) {
        return Err(invalid("note", "Enter up to 500 characters"));
    }
    Ok(())
}

/// The rule, which must be a payday: only a payday is settled with a payslip.
fn payday(conn: &Connection, rule_id: i64) -> Result<repo::recurring::Row, AppError> {
    let rule = recurring::stored(conn, rule_id)?;
    if !rule.is_payday {
        return Err(conflict("This rule isn't a payday"));
    }
    Ok(rule)
}

/// A payday is an income rule, so it always has a category (DATA_MODEL §4.1's CHECK).
fn gross_category(rule: &repo::recurring::Row) -> Result<i64, AppError> {
    rule.category_id.ok_or_else(|| AppError::Database {
        message: format!("Recurring rule {} has no category", rule.id),
    })
}

fn conflict(message: &str) -> AppError {
    AppError::Conflict {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::recurrence::Frequency;
    use crate::services::recurring::{RecurringMode, RecurringRuleFields};
    use crate::services::transactions::{TransactionFields, TransactionKind};
    use crate::services::{accounts, categories};
    use crate::test_support::{db, now};

    const SALARY: i64 = 1;

    fn d(text: &str) -> NaiveDate {
        transactions::real_date(text).unwrap()
    }

    fn on(date: &str) -> DateTime<Local> {
        d(date)
            .and_time(now().time())
            .and_local_timezone(Local)
            .unwrap()
    }

    fn id_of(conn: &Connection, table: &str, name: &str) -> i64 {
        conn.query_row(
            &format!("SELECT id FROM {table} WHERE name = ?1"),
            [name],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn bank(conn: &mut Connection, name: &str) -> i64 {
        let fields = accounts::AccountFields {
            name: name.into(),
            opening_balance: Money(0),
            credit_limit: None,
            statement_day: None,
            due_day: None,
        };
        let kind = accounts::AccountKind::Bank;
        accounts::create(conn, accounts::NewAccount { kind, fields }, now())
            .unwrap()
            .id
    }

    /// Salary of 4,200.00 net on the `start`'s day each month into `account`: a payday.
    fn rule_fields(account: i64, start: &str) -> RecurringRuleFields {
        RecurringRuleFields {
            transaction: TransactionFields {
                kind: TransactionKind::Income,
                amount: Money(420_000),
                date: start.into(),
                account_id: account,
                to_account_id: None,
                category_id: Some(SALARY),
                payee: Some("Employer".into()),
                payment_method_id: None,
                note: None,
            },
            frequency: Frequency::Monthly,
            interval: 1,
            end_date: None,
            max_occurrences: None,
            mode: RecurringMode::Confirm,
            is_payday: true,
        }
    }

    fn payday_rule(conn: &mut Connection, account: i64, start: &str, today: &str) -> i64 {
        recurring::create(conn, rule_fields(account, start), on(today))
            .unwrap()
            .id
    }

    /// 4,600.00 gross less SSS 180.00 and PhilHealth 115.00: 4,305.00 net.
    fn slip(conn: &Connection, account: i64, date: &str) -> PayslipFields {
        PayslipFields {
            date: date.into(),
            account_id: account,
            gross: Money(460_000),
            deductions: vec![
                Deduction {
                    category_id: id_of(conn, "categories", "SSS"),
                    amount: Money(18_000),
                },
                Deduction {
                    category_id: id_of(conn, "categories", "PhilHealth"),
                    amount: Money(11_500),
                },
            ],
            note: Some("  November  ".into()),
        }
    }

    /// The rule's processed dates: date, status and transaction.
    fn occurrences(conn: &Connection, rule: i64) -> Vec<(String, String, Option<i64>)> {
        conn.prepare(
            "SELECT occurrence_date, status, transaction_id FROM recurring_occurrences
             WHERE rule_id = ?1 ORDER BY occurrence_date",
        )
        .unwrap()
        .query_map([rule], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
    }

    fn rows(conn: &Connection) -> Vec<(String, i64, Option<i64>, Option<i64>)> {
        conn.prepare("SELECT kind, amount, category_id, payslip_id FROM transactions ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn balance(conn: &Connection, account: i64, today: &str) -> i64 {
        accounts::list(conn, d(today))
            .unwrap()
            .accounts
            .into_iter()
            .find(|a| a.id == account)
            .unwrap()
            .balance
            .0
    }

    fn field<T: std::fmt::Debug>(result: Result<T, AppError>) -> String {
        match result {
            Err(AppError::Validation { field, .. }) => field,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    fn conflict_of<T: std::fmt::Debug>(result: Result<T, AppError>) {
        assert!(
            matches!(result, Err(AppError::Conflict { .. })),
            "{result:?}"
        );
    }

    #[test]
    fn recording_a_due_payday_saves_gross_and_deductions_and_settles_it() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank");
        let rule = payday_rule(&mut conn, bank, "2026-11-15", "2026-11-15");
        let f = slip(&conn, bank, "2026-11-15");
        let gross = record(&mut conn, rule, "2026-11-15", f, on("2026-11-15")).unwrap();

        assert_eq!(gross.kind, TransactionKind::Income);
        assert_eq!(gross.payslip_id, Some(gross.id));
        assert_eq!(gross.payee.as_deref(), Some("Employer"), "the rule's payee");
        assert_eq!(gross.note.as_deref(), Some("November"));
        let sss = id_of(&conn, "categories", "SSS");
        let phil = id_of(&conn, "categories", "PhilHealth");
        let g = Some(gross.id);
        assert_eq!(
            rows(&conn),
            [
                ("income".into(), 460_000, Some(SALARY), g),
                ("expense".into(), 18_000, Some(sss), g),
                ("expense".into(), 11_500, Some(phil), g),
            ]
        );
        assert_eq!(
            occurrences(&conn, rule),
            [("2026-11-15".into(), "posted".into(), g)]
        );
        assert_eq!(balance(&conn, bank, "2026-11-15"), 430_500, "up by the net");
        assert!(
            recurring::list_due(&conn, d("2026-11-15"))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_payday_can_be_recorded_once_and_only_as_a_payday() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank");
        let rule = payday_rule(&mut conn, bank, "2026-10-15", "2026-11-15");
        let f = slip(&conn, bank, "2026-11-15");
        record(&mut conn, rule, "2026-11-15", f.clone(), on("2026-11-15")).unwrap();
        conflict_of(record(
            &mut conn,
            rule,
            "2026-11-15",
            f.clone(),
            on("2026-11-15"),
        ));

        let oct = repo::recurring::occurrence_on(&conn, rule, "2026-10-15")
            .unwrap()
            .unwrap();
        recurring::skip(&mut conn, oct.id, now()).unwrap();
        conflict_of(record(
            &mut conn,
            rule,
            "2026-10-15",
            f.clone(),
            on("2026-11-15"),
        ));

        let mut plain = rule_fields(bank, "2026-11-15");
        plain.is_payday = false;
        let plain = recurring::create(&mut conn, plain, on("2026-11-15"))
            .unwrap()
            .id;
        conflict_of(record(
            &mut conn,
            plain,
            "2026-11-15",
            f.clone(),
            on("2026-11-15"),
        ));
        conflict_of(defaults(&conn, plain, "2026-11-15", d("2026-11-15")));
        assert_eq!(rows(&conn).len(), 3, "only the first payslip was saved");
    }

    #[test]
    fn a_payday_recorded_early_is_never_created_again() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank");
        let mut fields = rule_fields(bank, "2026-10-15");
        fields.max_occurrences = Some(2); // Oct 15 and Nov 15
        let rule = recurring::create(&mut conn, fields, on("2026-11-13"))
            .unwrap()
            .id;
        let f = slip(&conn, bank, "2026-11-13");
        let gross = record(&mut conn, rule, "2026-11-15", f, on("2026-11-13")).unwrap();
        assert_eq!(
            gross.date,
            d("2026-11-13"),
            "the payslip takes the real date"
        );

        assert_eq!(
            recurring::process_due(&mut conn, on("2026-11-15")).unwrap(),
            0
        );
        assert_eq!(
            recurring::process_due(&mut conn, on("2026-12-20")).unwrap(),
            0
        );
        assert_eq!(
            occurrences(&conn, rule),
            [
                ("2026-10-15".into(), "pending".into(), None),
                ("2026-11-15".into(), "posted".into(), Some(gross.id)),
            ],
            "the early date counts toward the 2 times"
        );
        let listed = recurring::list(&conn).unwrap();
        assert_eq!(listed[0].next_date, None, "it has ended");
    }

    #[test]
    fn after_recording_early_the_next_date_is_the_following_payday() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank");
        let rule = payday_rule(&mut conn, bank, "2026-10-15", "2026-11-13");
        let f = slip(&conn, bank, "2026-11-13");
        record(&mut conn, rule, "2026-11-15", f, on("2026-11-13")).unwrap();
        let saved = &recurring::list(&conn).unwrap()[0];
        assert_eq!(saved.next_date, Some(d("2026-12-15")));
        assert!(
            recurring::upcoming(&conn, d("2026-11-13"))
                .unwrap()
                .is_empty()
        );

        // A schedule edit may start on the offered next date (UI_FLOWS §8.2).
        let mut edit = rule_fields(bank, "2026-12-15");
        edit.interval = 2;
        recurring::update(&mut conn, rule, edit, on("2026-11-13")).unwrap();
    }

    #[test]
    fn early_is_only_1_to_3_days_ahead_and_not_while_paused() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank");
        let rule = payday_rule(&mut conn, bank, "2026-10-17", "2026-11-13");
        let f = slip(&conn, bank, "2026-11-13");
        // Nov 17 is 4 days on.
        conflict_of(record(
            &mut conn,
            rule,
            "2026-11-17",
            f.clone(),
            on("2026-11-13"),
        ));
        // Not a date the rule falls on.
        conflict_of(record(
            &mut conn,
            rule,
            "2026-11-15",
            f.clone(),
            on("2026-11-13"),
        ));
        recurring::pause(&mut conn, rule, true, on("2026-11-14")).unwrap();
        conflict_of(record(
            &mut conn,
            rule,
            "2026-11-17",
            f.clone(),
            on("2026-11-14"),
        ));
        // The October date it queued before the pause is still due, and can be recorded.
        record(&mut conn, rule, "2026-10-17", f, on("2026-11-14")).unwrap();
    }

    #[test]
    fn a_payday_that_arrived_while_the_dialog_was_open_can_be_recorded() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank");
        let rule = payday_rule(&mut conn, bank, "2026-11-15", "2026-11-13");
        // Shown early on Nov 14, saved on Nov 15 before processing ran.
        let f = slip(&conn, bank, "2026-11-15");
        let gross = record(&mut conn, rule, "2026-11-15", f, on("2026-11-15")).unwrap();
        assert_eq!(
            occurrences(&conn, rule),
            [("2026-11-15".into(), "posted".into(), Some(gross.id))]
        );
    }

    #[test]
    fn defaults_come_from_the_rules_last_payslip_then_the_categorys_then_the_rule() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank");
        let first = payday_rule(&mut conn, bank, "2026-10-15", "2026-11-13");
        let second = payday_rule(&mut conn, bank, "2026-10-30", "2026-11-13");

        // Nothing recorded yet: the rule's amount, no lines, and today when it's early.
        let start = defaults(&conn, first, "2026-11-15", d("2026-11-13")).unwrap();
        assert_eq!(
            start,
            PayslipDefaults {
                account_id: bank,
                category_id: SALARY,
                date: d("2026-11-13"),
                gross: Money(420_000),
                deductions: vec![],
            }
        );

        // The second rule's payslip is the category's last one.
        let f = slip(&conn, bank, "2026-10-30");
        record(&mut conn, second, "2026-10-30", f.clone(), on("2026-11-13")).unwrap();
        let start = defaults(&conn, first, "2026-10-15", d("2026-11-13")).unwrap();
        assert_eq!((start.gross, start.deductions.len()), (Money(460_000), 2));
        assert_eq!(
            start.date,
            d("2026-10-15"),
            "a due payday starts on its date"
        );

        // The rule's own payslip wins once it has one, even an older one.
        let mut own = f;
        own.date = "2026-10-15".into();
        own.gross = Money(450_000);
        own.deductions.truncate(1);
        record(&mut conn, first, "2026-10-15", own, on("2026-11-13")).unwrap();
        let start = defaults(&conn, first, "2026-11-15", d("2026-11-13")).unwrap();
        assert_eq!((start.gross, start.deductions.len()), (Money(450_000), 1));

        // A line whose category is archived now is left out.
        let sss = id_of(&conn, "categories", "SSS");
        categories::archive(&mut conn, sss, now()).unwrap();
        let start = defaults(&conn, first, "2026-11-15", d("2026-11-13")).unwrap();
        assert_eq!((start.gross, start.deductions), (Money(450_000), vec![]));
    }

    #[test]
    fn the_net_must_be_above_0() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank");
        let rule = payday_rule(&mut conn, bank, "2026-11-15", "2026-11-15");
        let mut f = slip(&conn, bank, "2026-11-15");
        assert_eq!(preview(&f).net, Money(430_500));
        f.deductions[0].amount = Money(448_500); // 448,500 + 11,500 = the gross
        assert_eq!(preview(&f).net, Money(0));
        assert_eq!(
            field(record(
                &mut conn,
                rule,
                "2026-11-15",
                f.clone(),
                on("2026-11-15")
            )),
            "deductions"
        );
        f.deductions[0].amount = Money(500_000);
        assert_eq!(preview(&f).net, Money(-51_500));
        assert!(rows(&conn).is_empty(), "nothing was saved");
        let huge = PayslipFields {
            deductions: vec![
                Deduction {
                    category_id: 1,
                    amount: Money(i64::MAX),
                },
                Deduction {
                    category_id: 1,
                    amount: Money(i64::MAX),
                },
            ],
            ..f
        };
        assert_eq!(preview(&huge).net, Money(460_000 - i64::MAX));
    }

    #[test]
    fn a_payslip_follows_the_validation_rules() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank");
        let rule = payday_rule(&mut conn, bank, "2026-11-15", "2026-11-15");
        let good = slip(&conn, bank, "2026-11-15");
        let mut check =
            |f: PayslipFields| field(record(&mut conn, rule, "2026-11-15", f, on("2026-11-15")));

        let mut f = good.clone();
        f.gross = Money(0);
        assert_eq!(check(f), "gross");
        let mut f = good.clone();
        f.deductions = vec![f.deductions[0].clone(); 31];
        assert_eq!(check(f), "deductions");
        let mut f = good.clone();
        f.deductions[1].category_id = SALARY; // an income category
        assert_eq!(check(f), "deductions.1.categoryId");
        let mut f = good.clone();
        f.deductions[0].amount = Money(0);
        assert_eq!(check(f), "deductions.0.amount");
        let mut f = good.clone();
        f.date = "2026-02-30".into();
        assert_eq!(check(f), "date");
        let mut f = good.clone();
        f.note = Some("x".repeat(501));
        assert_eq!(check(f), "note");
        drop(check);

        let card = accounts::create(
            &mut conn,
            accounts::NewAccount {
                kind: accounts::AccountKind::CreditCard,
                fields: accounts::AccountFields {
                    name: "Card".into(),
                    opening_balance: Money(0),
                    credit_limit: Some(Money(1_000_000)),
                    statement_day: Some(20),
                    due_day: Some(5),
                },
            },
            now(),
        )
        .unwrap()
        .id;
        let mut f = good.clone();
        f.account_id = card;
        assert_eq!(
            field(record(&mut conn, rule, "2026-11-15", f, on("2026-11-15"))),
            "accountId"
        );
        let sss = id_of(&conn, "categories", "SSS");
        categories::archive(&mut conn, sss, now()).unwrap();
        assert_eq!(
            field(record(
                &mut conn,
                rule,
                "2026-11-15",
                good,
                on("2026-11-15")
            )),
            "deductions.0.categoryId"
        );
        assert!(rows(&conn).is_empty());
    }

    #[test]
    fn editing_a_payslip_keeps_its_gross_row_and_replaces_the_deductions() {
        let mut conn = db();
        let wallet = bank(&mut conn, "E-wallet");
        let bank = bank(&mut conn, "Bank");
        let rule = payday_rule(&mut conn, bank, "2026-11-15", "2026-11-15");
        let f = slip(&conn, bank, "2026-11-15");
        let gross = record(&mut conn, rule, "2026-11-15", f.clone(), on("2026-11-15")).unwrap();
        let sss = id_of(&conn, "categories", "SSS");
        categories::archive(&mut conn, sss, now()).unwrap();

        // Opened from a deduction row; SSS stays though it's archived now.
        let deduction = get(&conn, gross.id).unwrap();
        assert_eq!(deduction.deductions.len(), 2);
        let some_row: i64 = conn
            .query_row(
                "SELECT max(id) FROM transactions WHERE payslip_id = ?1",
                [gross.id],
                |r| r.get(0),
            )
            .unwrap();
        let edit = PayslipFields {
            date: "2026-11-16".into(),
            account_id: wallet,
            gross: Money(470_000),
            deductions: vec![f.deductions[0].clone()],
            note: None,
        };
        let saved = update(&mut conn, some_row, edit, on("2026-11-16")).unwrap();
        assert_eq!(saved.id, gross.id, "the gross row keeps its id");
        assert_eq!(saved.date, d("2026-11-16"));
        assert_eq!(saved.account_id, wallet);
        assert_eq!(saved.deductions, vec![f.deductions[0].clone()]);
        assert_eq!(
            occurrences(&conn, rule),
            [("2026-11-15".into(), "posted".into(), Some(gross.id))]
        );
        let moved: Vec<(String, i64)> = conn
            .prepare("SELECT date, account_id FROM transactions ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(moved, vec![("2026-11-16".to_string(), wallet); 2]);
        let kept = transactions::get(&conn, gross.id).unwrap();
        assert_eq!(kept.payee.as_deref(), Some("Employer"));
        assert_eq!(kept.category_id, Some(SALARY));
    }

    #[test]
    fn deleting_any_payslip_row_deletes_the_whole_payslip() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank");
        let rule = payday_rule(&mut conn, bank, "2026-11-15", "2026-11-15");
        let f = slip(&conn, bank, "2026-11-15");
        let gross = record(&mut conn, rule, "2026-11-15", f, on("2026-11-15")).unwrap();
        let deduction: i64 = conn
            .query_row(
                "SELECT max(id) FROM transactions WHERE payslip_id = ?1",
                [gross.id],
                |r| r.get(0),
            )
            .unwrap();

        // It's edited as a payslip, never as a transaction.
        let as_row = transactions::get(&conn, deduction).unwrap();
        assert_eq!(as_row.payslip_id, Some(gross.id));
        let mut fields = TransactionFields {
            kind: as_row.kind,
            amount: as_row.amount,
            date: as_row.date.to_string(),
            account_id: as_row.account_id,
            to_account_id: None,
            category_id: as_row.category_id,
            payee: None,
            payment_method_id: None,
            note: None,
        };
        fields.amount = Money(1);
        conflict_of(transactions::update(&mut conn, deduction, fields, now()));

        transactions::delete(&mut conn, deduction, now()).unwrap();
        assert!(rows(&conn).is_empty());
        assert_eq!(
            occurrences(&conn, rule),
            [("2026-11-15".into(), "posted".into(), None)],
            "the payday stays settled"
        );
        assert_eq!(
            recurring::process_due(&mut conn, on("2026-11-15")).unwrap(),
            0
        );
        assert!(
            recurring::list_due(&conn, d("2026-11-15"))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_payslip_still_opens_once_its_rule_is_deleted() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank");
        let rule = payday_rule(&mut conn, bank, "2026-11-15", "2026-11-15");
        let f = slip(&conn, bank, "2026-11-15");
        let gross = record(&mut conn, rule, "2026-11-15", f.clone(), on("2026-11-15")).unwrap();
        recurring::delete(&mut conn, rule).unwrap();
        assert_eq!(get(&conn, gross.id).unwrap().category_id, SALARY);
        update(&mut conn, gross.id, f, on("2026-11-15")).unwrap();
        let plain = transactions::create(
            &mut conn,
            TransactionFields {
                kind: TransactionKind::Income,
                amount: Money(100),
                date: "2026-11-15".into(),
                account_id: bank,
                to_account_id: None,
                category_id: Some(SALARY),
                payee: None,
                payment_method_id: None,
                note: None,
            },
            now(),
        )
        .unwrap();
        assert_eq!(plain.payslip_id, None);
        assert!(matches!(
            get(&conn, plain.id),
            Err(AppError::NotFound { .. })
        ));
    }
}
