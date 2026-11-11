use chrono::{DateTime, Utc};
use rusqlite::Transaction;
use rusqlite_migration::{HookResult, M, Migrations};

use super::seed;

/// Every schema change, oldest first. A released migration is never edited (DATA_MODEL §8).
/// `now` stamps the seed data that 0001 adds to a new database.
pub fn migrations(now: DateTime<Utc>) -> Migrations<'static> {
    let stamp = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    Migrations::new(vec![
        M::up_with_hook(
            include_str!("../../migrations/0001_initial.sql"),
            move |tx: &Transaction| -> HookResult { Ok(seed::seed(tx, &stamp)?) },
        ),
        M::up(include_str!("../../migrations/0002_planning.sql")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{open, prepare};
    use chrono::TimeZone;
    use rusqlite::Connection;
    use std::path::Path;

    const NOW: &str = "2026-10-26T09:30:00Z";

    fn migrated() -> Connection {
        let mut conn = open(Path::new(":memory:")).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 10, 26, 9, 30, 0).unwrap();
        prepare(
            &mut conn,
            &migrations(now),
            Path::new("unused"),
            crate::test_support::now(),
        )
        .unwrap();
        conn
    }

    /// Adds account 2 (Bank). The seed already holds account 1 (Cash), category 1 (Salary,
    /// income) and payment method 1 (Cash).
    fn add_basics(conn: &Connection) {
        conn.execute(
            "INSERT INTO accounts (name, kind, created_at, updated_at) VALUES ('Bank', 'bank', ?1, ?1)",
            [NOW],
        )
        .unwrap();
    }

    #[test]
    fn the_migrations_are_valid() {
        assert!(
            migrations(crate::test_support::now().to_utc())
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn the_schema_matches_data_model_sections_3_and_4() {
        let conn = migrated();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);

        let strict_tables: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_list WHERE schema = 'main' AND strict = 1 ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            strict_tables,
            [
                "accounts",
                "budgets",
                "categories",
                "goals",
                "payment_methods",
                "recurring_occurrences",
                "recurring_rules",
                "settings",
                "transactions"
            ]
        );

        let indexes: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_schema WHERE type = 'index' AND name LIKE 'idx_txn_%' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            indexes,
            [
                "idx_txn_account_date",
                "idx_txn_category_date",
                "idx_txn_date",
                "idx_txn_payee",
                "idx_txn_to_account"
            ]
        );
    }

    #[test]
    fn a_credit_card_needs_its_card_fields() {
        let conn = migrated();
        let card = conn.execute(
            "INSERT INTO accounts (name, kind, created_at, updated_at) VALUES ('Visa', 'credit_card', ?1, ?1)",
            [NOW],
        );
        assert!(card.is_err());
        let complete = conn.execute(
            "INSERT INTO accounts (name, kind, opening_balance, credit_limit, statement_day, due_day, created_at, updated_at) VALUES ('Visa', 'credit_card', -500000, 5000000, 15, 5, ?1, ?1)",
            [NOW],
        );
        assert!(complete.is_ok(), "{complete:?}");
    }

    #[test]
    fn a_transaction_needs_a_real_account() {
        let conn = migrated();
        let orphan = conn.execute(
            "INSERT INTO transactions (kind, amount, date, account_id, category_id, created_at, updated_at) VALUES ('income', 100, '2026-10-26', 99, 1, ?1, ?1)",
            [NOW],
        );
        assert!(orphan.is_err(), "foreign keys must be enforced");
    }

    #[test]
    fn a_transfer_has_no_category() {
        let conn = migrated();
        add_basics(&conn);
        let with_category = conn.execute(
            "INSERT INTO transactions (kind, amount, date, account_id, to_account_id, category_id, created_at, updated_at) VALUES ('transfer', 100, '2026-10-26', 1, 2, 1, ?1, ?1)",
            [NOW],
        );
        assert!(with_category.is_err());
        let plain = conn.execute(
            "INSERT INTO transactions (kind, amount, date, account_id, to_account_id, created_at, updated_at) VALUES ('transfer', 100, '2026-10-26', 1, 2, ?1, ?1)",
            [NOW],
        );
        assert!(plain.is_ok(), "{plain:?}");
    }

    #[test]
    fn the_checks_reject_rows_that_break_the_rules() {
        let conn = migrated();
        add_basics(&conn);
        let txn = "INSERT INTO transactions (kind, amount, date, account_id, to_account_id, category_id, payee, payment_method_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)";
        // kind, amount, date, account, to_account, category, payee, payment method
        type Row = (
            &'static str,
            i64,
            &'static str,
            i64,
            Option<i64>,
            Option<i64>,
            Option<&'static str>,
            Option<i64>,
        );
        let rejected: [Row; 7] = [
            ("income", 0, "2026-10-26", 1, None, Some(1), None, None),
            ("income", 100, "26-10-2026", 1, None, Some(1), None, None),
            ("income", 100, "2026-10-26", 1, None, None, None, None),
            ("income", 100, "2026-10-26", 1, Some(2), Some(1), None, None),
            ("transfer", 100, "2026-10-26", 1, Some(1), None, None, None),
            (
                "transfer",
                100,
                "2026-10-26",
                1,
                Some(2),
                None,
                Some("Corner Cafe"),
                None,
            ),
            (
                "transfer",
                100,
                "2026-10-26",
                1,
                Some(2),
                None,
                None,
                Some(1),
            ),
        ];
        let valid = conn.execute(
            txn,
            rusqlite::params![
                "income",
                100,
                "2026-10-26",
                1,
                None::<i64>,
                Some(1),
                Some("Corner Cafe"),
                Some(1),
                NOW
            ],
        );
        assert!(valid.is_ok(), "the statement itself must work: {valid:?}");
        for row @ (kind, amount, date, account, to_account, category, payee, method) in rejected {
            let result = conn.execute(
                txn,
                rusqlite::params![
                    kind, amount, date, account, to_account, category, payee, method, NOW
                ],
            );
            assert!(result.is_err(), "accepted {row:?}");
        }
        let non_card_limit = conn.execute(
            "INSERT INTO accounts (name, kind, credit_limit, created_at, updated_at) VALUES ('GCash', 'ewallet', 100, ?1, ?1)",
            [NOW],
        );
        assert!(non_card_limit.is_err());
    }

    #[test]
    fn an_account_in_use_cannot_be_deleted() {
        let conn = migrated();
        add_basics(&conn);
        conn.execute(
            "INSERT INTO transactions (kind, amount, date, account_id, category_id, created_at, updated_at) VALUES ('income', 100, '2026-10-26', 1, 1, ?1, ?1)",
            [NOW],
        )
        .unwrap();
        assert!(
            conn.execute("DELETE FROM accounts WHERE id = 1", [])
                .is_err()
        );
        assert!(
            conn.execute("DELETE FROM categories WHERE id = 1", [])
                .is_err()
        );
        assert_eq!(
            conn.execute("DELETE FROM accounts WHERE id = 2", [])
                .unwrap(),
            1,
            "unused ones can go"
        );
    }

    /// Adds rule 1: a monthly expense of 100 from Cash, in category 8 (the first expense one).
    fn add_rule(conn: &Connection) {
        conn.execute(
            "INSERT INTO recurring_rules (kind, amount, account_id, category_id, frequency, start_date, mode, created_at, updated_at) VALUES ('expense', 100, 1, 8, 'monthly', '2026-01-31', 'auto', ?1, ?1)",
            [NOW],
        )
        .unwrap();
    }

    #[test]
    fn a_rule_checks_its_template_like_a_transaction() {
        let conn = migrated();
        add_basics(&conn);
        let rule = "INSERT INTO recurring_rules (kind, amount, account_id, to_account_id, category_id, payee, frequency, interval, mode, start_date, created_at, updated_at) VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8, '2026-01-31', ?9, ?9)";
        // kind, amount, to_account, category, payee, frequency, interval, mode
        type Row = (
            &'static str,
            i64,
            Option<i64>,
            Option<i64>,
            Option<&'static str>,
            &'static str,
            i64,
            &'static str,
        );
        let valid: Row = ("transfer", 100, Some(2), None, None, "monthly", 1, "auto");
        let rejected: [Row; 7] = [
            ("expense", 0, None, Some(8), None, "monthly", 1, "auto"),
            ("expense", 100, None, None, None, "monthly", 1, "auto"),
            ("transfer", 100, Some(1), None, None, "monthly", 1, "auto"),
            (
                "transfer",
                100,
                Some(2),
                None,
                Some("Landlord"),
                "monthly",
                1,
                "auto",
            ),
            ("expense", 100, None, Some(8), None, "hourly", 1, "auto"),
            ("expense", 100, None, Some(8), None, "monthly", 0, "auto"),
            ("expense", 100, None, Some(8), None, "monthly", 1, "ask"),
        ];
        for row @ (kind, amount, to_account, category, payee, frequency, interval, mode) in
            std::iter::once(valid).chain(rejected)
        {
            let result = conn.execute(
                rule,
                rusqlite::params![
                    kind, amount, to_account, category, payee, frequency, interval, mode, NOW
                ],
            );
            assert_eq!(result.is_ok(), row == valid, "{row:?}: {result:?}");
        }
    }

    #[test]
    fn a_rule_date_is_processed_once_and_its_rows_go_with_it() {
        let conn = migrated();
        add_rule(&conn);
        let txn = "INSERT INTO transactions (kind, amount, date, account_id, category_id, created_at, updated_at) VALUES ('expense', 100, ?1, 1, 8, ?2, ?2)";
        let occurrence = "INSERT INTO recurring_occurrences (rule_id, occurrence_date, status, transaction_id, created_at, updated_at) VALUES (1, ?1, ?2, ?3, ?4, ?4)";
        conn.execute(txn, ["2026-01-31", NOW]).unwrap();
        conn.execute(
            occurrence,
            rusqlite::params!["2026-01-31", "posted", Some(1), NOW],
        )
        .unwrap();
        assert!(
            conn.execute(
                occurrence,
                rusqlite::params!["2026-01-31", "pending", None::<i64>, NOW]
            )
            .is_err(),
            "the same date can never be created twice (REC-4)"
        );
        assert!(
            conn.execute(
                occurrence,
                rusqlite::params!["2026-02-28", "done", None::<i64>, NOW]
            )
            .is_err()
        );

        conn.execute("DELETE FROM transactions WHERE id = 1", [])
            .unwrap();
        let kept: Option<i64> = conn
            .query_row(
                "SELECT transaction_id FROM recurring_occurrences WHERE rule_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(kept, None, "deleting the transaction keeps the occurrence");

        conn.execute(txn, ["2026-02-28", NOW]).unwrap();
        conn.execute(
            occurrence,
            rusqlite::params!["2026-02-28", "posted", conn.last_insert_rowid(), NOW],
        )
        .unwrap();
        conn.execute("DELETE FROM recurring_rules WHERE id = 1", [])
            .unwrap();
        let left: (i64, i64) = conn
            .query_row(
                "SELECT (SELECT count(*) FROM recurring_occurrences), (SELECT count(*) FROM transactions)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            left,
            (0, 1),
            "deleting a rule deletes its occurrences, not its transactions"
        );
    }

    #[test]
    fn a_category_or_account_in_a_plan_cannot_be_deleted() {
        let conn = migrated();
        add_basics(&conn);
        add_rule(&conn);
        conn.execute(
            "INSERT INTO budgets (category_id, period, amount, created_at, updated_at) VALUES (9, 'monthly', 100, ?1, ?1)",
            [NOW],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO goals (name, target_amount, account_id, created_at, updated_at) VALUES ('Trip', 100, 2, ?1, ?1)",
            [NOW],
        )
        .unwrap();
        for delete in [
            "DELETE FROM categories WHERE id = 8",
            "DELETE FROM categories WHERE id = 9",
            "DELETE FROM accounts WHERE id = 1",
            "DELETE FROM accounts WHERE id = 2",
        ] {
            assert!(conn.execute(delete, []).is_err(), "{delete}");
        }
    }

    #[test]
    fn budgets_are_one_per_category_and_period() {
        let conn = migrated();
        let budget = "INSERT INTO budgets (category_id, period, amount, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)";
        // category, period, amount, accepted
        let rows: [(Option<i64>, &str, i64, bool); 7] = [
            (Some(8), "monthly", 100, true),
            (Some(8), "weekly", 100, true),
            (Some(8), "monthly", 200, false),
            (None, "monthly", 100, true),
            (None, "monthly", 200, false),
            (Some(9), "daily", 100, false),
            (Some(9), "monthly", 0, false),
        ];
        for row @ (category, period, amount, accepted) in rows {
            let result = conn.execute(budget, rusqlite::params![category, period, amount, NOW]);
            assert_eq!(result.is_ok(), accepted, "{row:?}: {result:?}");
        }
    }

    #[test]
    fn goal_names_are_unique_ignoring_case() {
        let conn = migrated();
        let goal = "INSERT INTO goals (name, target_amount, account_id, created_at, updated_at) VALUES (?1, ?2, 1, ?3, ?3)";
        assert!(
            conn.execute(goal, rusqlite::params!["Trip", 100, NOW])
                .is_ok()
        );
        assert!(
            conn.execute(goal, rusqlite::params!["trip", 100, NOW])
                .is_err()
        );
        assert!(
            conn.execute(goal, rusqlite::params!["Laptop", 0, NOW])
                .is_err()
        );
    }
}
