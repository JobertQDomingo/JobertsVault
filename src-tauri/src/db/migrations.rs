use chrono::{DateTime, Utc};
use rusqlite::Transaction;
use rusqlite_migration::{HookResult, M, Migrations};

use super::seed;

/// Every schema change, oldest first. A released migration is never edited (DATA_MODEL §8).
/// `now` stamps the seed data that 0001 adds to a new database, and the categories 0003 adds.
pub fn migrations(now: DateTime<Utc>) -> Migrations<'static> {
    Migrations::new(migration_list(now))
}

/// The same migrations as a list, so a test can add one after the real ones.
pub fn migration_list(now: DateTime<Utc>) -> Vec<M<'static>> {
    let stamp = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let stamp_0003 = stamp.clone();
    vec![
        M::up_with_hook(
            include_str!("../../migrations/0001_initial.sql"),
            move |tx: &Transaction| -> HookResult { Ok(seed::seed(tx, &stamp)?) },
        ),
        M::up(include_str!("../../migrations/0002_planning.sql")),
        M::up_with_hook(
            include_str!("../../migrations/0003_loans_payroll.sql"),
            move |tx: &Transaction| -> HookResult {
                Ok(seed::phase_4_categories(tx, &stamp_0003)?)
            },
        ),
        M::up(include_str!("../../migrations/0004_card_spend_levels.sql")),
        M::up(include_str!("../../migrations/0005_checkins.sql")),
        M::up(include_str!("../../migrations/0006_hmo_category.sql")),
    ]
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
        assert_eq!(version, 6);

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
                "checkins",
                "goals",
                "loan_payments",
                "loans",
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
        let loan_index: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE type = 'index' AND name = 'idx_loan_payments_loan'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(loan_index, 1);
        let checkin_index: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE type = 'index' AND name = 'idx_checkins_completed'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(checkin_index, 1);
        assert_eq!(
            indexes,
            [
                "idx_txn_account_date",
                "idx_txn_category_date",
                "idx_txn_date",
                "idx_txn_payee",
                "idx_txn_payslip",
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

    #[test]
    fn a_loan_needs_a_real_account_and_category_and_a_new_one_its_date() {
        let conn = migrated();
        let loan = "INSERT INTO loans (name, account_id, category_id, is_new, amount_borrowed, borrowed_on, added_on, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '2026-11-13', ?7, ?7)";
        // name, account, category, is_new, amount borrowed, date borrowed, accepted
        type Row = (
            &'static str,
            i64,
            i64,
            i64,
            Option<i64>,
            Option<&'static str>,
            bool,
        );
        let rows: [Row; 9] = [
            ("E-wallet loan", 1, 8, 0, None, None, true),
            (
                "Bank loan",
                1,
                8,
                1,
                Some(500_000),
                Some("2026-11-13"),
                true,
            ),
            ("e-wallet LOAN", 1, 8, 0, None, None, false),
            ("No date", 1, 8, 1, None, None, false),
            ("No account", 99, 8, 0, None, None, false),
            ("No category", 1, 99, 0, None, None, false),
            ("Odd flag", 1, 8, 2, None, None, false),
            ("Nothing borrowed", 1, 8, 0, Some(0), None, false),
            ("US date", 1, 8, 1, None, Some("11/13/2026"), false),
        ];
        for row @ (name, account, category, is_new, borrowed, borrowed_on, accepted) in rows {
            let result = conn.execute(
                loan,
                rusqlite::params![name, account, category, is_new, borrowed, borrowed_on, NOW],
            );
            assert_eq!(result.is_ok(), accepted, "{row:?}: {result:?}");
        }
    }

    /// Adds loan 1 (paid from Cash, in category 8) with payments 1 (Nov 20) and 2 (Dec 20), and
    /// transaction 1, an expense that can pay one of them.
    fn add_loan(conn: &Connection) {
        conn.execute(
            "INSERT INTO loans (name, account_id, category_id, is_new, added_on, created_at, updated_at) VALUES ('E-wallet loan', 1, 8, 0, '2026-11-13', ?1, ?1)",
            [NOW],
        )
        .unwrap();
        for date in ["2026-11-20", "2026-12-20"] {
            conn.execute(
                "INSERT INTO loan_payments (loan_id, due_date, amount, created_at, updated_at) VALUES (1, ?1, 100000, ?2, ?2)",
                [date, NOW],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO transactions (kind, amount, date, account_id, category_id, created_at, updated_at) VALUES ('expense', 100000, '2026-11-20', 1, 8, ?1, ?1)",
            [NOW],
        )
        .unwrap();
    }

    #[test]
    fn a_loan_payment_is_paid_by_one_transaction_or_paid_before_never_both() {
        let conn = migrated();
        add_loan(&conn);
        let link = "UPDATE loan_payments SET transaction_id = ?1, paid_before = ?2 WHERE id = ?3";
        assert!(conn.execute(link, rusqlite::params![Some(1), 0, 1]).is_ok());
        assert!(
            conn.execute(link, rusqlite::params![Some(1), 0, 2])
                .is_err(),
            "one transaction pays one payment"
        );
        assert!(
            conn.execute(link, rusqlite::params![Some(1), 1, 1])
                .is_err(),
            "a paid-before payment has no transaction"
        );
        assert!(
            conn.execute(link, rusqlite::params![None::<i64>, 2, 2])
                .is_err()
        );
        let payment = "INSERT INTO loan_payments (loan_id, due_date, amount, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)";
        for (loan, date, amount) in [
            (1, "2027-01-20", 0),
            (1, "20-01-2027", 100),
            (9, "2027-01-20", 100),
        ] {
            assert!(
                conn.execute(payment, rusqlite::params![loan, date, amount, NOW])
                    .is_err(),
                "{loan} {date} {amount}"
            );
        }

        conn.execute("DELETE FROM transactions WHERE id = 1", [])
            .unwrap();
        let unpaid: Option<i64> = conn
            .query_row(
                "SELECT transaction_id FROM loan_payments WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            unpaid, None,
            "deleting the expense makes the payment unpaid again"
        );

        for delete in [
            "DELETE FROM accounts WHERE id = 1",
            "DELETE FROM categories WHERE id = 8",
        ] {
            assert!(
                conn.execute(delete, []).is_err(),
                "a loan puts it in use: {delete}"
            );
        }
        conn.execute("DELETE FROM loans WHERE id = 1", []).unwrap();
        let left: i64 = conn
            .query_row("SELECT count(*) FROM loan_payments", [], |row| row.get(0))
            .unwrap();
        assert_eq!(left, 0, "a loan's payments go with it");
    }

    #[test]
    fn a_payslip_goes_with_its_gross_row() {
        let conn = migrated();
        let txn = "INSERT INTO transactions (kind, amount, date, account_id, category_id, payslip_id, created_at, updated_at) VALUES (?1, ?2, '2026-11-15', 1, ?3, ?4, ?5, ?5)";
        conn.execute(
            txn,
            rusqlite::params!["income", 1_200_000, 1, None::<i64>, NOW],
        )
        .unwrap();
        conn.execute("UPDATE transactions SET payslip_id = id WHERE id = 1", [])
            .unwrap();
        conn.execute(txn, rusqlite::params!["expense", 50_000, 8, Some(1), NOW])
            .unwrap();
        conn.execute(
            txn,
            rusqlite::params!["expense", 2_000, 8, None::<i64>, NOW],
        )
        .unwrap();
        assert!(
            conn.execute(txn, rusqlite::params!["expense", 2_000, 8, Some(99), NOW])
                .is_err(),
            "a payslip row points at a real gross row"
        );

        conn.execute("DELETE FROM transactions WHERE id = 1", [])
            .unwrap();
        let left: Vec<i64> = conn
            .prepare("SELECT id FROM transactions ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(left, [3], "the deduction goes with its gross row");
    }

    #[test]
    fn a_rule_is_a_payday_only_when_marked() {
        let conn = migrated();
        add_rule(&conn);
        let payday: i64 = conn
            .query_row("SELECT is_payday FROM recurring_rules", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(payday, 0);
        assert!(
            conn.execute("UPDATE recurring_rules SET is_payday = 1", [])
                .is_ok()
        );
        assert!(
            conn.execute("UPDATE recurring_rules SET is_payday = 2", [])
                .is_err()
        );
    }

    /// The expense categories with an id above `after`: name, icon, color and sort order.
    fn expense_categories_after(
        conn: &Connection,
        after: i64,
    ) -> Vec<(String, String, String, i64)> {
        conn.prepare(
            "SELECT name, icon, color, sort_order FROM categories WHERE kind = 'expense' AND id > ?1 ORDER BY id",
        )
        .unwrap()
        .query_map([after], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
    }

    fn owned(rows: &[(&str, &str, &str, i64)]) -> Vec<(String, String, String, i64)> {
        rows.iter()
            .map(|(a, b, c, d)| ((*a).into(), (*b).into(), (*c).into(), *d))
            .collect()
    }

    #[test]
    fn a_new_database_gets_the_phase_4_categories_after_the_others() {
        let conn = migrated();
        // 24 seeded categories come first.
        assert_eq!(
            expense_categories_after(&conn, 24),
            owned(&[
                ("Loan", "coins", "cat-2", 17),
                ("SSS", "shield", "cat-6", 18),
                ("PhilHealth", "heart-pulse", "cat-7", 19),
                ("Pag-IBIG", "house", "cat-8", 20),
                ("Withholding tax", "receipt", "cat-9", 21),
                ("Salary loan", "banknote", "cat-10", 22),
                ("Other deductions", "ellipsis", "cat-12", 23),
                // 0006: every color is now used twice, so the tie goes to the lowest number.
                ("HMO", "pill", "cat-1", 24),
            ]),
            "DATA_MODEL §7: each takes the least-used color of the active expense categories"
        );
    }

    #[test]
    fn an_existing_database_keeps_a_category_it_already_has() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        let all = migrations(Utc.with_ymd_and_hms(2026, 10, 26, 9, 30, 0).unwrap());
        all.to_version(&mut conn, 2).unwrap();
        let category = "INSERT INTO categories (name, kind, color, icon, archived, created_at, updated_at) VALUES (?1, ?2, ?3, 'coins', ?4, ?5, ?5)";
        // The user's own "loan", archived, and an income category that shares a deduction's name.
        conn.execute(
            category,
            rusqlite::params!["loan", "expense", "cat-5", 1, NOW],
        )
        .unwrap();
        conn.execute(
            category,
            rusqlite::params!["SSS", "income", "cat-1", 0, NOW],
        )
        .unwrap();

        all.to_latest(&mut conn).unwrap();

        let loans: Vec<(String, String, i64)> = conn
            .prepare("SELECT name, color, archived FROM categories WHERE kind = 'expense' AND name = 'Loan'")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            loans,
            [("loan".to_owned(), "cat-5".to_owned(), 1)],
            "the user's own Loan is kept as it is, and no second one is added"
        );
        // After the 24 seeded ones and the user's two (ids 25 and 26).
        assert_eq!(
            expense_categories_after(&conn, 26),
            owned(&[
                ("SSS", "shield", "cat-2", 17),
                ("PhilHealth", "heart-pulse", "cat-6", 18),
                ("Pag-IBIG", "house", "cat-7", 19),
                ("Withholding tax", "receipt", "cat-8", 20),
                ("Salary loan", "banknote", "cat-9", 21),
                ("Other deductions", "ellipsis", "cat-10", 22),
                // 0006: cat-12 is the one color used only once.
                ("HMO", "pill", "cat-12", 23),
            ]),
            "an archived category doesn't count toward the colors, and an income one doesn't block an expense one"
        );
    }

    /// The expense categories named HMO, ignoring case: everything but the id.
    fn hmo_rows(conn: &Connection) -> Vec<(String, String, String, i64, i64, String)> {
        conn.prepare(
            "SELECT name, icon, color, sort_order, archived, created_at FROM categories WHERE kind = 'expense' AND name = 'HMO'",
        )
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
    }

    #[test]
    fn an_hmo_the_user_made_is_kept_as_it_is() {
        // The owner's own, as it was made by hand, then other spellings and an archived one.
        for (name, icon, color, archived) in [
            ("HMO", "heart-pulse", "cat-9", 0),
            ("hmo", "shield", "cat-3", 0),
            ("Hmo", "coins", "cat-5", 1),
        ] {
            let mut conn = open(Path::new(":memory:")).unwrap();
            let all = migrations(Utc.with_ymd_and_hms(2026, 10, 26, 9, 30, 0).unwrap());
            all.to_version(&mut conn, 5).unwrap();
            conn.execute(
                "INSERT INTO categories (name, kind, color, icon, sort_order, archived, created_at, updated_at) VALUES (?1, 'expense', ?2, ?3, 21, ?4, ?5, ?5)",
                rusqlite::params![name, color, icon, archived, NOW],
            )
            .unwrap();
            let before = hmo_rows(&conn);

            all.to_latest(&mut conn).unwrap();

            assert_eq!(
                hmo_rows(&conn),
                before,
                "{name} is kept, and no second HMO is added"
            );
        }
    }

    #[test]
    fn an_existing_database_is_backed_up_then_gets_hmo_last() {
        let dir = crate::test_support::TempDir::new("mig-0006");
        let backups = dir.path().join("pre-migration");
        let mut conn = open(&dir.path().join("test.db")).unwrap();
        let all = migrations(Utc.with_ymd_and_hms(2026, 10, 26, 9, 30, 0).unwrap());
        all.to_version(&mut conn, 5).unwrap();
        add_basics(&conn);
        // An income HMO doesn't block the expense one, and an archived color doesn't count.
        conn.execute(
            "INSERT INTO categories (name, kind, color, icon, archived, sort_order, created_at, updated_at) VALUES ('HMO', 'income', 'cat-12', 'gift', 0, 9, ?1, ?1), ('Old', 'expense', 'cat-1', 'coins', 1, 30, ?1, ?1)",
            [NOW],
        )
        .unwrap();

        prepare(&mut conn, &all, &backups, crate::test_support::now()).unwrap();

        let copy = Connection::open(backups.join(format!(
            "{}-pre-migration-v5-2026-11-03_1405.db",
            crate::db::NAME
        )))
        .unwrap();
        let copy_version: i64 = copy
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(copy_version, 5, "BAK-5: the copy is from before 0006");
        let rows = hmo_rows(&conn);
        assert_eq!(rows.len(), 1);
        let (name, icon, color, sort_order, archived, created_at) = &rows[0];
        assert_eq!(
            (
                name.as_str(),
                icon.as_str(),
                color.as_str(),
                *sort_order,
                *archived
            ),
            ("HMO", "pill", "cat-1", 31, 0),
            "after the archived one's 30, in the lowest of the colors used twice"
        );
        assert_eq!(created_at.len(), "2026-10-26T09:30:00Z".len());
    }

    /// Migrates to version 3, adds a card and a bank with `decimals` stored, then runs 0004.
    fn levels_after_0004(decimals: Option<&str>) -> Vec<(String, [Option<i64>; 3])> {
        let mut conn = open(Path::new(":memory:")).unwrap();
        let all = migrations(Utc.with_ymd_and_hms(2026, 10, 26, 9, 30, 0).unwrap());
        all.to_version(&mut conn, 3).unwrap();
        if let Some(decimals) = decimals {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES ('currency_decimals', ?1)",
                [decimals],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO accounts (name, kind, opening_balance, credit_limit, statement_day, due_day, created_at, updated_at) VALUES ('Visa', 'credit_card', 0, 5000000, 15, 5, ?1, ?1)",
            [NOW],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO accounts (name, kind, created_at, updated_at) VALUES ('Bank', 'bank', ?1, ?1)",
            [NOW],
        )
        .unwrap();
        all.to_latest(&mut conn).unwrap();
        conn.prepare("SELECT name, spend_soft, spend_mid, spend_hard FROM accounts ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, [r.get(1)?, r.get(2)?, r.get(3)?])))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn existing_cards_get_the_default_spending_levels_in_the_main_unit() {
        let card = |soft, mid, hard| ("Visa".to_owned(), [Some(soft), Some(mid), Some(hard)]);
        let none = |name: &str| (name.to_owned(), [None; 3]);
        for (decimals, levels) in [
            (Some("2"), card(300_000, 400_000, 500_000)),
            (None, card(300_000, 400_000, 500_000)),
            (Some("0"), card(3_000, 4_000, 5_000)),
        ] {
            assert_eq!(
                levels_after_0004(decimals),
                [none("Cash"), levels, none("Bank")],
                "decimals {decimals:?}"
            );
        }
    }

    #[test]
    fn spending_levels_are_for_cards_only_and_rise_from_soft_to_hard() {
        let conn = migrated();
        let insert = |name: &str, kind: &str, levels: [Option<i64>; 3]| {
            let card = kind == "credit_card";
            conn.execute(
                "INSERT INTO accounts (name, kind, credit_limit, statement_day, due_day, spend_soft, spend_mid, spend_hard, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7, ?8, ?8)",
                rusqlite::params![
                    name,
                    kind,
                    card.then_some(5_000_000),
                    card.then_some(15),
                    levels[0],
                    levels[1],
                    levels[2],
                    NOW
                ],
            )
        };
        for (kind, levels) in [
            ("bank", [Some(1), None, None]),
            ("bank", [None, None, Some(3)]),
            ("credit_card", [Some(0), Some(2), Some(3)]),
            ("credit_card", [Some(2), Some(2), Some(3)]),
            ("credit_card", [Some(1), Some(3), Some(3)]),
            ("credit_card", [None, None, Some(3)]),
            ("credit_card", [Some(1), None, None]),
            ("credit_card", [Some(1), Some(2), None]),
        ] {
            assert!(insert("X", kind, levels).is_err(), "{kind} {levels:?}");
        }
        assert!(insert("Visa", "credit_card", [Some(1), Some(2), Some(3)]).is_ok());
    }

    /// Focus columns as `(column, SQL literal)` pairs.
    type Cols = Vec<(&'static str, &'static str)>;

    /// A logging focus for the week of Nov 16, valid as it is.
    fn logging() -> Cols {
        vec![
            ("focus_kind", "'logging'"),
            ("focus_target", "5"),
            ("focus_start", "'2026-11-16'"),
            ("focus_end", "'2026-11-22'"),
        ]
    }

    /// `cols` with `column` set to `value`, added when it isn't there.
    fn with(mut cols: Cols, column: &'static str, value: &'static str) -> Cols {
        cols.retain(|(c, _)| *c != column);
        cols.push((column, value));
        cols
    }

    /// Inserts a check-in row for `kind` and `start` with the extra `cols`. The rest of the
    /// row is a valid snapshot. The SQL is built from the test's own literals only.
    fn insert_checkin(
        conn: &Connection,
        kind: &str,
        start: &str,
        cols: &[(&str, &str)],
    ) -> rusqlite::Result<usize> {
        let names: String = cols.iter().map(|(c, _)| format!(", {c}")).collect();
        let values: String = cols.iter().map(|(_, v)| format!(", {v}")).collect();
        conn.execute(
            &format!("INSERT INTO checkins (kind, period_start, period_end, completed_on, level, stat_habit, created_at, updated_at{names}) VALUES (?1, ?2, ?2, '2026-11-16', 4, 80, ?3, ?3{values})"),
            rusqlite::params![kind, start, NOW],
        )
    }

    #[test]
    fn a_checkin_row_holds_its_shape() {
        let conn = migrated();
        assert!(insert_checkin(&conn, "week", "2026-11-09", &[]).is_ok());
        assert!(insert_checkin(&conn, "month", "2026-11-09", &[]).is_ok());
        assert!(
            insert_checkin(&conn, "week", "2026-11-09", &[]).is_err(),
            "a period is never reviewed twice"
        );
        assert!(insert_checkin(&conn, "week", "11/02/2026", &[]).is_err());
        let rejected: [(&str, Cols, &str); 13] = [
            ("day", vec![], "an unknown kind"),
            (
                "week",
                vec![("focus_category_id", "8")],
                "focus fields without a focus",
            ),
            (
                "week",
                vec![("focus_target", "5")],
                "focus fields without a focus",
            ),
            ("week", vec![("stat_debt", "-1")], "a stat below 0"),
            ("week", vec![("stat_card", "101")], "a stat above 100"),
            ("week", vec![("debt_free_on", "'soon'")], "a bad date"),
            (
                "week",
                with(logging(), "focus_end", "'2026-11-2'"),
                "a bad date",
            ),
            (
                "week",
                with(logging(), "focus_kind", "'budget'"),
                "an unknown focus",
            ),
            (
                "week",
                with(logging(), "focus_target", "0"),
                "a target of 0",
            ),
            (
                "week",
                with(logging(), "focus_result", "'late'"),
                "an unknown result",
            ),
            (
                "week",
                with(logging(), "focus_start", "NULL"),
                "a focus without its dates",
            ),
            ("year", logging(), "a year row with a focus"),
            (
                "week",
                with(logging(), "focus_kind", "'category_cap'"),
                "a cap without its category",
            ),
        ];
        // Each case has a period of its own, so it can only fail on its own check.
        let starts = (1..=12).map(|m| format!("2025-{m:02}-01"));
        for ((kind, cols, why), start) in rejected
            .into_iter()
            .zip(starts.chain(["2024-01-01".into()]))
        {
            assert!(
                insert_checkin(&conn, kind, &start, &cols).is_err(),
                "{why}: {cols:?}"
            );
        }
        assert!(
            insert_checkin(
                &conn,
                "week",
                "2024-02-05",
                &with(logging(), "focus_kind", "'card_level'")
            )
            .is_err(),
            "a card level without its card"
        );
        for (start, cols) in [
            ("2026-10-26", with(logging(), "focus_result", "'kept'")),
            // A removed payment leaves the focus without one (ON DELETE SET NULL).
            (
                "2026-10-19",
                with(
                    with(logging(), "focus_kind", "'loan_on_time'"),
                    "focus_target",
                    "NULL",
                ),
            ),
        ] {
            assert!(
                insert_checkin(&conn, "week", start, &cols).is_ok(),
                "{cols:?}"
            );
        }
        // The helper already sets the level, so this one is checked by an update.
        assert!(
            conn.execute("UPDATE checkins SET level = 11 WHERE id = 1", [])
                .is_err()
        );
    }

    #[test]
    fn a_focus_puts_its_category_and_card_in_use_and_loses_a_removed_payment() {
        let conn = migrated();
        add_loan(&conn);
        conn.execute(
            "INSERT INTO accounts (name, kind, credit_limit, statement_day, due_day, spend_soft, spend_mid, spend_hard, created_at, updated_at) VALUES ('Visa', 'credit_card', 5000000, 15, 5, 1, 2, 3, ?1, ?1)",
            [NOW],
        )
        .unwrap();
        let card = conn.last_insert_rowid();
        let cap = with(
            with(logging(), "focus_kind", "'category_cap'"),
            "focus_category_id",
            "9",
        );
        insert_checkin(&conn, "week", "2026-11-09", &cap).unwrap();
        let level = with(
            with(logging(), "focus_kind", "'card_level'"),
            "focus_account_id",
            "2",
        );
        insert_checkin(&conn, "week", "2026-11-02", &level).unwrap();
        let on_time = with(
            with(
                with(logging(), "focus_kind", "'loan_on_time'"),
                "focus_target",
                "NULL",
            ),
            "focus_loan_payment_id",
            "1",
        );
        insert_checkin(&conn, "month", "2026-11-01", &on_time).unwrap();
        assert_eq!(card, 2, "the card is account 2");
        for delete in [
            "DELETE FROM categories WHERE id = 9",
            "DELETE FROM accounts WHERE id = 2",
        ] {
            assert!(conn.execute(delete, []).is_err(), "{delete}");
        }
        conn.execute("DELETE FROM loans WHERE id = 1", []).unwrap();
        let payment: Option<i64> = conn
            .query_row(
                "SELECT focus_loan_payment_id FROM checkins WHERE kind = 'month'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(payment, None, "the focus is kept and graded void later");
    }

    #[test]
    fn an_existing_database_is_backed_up_then_gets_the_checkins_and_keeps_its_data() {
        let dir = crate::test_support::TempDir::new("mig-0005");
        let backups = dir.path().join("pre-migration");
        let mut conn = open(&dir.path().join("test.db")).unwrap();
        let all = migrations(Utc.with_ymd_and_hms(2026, 10, 26, 9, 30, 0).unwrap());
        all.to_version(&mut conn, 4).unwrap();
        add_basics(&conn);
        conn.execute(
            "INSERT INTO goals (name, target_amount, target_date, account_id, created_at, updated_at) VALUES ('Trip', 100, '2027-03-01', 2, ?1, ?1)",
            [NOW],
        )
        .unwrap();

        prepare(&mut conn, &all, &backups, crate::test_support::now()).unwrap();

        let copy = Connection::open(backups.join(format!(
            "{}-pre-migration-v4-2026-11-03_1405.db",
            crate::db::NAME
        )))
        .unwrap();
        let copy_version: i64 = copy
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(copy_version, 4, "BAK-5: the copy is from before 0005");
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 6);
        let goal: (String, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT target_date, moved_on, moved_from FROM goals WHERE name = 'Trip'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(goal, ("2027-03-01".to_owned(), None, None));
        let checkins: i64 = conn
            .query_row("SELECT count(*) FROM checkins", [], |row| row.get(0))
            .unwrap();
        assert_eq!(checkins, 0, "nothing is checked in by the update");
        for (moved_on, moved_from, accepted) in [
            ("'2026-11-16'", "NULL", false),
            ("NULL", "'2027-02-01'", false),
            ("'16/11/2026'", "'2027-02-01'", false),
            ("'2026-11-16'", "'Feb 1'", false),
            ("'2026-11-16'", "'2027-02-01'", true),
            ("NULL", "NULL", true),
        ] {
            let result = conn.execute(
                &format!("UPDATE goals SET moved_on = {moved_on}, moved_from = {moved_from}"),
                [],
            );
            assert_eq!(
                result.is_ok(),
                accepted,
                "{moved_on} {moved_from}: {result:?}"
            );
        }
    }
}
