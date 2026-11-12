use chrono::{DateTime, Days, Local, NaiveDate};
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
    catch_up(&tx, self::stored(&tx, id)?, now)?;
    let saved = get(&tx, id)?;
    tx.commit()?;
    Ok(saved)
}

/// Changes future dates only (REC-5). A new frequency, interval or start date starts the
/// schedule again from the start date, which can't bring back a date already settled.
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
    let next_index = if rescheduled {
        let start = transactions::real_date(&fields.transaction.date)
            .ok_or_else(|| invalid("date", "Enter a real date"))?;
        if start < earliest_restart(&stored, now.date_naive())? {
            return Err(invalid(
                "date",
                "Choose a later start date. This rule already created or passed over the dates before it.",
            ));
        }
        0
    } else {
        // An edit never fills in the past: a rule that had ended and is given a later end date or
        // more times carries on from today (DATA_MODEL §4.1).
        let unbounded = Schedule {
            end: None,
            ..schedule(&stored)?
        };
        let from = index(&stored, stored.next_index)?;
        unbounded.first_on_or_after(from, now.date_naive()).into()
    };
    repo::recurring::update(&tx, id, &values(&fields), next_index, &stamp(now))?;
    catch_up(&tx, self::stored(&tx, id)?, now)?;
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
    catch_up(&tx, self::stored(&tx, id)?, now)?;
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

/// A date a Confirm rule queued, waiting for Post, Edit & post or Skip (REC-3, UI_FLOWS F10).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DueOccurrence {
    #[ts(type = "number")]
    pub id: i64,
    /// What Post creates: the rule's template as it is now, dated the day it fell due.
    pub transaction: TransactionFields,
}

/// The Due list (F10), oldest first.
pub fn list_due(conn: &Connection) -> Result<Vec<DueOccurrence>, AppError> {
    repo::recurring::pending(conn)?
        .into_iter()
        .map(|o| {
            let transaction = TransactionFields {
                date: o.date,
                ..fields_of(&stored(conn, o.rule_id)?)?
            };
            Ok(DueOccurrence {
                id: o.id,
                transaction,
            })
        })
        .collect()
}

/// One date a rule will post or queue in the next 7 days (DSH-7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpcomingDate {
    #[ts(type = "number")]
    pub rule_id: i64,
    /// Auto posts it on the day; Confirm queues it in the Due list.
    pub mode: RecurringMode,
    /// The rule's template as it is now, dated that day.
    pub transaction: TransactionFields,
}

/// Every date an active rule falls on after today and up to a week on, soonest first, then by
/// rule (DSH-7). Paused and ended rules have none. One-off future-dated transactions aren't here:
/// they're under Upcoming on the transaction list (TXN-8).
pub fn upcoming(conn: &Connection, today: NaiveDate) -> Result<Vec<UpcomingDate>, AppError> {
    let until = today
        .checked_add_days(Days::new(7))
        .unwrap_or(NaiveDate::MAX);
    let mut list = Vec::new();
    for row in repo::recurring::all(conn)? {
        if row.paused {
            continue;
        }
        let dates = schedule(&row)?.dates_between(
            index(&row, row.next_index)?,
            index(&row, row.done)?,
            today,
            until,
        );
        let rule = view(row)?;
        list.extend(dates.into_iter().map(|date| UpcomingDate {
            rule_id: rule.id,
            mode: rule.mode,
            transaction: TransactionFields {
                date: date.to_string(),
                ..rule.transaction.clone()
            },
        }));
    }
    list.sort_by(|a, b| (&a.transaction.date, a.rule_id).cmp(&(&b.transaction.date, b.rule_id)));
    Ok(list)
}

/// Post and Edit & post (F10): the date's transaction from the edited `fields`, or from the
/// rule's template dated the day it fell due. Either way it follows the transaction rules and may
/// keep the template's archived records (DATA_MODEL §4.2). The rule itself doesn't change.
pub fn post(
    conn: &mut Connection,
    id: i64,
    fields: Option<TransactionFields>,
    now: DateTime<Local>,
) -> Result<Transaction, AppError> {
    let tx = conn.transaction()?;
    let (occurrence, rule) = due(&tx, id)?;
    let mut posted = match fields {
        Some(fields) => fields,
        None => TransactionFields {
            date: occurrence.date,
            ..fields_of(&rule)?
        },
    };
    transactions::validate(&tx, &mut posted, Some(&template(&rule)?), now.date_naive())?;
    let stamp = stamp(now);
    let txn = repo::transactions::insert(&tx, &transactions::values(&posted), &stamp)?;
    repo::recurring::settle_occurrence(&tx, id, "posted", Some(txn), &stamp)?;
    let saved = transactions::get(&tx, txn)?;
    tx.commit()?;
    Ok(saved)
}

/// Skip (F10): the date is settled with no transaction, so processing never brings it back.
pub fn skip(conn: &mut Connection, id: i64, now: DateTime<Local>) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    due(&tx, id)?;
    repo::recurring::settle_occurrence(&tx, id, "skipped", None, &stamp(now))?;
    tx.commit()?;
    Ok(())
}

/// A date still in the Due list, and its rule. One already posted or skipped is a `Conflict`.
fn due(
    conn: &Connection,
    id: i64,
) -> Result<(repo::recurring::Occurrence, repo::recurring::Row), AppError> {
    let occurrence = repo::recurring::find_occurrence(conn, id)?
        .ok_or_else(|| not_found("recurring date", id))?;
    if occurrence.status != "pending" {
        return Err(AppError::Conflict {
            message: "This date was already posted or skipped".into(),
        });
    }
    let rule = stored(conn, occurrence.rule_id)?;
    Ok((occurrence, rule))
}

/// Startup step 8, and whenever the window regains focus (ARCHITECTURE §6, §8): posts or queues
/// every rule's dates due up to today. Each rule is one database transaction, so one that fails
/// doesn't hold back the rest. How many dates it posted or queued, or the first failure.
pub fn process_due(conn: &mut Connection, now: DateTime<Local>) -> Result<u32, AppError> {
    let mut processed = 0;
    let mut failed = None;
    for row in repo::recurring::all(conn)? {
        // A failure drops `tx`, which rolls back this rule only.
        let one = conn.transaction().map_err(AppError::from).and_then(|tx| {
            let n = catch_up(&tx, row, now)?;
            tx.commit()?;
            Ok(n)
        });
        match one {
            Ok(n) => processed += n,
            Err(err) => {
                failed.get_or_insert(err);
            }
        }
    }
    failed.map_or(Ok(processed), Err)
}

/// Posts (auto) or queues (confirm) the rule's dates up to today that it hasn't processed yet
/// (ARCHITECTURE §8). A date already recorded is passed over, so none is created twice (REC-4).
/// The transaction keeps the template's account, category and payment method even once they're
/// archived, as an edit of the rule may (DATA_MODEL §4.2). How many dates it processed.
fn catch_up(
    conn: &Connection,
    row: repo::recurring::Row,
    now: DateTime<Local>,
) -> Result<u32, AppError> {
    if row.paused {
        return Ok(0);
    }
    let id = row.id;
    let schedule = schedule(&row)?;
    let first = index(&row, row.next_index)?;
    let mut done = index(&row, row.done)?;
    let rule = view(row)?;
    let (status, auto) = match rule.mode {
        RecurringMode::Auto => ("posted", true),
        RecurringMode::Confirm => ("pending", false),
    };
    let stamp = stamp(now);
    let mut next = first;
    let mut processed = 0;
    while let Some(date) = schedule
        .next(next, done)
        .filter(|date| *date <= now.date_naive())
    {
        let mut posted = rule.transaction.clone();
        posted.date = date.to_string();
        if let Some(occurrence) =
            repo::recurring::insert_occurrence(conn, id, &posted.date, status, &stamp)?
        {
            if auto {
                let txn = repo::transactions::insert(conn, &transactions::values(&posted), &stamp)?;
                repo::recurring::settle_occurrence(conn, occurrence, status, Some(txn), &stamp)?;
            }
            done += 1;
            processed += 1;
        }
        next += 1;
    }
    if next != first {
        repo::recurring::set_next_index(conn, id, next.into())?;
    }
    Ok(processed)
}

/// The earliest start date a schedule edit may take (DATA_MODEL §4.1): after the last date
/// processed, and not before today or the rule's next date, whichever is earlier, so the dates a
/// resume passed over are never created.
fn earliest_restart(row: &repo::recurring::Row, today: NaiveDate) -> Result<NaiveDate, AppError> {
    let next = schedule(row)?.next(index(row, row.next_index)?, index(row, row.done)?);
    let floor = next.map_or(today, |next| next.min(today));
    let after_last = row
        .last_date
        .as_deref()
        .map(|last| transactions::real_date(last).ok_or_else(|| bad(row, "date")))
        .transpose()?
        .and_then(|last| last.succ_opt());
    Ok(after_last.map_or(floor, |after| after.max(floor)))
}

fn get(conn: &Connection, id: i64) -> Result<RecurringRule, AppError> {
    view(stored(conn, id)?)
}

fn stored(conn: &Connection, id: i64) -> Result<repo::recurring::Row, AppError> {
    repo::recurring::find(conn, id)?.ok_or_else(|| not_found("recurring rule", id))
}

/// The template follows the transaction rules, then the schedule's own rules (DATA_MODEL §6).
/// `stored` is the rule being edited.
fn validate(
    conn: &Connection,
    f: &mut RecurringRuleFields,
    stored: Option<&repo::recurring::Row>,
    today: NaiveDate,
) -> Result<(), AppError> {
    let kept = stored.map(template).transpose()?;
    transactions::validate(conn, &mut f.transaction, kept.as_ref(), today)?;
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

    fn field<T: std::fmt::Debug>(result: Result<T, AppError>) -> String {
        match result {
            Err(AppError::Validation { field, .. }) => field,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    /// `date` at the tests' time of day: the `now` a save or a run of processing gets.
    fn on(date: &str) -> DateTime<Local> {
        d(date)
            .and_time(now().time())
            .and_local_timezone(Local)
            .unwrap()
    }

    /// The rule's processed dates, oldest first: date, status and transaction.
    fn occurrences(conn: &Connection, id: i64) -> Vec<(String, String, Option<i64>)> {
        conn.prepare(
            "SELECT occurrence_date, status, transaction_id FROM recurring_occurrences
             WHERE rule_id = ?1 ORDER BY occurrence_date",
        )
        .unwrap()
        .query_map([id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
    }

    fn dates(conn: &Connection, id: i64) -> Vec<String> {
        occurrences(conn, id).into_iter().map(|o| o.0).collect()
    }

    fn transaction_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT count(*) FROM transactions", [], |r| r.get(0))
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

    fn rule(conn: &Connection, id: i64) -> RecurringRule {
        get(conn, id).unwrap()
    }

    #[test]
    fn upcoming_lists_each_active_rules_dates_in_the_next_7_days() {
        let mut conn = db();
        let today = on("2026-11-12");
        let monthly = create(&mut conn, rent("2026-11-15"), today).unwrap();
        let mut every_3 = rent("2026-11-10");
        every_3.frequency = Frequency::Daily;
        every_3.interval = 3;
        every_3.mode = RecurringMode::Confirm;
        let daily = create(&mut conn, every_3, today).unwrap();
        // Nov 10 is already in the Due list, so it isn't coming up.
        assert_eq!(dates(&conn, daily.id), ["2026-11-10"]);
        let mut weekly = rent("2026-11-14");
        weekly.frequency = Frequency::Weekly;
        let paused = create(&mut conn, weekly, today).unwrap();
        pause(&mut conn, paused.id, true, today).unwrap();
        create(&mut conn, rent("2026-11-20"), today).unwrap();
        let shown: Vec<_> = upcoming(&conn, d("2026-11-12"))
            .unwrap()
            .into_iter()
            .map(|u| (u.transaction.date, u.rule_id, u.mode))
            .collect();
        let confirm = RecurringMode::Confirm;
        assert_eq!(
            shown,
            [
                ("2026-11-13".to_string(), daily.id, confirm),
                ("2026-11-15".to_string(), monthly.id, RecurringMode::Auto),
                ("2026-11-16".to_string(), daily.id, confirm),
                ("2026-11-19".to_string(), daily.id, confirm),
            ]
        );
        let first = &upcoming(&conn, d("2026-11-12")).unwrap()[0];
        assert_eq!(first.transaction.amount, Money(1_500_000));
        assert_eq!(first.transaction.payee.as_deref(), Some("Landlord"));
    }

    #[test]
    fn a_new_rule_is_saved_trimmed_and_starts_on_its_start_date() {
        let mut conn = db();
        let saved = create(&mut conn, rent("2026-11-30"), now()).unwrap();
        assert_eq!(saved.transaction.payee.as_deref(), Some("Landlord"));
        assert_eq!(saved.transaction.note, None);
        assert_eq!(saved.next_date, Some(d("2026-11-30")));
        assert!(!saved.paused);
        assert_eq!(saved.mode, RecurringMode::Auto);
        assert_eq!(list(&conn).unwrap(), [saved]);
    }

    #[test]
    fn a_start_date_may_be_in_the_past_or_the_future() {
        let mut conn = db();
        let later = create(&mut conn, rent("2027-03-15"), now()).unwrap();
        assert_eq!(later.next_date, Some(d("2027-03-15")));
        // now() is Nov 3, 2026: Jun 1, 2025 to Nov 1, 2026 are filled in on save.
        let past = create(&mut conn, rent("2025-06-01"), now()).unwrap();
        assert_eq!(past.next_date, Some(d("2026-12-01")));
        assert_eq!(dates(&conn, past.id).len(), 18);
        assert_eq!(transaction_count(&conn), 18);
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
        let feb_28 = on("2026-02-28");
        let id = create(&mut conn, rent("2026-01-31"), feb_28).unwrap().id;
        let mut edit = rent("2026-01-31");
        edit.transaction.amount = Money(1_600_000);
        edit.mode = RecurringMode::Confirm;
        edit.max_occurrences = Some(5);
        let saved = update(&mut conn, id, edit, feb_28).unwrap();
        assert_eq!(saved.next_date, Some(d("2026-03-31")), "the place is kept");
        assert_eq!(saved.mode, RecurringMode::Confirm);
        assert_eq!(occurrences(&conn, id)[1].1, "posted", "Feb 28 stays posted");
    }

    #[test]
    fn a_schedule_edit_starts_again_after_the_dates_already_processed() {
        let mut conn = db();
        let feb_28 = on("2026-02-28");
        let mut fields = rent("2026-01-31");
        fields.max_occurrences = Some(4);
        let id = create(&mut conn, fields, feb_28).unwrap().id;
        assert_eq!(dates(&conn, id), ["2026-01-31", "2026-02-28"]);

        let mut too_early = rent("2026-02-28");
        too_early.max_occurrences = Some(4);
        assert_eq!(field(update(&mut conn, id, too_early, feb_28)), "date");

        // A 31st whose next date was Mar 31 moves to every 2 months from Mar 31, not the 28th.
        let mut every_2 = rent("2026-03-31");
        every_2.interval = 2;
        every_2.max_occurrences = Some(4);
        let saved = update(&mut conn, id, every_2, feb_28).unwrap();
        assert_eq!(saved.next_date, Some(d("2026-03-31")));
        process_due(&mut conn, on("2026-03-31")).unwrap();
        assert_eq!(rule(&conn, id).next_date, Some(d("2026-05-31")));
        process_due(&mut conn, on("2026-05-31")).unwrap();
        assert_eq!(
            rule(&conn, id).next_date,
            None,
            "4 times in all, the 2 before the edit included"
        );
        assert_eq!(dates(&conn, id).len(), 4);
    }

    #[test]
    fn a_schedule_edit_never_brings_back_dates_passed_over_while_paused() {
        let mut conn = db();
        let id = create(&mut conn, rent("2026-01-31"), on("2026-01-31"))
            .unwrap()
            .id;
        pause(&mut conn, id, true, now()).unwrap();
        pause(&mut conn, id, false, now()).unwrap();
        let mut back_in_march = rent("2026-03-31");
        back_in_march.interval = 2;
        assert_eq!(field(update(&mut conn, id, back_in_march, now())), "date");
        // now() is Nov 3, 2026: starting again today posts today on save.
        let mut from_today = rent("2026-11-03");
        from_today.interval = 2;
        assert_eq!(
            update(&mut conn, id, from_today, now()).unwrap().next_date,
            Some(d("2027-01-03"))
        );
        assert_eq!(dates(&conn, id), ["2026-01-31", "2026-11-03"]);
    }

    #[test]
    fn extending_a_rule_that_ended_carries_on_from_today() {
        let mut conn = db();
        // now() is Nov 3, 2026: both end after Feb 28.
        let mut twice = rent("2026-01-31");
        twice.max_occurrences = Some(2);
        let twice = create(&mut conn, twice, now()).unwrap().id;
        let mut until = rent("2026-01-31");
        until.end_date = Some("2026-03-15".into());
        let until = create(&mut conn, until, now()).unwrap().id;
        assert_eq!(rule(&conn, twice).next_date, None);

        let mut more = rent("2026-01-31");
        more.max_occurrences = Some(12);
        let mut later = rent("2026-01-31");
        later.end_date = Some("2026-12-31".into());
        for (id, edit) in [(twice, more), (until, later)] {
            let saved = update(&mut conn, id, edit, now()).unwrap();
            assert_eq!(saved.next_date, Some(d("2026-11-30")), "Mar–Oct never come");
            assert_eq!(dates(&conn, id), ["2026-01-31", "2026-02-28"]);
        }
    }

    #[test]
    fn a_rule_not_started_yet_can_start_earlier_but_not_in_the_past() {
        let mut conn = db();
        let id = create(&mut conn, rent("2027-01-31"), now()).unwrap().id;
        assert_eq!(
            update(&mut conn, id, rent("2026-12-15"), now())
                .unwrap()
                .next_date,
            Some(d("2026-12-15"))
        );
        assert_eq!(
            field(update(&mut conn, id, rent("2026-10-01"), now())),
            "date"
        );
    }

    #[test]
    fn resuming_skips_the_dates_it_was_paused_for() {
        let mut conn = db();
        let id = create(&mut conn, rent("2026-01-31"), on("2026-01-31"))
            .unwrap()
            .id;
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
        assert_eq!(dates(&conn, id), ["2026-01-31"]);
    }

    #[test]
    fn a_resumed_rule_still_counts_only_processed_dates() {
        let mut conn = db();
        let mut fields = rent("2026-01-31");
        fields.max_occurrences = Some(2);
        let id = create(&mut conn, fields, on("2026-01-31")).unwrap().id;
        pause(&mut conn, id, true, now()).unwrap();
        let resumed = pause(&mut conn, id, false, now()).unwrap();
        assert_eq!(resumed.next_date, Some(d("2026-11-30")), "one time left");
    }

    #[test]
    fn deleting_a_rule_keeps_its_transactions() {
        let mut conn = db();
        let id = create(&mut conn, rent("2026-01-31"), on("2026-01-31"))
            .unwrap()
            .id;
        let txn = occurrences(&conn, id)[0].2.unwrap();
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
        let bank = bank(&mut conn, "Bank");
        // Not due yet, so only the rule uses them.
        let mut fields = rent("2026-12-31");
        fields.transaction.account_id = bank;
        create(&mut conn, fields, now()).unwrap();
        assert!(accounts::get(&conn, bank, today).unwrap().in_use);
        assert!(matches!(
            accounts::delete(&mut conn, bank, now()),
            Err(AppError::Conflict { .. })
        ));
        let food = categories::list(&conn).unwrap().categories;
        let food = food.iter().find(|c| c.id == FOOD).unwrap();
        assert!(food.in_use && food.transaction_count == 0);
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
        let savings = bank(&mut conn, "Savings");
        let mut fields = rent("2026-12-31");
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

    #[test]
    fn processing_posts_each_date_due_up_to_today_once() {
        let mut conn = db();
        let id = create(&mut conn, rent("2026-01-31"), on("2026-01-01"))
            .unwrap()
            .id;
        assert!(dates(&conn, id).is_empty(), "not due yet");

        assert_eq!(process_due(&mut conn, on("2026-04-30")).unwrap(), 4);
        assert_eq!(
            process_due(&mut conn, on("2026-04-30")).unwrap(),
            0,
            "a second run creates nothing"
        );
        assert_eq!(process_due(&mut conn, on("2026-05-30")).unwrap(), 0);
        assert_eq!(process_due(&mut conn, on("2026-05-31")).unwrap(), 1);

        let done = occurrences(&conn, id);
        let expected = [
            "2026-01-31",
            "2026-02-28",
            "2026-03-31",
            "2026-04-30",
            "2026-05-31",
        ];
        assert_eq!(dates(&conn, id), expected);
        for (date, status, txn) in done {
            assert_eq!(status, "posted");
            let txn = transactions::get(&conn, txn.unwrap()).unwrap();
            assert_eq!(txn.date, d(&date));
            assert_eq!(
                (txn.kind, txn.amount, txn.account_id, txn.category_id),
                (TransactionKind::Expense, Money(1_500_000), CASH, Some(FOOD))
            );
            assert_eq!(txn.payee.as_deref(), Some("Landlord"));
            assert_eq!((txn.payment_method_id, txn.note), (Some(CASH_METHOD), None));
        }
        assert_eq!(transaction_count(&conn), 5);
        let cash = accounts::get(&conn, CASH, d("2026-05-31")).unwrap();
        assert_eq!(cash.balance, Money(-7_500_000));
        assert_eq!(rule(&conn, id).next_date, Some(d("2026-06-30")));
    }

    #[test]
    fn a_confirm_rule_queues_its_dates_without_posting() {
        let mut conn = db();
        let mut fields = rent("2026-10-31");
        fields.mode = RecurringMode::Confirm;
        let id = create(&mut conn, fields, now()).unwrap().id;
        assert_eq!(
            occurrences(&conn, id),
            [("2026-10-31".into(), "pending".into(), None)]
        );
        assert_eq!(transaction_count(&conn), 0);
        assert_eq!(rule(&conn, id).next_date, Some(d("2026-11-30")));
    }

    #[test]
    fn a_date_already_recorded_is_never_created_again() {
        let mut conn = db();
        let id = create(&mut conn, rent("2026-01-31"), on("2026-01-31"))
            .unwrap()
            .id;
        // Deleting a posted transaction keeps its date recorded (DATA_MODEL §4.2).
        let first = occurrences(&conn, id)[0].2.unwrap();
        transactions::delete(&mut conn, first).unwrap();
        // A date ahead that's already recorded: the UNIQUE key passes it over.
        conn.execute(
            "INSERT INTO recurring_occurrences (rule_id, occurrence_date, status, created_at, updated_at) VALUES (?1, '2026-02-28', 'skipped', 'x', 'x')",
            [id],
        )
        .unwrap();

        assert_eq!(process_due(&mut conn, on("2026-03-31")).unwrap(), 1);
        let done: Vec<_> = occurrences(&conn, id)
            .into_iter()
            .map(|(date, status, txn)| (date, status, txn.is_some()))
            .collect();
        assert_eq!(
            done,
            [
                ("2026-01-31".into(), "posted".into(), false),
                ("2026-02-28".into(), "skipped".into(), false),
                ("2026-03-31".into(), "posted".into(), true),
            ]
        );
        assert_eq!(transaction_count(&conn), 1);
        assert_eq!(rule(&conn, id).next_date, Some(d("2026-04-30")));
    }

    #[test]
    fn a_paused_rule_is_not_processed() {
        let mut conn = db();
        let id = create(&mut conn, rent("2026-01-31"), on("2026-01-01"))
            .unwrap()
            .id;
        pause(&mut conn, id, true, on("2026-01-01")).unwrap();
        assert_eq!(process_due(&mut conn, on("2026-03-31")).unwrap(), 0);
        assert!(dates(&conn, id).is_empty());
    }

    #[test]
    fn processing_stops_where_the_rule_ends() {
        let mut conn = db();
        let mut until = rent("2026-01-31");
        until.end_date = Some("2026-03-15".into());
        let until = create(&mut conn, until, on("2026-01-01")).unwrap().id;
        let mut times = rent("2026-01-31");
        times.max_occurrences = Some(3);
        let times = create(&mut conn, times, on("2026-01-01")).unwrap().id;

        assert_eq!(process_due(&mut conn, on("2026-12-31")).unwrap(), 5);
        assert_eq!(dates(&conn, until), ["2026-01-31", "2026-02-28"]);
        assert_eq!(dates(&conn, times).len(), 3);
        assert_eq!(rule(&conn, until).next_date, None);
        assert_eq!(rule(&conn, times).next_date, None);
    }

    #[test]
    fn saving_or_resuming_a_rule_processes_it_at_once() {
        let mut conn = db();
        let id = create(&mut conn, rent("2026-12-01"), now()).unwrap().id;
        assert!(dates(&conn, id).is_empty());
        // now() is Nov 3, 2026.
        let saved = update(&mut conn, id, rent("2026-11-03"), now()).unwrap();
        assert_eq!(
            saved.next_date,
            Some(d("2026-12-03")),
            "Nov 3 posted on save"
        );
        pause(&mut conn, id, true, now()).unwrap();
        let resumed = pause(&mut conn, id, false, on("2026-12-03")).unwrap();
        assert_eq!(
            resumed.next_date,
            Some(d("2027-01-03")),
            "Dec 3 posted on resume"
        );
        assert_eq!(dates(&conn, id), ["2026-11-03", "2026-12-03"]);
        assert_eq!(transaction_count(&conn), 2);
    }

    #[test]
    fn a_rule_still_posts_with_what_it_uses_archived() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank");
        let mut fields = rent("2026-01-31");
        fields.transaction.account_id = bank;
        let id = create(&mut conn, fields, on("2026-01-01")).unwrap().id;
        accounts::archive(&mut conn, bank, false, now()).unwrap();
        categories::archive(&mut conn, FOOD, now()).unwrap();
        payment_methods::archive(&mut conn, CASH_METHOD, now()).unwrap();

        assert_eq!(process_due(&mut conn, on("2026-02-28")).unwrap(), 2);
        let txn = transactions::get(&conn, occurrences(&conn, id)[1].2.unwrap()).unwrap();
        assert_eq!(
            (txn.account_id, txn.category_id, txn.payment_method_id),
            (bank, Some(FOOD), Some(CASH_METHOD))
        );
    }

    #[test]
    fn a_broken_rule_does_not_hold_back_the_others() {
        let mut conn = db();
        let jan = on("2026-01-01");
        let broken = create(&mut conn, rent("2026-01-31"), jan).unwrap().id;
        let fine = create(&mut conn, rent("2026-01-31"), jan).unwrap().id;
        conn.execute(
            "UPDATE recurring_rules SET start_date = 'soon' WHERE id = ?1",
            [broken],
        )
        .unwrap();
        assert!(matches!(
            process_due(&mut conn, on("2026-02-28")),
            Err(AppError::Database { .. })
        ));
        assert_eq!(dates(&conn, fine).len(), 2);
    }

    /// A Confirm rule that queued Jan 31 and Feb 28.
    fn queued(conn: &mut Connection) -> i64 {
        let mut fields = rent("2026-01-31");
        fields.mode = RecurringMode::Confirm;
        create(conn, fields, on("2026-02-28")).unwrap().id
    }

    fn conflict<T: std::fmt::Debug>(result: Result<T, AppError>) {
        assert!(
            matches!(result, Err(AppError::Conflict { .. })),
            "{result:?}"
        );
    }

    #[test]
    fn the_due_list_holds_each_queued_date_oldest_first() {
        let mut conn = db();
        let rent_id = queued(&mut conn);
        let mut water = rent("2026-02-10");
        water.mode = RecurringMode::Confirm;
        water.transaction.amount = Money(3_240);
        create(&mut conn, water, on("2026-02-28")).unwrap();
        create(&mut conn, rent("2026-02-01"), on("2026-02-28")).unwrap(); // auto: never due

        let due = list_due(&conn).unwrap();
        let dated: Vec<_> = due
            .iter()
            .map(|o| (o.transaction.date.as_str(), o.transaction.amount))
            .collect();
        assert_eq!(
            dated,
            [
                ("2026-01-31", Money(1_500_000)),
                ("2026-02-10", Money(3_240)),
                ("2026-02-28", Money(1_500_000)),
            ]
        );
        assert_eq!(
            due[0].transaction,
            TransactionFields {
                date: "2026-01-31".into(),
                ..rule(&conn, rent_id).transaction
            },
            "the template, dated the day it fell due"
        );
    }

    #[test]
    fn post_creates_the_template_on_its_due_date() {
        let mut conn = db();
        let id = queued(&mut conn);
        let first = list_due(&conn).unwrap()[0].id;
        let posted = post(&mut conn, first, None, now()).unwrap();
        assert_eq!(posted.date, d("2026-01-31"));
        assert_eq!(
            (posted.amount, posted.category_id, posted.payee.as_deref()),
            (Money(1_500_000), Some(FOOD), Some("Landlord"))
        );
        assert_eq!(
            occurrences(&conn, id)[0],
            ("2026-01-31".into(), "posted".into(), Some(posted.id))
        );
        assert_eq!(list_due(&conn).unwrap().len(), 1);
        let cash = accounts::get(&conn, CASH, now().date_naive()).unwrap();
        assert_eq!(cash.balance, Money(-1_500_000));
    }

    #[test]
    fn edit_and_post_saves_the_edited_fields_for_that_date_only() {
        let mut conn = db();
        let id = queued(&mut conn);
        let first = list_due(&conn).unwrap()[0].clone();
        let mut edited = first.transaction.clone();
        edited.amount = Money(0);
        assert_eq!(
            field(post(&mut conn, first.id, Some(edited.clone()), now())),
            "amount"
        );
        assert_eq!(transaction_count(&conn), 0, "a refused edit posts nothing");

        edited.amount = Money(1_450_000);
        edited.date = "2026-02-02".into();
        edited.note = Some("  paid late ".into());
        let posted = post(&mut conn, first.id, Some(edited), now()).unwrap();
        assert_eq!(posted.date, d("2026-02-02"));
        assert_eq!(posted.amount, Money(1_450_000));
        assert_eq!(posted.note.as_deref(), Some("paid late"));
        assert_eq!(
            occurrences(&conn, id)[0].0,
            "2026-01-31",
            "still its due date"
        );
        assert_eq!(
            rule(&conn, id).transaction.amount,
            Money(1_500_000),
            "the rule is unchanged"
        );
    }

    #[test]
    fn a_due_date_may_keep_the_rule_s_archived_records_but_not_pick_others() {
        let mut conn = db();
        queued(&mut conn);
        categories::archive(&mut conn, FOOD, now()).unwrap();
        categories::archive(&mut conn, DINING, now()).unwrap();
        let [first, second] = list_due(&conn).unwrap().try_into().unwrap();
        assert_eq!(
            post(&mut conn, first.id, None, now()).unwrap().category_id,
            Some(FOOD)
        );
        let mut dining = second.transaction.clone();
        dining.category_id = Some(DINING);
        assert!(matches!(
            post(&mut conn, second.id, Some(dining), now()),
            Err(AppError::Validation { field, .. }) if field == "categoryId"
        ));
    }

    #[test]
    fn skip_settles_a_date_without_a_transaction_and_it_never_comes_back() {
        let mut conn = db();
        let id = queued(&mut conn);
        let first = list_due(&conn).unwrap()[0].id;
        skip(&mut conn, first, now()).unwrap();
        assert_eq!(
            occurrences(&conn, id)[0],
            ("2026-01-31".into(), "skipped".into(), None)
        );
        process_due(&mut conn, on("2026-02-28")).unwrap();
        assert_eq!(list_due(&conn).unwrap().len(), 1, "only Feb 28 is left");
        assert_eq!(transaction_count(&conn), 0);
    }

    #[test]
    fn only_a_date_still_due_can_be_posted_or_skipped() {
        let mut conn = db();
        queued(&mut conn);
        let [first, second] = list_due(&conn).unwrap().try_into().unwrap();
        post(&mut conn, first.id, None, now()).unwrap();
        conflict(post(&mut conn, first.id, None, now()));
        conflict(skip(&mut conn, first.id, now()));
        skip(&mut conn, second.id, now()).unwrap();
        conflict(post(&mut conn, second.id, None, now()));
        assert_eq!(transaction_count(&conn), 1);
        assert!(matches!(
            skip(&mut conn, 999, now()),
            Err(AppError::NotFound { .. })
        ));
    }
}
