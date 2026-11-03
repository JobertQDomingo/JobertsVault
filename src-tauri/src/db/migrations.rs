use rusqlite_migration::{M, Migrations};

/// Every schema change, oldest first. A released migration is never edited (DATA_MODEL §8).
pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(include_str!(
        "../../migrations/0001_initial.sql"
    ))])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{open, prepare};
    use rusqlite::Connection;
    use std::path::Path;

    const NOW: &str = "2026-10-26T09:30:00Z";

    fn migrated() -> Connection {
        let mut conn = open(Path::new(":memory:")).unwrap();
        prepare(&mut conn, &migrations()).unwrap();
        conn
    }

    /// Accounts 1 (Cash) and 2 (Bank), category 1 (Salary, income), payment method 1 (Cash).
    fn add_basics(conn: &Connection) {
        for sql in [
            "INSERT INTO accounts (name, kind, created_at, updated_at) VALUES ('Cash', 'cash', ?1, ?1), ('Bank', 'bank', ?1, ?1)",
            "INSERT INTO categories (name, kind, color, icon, created_at, updated_at) VALUES ('Salary', 'income', 'cat-12', 'briefcase', ?1, ?1)",
            "INSERT INTO payment_methods (name, created_at, updated_at) VALUES ('Cash', ?1, ?1)",
        ] {
            conn.execute(sql, [NOW]).unwrap();
        }
    }

    #[test]
    fn the_migrations_are_valid() {
        assert!(migrations().validate().is_ok());
    }

    #[test]
    fn the_schema_matches_data_model_section_3() {
        let conn = migrated();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1);

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
                "categories",
                "payment_methods",
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
        conn.execute(
            "INSERT INTO categories (name, kind, color, icon, created_at, updated_at) VALUES ('Salary', 'income', 'cat-12', 'briefcase', ?1, ?1)",
            [NOW],
        )
        .unwrap();
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
}
