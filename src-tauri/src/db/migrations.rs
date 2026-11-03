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
        conn.execute_batch(&format!(
            "INSERT INTO accounts (name, kind, created_at, updated_at) VALUES ('Cash', 'cash', '{NOW}', '{NOW}'), ('Bank', 'bank', '{NOW}', '{NOW}');
             INSERT INTO categories (name, kind, color, icon, created_at, updated_at) VALUES ('Salary', 'income', 'cat-12', 'briefcase', '{NOW}', '{NOW}');"
        ))
        .unwrap();
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
}
