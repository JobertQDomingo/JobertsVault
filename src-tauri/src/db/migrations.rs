use chrono::{DateTime, Utc};
use rusqlite::Transaction;
use rusqlite_migration::{HookResult, M, Migrations};

use super::seed;

/// Every schema change, oldest first. A released migration is never edited (DATA_MODEL §8).
/// `now` stamps the seed data that 0001 adds to a new database, and the categories 0003 adds.
pub fn migrations(now: DateTime<Utc>) -> Migrations<'static> {
    let stamp = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let stamp_0003 = stamp.clone();
    Migrations::new(vec![
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
        assert_eq!(version, 3);

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
        let rows: [Row; 8] = [
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

    /// The expense categories after the 17 seeded ones: name, icon, color and sort order.
    fn added_expense_categories(conn: &Connection) -> Vec<(String, String, String, i64)> {
        conn.prepare(
            "SELECT name, icon, color, sort_order FROM categories WHERE kind = 'expense' AND id > 24 ORDER BY id",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
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
        assert_eq!(
            added_expense_categories(&conn),
            owned(&[
                ("Loan", "coins", "cat-2", 17),
                ("SSS", "shield", "cat-6", 18),
                ("PhilHealth", "heart-pulse", "cat-7", 19),
                ("Pag-IBIG", "house", "cat-8", 20),
                ("Withholding tax", "receipt", "cat-9", 21),
                ("Salary loan", "banknote", "cat-10", 22),
                ("Other deductions", "ellipsis", "cat-12", 23),
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
        let added: Vec<(String, String)> = conn
            .prepare(
                "SELECT name, color FROM categories WHERE kind = 'expense' AND id > 26 ORDER BY id",
            )
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let expected: Vec<(String, String)> = [
            ("SSS", "cat-2"),
            ("PhilHealth", "cat-6"),
            ("Pag-IBIG", "cat-7"),
            ("Withholding tax", "cat-8"),
            ("Salary loan", "cat-9"),
            ("Other deductions", "cat-10"),
        ]
        .iter()
        .map(|(n, c)| ((*n).into(), (*c).into()))
        .collect();
        assert_eq!(
            added, expected,
            "an archived category doesn't count toward the colors, and an income one doesn't block an expense one"
        );
    }
}
