use rusqlite::Connection;

/// Whether any account, archived or not, stores an amount: an opening balance or a credit limit.
pub fn any_with_amount(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM accounts WHERE opening_balance <> 0 OR credit_limit IS NOT NULL)",
        [],
        |row| row.get(0),
    )
}
