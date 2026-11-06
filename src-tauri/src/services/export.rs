use std::fs;
use std::path::Path;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::csv;
use crate::domain::money::{Decimals, Money};
use crate::error::AppError;
use crate::repo;
use crate::services::transactions::parse_date;
use crate::services::{invalid, settings};

/// The dates to export, both included, as `YYYY-MM-DD`.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DateRange {
    pub start: String,
    pub end: String,
}

/// What `export_csv` returns: how many transactions it wrote.
#[derive(Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CsvExport {
    pub rows: usize,
}

const HEADER: [&str; 9] = [
    "date",
    "type",
    "amount",
    "account",
    "to_account",
    "category",
    "payee",
    "payment_method",
    "note",
];

/// Export to CSV (BAK-6, DATA_MODEL §8): every transaction from `range.start` to `range.end`,
/// oldest first, written to `path`, a `.csv` file the user chose in the save dialog. The file is
/// written in full beside it first, so a failure never leaves half a file or spoils one it was
/// to replace.
pub fn export_csv(
    conn: &Connection,
    range: &DateRange,
    path: &Path,
) -> Result<CsvExport, AppError> {
    let start = parse_date(&range.start).ok_or_else(|| invalid("start", "Enter a real date"))?;
    let end = parse_date(&range.end).ok_or_else(|| invalid("end", "Enter a real date"))?;
    if end < start {
        return Err(invalid(
            "end",
            "The end date can't be before the start date",
        ));
    }
    let csv_file = path.is_absolute()
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
        && path.parent().is_some_and(Path::is_dir);
    if !csv_file {
        return Err(invalid(
            "path",
            "Choose a .csv file in a folder that exists.",
        ));
    }
    let decimals = match settings::stored(conn)?.currency_decimals {
        0 => Decimals::Zero,
        _ => Decimals::Two,
    };
    let rows = repo::transactions::export(conn, &start.to_string(), &end.to_string())?;
    let count = rows.len();
    let text = csv::document(
        &HEADER,
        rows.into_iter().map(|row| {
            vec![
                row.date,
                row.kind,
                // Stored amounts are positive minor units: no float ever touches them.
                Money(row.amount).to_plain(decimals),
                row.account,
                row.to_account.unwrap_or_default(),
                row.category.unwrap_or_default(),
                row.payee.unwrap_or_default(),
                row.payment_method.unwrap_or_default(),
                row.note.unwrap_or_default(),
            ]
        }),
    );
    let tmp = path.with_extension("csv.tmp");
    fs::write(&tmp, text)
        .and_then(|()| fs::rename(&tmp, path))
        .map_err(|err| {
            let _ = fs::remove_file(&tmp);
            AppError::Io {
                message: format!("Couldn't save the CSV file. {err}"),
            }
        })?;
    Ok(CsvExport { rows: count })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempDir, db};

    fn id(conn: &Connection, table: &str, name: &str) -> i64 {
        conn.query_row(
            &format!("SELECT id FROM {table} WHERE name = ?1"),
            [name],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn add(conn: &Connection, values: (&str, i64, &str, i64, Option<i64>, Option<i64>)) {
        let (kind, amount, date, account, to_account, category) = values;
        conn.execute(
            "INSERT INTO transactions (kind, amount, date, account_id, to_account_id, category_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'x', 'x')",
            rusqlite::params![kind, amount, date, account, to_account, category],
        )
        .unwrap();
    }

    fn range(start: &str, end: &str) -> DateRange {
        DateRange {
            start: start.into(),
            end: end.into(),
        }
    }

    #[test]
    fn every_transaction_in_the_range_is_written_oldest_first() {
        let dir = TempDir::new("bak6-export");
        let conn = db();
        conn.execute(
            "INSERT INTO accounts (name, kind, created_at, updated_at) VALUES ('Bank, Savings', 'bank', 'x', 'x')",
            [],
        )
        .unwrap();
        let cash = id(&conn, "accounts", "Cash");
        let bank = id(&conn, "accounts", "Bank, Savings");
        let salary = id(&conn, "categories", "Salary");
        let food = id(&conn, "categories", "Food & Groceries");
        // Entered out of date order, plus a row on each side of the range.
        add(
            &conn,
            ("transfer", 500000, "2026-10-25", bank, Some(cash), None),
        );
        add(&conn, ("expense", 5, "2026-10-25", cash, None, Some(food)));
        add(
            &conn,
            ("income", 2500000, "2026-10-01", bank, None, Some(salary)),
        );
        add(
            &conn,
            ("expense", 100, "2026-09-30", cash, None, Some(food)),
        );
        add(
            &conn,
            ("expense", 100, "2026-10-26", cash, None, Some(food)),
        );
        conn.execute_batch(
            "UPDATE transactions SET payee = 'Caf\u{e9} \"Kape\", Inc.', payment_method_id = 1,
                 note = 'line 1' || char(13, 10) || 'line 2' WHERE kind = 'expense' AND amount = 5;
             UPDATE transactions SET note = 'October bill' WHERE kind = 'transfer';
             UPDATE categories SET archived = 1 WHERE name = 'Food & Groceries';",
        )
        .unwrap();
        let path = dir.path().join("October.csv");

        let done = export_csv(&conn, &range("2026-10-01", "2026-10-25"), &path).unwrap();

        assert_eq!(done, CsvExport { rows: 3 });
        let bytes = fs::read(&path).unwrap();
        assert_eq!(&bytes[..3], [0xEF, 0xBB, 0xBF], "UTF-8 with a BOM");
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            concat!(
                "\u{FEFF}date,type,amount,account,to_account,category,payee,payment_method,note\r\n",
                "2026-10-01,income,25000.00,\"Bank, Savings\",,Salary,,,\r\n",
                "2026-10-25,transfer,5000.00,\"Bank, Savings\",Cash,,,,October bill\r\n",
                "2026-10-25,expense,0.05,Cash,,Food & Groceries,\"Caf\u{e9} \"\"Kape\"\", Inc.\",Cash,\"line 1\r\nline 2\"\r\n",
            )
        );
        assert!(!dir.path().join("October.csv.tmp").exists());
    }

    #[test]
    fn amounts_follow_the_decimal_places_setting() {
        let dir = TempDir::new("bak6-decimals");
        let conn = db();
        repo::settings::set(&conn, "currency_decimals", "0").unwrap();
        let food = id(&conn, "categories", "Food & Groceries");
        add(&conn, ("expense", 250, "2026-10-01", 1, None, Some(food)));
        add(
            &conn,
            (
                "expense",
                999_999_999_999,
                "2026-10-02",
                1,
                None,
                Some(food),
            ),
        );
        let path = dir.path().join("whole.csv");

        export_csv(&conn, &range("2026-10-01", "2026-10-31"), &path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        let amounts: Vec<&str> = text
            .lines()
            .skip(1)
            .map(|line| line.split(',').nth(2).unwrap())
            .collect();
        assert_eq!(amounts, ["250", "999999999999"]);

        repo::settings::set(&conn, "currency_decimals", "2").unwrap();
        export_csv(&conn, &range("2026-10-01", "2026-10-31"), &path).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.contains(",2.50,") && text.contains(",9999999999.99,"),
            "{text}"
        );
    }

    #[test]
    fn an_empty_range_writes_the_header_alone() {
        let dir = TempDir::new("bak6-empty");
        let path = dir.path().join("none.CSV");
        let done = export_csv(&db(), &range("2026-10-01", "2026-10-01"), &path).unwrap();
        assert_eq!(done.rows, 0);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "\u{FEFF}date,type,amount,account,to_account,category,payee,payment_method,note\r\n"
        );
    }

    #[test]
    fn bad_dates_and_paths_are_refused_and_nothing_is_written() {
        let dir = TempDir::new("bak6-refused");
        let conn = db();
        let good = dir.path().join("out.csv");
        let cases: [(DateRange, std::path::PathBuf, &str); 7] = [
            (range("2026-10-32", "2026-10-31"), good.clone(), "start"),
            (range("2026-10-01", "Oct 31"), good.clone(), "end"),
            (range("2026-10-31", "2026-10-01"), good.clone(), "end"),
            (range("2026-10-01", "2026-10-31"), "out.csv".into(), "path"),
            (
                range("2026-10-01", "2026-10-31"),
                dir.path().join("out.db"),
                "path",
            ),
            (
                range("2026-10-01", "2026-10-31"),
                dir.path().join("out"),
                "path",
            ),
            (
                range("2026-10-01", "2026-10-31"),
                dir.path().join("missing").join("out.csv"),
                "path",
            ),
        ];
        for (dates, path, expected) in cases {
            let result = export_csv(&conn, &dates, &path);
            assert!(
                matches!(&result, Err(AppError::Validation { field, .. }) if field == expected),
                "{dates:?} {path:?}: {result:?}"
            );
        }
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn a_failed_write_leaves_no_partial_file() {
        let dir = TempDir::new("bak6-failed");
        // A folder where the file should go: the final rename can't replace it.
        let path = dir.path().join("taken.csv");
        fs::create_dir(&path).unwrap();
        let result = export_csv(&db(), &range("2026-10-01", "2026-10-31"), &path);
        assert!(
            matches!(&result, Err(AppError::Io { message }) if message.starts_with("Couldn't save the CSV file.")),
            "{result:?}"
        );
        assert!(path.is_dir());
        assert!(!dir.path().join("taken.csv.tmp").exists());
    }

    #[test]
    fn an_existing_file_the_user_chose_is_replaced_whole() {
        let dir = TempDir::new("bak6-replace");
        let path = dir.path().join("out.csv");
        fs::write(
            &path,
            "an older, longer export that must not leave a tail behind",
        )
        .unwrap();
        export_csv(&db(), &range("2026-10-01", "2026-10-31"), &path).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "\u{FEFF}date,type,amount,account,to_account,category,payee,payment_method,note\r\n"
        );
    }
}
