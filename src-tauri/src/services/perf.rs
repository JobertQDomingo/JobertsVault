//! The 50,000-transaction load check (ARCHITECTURE §12). Ignored by default; run it with
//! `cargo test --manifest-path src-tauri/Cargo.toml --release perf -- --ignored --nocapture`.

use std::time::{Duration, Instant};

use chrono::NaiveDate;
use rusqlite::{Connection, params};

use crate::services::period::PeriodQuery;
use crate::services::transactions::{self, TransactionFilter, TransactionKind};
use crate::services::{dashboard, reports};
use crate::test_support::{TempDir, now};

const ROWS: i64 = 50_000;
const LIMIT: Duration = Duration::from_millis(300);

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
    tx.commit().unwrap();
}

/// The slowest of three runs, so a lucky cache doesn't hide a slow query.
fn time<T>(label: &str, mut run: impl FnMut() -> T) {
    let slowest = (0..3)
        .map(|_| {
            let start = Instant::now();
            std::hint::black_box(run());
            start.elapsed()
        })
        .max()
        .unwrap();
    println!("{label:<36} {:>7.1} ms", slowest.as_secs_f64() * 1000.0);
    assert!(slowest < LIMIT, "{label} took {slowest:?}");
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
    let conn = crate::db::open(&path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM transactions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, ROWS);

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
    for (label, query) in [
        ("chart: month", month()),
        ("chart: year", PeriodQuery::Year { anchor: None }),
        ("chart: all time", all()),
    ] {
        time(label, || {
            reports::timeseries(&conn, &query, today()).unwrap()
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
}
