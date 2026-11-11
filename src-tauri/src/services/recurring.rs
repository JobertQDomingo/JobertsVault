use chrono::{DateTime, Local, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::money::Money;
use crate::domain::recurrence::{Frequency, Schedule};
use crate::error::AppError;
use crate::repo;
use crate::services::transactions::{self, Transaction, TransactionFields};
use crate::services::{invalid, not_found, stamp};

/// Auto-post creates the transaction; Confirm queues it in the Due list (REC-3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RecurringMode {
    Auto,
    Confirm,
}

/// What the rule form sends: the transaction form plus a schedule (UI_FLOWS §8.2).
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RecurringRuleFields {
    /// The template. Its `date` is the start date, which may be in the future.
    pub transaction: TransactionFields,
    pub frequency: Frequency,
    /// Every *N*: 1–999.
    #[ts(type = "number")]
    pub interval: i64,
    /// `YYYY-MM-DD`, included. A rule ends on a date or after a number of times, not both.
    pub end_date: Option<String>,
    /// 1–9,999 dates in the rule's life, counting those already processed (DATA_MODEL §4.1).
    #[ts(type = "number | null")]
    pub max_occurrences: Option<i64>,
    pub mode: RecurringMode,
}

/// One stored rule (DATA_MODEL §4.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RecurringRule {
    #[ts(type = "number")]
    pub id: i64,
    /// The template. Its `date` is the start date.
    pub transaction: TransactionFields,
    pub frequency: Frequency,
    #[ts(type = "number")]
    pub interval: i64,
    pub end_date: Option<NaiveDate>,
    #[ts(type = "number | null")]
    pub max_occurrences: Option<i64>,
    pub mode: RecurringMode,
    pub paused: bool,
    /// The next date it will post or queue; none once it has ended. An edit that changes the
    /// schedule must start after the last date already processed, which is before this one.
    pub next_date: Option<NaiveDate>,
}

pub fn list(conn: &Connection) -> Result<Vec<RecurringRule>, AppError> {
    repo::recurring::all(conn)?.into_iter().map(view).collect()
}

/// A new rule (REC-1). Its first date may be in the past: processing fills in the dates since.
pub fn create(
    conn: &mut Connection,
    mut fields: RecurringRuleFields,
    now: DateTime<Local>,
) -> Result<RecurringRule, AppError> {
    let tx = conn.transaction()?;
    validate(&tx, &mut fields, None, now.date_naive())?;
    let id = repo::recurring::insert(&tx, &values(&fields), &stamp(now))?;
    let saved = get(&tx, id)?;
    tx.commit()?;
    Ok(saved)
}

/// Changes future dates only (REC-5). A new frequency, interval or start date starts the
/// schedule again from the start date, which must be after the last date already processed.
pub fn update(
    conn: &mut Connection,
    id: i64,
    mut fields: RecurringRuleFields,
    now: DateTime<Local>,
) -> Result<RecurringRule, AppError> {
    let tx = conn.transaction()?;
    let stored = stored(&tx, id)?;
    validate(&tx, &mut fields, Some(&stored), now.date_naive())?;
    let rescheduled = frequency_text(fields.frequency) != stored.frequency
        || fields.interval != stored.interval
        || fields.transaction.date != stored.start_date;
    let mut next_index = stored.next_index;
    if rescheduled {
        // Same-format date text compares in date order (DATA_MODEL §1).
        if stored
            .last_date
            .as_deref()
            .is_some_and(|last| fields.transaction.date.as_str() <= last)
        {
            return Err(invalid(
                "date",
                "Choose a start date after the last one this rule already created",
            ));
        }
        next_index = 0;
    }
    repo::recurring::update(&tx, id, &values(&fields), next_index, &stamp(now))?;
    let saved = get(&tx, id)?;
    tx.commit()?;
    Ok(saved)
}

/// A paused rule isn't processed. Resuming moves on to the first date from today, so the dates
/// it was paused for are never created (DATA_MODEL §4.1, REC-5).
pub fn pause(
    conn: &mut Connection,
    id: i64,
    paused: bool,
    now: DateTime<Local>,
) -> Result<RecurringRule, AppError> {
    let tx = conn.transaction()?;
    let stored = stored(&tx, id)?;
    if stored.paused != paused {
        let mut next_index = stored.next_index;
        if !paused {
            let from = index(&stored, stored.next_index)?;
            next_index = schedule(&stored)?
                .first_on_or_after(from, now.date_naive())
                .into();
        }
        repo::recurring::set_paused(&tx, id, paused, next_index, &stamp(now))?;
    }
    let saved = get(&tx, id)?;
    tx.commit()?;
    Ok(saved)
}

/// The transactions it created stay; its occurrence rows go (DATA_MODEL §4.2).
pub fn delete(conn: &mut Connection, id: i64) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    if repo::recurring::delete(&tx, id)? == 0 {
        return Err(not_found("recurring rule", id));
    }
    tx.commit()?;
    Ok(())
}

fn get(conn: &Connection, id: i64) -> Result<RecurringRule, AppError> {
    view(stored(conn, id)?)
}

fn stored(conn: &Connection, id: i64) -> Result<repo::recurring::Row, AppError> {
    repo::recurring::find(conn, id)?.ok_or_else(|| not_found("recurring rule", id))
}

/// The template follows the transaction rules, except that its date may be in the future; then
/// the schedule's own rules (DATA_MODEL §6). `stored` is the rule being edited.
fn validate(
    conn: &Connection,
    f: &mut RecurringRuleFields,
    stored: Option<&repo::recurring::Row>,
    today: NaiveDate,
) -> Result<(), AppError> {
    let kept = stored.map(template).transpose()?;
    transactions::validate(conn, &mut f.transaction, kept.as_ref(), today, true)?;
    if !(1..=999).contains(&f.interval) {
        return Err(invalid("interval", "Enter a number from 1 to 999"));
    }
    if f.max_occurrences.is_some_and(|n| !(1..=9_999).contains(&n)) {
        return Err(invalid("maxOccurrences", "Enter a number from 1 to 9,999"));
    }
    if let Some(end) = &f.end_date {
        if transactions::real_date(end).is_none() {
            return Err(invalid("endDate", "Enter a real date"));
        }
        if f.max_occurrences.is_some() {
            return Err(invalid(
                "endDate",
                "Choose an end date or a number of times, not both",
            ));
        }
        // Both are checked YYYY-MM-DD text, which compares in date order.
        if *end < f.transaction.date {
            return Err(invalid(
                "endDate",
                "Choose a date on or after the start date",
            ));
        }
    }
    Ok(())
}

fn values(f: &RecurringRuleFields) -> repo::recurring::Values<'_> {
    repo::recurring::Values {
        template: transactions::values(&f.transaction),
        frequency: frequency_text(f.frequency),
        interval: f.interval,
        end_date: f.end_date.as_deref(),
        max_occurrences: f.max_occurrences,
        mode: match f.mode {
            RecurringMode::Auto => "auto",
            RecurringMode::Confirm => "confirm",
        },
    }
}

fn bad(row: &repo::recurring::Row, what: &str) -> AppError {
    AppError::Database {
        message: format!("Recurring rule {} has an invalid {what}", row.id),
    }
}

fn view(row: repo::recurring::Row) -> Result<RecurringRule, AppError> {
    let schedule = schedule(&row)?;
    let next_date = schedule.next(index(&row, row.next_index)?, index(&row, row.done)?);
    let mode = match row.mode.as_str() {
        "auto" => RecurringMode::Auto,
        "confirm" => RecurringMode::Confirm,
        _ => return Err(bad(&row, "mode")),
    };
    Ok(RecurringRule {
        id: row.id,
        transaction: fields_of(&row)?,
        frequency: schedule.frequency,
        interval: row.interval,
        end_date: schedule.end,
        max_occurrences: row.max_occurrences,
        mode,
        paused: row.paused,
        next_date,
    })
}

fn schedule(row: &repo::recurring::Row) -> Result<Schedule, AppError> {
    let date = |text: &str| transactions::real_date(text).ok_or_else(|| bad(row, "date"));
    let frequency = [
        Frequency::Daily,
        Frequency::Weekly,
        Frequency::Monthly,
        Frequency::Yearly,
    ]
    .into_iter()
    .find(|f| frequency_text(*f) == row.frequency)
    .ok_or_else(|| bad(row, "frequency"))?;
    Ok(Schedule {
        frequency,
        interval: index(row, row.interval)?,
        start: date(&row.start_date)?,
        end: row.end_date.as_deref().map(date).transpose()?,
        max_occurrences: row.max_occurrences.map(|n| index(row, n)).transpose()?,
    })
}

/// A stored count or index as the schedule's `u32`.
fn index(row: &repo::recurring::Row, n: i64) -> Result<u32, AppError> {
    u32::try_from(n).map_err(|_| bad(row, "count"))
}

fn fields_of(row: &repo::recurring::Row) -> Result<TransactionFields, AppError> {
    Ok(TransactionFields {
        kind: transactions::kind_of(&row.kind).ok_or_else(|| bad(row, "kind"))?,
        amount: Money(row.amount),
        date: row.start_date.clone(),
        account_id: row.account_id,
        to_account_id: row.to_account_id,
        category_id: row.category_id,
        payee: row.payee.clone(),
        payment_method_id: row.payment_method_id,
        note: row.note.clone(),
    })
}

/// The stored template in the shape the transaction rules take, so an edit can keep an archived
/// account, category or payment method the rule already uses.
fn template(row: &repo::recurring::Row) -> Result<Transaction, AppError> {
    let f = fields_of(row)?;
    Ok(Transaction {
        id: row.id,
        kind: f.kind,
        amount: f.amount,
        date: schedule(row)?.start,
        account_id: f.account_id,
        to_account_id: f.to_account_id,
        category_id: f.category_id,
        payee: f.payee,
        payment_method_id: f.payment_method_id,
        note: f.note,
    })
}

fn frequency_text(frequency: Frequency) -> &'static str {
    match frequency {
        Frequency::Daily => "daily",
        Frequency::Weekly => "weekly",
        Frequency::Monthly => "monthly",
        Frequency::Yearly => "yearly",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::transactions::TransactionKind;
    use crate::services::{accounts, categories, payment_methods, settings};
    use crate::test_support::{db, now};

    const CASH: i64 = 1;
    const FOOD: i64 = 8;
    const DINING: i64 = 9;
    const CASH_METHOD: i64 = 1;

    fn rent(start: &str) -> RecurringRuleFields {
        RecurringRuleFields {
            transaction: TransactionFields {
                kind: TransactionKind::Expense,
                amount: Money(1_500_000),
                date: start.into(),
                account_id: CASH,
                to_account_id: None,
                category_id: Some(FOOD),
                payee: Some("  Landlord ".into()),
                payment_method_id: Some(CASH_METHOD),
                note: Some(" ".into()),
            },
            frequency: Frequency::Monthly,
            interval: 1,
            end_date: None,
            max_occurrences: None,
            mode: RecurringMode::Auto,
        }
    }

    fn d(text: &str) -> NaiveDate {
        transactions::real_date(text).unwrap()
    }

    fn field(result: Result<RecurringRule, AppError>) -> String {
        match result {
            Err(AppError::Validation { field, .. }) => field,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    /// Stands in for processing (REC-4, a later task): records `dates` as posted and moves on.
    fn process(conn: &Connection, id: i64, dates: &[&str]) {
        for date in dates {
            conn.execute(
                "INSERT INTO recurring_occurrences (rule_id, occurrence_date, status, created_at, updated_at) VALUES (?1, ?2, 'posted', 'x', 'x')",
                rusqlite::params![id, date],
            )
            .unwrap();
        }
        conn.execute(
            "UPDATE recurring_rules SET next_index = next_index + ?2 WHERE id = ?1",
            rusqlite::params![id, dates.len() as i64],
        )
        .unwrap();
    }

    fn rule(conn: &Connection, id: i64) -> RecurringRule {
        get(conn, id).unwrap()
    }

    #[test]
    fn a_new_rule_is_saved_trimmed_and_starts_on_its_start_date() {
        let mut conn = db();
        let saved = create(&mut conn, rent("2026-01-31"), now()).unwrap();
        assert_eq!(saved.transaction.payee.as_deref(), Some("Landlord"));
        assert_eq!(saved.transaction.note, None);
        assert_eq!(saved.next_date, Some(d("2026-01-31")));
        assert!(!saved.paused);
        assert_eq!(saved.mode, RecurringMode::Auto);
        assert_eq!(list(&conn).unwrap(), [saved]);
    }

    #[test]
    fn a_start_date_may_be_in_the_past_or_the_future() {
        let mut conn = db();
        for start in ["2025-06-01", "2027-03-15"] {
            assert_eq!(
                create(&mut conn, rent(start), now()).unwrap().next_date,
                Some(d(start))
            );
        }
    }

    #[test]
    fn the_template_follows_the_transaction_rules() {
        let mut conn = db();
        let mut transfer = rent("2026-11-30");
        transfer.transaction.kind = TransactionKind::Transfer;
        assert_eq!(field(create(&mut conn, transfer, now())), "categoryId");
        let mut zero = rent("2026-11-30");
        zero.transaction.amount = Money(0);
        assert_eq!(field(create(&mut conn, zero, now())), "amount");
        let mut bad_date = rent("2026-11-31");
        bad_date.transaction.date = "2026-11-31".into();
        assert_eq!(field(create(&mut conn, bad_date, now())), "date");
        let mut income = rent("2026-11-30");
        income.transaction.kind = TransactionKind::Income;
        assert_eq!(field(create(&mut conn, income, now())), "categoryId");
    }

    #[test]
    fn the_schedule_is_checked() {
        let mut conn = db();
        type Case = (fn(&mut RecurringRuleFields), &'static str);
        let cases: [Case; 7] = [
            (|f| f.interval = 0, "interval"),
            (|f| f.interval = 1000, "interval"),
            (|f| f.max_occurrences = Some(0), "maxOccurrences"),
            (|f| f.max_occurrences = Some(10_000), "maxOccurrences"),
            (|f| f.end_date = Some("2026-02-30".into()), "endDate"),
            (|f| f.end_date = Some("2026-01-30".into()), "endDate"),
            (
                |f| {
                    f.end_date = Some("2026-12-31".into());
                    f.max_occurrences = Some(3);
                },
                "endDate",
            ),
        ];
        for (change, expected) in cases {
            let mut fields = rent("2026-01-31");
            change(&mut fields);
            assert_eq!(field(create(&mut conn, fields, now())), expected);
        }
        let mut same_day = rent("2026-01-31");
        same_day.end_date = Some("2026-01-31".into());
        assert_eq!(
            create(&mut conn, same_day, now()).unwrap().end_date,
            Some(d("2026-01-31"))
        );
        assert!(list(&conn).unwrap().len() == 1, "nothing else was saved");
    }

    #[test]
    fn an_archived_category_can_be_kept_but_not_chosen() {
        let mut conn = db();
        let id = create(&mut conn, rent("2026-01-31"), now()).unwrap().id;
        categories::archive(&mut conn, FOOD, now()).unwrap();
        assert_eq!(
            field(create(&mut conn, rent("2026-01-31"), now())),
            "categoryId"
        );
        let mut cheaper = rent("2026-01-31");
        cheaper.transaction.amount = Money(1_200_000);
        assert_eq!(
            update(&mut conn, id, cheaper, now())
                .unwrap()
                .transaction
                .amount,
            Money(1_200_000)
        );
    }

    #[test]
    fn a_template_edit_changes_future_dates_only() {
        let mut conn = db();
        let id = create(&mut conn, rent("2026-01-31"), now()).unwrap().id;
        process(&conn, id, &["2026-01-31", "2026-02-28"]);
        let mut edit = rent("2026-01-31");
        edit.transaction.amount = Money(1_600_000);
        edit.mode = RecurringMode::Confirm;
        edit.max_occurrences = Some(5);
        let saved = update(&mut conn, id, edit, now()).unwrap();
        assert_eq!(saved.next_date, Some(d("2026-03-31")), "the place is kept");
        assert_eq!(saved.mode, RecurringMode::Confirm);
    }

    #[test]
    fn a_schedule_edit_starts_again_after_the_dates_already_processed() {
        let mut conn = db();
        let mut fields = rent("2026-01-31");
        fields.max_occurrences = Some(4);
        let id = create(&mut conn, fields, now()).unwrap().id;
        process(&conn, id, &["2026-01-31", "2026-02-28"]);

        let mut too_early = rent("2026-02-28");
        too_early.max_occurrences = Some(4);
        assert_eq!(field(update(&mut conn, id, too_early, now())), "date");

        // A 31st whose next date was Mar 31 moves to every 2 months from Mar 31, not the 28th.
        let mut every_2 = rent("2026-03-31");
        every_2.interval = 2;
        every_2.max_occurrences = Some(4);
        let saved = update(&mut conn, id, every_2, now()).unwrap();
        assert_eq!(saved.next_date, Some(d("2026-03-31")));
        process(&conn, id, &["2026-03-31"]);
        assert_eq!(rule(&conn, id).next_date, Some(d("2026-05-31")));
        process(&conn, id, &["2026-05-31"]);
        assert_eq!(
            rule(&conn, id).next_date,
            None,
            "4 times in all, the 2 before the edit included"
        );
    }

    #[test]
    fn resuming_skips_the_dates_it_was_paused_for() {
        let mut conn = db();
        let id = create(&mut conn, rent("2026-01-31"), now()).unwrap().id;
        process(&conn, id, &["2026-01-31"]);
        let paused = pause(&mut conn, id, true, now()).unwrap();
        assert!(paused.paused);
        assert_eq!(
            paused.next_date,
            Some(d("2026-02-28")),
            "pausing moves nothing"
        );
        assert_eq!(pause(&mut conn, id, true, now()).unwrap(), paused);

        // now() is Nov 3, 2026: Feb 28 through Oct 31 are never created.
        let resumed = pause(&mut conn, id, false, now()).unwrap();
        assert!(!resumed.paused);
        assert_eq!(resumed.next_date, Some(d("2026-11-30")));
        assert_eq!(
            pause(&mut conn, id, false, now()).unwrap(),
            resumed,
            "resuming twice moves nothing"
        );
    }

    #[test]
    fn a_resumed_rule_still_counts_only_processed_dates() {
        let mut conn = db();
        let mut fields = rent("2026-01-31");
        fields.max_occurrences = Some(2);
        let id = create(&mut conn, fields, now()).unwrap().id;
        process(&conn, id, &["2026-01-31"]);
        pause(&mut conn, id, true, now()).unwrap();
        let resumed = pause(&mut conn, id, false, now()).unwrap();
        assert_eq!(resumed.next_date, Some(d("2026-11-30")), "one time left");
    }

    #[test]
    fn deleting_a_rule_keeps_its_transactions() {
        let mut conn = db();
        let id = create(&mut conn, rent("2026-01-31"), now()).unwrap().id;
        let mut posted = rent("2026-01-31").transaction;
        posted.date = "2026-01-31".into();
        let txn = transactions::create(&mut conn, posted, now()).unwrap().id;
        conn.execute(
            "INSERT INTO recurring_occurrences (rule_id, occurrence_date, status, transaction_id, created_at, updated_at) VALUES (?1, '2026-01-31', 'posted', ?2, 'x', 'x')",
            [id, txn],
        )
        .unwrap();
        delete(&mut conn, id).unwrap();
        assert!(list(&conn).unwrap().is_empty());
        assert!(transactions::get(&conn, txn).is_ok());
        assert!(matches!(
            delete(&mut conn, id),
            Err(AppError::NotFound { .. })
        ));
        assert!(matches!(
            pause(&mut conn, id, true, now()),
            Err(AppError::NotFound { .. })
        ));
    }

    #[test]
    fn what_a_rule_uses_is_in_use() {
        let mut conn = db();
        let today = now().date_naive();
        let bank = accounts::create(
            &mut conn,
            accounts::NewAccount {
                kind: accounts::AccountKind::Bank,
                fields: accounts::AccountFields {
                    name: "Bank".into(),
                    opening_balance: Money(0),
                    credit_limit: None,
                    statement_day: None,
                    due_day: None,
                },
            },
            now(),
        )
        .unwrap()
        .id;
        let mut fields = rent("2026-01-31");
        fields.transaction.account_id = bank;
        create(&mut conn, fields, now()).unwrap();
        assert!(accounts::get(&conn, bank, today).unwrap().in_use);
        assert!(matches!(
            accounts::delete(&mut conn, bank, now()),
            Err(AppError::Conflict { .. })
        ));
        assert!(matches!(
            categories::delete(&mut conn, FOOD),
            Err(AppError::Conflict { .. })
        ));
        assert!(matches!(
            payment_methods::delete(&mut conn, CASH_METHOD),
            Err(AppError::Conflict { .. })
        ));
        assert!(
            settings::get(&conn).unwrap().decimals_locked,
            "a rule's amount locks the decimals"
        );
    }

    #[test]
    fn a_transfer_rule_puts_its_destination_in_use() {
        let mut conn = db();
        let today = now().date_naive();
        let savings = accounts::create(
            &mut conn,
            accounts::NewAccount {
                kind: accounts::AccountKind::Bank,
                fields: accounts::AccountFields {
                    name: "Savings".into(),
                    opening_balance: Money(0),
                    credit_limit: None,
                    statement_day: None,
                    due_day: None,
                },
            },
            now(),
        )
        .unwrap()
        .id;
        let mut fields = rent("2026-01-31");
        fields.transaction.kind = TransactionKind::Transfer;
        fields.transaction.to_account_id = Some(savings);
        fields.transaction.category_id = None;
        fields.transaction.payee = None;
        fields.transaction.payment_method_id = None;
        create(&mut conn, fields, now()).unwrap();
        assert!(accounts::get(&conn, savings, today).unwrap().in_use);
    }

    #[test]
    fn merging_categories_moves_their_rules() {
        let mut conn = db();
        let id = create(&mut conn, rent("2026-01-31"), now()).unwrap().id;
        categories::merge(&mut conn, FOOD, DINING, now()).unwrap();
        assert_eq!(rule(&conn, id).transaction.category_id, Some(DINING));
    }
}
