use rusqlite::{Transaction, params};

const PAYMENT_METHODS: [&str; 6] = [
    "Cash",
    "Debit Card",
    "Credit Card",
    "Bank Transfer",
    "E-wallet",
    "Other",
];

/// Name, icon, color.
const INCOME: [(&str, &str, &str); 7] = [
    ("Salary", "briefcase", "cat-12"),
    ("Bonus", "award", "cat-9"),
    ("Freelance/Side income", "laptop", "cat-3"),
    ("Interest", "percent", "cat-7"),
    ("Gifts received", "gift", "cat-4"),
    ("Refund", "rotate-ccw", "cat-10"),
    ("Other income", "circle-plus", "cat-11"),
];

const EXPENSE: [(&str, &str, &str); 17] = [
    ("Food & Groceries", "shopping-cart", "cat-2"),
    ("Dining Out", "utensils", "cat-6"),
    ("Transportation", "car", "cat-1"),
    ("Rent/Housing", "house", "cat-11"),
    ("Utilities", "zap", "cat-7"),
    ("Phone", "smartphone", "cat-5"),
    ("Shopping", "shopping-bag", "cat-4"),
    ("Health & Medical", "heart-pulse", "cat-8"),
    ("Insurance", "shield", "cat-3"),
    ("Entertainment", "film", "cat-10"),
    ("Subscriptions", "repeat", "cat-9"),
    ("Personal Care", "sparkles", "cat-12"),
    ("Education", "graduation-cap", "cat-1"),
    ("Gifts & Donations", "heart-handshake", "cat-4"),
    ("Travel", "plane", "cat-3"),
    ("Fees & Charges", "receipt", "cat-5"),
    ("Other expense", "ellipsis", "cat-11"),
];

/// The first-launch data (DATA_MODEL §7). It runs inside migration 0001's transaction, so a new
/// database gets it together with its schema, and an existing one never gets it again.
///
/// Frozen with 0001: it runs before every later migration, so it may only use 0001's columns.
/// Changing the seed for new installs means a new migration, never an edit here.
pub fn seed(tx: &Transaction, now: &str) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO accounts (name, kind, opening_balance, created_at, updated_at) VALUES ('Cash', 'cash', 0, ?1, ?1)",
        [now],
    )?;
    let mut method = tx.prepare(
        "INSERT INTO payment_methods (name, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
    )?;
    for (order, name) in (0_i64..).zip(PAYMENT_METHODS) {
        method.execute(params![name, order, now])?;
    }
    let mut category = tx.prepare(
        "INSERT INTO categories (name, kind, color, icon, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
    )?;
    for (kind, list) in [("income", &INCOME[..]), ("expense", &EXPENSE[..])] {
        for (order, (name, icon, color)) in (0_i64..).zip(list) {
            category.execute(params![name, kind, color, icon, order, now])?;
        }
    }
    Ok(())
}

/// Name, icon: the expense categories Phase 4 adds (DATA_MODEL §7, PRD Appendix A).
const PHASE_4_EXPENSE: [(&str, &str); 7] = [
    ("Loan", "coins"),
    ("SSS", "shield"),
    ("PhilHealth", "heart-pulse"),
    ("Pag-IBIG", "house"),
    ("Withholding tax", "receipt"),
    ("Salary loan", "banknote"),
    ("Other deductions", "ellipsis"),
];

/// The palette as migration 0003 knew it. 0003 keeps its own copy of the palette and of the
/// least-used rule, so a later change to `domain::category` can't change what it does.
const PALETTE_0003: [&str; 12] = [
    "cat-1", "cat-2", "cat-3", "cat-4", "cat-5", "cat-6", "cat-7", "cat-8", "cat-9", "cat-10",
    "cat-11", "cat-12",
];

/// The least-used palette key in `used`, the lowest number on a tie (DATA_MODEL §7).
fn least_used_0003(used: &[String]) -> &'static str {
    // min_by_key keeps the first of equal minimums, which is the lowest number.
    PALETTE_0003
        .into_iter()
        .min_by_key(|key| used.iter().filter(|color| color == key).count())
        .unwrap_or(PALETTE_0003[0])
}

/// Adds the Phase 4 expense categories a database doesn't have yet, inside migration 0003's
/// transaction, so new and existing databases both get them. One already there under the same
/// name (ignoring case) is kept as it is. Each new one goes last and takes the least-used color
/// of the active expense categories at that point, like a category the user adds (DATA_MODEL §7).
///
/// Frozen with 0003, as `seed` is with 0001.
pub fn phase_4_categories(tx: &Transaction, now: &str) -> rusqlite::Result<()> {
    let mut colors: Vec<String> = tx
        .prepare("SELECT color FROM categories WHERE kind = 'expense' AND archived = 0")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for (name, icon) in PHASE_4_EXPENSE {
        // `name` is COLLATE NOCASE, so this ignores case.
        let exists: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM categories WHERE kind = 'expense' AND name = ?1)",
            [name],
            |row| row.get(0),
        )?;
        if exists {
            continue;
        }
        let color = least_used_0003(&colors);
        tx.execute(
            "INSERT INTO categories (name, kind, color, icon, sort_order, created_at, updated_at) \
             SELECT ?1, 'expense', ?2, ?3, coalesce(max(sort_order) + 1, 0), ?4, ?4 \
             FROM categories WHERE kind = 'expense'",
            params![name, color, icon, now],
        )?;
        colors.push(color.to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::db::{migrations, open, prepare};
    use crate::state::AppState;
    use crate::test_support::TempDir;
    use chrono::{TimeZone, Utc};
    use rusqlite::Connection;
    use std::path::Path;

    fn seeded() -> Connection {
        let now = Utc.with_ymd_and_hms(2026, 10, 26, 9, 30, 0).unwrap();
        let mut conn = open(Path::new(":memory:")).unwrap();
        prepare(
            &mut conn,
            &migrations(now),
            Path::new("unused"),
            crate::test_support::now(),
        )
        .unwrap();
        conn
    }

    fn strings(conn: &Connection, sql: &str) -> Vec<String> {
        conn.prepare(sql)
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |row| row.get(0)).unwrap()
    }

    #[test]
    fn a_new_database_gets_the_seed_data() {
        let conn = seeded();
        let cash: (String, String, i64, String, String) = conn
            .query_row(
                "SELECT name, kind, opening_balance, created_at, updated_at FROM accounts",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            cash,
            (
                "Cash".into(),
                "cash".into(),
                0,
                "2026-10-26T09:30:00Z".into(),
                "2026-10-26T09:30:00Z".into()
            )
        );
        assert_eq!(
            strings(
                &conn,
                "SELECT name FROM payment_methods ORDER BY sort_order"
            ),
            [
                "Cash",
                "Debit Card",
                "Credit Card",
                "Bank Transfer",
                "E-wallet",
                "Other"
            ]
        );
        assert_eq!(
            count(
                &conn,
                "SELECT count(*) FROM categories WHERE kind = 'income'"
            ),
            7
        );
        assert_eq!(
            count(
                &conn,
                "SELECT count(*) FROM categories WHERE kind = 'expense'"
            ),
            17 + 7,
            "the 17 from 0001, then Loan and the payslip deductions from 0003"
        );
        assert_eq!(
            count(&conn, "SELECT count(*) FROM categories WHERE archived = 1"),
            0
        );

        let category = |name: &str| -> (String, String, String, i64) {
            conn.query_row(
                "SELECT kind, icon, color, sort_order FROM categories WHERE name = ?1",
                [name],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap()
        };
        assert_eq!(
            category("Salary"),
            ("income".into(), "briefcase".into(), "cat-12".into(), 0)
        );
        assert_eq!(
            category("Refund"),
            ("income".into(), "rotate-ccw".into(), "cat-10".into(), 5)
        );
        assert_eq!(
            category("Food & Groceries"),
            ("expense".into(), "shopping-cart".into(), "cat-2".into(), 0)
        );
        assert_eq!(
            category("Other expense"),
            ("expense".into(), "ellipsis".into(), "cat-11".into(), 16)
        );
    }

    #[test]
    fn the_seed_runs_only_on_first_launch() {
        let dir = TempDir::new("seed-once");
        for _launch in 0..2 {
            let state = AppState::open(
                dir.path().to_path_buf(),
                &migrations(crate::test_support::now().to_utc()),
                crate::test_support::now(),
            );
            let conn = state.conn().unwrap();
            assert_eq!(count(&conn, "SELECT count(*) FROM accounts"), 1);
            // 7 income and 17 expense from 0001, and the 7 expense ones 0003 adds.
            assert_eq!(count(&conn, "SELECT count(*) FROM categories"), 31);
            assert_eq!(count(&conn, "SELECT count(*) FROM payment_methods"), 6);
        }
    }

    #[test]
    fn every_seed_color_is_a_palette_key() {
        let palette: Vec<String> = (1..=12).map(|n| format!("cat-{n}")).collect();
        for (name, _, color) in super::INCOME.iter().chain(&super::EXPENSE) {
            assert!(palette.iter().any(|key| key == color), "{name} has {color}");
        }
    }

    #[test]
    fn every_seed_icon_is_curated() {
        let seeded = super::INCOME
            .iter()
            .chain(&super::EXPENSE)
            .map(|(name, icon, _)| (name, icon));
        let phase_4 = super::PHASE_4_EXPENSE
            .iter()
            .map(|(name, icon)| (name, icon));
        for (name, icon) in seeded.chain(phase_4) {
            assert!(
                crate::domain::category::ICONS.contains(icon),
                "{name} has {icon}"
            );
        }
    }

    #[test]
    fn the_12_common_expense_categories_each_have_their_own_color() {
        let mut colors: Vec<&str> = super::EXPENSE[..12]
            .iter()
            .map(|(_, _, color)| *color)
            .collect();
        colors.sort_unstable();
        colors.dedup();
        assert_eq!(
            colors.len(),
            12,
            "DATA_MODEL §7: only the 5 rarely used ones share"
        );
    }
}
