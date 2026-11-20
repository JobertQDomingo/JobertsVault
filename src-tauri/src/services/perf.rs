//! The 50,000-transaction load check (ARCHITECTURE §12). Ignored by default; run it with
//! `cargo test --manifest-path src-tauri/Cargo.toml --release perf -- --ignored --nocapture`.

use std::time::{Duration, Instant};

use chrono::{Datelike, NaiveDate};
use rusqlite::{Connection, params};

use crate::domain::period::ComparisonMode;
use crate::services::categories::CategoryKind;
use crate::services::forecast::{self, ForecastGrain};
use crate::services::period::PeriodQuery;
use crate::services::transactions::{self, TransactionFilter, TransactionKind};
use crate::services::{dashboard, reports};
use crate::test_support::{TempDir, now};

const ROWS: i64 = 50_000;
const LIMIT: Duration = Duration::from_millis(300);
/// Backup and restore run on demand, and may take seconds (ARCHITECTURE §4.3).
const FILE_LIMIT: Duration = Duration::from_secs(2);

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 3).unwrap()
}

/// About 28 rows a day from 2022-01-01 to today: 70 % expenses, 20 % income, 10 % transfers.
fn seed(conn: &mut Connection) {
    let tx = conn.transaction().unwrap();
    tx.execute_batch(
        "INSERT INTO accounts (name, kind, opening_balance, credit_limit, statement_day, due_day, created_at, updated_at) VALUES
           ('Perf cash', 'cash', 500000, NULL, NULL, NULL, 'x', 'x'),
           ('Perf bank', 'bank', 10000000, NULL, NULL, NULL, 'x', 'x'),
           ('Perf wallet', 'ewallet', 0, NULL, NULL, NULL, 'x', 'x'),
           ('Perf visa', 'credit_card', 0, 5000000, 15, 5, 'x', 'x');",
    )
    .unwrap();
    let ids = |sql: &str| -> Vec<i64> {
        let mut stmt = tx.prepare(sql).unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    let accounts = ids("SELECT id FROM accounts ORDER BY id");
    let income = ids("SELECT id FROM categories WHERE kind = 'income'");
    let expense = ids("SELECT id FROM categories WHERE kind = 'expense'");
    let methods = ids("SELECT id FROM payment_methods");
    let start = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap();
    let days = (today() - start).num_days() + 1;
    {
        let mut insert = tx
            .prepare(
                "INSERT INTO transactions (kind, amount, date, account_id, to_account_id, category_id, payee, payment_method_id, note, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'x', 'x')",
            )
            .unwrap();
        for i in 0..ROWS {
            let date = (start + chrono::Days::new((i * days / ROWS) as u64)).to_string();
            // Mixed so the account, category and method don't follow the kind (i % 10).
            let mix = |salt: u64| {
                let x = (i as u64 ^ salt).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                (x ^ (x >> 31)) as usize
            };
            let pick = |list: &[i64], salt: u64| list[mix(salt) % list.len()];
            let a = mix(1) % accounts.len();
            let account = accounts[a];
            // Any account but the one it leaves.
            let to = accounts[(a + 1 + mix(2) % (accounts.len() - 1)) % accounts.len()];
            let amount = 100 + (i * 7919) % 500_000;
            let note = (i % 5 == 0).then(|| format!("note {}", i % 97));
            match i % 10 {
                0 => insert.execute(params![
                    "transfer",
                    amount,
                    date,
                    account,
                    to,
                    None::<i64>,
                    None::<String>,
                    None::<i64>,
                    note
                ]),
                1 | 2 => insert.execute(params![
                    "income",
                    amount,
                    date,
                    account,
                    None::<i64>,
                    pick(&income, 3),
                    format!("Employer {}", i % 20),
                    None::<i64>,
                    note
                ]),
                _ => insert.execute(params![
                    "expense",
                    amount,
                    date,
                    account,
                    None::<i64>,
                    pick(&expense, 5),
                    format!("Store {}", i % 300),
                    pick(&methods, 2),
                    note
                ]),
            }
            .unwrap();
        }
    }
    // A year of plans: paydays on the 15th and 30th, a daily, a weekly and a monthly rule, and a
    // loan with a payment each month (ARCHITECTURE §12).
    tx.execute_batch(
        "INSERT INTO recurring_rules (kind, amount, account_id, category_id, frequency, interval, start_date, mode, is_payday, created_at, updated_at) VALUES
           ('income', 1050000, 2, (SELECT id FROM categories WHERE name = 'Salary'), 'monthly', 1, '2026-11-15', 'confirm', 1, 'x', 'x'),
           ('income', 1050000, 2, (SELECT id FROM categories WHERE name = 'Salary'), 'monthly', 1, '2026-11-30', 'confirm', 1, 'x', 'x'),
           ('expense', 15000, 1, (SELECT id FROM categories WHERE kind = 'expense' LIMIT 1), 'daily', 1, '2026-11-04', 'auto', 0, 'x', 'x'),
           ('expense', 80000, 4, (SELECT id FROM categories WHERE kind = 'expense' LIMIT 1), 'weekly', 1, '2026-11-06', 'auto', 0, 'x', 'x'),
           ('expense', 260000, 2, (SELECT id FROM categories WHERE kind = 'expense' LIMIT 1), 'monthly', 1, '2026-11-10', 'confirm', 0, 'x', 'x');
         INSERT INTO loans (name, account_id, category_id, is_new, added_on, created_at, updated_at)
           VALUES ('Perf loan', 2, (SELECT id FROM categories WHERE name = 'Loan'), 0, '2025-11-01', 'x', 'x');",
    )
    .unwrap();
    seed_last_year(&tx);
    for month in 0..12 {
        let due = NaiveDate::from_ymd_opt(2026, 11, 20)
            .unwrap()
            .checked_add_months(chrono::Months::new(month))
            .unwrap();
        tx.execute(
            "INSERT INTO loan_payments (loan_id, due_date, amount, created_at, updated_at)
             VALUES ((SELECT id FROM loans), ?1, 100000, 'x', 'x')",
            [due.to_string()],
        )
        .unwrap();
    }
    tx.commit().unwrap();
}

/// Payslips (gross plus 3 deductions) on the 15th and 30th, and loan payments paid on the 20th.
const PAYSLIP_ROWS: i64 = 24 * 4;
const PAID_LOAN_ROWS: i64 = 12;

/// The year before today, for the check-in and the Vault level (ARCHITECTURE §12): 24 payslips,
/// 12 loan payments paid on time, and the check-ins finished each week and month, each with a
/// graded focus. The newest week and month (Oct 26 – Nov 1, October) stay due, so `get_checkin`
/// times the merged check-in, its fullest form.
fn seed_last_year(tx: &rusqlite::Transaction) {
    let category = |name: &str| -> i64 {
        tx.query_row("SELECT id FROM categories WHERE name = ?1", [name], |r| {
            r.get(0)
        })
        .unwrap()
    };
    let salary = category("Salary");
    let deductions = [
        (category("SSS"), 58_000),
        (category("PhilHealth"), 26_250),
        (category("Withholding tax"), 95_000),
    ];
    let insert = |kind: &str, amount: i64, date: &str, category: i64, payslip: Option<i64>| {
        tx.execute(
            "INSERT INTO transactions (kind, amount, date, account_id, category_id, payee, payslip_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, 2, ?4, 'Perf employer', ?5, 'x', 'x')",
            params![kind, amount, date, category, payslip],
        )
        .unwrap();
        tx.last_insert_rowid()
    };
    let start = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();
    for month in 0..12 {
        let first = start
            .checked_add_months(chrono::Months::new(month))
            .unwrap();
        // The 30th, or the month's last day (February).
        let last = first
            .checked_add_months(chrono::Months::new(1))
            .unwrap()
            .pred_opt()
            .unwrap();
        for day in [
            first.with_day(15).unwrap(),
            first.with_day(30).unwrap_or(last),
        ] {
            let date = day.to_string();
            let gross = insert("income", 1_250_000, &date, salary, None);
            tx.execute(
                "UPDATE transactions SET payslip_id = id WHERE id = ?1",
                [gross],
            )
            .unwrap();
            for (category, amount) in deductions {
                insert("expense", amount, &date, category, Some(gross));
            }
        }
        let due = first.with_day(20).unwrap().to_string();
        let paid = insert("expense", 100_000, &due, category("Loan"), None);
        tx.execute(
            "INSERT INTO loan_payments (loan_id, due_date, amount, transaction_id, created_at, updated_at)
             VALUES ((SELECT id FROM loans), ?1, 100000, ?2, 'x', 'x')",
            params![due, paid],
        )
        .unwrap();
    }
    // Check-ins: a snapshot each, a focus for the next period, graded kept or missed.
    let checkin = |kind: &str, start: NaiveDate, end: NaiveDate, next_end: NaiveDate, i: i64| {
        let focus = (kind == "week").then_some("spending_cap");
        tx.execute(
            "INSERT INTO checkins (kind, period_start, period_end, completed_on, level,
                 stat_cashflow, stat_bills, stat_card, stat_debt, stat_savings, stat_habit, debt_free_on,
                 focus_kind, focus_target, focus_start, focus_end, focus_result, focus_actual, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 90, ?7, 60, ?8, 80, '2027-10-20',
                 ?9, ?10, ?11, ?12, ?13, ?14, 'x', 'x')",
            params![
                kind,
                start.to_string(),
                end.to_string(),
                end.succ_opt().unwrap().to_string(),
                2 + i % 4,
                40 + i % 50,
                50 + i % 40,
                20 + i % 30,
                focus,
                focus.map(|_| 500_000),
                focus.map(|_| end.succ_opt().unwrap().to_string()),
                focus.map(|_| next_end.to_string()),
                focus.map(|_| if i % 3 == 0 { "missed" } else { "kept" }),
                focus.map(|_| 400_000 + (i % 3) * 60_000),
            ],
        )
        .unwrap();
    };
    let monday = NaiveDate::from_ymd_opt(2025, 11, 3).unwrap();
    for week in 0..51 {
        let start = monday + chrono::Days::new(7 * week);
        let end = start + chrono::Days::new(6);
        checkin("week", start, end, end + chrono::Days::new(7), week as i64);
    }
    for month in 0..11 {
        let first = start
            .checked_add_months(chrono::Months::new(month))
            .unwrap();
        let end = first
            .checked_add_months(chrono::Months::new(1))
            .unwrap()
            .pred_opt()
            .unwrap();
        checkin("month", first, end, end, month as i64);
    }
    // The last week's focus (Oct 26 – Nov 1) is graded by the check-in that's due.
    tx.execute(
        "UPDATE checkins SET focus_result = NULL, focus_actual = NULL
         WHERE kind = 'week' AND period_start = '2026-10-19'",
        [],
    )
    .unwrap();
}

/// The slowest of three runs, so a lucky cache doesn't hide a slow query.
fn time<T>(label: &str, run: impl FnMut() -> T) {
    time_within(LIMIT, label, run);
}

fn time_within<T>(limit: Duration, label: &str, mut run: impl FnMut() -> T) {
    let slowest = (0..3)
        .map(|_| {
            let start = Instant::now();
            std::hint::black_box(run());
            start.elapsed()
        })
        .max()
        .unwrap();
    println!("{label:<36} {:>7.1} ms", slowest.as_secs_f64() * 1000.0);
    assert!(slowest < limit, "{label} took {slowest:?}");
}

fn filter(period: PeriodQuery) -> TransactionFilter {
    TransactionFilter {
        period,
        kind: None,
        account_id: None,
        category_id: None,
        payment_method_id: None,
        search: None,
    }
}

#[test]
#[ignore = "slow; run on demand in release mode"]
fn perf_50k_dashboard_list_and_reports_under_300_ms() {
    let dir = TempDir::new("perf");
    let path = dir.path().join("perf.db");
    let mut conn = crate::db::open(&path).unwrap();
    let ms = crate::db::migrations(now().to_utc());
    crate::db::prepare(&mut conn, &ms, dir.path(), now()).unwrap();
    seed(&mut conn);
    drop(conn);
    // A fresh connection, as at startup: SQLite's cache is empty, though Windows may still hold
    // the file in memory, so this times the queries, not reading the disk.
    let mut conn = crate::db::open(&path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM transactions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, ROWS + PAYSLIP_ROWS + PAID_LOAN_ROWS);

    let all = || PeriodQuery::Custom {
        start: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
        end: today(),
    };
    // October 2026: a whole month, where the current one has only 3 days so far.
    let month = || PeriodQuery::Month {
        anchor: NaiveDate::from_ymd_opt(2026, 10, 1),
    };
    let busiest = |column: &str| -> i64 {
        conn.query_row(
            &format!("SELECT {column} FROM transactions WHERE {column} IS NOT NULL GROUP BY 1 ORDER BY COUNT(*) DESC LIMIT 1"),
            [],
            |r| r.get(0),
        )
        .unwrap()
    };
    let (account, category) = (busiest("account_id"), busiest("category_id"));
    let last_page = ((ROWS - 1) / i64::from(transactions::PAGE_SIZE)) as u32;
    for (label, query) in [
        ("dashboard: month", month()),
        ("dashboard: year", PeriodQuery::Year { anchor: None }),
        ("dashboard: all time", all()),
    ] {
        time(label, || dashboard::get(&conn, &query, today()).unwrap());
    }
    for (label, query, account) in [
        ("chart: month", month(), None),
        ("chart: year", PeriodQuery::Year { anchor: None }, None),
        ("chart: all time", all(), None),
        ("chart: all time, one account", all(), Some(account)),
    ] {
        time(label, || {
            reports::timeseries(&conn, &query, account, today()).unwrap()
        });
    }
    for (label, query, kind, account) in [
        (
            "breakdown: month, expense",
            month(),
            CategoryKind::Expense,
            None,
        ),
        (
            "breakdown: all time, expense",
            all(),
            CategoryKind::Expense,
            None,
        ),
        (
            "breakdown: all time, income",
            all(),
            CategoryKind::Income,
            None,
        ),
        (
            "breakdown: all time, one account",
            all(),
            CategoryKind::Expense,
            Some(account),
        ),
    ] {
        time(label, || {
            reports::breakdown(&conn, &query, kind, account, today()).unwrap()
        });
    }
    for (label, query, mode, account) in [
        (
            "comparison: month so far",
            PeriodQuery::Month { anchor: None },
            ComparisonMode::ToDate,
            None,
        ),
        (
            "comparison: October, full",
            month(),
            ComparisonMode::Full,
            None,
        ),
        ("comparison: all time", all(), ComparisonMode::Full, None),
        (
            "comparison: all time, one account",
            all(),
            ComparisonMode::Full,
            Some(account),
        ),
    ] {
        time(label, || {
            reports::comparison(&conn, &query, Some(mode), account, today()).unwrap()
        });
    }
    // Phase 4: the loans, and the forecast both ways; the dashboard above works out Safe to spend.
    time("list loans", || {
        crate::services::loans::list(&conn, today()).unwrap()
    });
    for (label, grain) in [
        ("forecast: pay windows", ForecastGrain::Window),
        ("forecast: months", ForecastGrain::Month),
    ] {
        time(label, || forecast::get(&conn, grain, today()).unwrap());
    }
    assert!(
        forecast::get(&conn, ForecastGrain::Window, today())
            .unwrap()
            .safe_to_spend
            .is_some()
    );
    // Phase 5: the Vault level, which the sidebar's level chip asks for on every screen.
    time("vault status", || {
        crate::services::vault::get(&mut conn, today()).unwrap()
    });
    time("check-in", || {
        crate::services::checkin::get(&mut conn, today()).unwrap()
    });
    // The merged check-in (October and Oct 26 – Nov 1), with a year of history behind it.
    let due = crate::services::checkin::get(&mut conn, today())
        .unwrap()
        .expect("a check-in is due");
    assert_eq!(due.kinds.len(), 2, "merged: {:?}", due.kinds);
    // TXN-6: one letter matches all 300 payees; "store 12" narrows to 11.
    for (label, prefix) in [("payees: \"s\"", "s"), ("payees: \"store 12\"", "store 12")] {
        time(label, || {
            transactions::suggest_payees(&conn, prefix, CategoryKind::Expense, today()).unwrap()
        });
    }
    let list = |label: &str, f: TransactionFilter, page: u32| {
        time(label, || {
            transactions::list(&conn, f.clone(), page, today()).unwrap()
        });
    };
    list("list: month", filter(month()), 0);
    list("list: all time", filter(all()), 0);
    list("list: all time, last page", filter(all()), last_page);
    list(
        "list: all time, expenses",
        TransactionFilter {
            kind: Some(TransactionKind::Expense),
            ..filter(all())
        },
        0,
    );
    list(
        "list: all time, one account",
        TransactionFilter {
            account_id: Some(account),
            ..filter(all())
        },
        0,
    );
    list(
        "list: all time, one category",
        TransactionFilter {
            category_id: Some(category),
            ..filter(all())
        },
        0,
    );
    list(
        "list: all time, search \"store 12\"",
        TransactionFilter {
            search: Some("store 12".into()),
            ..filter(all())
        },
        0,
    );
    list(
        "list: all time, search no match",
        TransactionFilter {
            search: Some("zzz".into()),
            ..filter(all())
        },
        0,
    );
    // Back up now holds the connection while it copies, so every view waits for it (BAK-1).
    time_within(FILE_LIMIT, "back up now (VACUUM INTO)", || {
        crate::services::backup::back_up_now(&conn, dir.path(), now()).unwrap()
    });
    // Export to CSV holds it while it reads every row and writes the file (BAK-6).
    let csv = dir.path().join("all.csv");
    let everything = crate::services::export::DateRange {
        start: "2022-01-01".into(),
        end: today().to_string(),
    };
    time_within(FILE_LIMIT, "export all time to CSV", || {
        let file = crate::services::export::prepare(&conn, &everything, &csv).unwrap();
        file.save().unwrap()
    });
    // Restore holds it from the first check to the reopen (BAK-4).
    use crate::services::backup;
    let file = backup::back_up_now(&conn, dir.path(), now())
        .unwrap()
        .file_name;
    let chosen = backup::folder(&conn, dir.path()).unwrap().join(file);
    time_within(FILE_LIMIT, "inspect a backup", || {
        backup::inspect(dir.path(), &path, &chosen, now()).unwrap()
    });
    let mut conn = conn;
    time_within(FILE_LIMIT, "restore a backup", || {
        backup::restore(&mut conn, dir.path(), &path, &chosen, now()).unwrap()
    });
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM transactions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        count,
        ROWS + PAYSLIP_ROWS + PAID_LOAN_ROWS,
        "three restores in a row"
    );
}
