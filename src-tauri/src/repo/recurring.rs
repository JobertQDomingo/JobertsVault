use rusqlite::{Connection, OptionalExtension, params};

use super::transactions::Values as Template;

/// Whether any recurring rule exists at all.
pub fn any(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row("SELECT EXISTS (SELECT 1 FROM recurring_rules)", [], |row| {
        row.get(0)
    })
}

/// One `recurring_rules` row as stored (DATA_MODEL §4.1), with what its occurrences tell.
pub struct Row {
    pub id: i64,
    pub kind: String,
    pub amount: i64,
    pub account_id: i64,
    pub to_account_id: Option<i64>,
    pub category_id: Option<i64>,
    pub payee: Option<String>,
    pub payment_method_id: Option<i64>,
    pub note: Option<String>,
    pub frequency: String,
    pub interval: i64,
    pub start_date: String,
    pub end_date: Option<String>,
    pub max_occurrences: Option<i64>,
    pub next_index: i64,
    pub mode: String,
    pub paused: bool,
    /// How many dates it has processed: posted, pending or skipped.
    pub done: i64,
    /// The latest date it has processed.
    pub last_date: Option<String>,
}

/// The columns a create or an edit writes. The template's `date` is the start date.
pub struct Values<'a> {
    pub template: Template<'a>,
    pub frequency: &'a str,
    pub interval: i64,
    pub end_date: Option<&'a str>,
    pub max_occurrences: Option<i64>,
    pub mode: &'a str,
}

const SELECT: &str = "SELECT r.id, r.kind, r.amount, r.account_id, r.to_account_id, r.category_id,
            r.payee, r.payment_method_id, r.note, r.frequency, r.interval, r.start_date,
            r.end_date, r.max_occurrences, r.next_index, r.mode, r.paused,
            (SELECT count(*) FROM recurring_occurrences o WHERE o.rule_id = r.id),
            (SELECT max(o.occurrence_date) FROM recurring_occurrences o WHERE o.rule_id = r.id)
     FROM recurring_rules r";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(0)?,
        kind: r.get(1)?,
        amount: r.get(2)?,
        account_id: r.get(3)?,
        to_account_id: r.get(4)?,
        category_id: r.get(5)?,
        payee: r.get(6)?,
        payment_method_id: r.get(7)?,
        note: r.get(8)?,
        frequency: r.get(9)?,
        interval: r.get(10)?,
        start_date: r.get(11)?,
        end_date: r.get(12)?,
        max_occurrences: r.get(13)?,
        next_index: r.get(14)?,
        mode: r.get(15)?,
        paused: r.get(16)?,
        done: r.get(17)?,
        last_date: r.get(18)?,
    })
}

/// Every rule, oldest first.
pub fn all(conn: &Connection) -> rusqlite::Result<Vec<Row>> {
    conn.prepare(&format!("{SELECT} ORDER BY r.id"))?
        .query_map([], row)?
        .collect()
}

pub fn find(conn: &Connection, id: i64) -> rusqlite::Result<Option<Row>> {
    conn.query_row(&format!("{SELECT} WHERE r.id = ?1"), [id], row)
        .optional()
}

/// A new rule starts at its first date and isn't paused.
pub fn insert(conn: &Connection, v: &Values, now: &str) -> rusqlite::Result<i64> {
    let t = &v.template;
    conn.execute(
        "INSERT INTO recurring_rules (kind, amount, account_id, to_account_id, category_id, payee,
                                      payment_method_id, note, frequency, interval, start_date,
                                      end_date, max_occurrences, mode, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
        params![
            t.kind,
            t.amount,
            t.account_id,
            t.to_account_id,
            t.category_id,
            t.payee,
            t.payment_method_id,
            t.note,
            v.frequency,
            v.interval,
            t.date,
            v.end_date,
            v.max_occurrences,
            v.mode,
            now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(
    conn: &Connection,
    id: i64,
    v: &Values,
    next_index: i64,
    now: &str,
) -> rusqlite::Result<()> {
    let t = &v.template;
    conn.execute(
        "UPDATE recurring_rules
         SET kind = ?2, amount = ?3, account_id = ?4, to_account_id = ?5, category_id = ?6,
             payee = ?7, payment_method_id = ?8, note = ?9, frequency = ?10, interval = ?11,
             start_date = ?12, end_date = ?13, max_occurrences = ?14, mode = ?15,
             next_index = ?16, updated_at = ?17
         WHERE id = ?1",
        params![
            id,
            t.kind,
            t.amount,
            t.account_id,
            t.to_account_id,
            t.category_id,
            t.payee,
            t.payment_method_id,
            t.note,
            v.frequency,
            v.interval,
            t.date,
            v.end_date,
            v.max_occurrences,
            v.mode,
            next_index,
            now
        ],
    )?;
    Ok(())
}

pub fn set_paused(
    conn: &Connection,
    id: i64,
    paused: bool,
    next_index: i64,
    now: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE recurring_rules SET paused = ?2, next_index = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, paused, next_index, now],
    )?;
    Ok(())
}

/// Where processing has got to (DATA_MODEL §4.1).
pub fn set_next_index(conn: &Connection, id: i64, next_index: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE recurring_rules SET next_index = ?2 WHERE id = ?1",
        params![id, next_index],
    )?;
    Ok(())
}

/// Records `date` as processed, unless it already is: the UNIQUE key never lets a date be
/// created twice (REC-4). The new row's id, or `None` when the date was already there.
pub fn insert_occurrence(
    conn: &Connection,
    rule_id: i64,
    date: &str,
    status: &str,
    now: &str,
) -> rusqlite::Result<Option<i64>> {
    let added = conn.execute(
        "INSERT INTO recurring_occurrences (rule_id, occurrence_date, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)
         ON CONFLICT (rule_id, occurrence_date) DO NOTHING",
        params![rule_id, date, status, now],
    )?;
    Ok((added == 1).then(|| conn.last_insert_rowid()))
}

pub fn settle_occurrence(
    conn: &Connection,
    id: i64,
    status: &str,
    transaction_id: Option<i64>,
    now: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE recurring_occurrences SET status = ?2, transaction_id = ?3, updated_at = ?4
         WHERE id = ?1",
        params![id, status, transaction_id, now],
    )?;
    Ok(())
}

/// Its occurrences go with it (`ON DELETE CASCADE`); the transactions they created stay.
pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM recurring_rules WHERE id = ?1", [id])
}
