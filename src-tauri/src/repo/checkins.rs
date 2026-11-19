//! The `checkins` table (DATA_MODEL §4.9), and the few figures the check-in and the Vault level
//! need that no other query gives: everyday spending (§5.22), logged days and payslip
//! deductions (§5.20).

use rusqlite::{Connection, params};

/// One `checkins` row as stored. Dates are `YYYY-MM-DD` text.
pub struct Row {
    pub id: i64,
    pub kind: String,
    pub period_start: String,
    pub period_end: String,
    pub completed_on: String,
    pub level: i64,
    /// `stat_cashflow` to `stat_habit`, in §5.20's order.
    pub stats: [Option<i64>; 6],
    pub debt_free_on: Option<String>,
    pub focus_kind: Option<String>,
    pub focus_category_id: Option<i64>,
    pub focus_account_id: Option<i64>,
    pub focus_loan_payment_id: Option<i64>,
    pub focus_target: Option<i64>,
    pub focus_start: Option<String>,
    pub focus_end: Option<String>,
    pub focus_result: Option<String>,
    pub focus_actual: Option<i64>,
}

const SELECT: &str = "SELECT id, kind, period_start, period_end, completed_on, level,
            stat_cashflow, stat_bills, stat_card, stat_debt, stat_savings, stat_habit, debt_free_on,
            focus_kind, focus_category_id, focus_account_id, focus_loan_payment_id, focus_target,
            focus_start, focus_end, focus_result, focus_actual
     FROM checkins";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(0)?,
        kind: r.get(1)?,
        period_start: r.get(2)?,
        period_end: r.get(3)?,
        completed_on: r.get(4)?,
        level: r.get(5)?,
        stats: [
            r.get(6)?,
            r.get(7)?,
            r.get(8)?,
            r.get(9)?,
            r.get(10)?,
            r.get(11)?,
        ],
        debt_free_on: r.get(12)?,
        focus_kind: r.get(13)?,
        focus_category_id: r.get(14)?,
        focus_account_id: r.get(15)?,
        focus_loan_payment_id: r.get(16)?,
        focus_target: r.get(17)?,
        focus_start: r.get(18)?,
        focus_end: r.get(19)?,
        focus_result: r.get(20)?,
        focus_actual: r.get(21)?,
    })
}

/// Every row, the latest `completed_on` first, then the latest id.
pub fn all(conn: &Connection) -> rusqlite::Result<Vec<Row>> {
    conn.prepare(&format!("{SELECT} ORDER BY completed_on DESC, id DESC"))?
        .query_map([], row)?
        .collect()
}

/// Whether a focus stores an amount, which locks `currency_decimals` (DATA_MODEL §1).
pub fn any_amount(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM checkins
                        WHERE focus_kind IN ('category_cap', 'spending_cap', 'card_level'))",
        [],
        |r| r.get(0),
    )
}

/// The earliest transaction's date, of any kind.
pub fn earliest_transaction(conn: &Connection) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT MIN(date) FROM transactions", [], |r| r.get(0))
}

/// The dates from `start` to `end` with at least one transaction of any kind (§5.20, §5.22).
pub fn logged_days(conn: &Connection, start: &str, end: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT count(DISTINCT date) FROM transactions WHERE date BETWEEN ?1 AND ?2",
        params![start, end],
        |r| r.get(0),
    )
}

/// The payslip deductions dated `start` to `end`: expenses with `payslip_id` set (§4.7).
pub fn deductions(conn: &Connection, start: &str, end: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(amount), 0) FROM transactions
         WHERE kind = 'expense' AND payslip_id IS NOT NULL AND date BETWEEN ?1 AND ?2",
        params![start, end],
        |r| r.get(0),
    )
}

/// Everyday spending (§5.22): the expenses dated `start` to `end` that pay no loan payment, that
/// no recurring date posted and that aren't payslip deductions. `category` none means every
/// category.
pub fn everyday_spending(
    conn: &Connection,
    start: &str,
    end: &str,
    category: Option<i64>,
) -> rusqlite::Result<i64> {
    conn.query_row(
        &format!(
            "SELECT COALESCE(SUM(t.amount), 0) {EVERYDAY} AND (?3 IS NULL OR t.category_id = ?3)"
        ),
        params![start, end, category],
        |r| r.get(0),
    )
}

const EVERYDAY: &str = "FROM transactions t
     WHERE t.kind = 'expense' AND t.date BETWEEN ?1 AND ?2 AND t.payslip_id IS NULL
       AND NOT EXISTS (SELECT 1 FROM loan_payments p WHERE p.transaction_id = t.id)
       AND NOT EXISTS (SELECT 1 FROM recurring_occurrences o WHERE o.transaction_id = t.id)";

/// Everyday spending dated `start` to `end` for each category that has some.
pub fn everyday_by_category(
    conn: &Connection,
    start: &str,
    end: &str,
) -> rusqlite::Result<Vec<(i64, i64)>> {
    conn.prepare(&format!(
        "SELECT t.category_id, SUM(t.amount) {EVERYDAY} AND t.category_id IS NOT NULL
         GROUP BY t.category_id"
    ))?
    .query_map(params![start, end], |r| Ok((r.get(0)?, r.get(1)?)))?
    .collect()
}

/// A focus as a new row stores it.
pub struct NewFocus<'a> {
    pub kind: &'a str,
    pub category_id: Option<i64>,
    pub account_id: Option<i64>,
    pub loan_payment_id: Option<i64>,
    pub target: Option<i64>,
    pub start: &'a str,
    pub end: &'a str,
}

/// The columns a finished check-in writes (§4.9).
pub struct NewRow<'a> {
    pub kind: &'a str,
    pub period_start: &'a str,
    pub period_end: &'a str,
    pub completed_on: &'a str,
    pub level: i64,
    pub stats: [Option<i64>; 6],
    pub debt_free_on: Option<&'a str>,
    pub focus: Option<NewFocus<'a>>,
}

pub fn insert(conn: &Connection, v: &NewRow, now: &str) -> rusqlite::Result<i64> {
    let f = v.focus.as_ref();
    conn.execute(
        "INSERT INTO checkins (kind, period_start, period_end, completed_on, level,
             stat_cashflow, stat_bills, stat_card, stat_debt, stat_savings, stat_habit, debt_free_on,
             focus_kind, focus_category_id, focus_account_id, focus_loan_payment_id, focus_target,
             focus_start, focus_end, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
                 ?19, ?20, ?20)",
        params![
            v.kind,
            v.period_start,
            v.period_end,
            v.completed_on,
            v.level,
            v.stats[0],
            v.stats[1],
            v.stats[2],
            v.stats[3],
            v.stats[4],
            v.stats[5],
            v.debt_free_on,
            f.map(|f| f.kind),
            f.and_then(|f| f.category_id),
            f.and_then(|f| f.account_id),
            f.and_then(|f| f.loan_payment_id),
            f.and_then(|f| f.target),
            f.map(|f| f.start),
            f.map(|f| f.end),
            now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Stores a focus's grade, once: a row already graded is left as it is (§4.9).
pub fn set_grade(
    conn: &Connection,
    id: i64,
    result: &str,
    actual: Option<i64>,
    now: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE checkins SET focus_result = ?2, focus_actual = ?3, updated_at = ?4
         WHERE id = ?1 AND focus_result IS NULL",
        params![id, result, actual, now],
    )?;
    Ok(())
}
