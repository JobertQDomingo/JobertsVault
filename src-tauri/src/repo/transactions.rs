use rusqlite::{Connection, OptionalExtension, ToSql, params};

/// Whether any transaction exists at all.
pub fn any(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row("SELECT EXISTS (SELECT 1 FROM transactions)", [], |row| {
        row.get(0)
    })
}

/// One `transactions` row as stored (DATA_MODEL §3.4).
pub struct Row {
    pub id: i64,
    pub kind: String,
    pub amount: i64,
    pub date: String,
    pub account_id: i64,
    pub to_account_id: Option<i64>,
    pub category_id: Option<i64>,
    pub payee: Option<String>,
    pub payment_method_id: Option<i64>,
    pub note: Option<String>,
}

/// The columns a create or an edit writes.
pub struct Values<'a> {
    pub kind: &'a str,
    pub amount: i64,
    pub date: &'a str,
    pub account_id: i64,
    pub to_account_id: Option<i64>,
    pub category_id: Option<i64>,
    pub payee: Option<&'a str>,
    pub payment_method_id: Option<i64>,
    pub note: Option<&'a str>,
}

const SELECT: &str = "SELECT id, kind, amount, date, account_id, to_account_id, category_id,
            payee, payment_method_id, note
     FROM transactions";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(0)?,
        kind: r.get(1)?,
        amount: r.get(2)?,
        date: r.get(3)?,
        account_id: r.get(4)?,
        to_account_id: r.get(5)?,
        category_id: r.get(6)?,
        payee: r.get(7)?,
        payment_method_id: r.get(8)?,
        note: r.get(9)?,
    })
}

pub fn find(conn: &Connection, id: i64) -> rusqlite::Result<Option<Row>> {
    conn.query_row(&format!("{SELECT} WHERE id = ?1"), [id], row)
        .optional()
}

pub fn insert(conn: &Connection, v: &Values, now: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO transactions (kind, amount, date, account_id, to_account_id, category_id,
                                   payee, payment_method_id, note, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![
            v.kind,
            v.amount,
            v.date,
            v.account_id,
            v.to_account_id,
            v.category_id,
            v.payee,
            v.payment_method_id,
            v.note,
            now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, v: &Values, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE transactions SET kind = ?2, amount = ?3, date = ?4, account_id = ?5,
                to_account_id = ?6, category_id = ?7, payee = ?8, payment_method_id = ?9,
                note = ?10, updated_at = ?11
         WHERE id = ?1",
        params![
            id,
            v.kind,
            v.amount,
            v.date,
            v.account_id,
            v.to_account_id,
            v.category_id,
            v.payee,
            v.payment_method_id,
            v.note,
            now
        ],
    )?;
    Ok(())
}

/// The list's filters (TXN-4). `None` means any. `search` is a LIKE pattern escaped with `\`.
#[derive(Clone, Copy)]
pub struct Filter<'a> {
    pub start: &'a str,
    pub end: &'a str,
    pub kind: Option<&'a str>,
    /// Matches either side, so an account's list includes incoming transfers.
    pub account_id: Option<i64>,
    pub category_id: Option<i64>,
    pub payment_method_id: Option<i64>,
    pub search: Option<&'a str>,
}

impl<'a> Filter<'a> {
    /// Every row dated `start` to `end`, with no other filter.
    pub fn dates(start: &'a str, end: &'a str) -> Self {
        Filter {
            start,
            end,
            kind: None,
            account_id: None,
            category_id: None,
            payment_method_id: None,
            search: None,
        }
    }

    fn values(&self) -> [&dyn ToSql; 7] {
        [
            &self.start,
            &self.end,
            &self.kind,
            &self.account_id,
            &self.category_id,
            &self.payment_method_id,
            &self.search,
        ]
    }
}

const WHERE: &str = r"WHERE date BETWEEN ?1 AND ?2
       AND (?3 IS NULL OR kind = ?3)
       AND (?4 IS NULL OR account_id = ?4 OR to_account_id = ?4)
       AND (?5 IS NULL OR category_id = ?5)
       AND (?6 IS NULL OR payment_method_id = ?6)
       AND (?7 IS NULL OR payee LIKE ?7 ESCAPE '\' OR note LIKE ?7 ESCAPE '\')";

/// The matching rows, newest first (TXN-3).
pub fn page(conn: &Connection, f: &Filter, limit: u32, offset: i64) -> rusqlite::Result<Vec<Row>> {
    let mut values = f.values().to_vec();
    values.extend([&limit as &dyn ToSql, &offset]);
    conn.prepare(&format!(
        "{SELECT} {WHERE} ORDER BY date DESC, id DESC LIMIT ?8 OFFSET ?9"
    ))?
    .query_map(&*values, row)?
    .collect()
}

/// The number of matching rows and their income and expense totals. Transfers count in neither.
pub fn totals(conn: &Connection, f: &Filter) -> rusqlite::Result<(i64, i64, i64)> {
    conn.query_row(
        &format!(
            "SELECT count(*),
                    COALESCE(SUM(CASE kind WHEN 'income' THEN amount END), 0),
                    COALESCE(SUM(CASE kind WHEN 'expense' THEN amount END), 0)
             FROM transactions {WHERE}"
        ),
        &f.values()[..],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
}

/// Income minus expense for each date with matching rows, newest first.
pub fn day_nets(conn: &Connection, f: &Filter) -> rusqlite::Result<Vec<(String, i64)>> {
    conn.prepare(&format!(
        "SELECT date, SUM(CASE kind WHEN 'income' THEN amount WHEN 'expense' THEN -amount ELSE 0 END)
         FROM transactions {WHERE} GROUP BY date ORDER BY date DESC"
    ))?
    .query_map(&f.values()[..], |r| Ok((r.get(0)?, r.get(1)?)))?
    .collect()
}

/// The income and expense totals of each date with matching rows, oldest first.
pub fn daily_totals(conn: &Connection, f: &Filter) -> rusqlite::Result<Vec<(String, i64, i64)>> {
    conn.prepare(&format!(
        "SELECT date,
                COALESCE(SUM(CASE kind WHEN 'income' THEN amount END), 0),
                COALESCE(SUM(CASE kind WHEN 'expense' THEN amount END), 0)
         FROM transactions {WHERE} GROUP BY date ORDER BY date"
    ))?
    .query_map(&f.values()[..], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
    .collect()
}

/// Each category's total over the matching rows, largest first, then by category id.
pub fn category_totals(conn: &Connection, f: &Filter) -> rusqlite::Result<Vec<(i64, i64)>> {
    conn.prepare(&format!(
        "SELECT category_id, SUM(amount) FROM transactions {WHERE} AND category_id IS NOT NULL
         GROUP BY category_id ORDER BY 2 DESC, 1"
    ))?
    .query_map(&f.values()[..], |r| Ok((r.get(0)?, r.get(1)?)))?
    .collect()
}

/// The payees on `kind` rows matching the LIKE `pattern`, one per payee ignoring ASCII case, most
/// recently used first. Each comes with its latest such row's spelling and category, the category
/// only while it isn't archived.
pub fn payees(
    conn: &Connection,
    kind: &str,
    pattern: &str,
    limit: u32,
) -> rusqlite::Result<Vec<(String, Option<i64>)>> {
    conn.prepare(
        "SELECT t.payee, CASE WHEN c.archived = 0 THEN t.category_id END
         FROM (SELECT payee, category_id, date, id,
                      ROW_NUMBER() OVER (PARTITION BY payee COLLATE NOCASE
                                         ORDER BY date DESC, id DESC) AS n
               FROM transactions
               WHERE kind = ?1 AND payee LIKE ?2 ESCAPE '\\') t
         LEFT JOIN categories c ON c.id = t.category_id
         WHERE t.n = 1
         ORDER BY t.date DESC, t.id DESC
         LIMIT ?3",
    )?
    .query_map(params![kind, pattern, limit], |r| {
        Ok((r.get(0)?, r.get(1)?))
    })?
    .collect()
}

/// The `limit` newest transactions dated `as_of` or earlier, newest first.
pub fn recent(conn: &Connection, as_of: &str, limit: u32) -> rusqlite::Result<Vec<Row>> {
    conn.prepare(&format!(
        "{SELECT} WHERE date <= ?1 ORDER BY date DESC, id DESC LIMIT ?2"
    ))?
    .query_map(params![as_of, limit], row)?
    .collect()
}

/// The source account of the latest transaction whose source is an active non-card account.
pub fn latest_non_card_source(conn: &Connection) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        "SELECT t.account_id FROM transactions t JOIN accounts a ON a.id = t.account_id
         WHERE a.archived = 0 AND a.kind <> 'credit_card'
         ORDER BY t.date DESC, t.id DESC LIMIT 1",
        [],
        |r| r.get(0),
    )
    .optional()
}

/// The number of rows deleted: 0 when there's no such transaction.
pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM transactions WHERE id = ?1", [id])
}
