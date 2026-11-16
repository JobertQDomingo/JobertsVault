use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, TimeZone};

/// The fixed time tests run at: 2026-11-03 14:05 local.
pub fn now() -> DateTime<Local> {
    Local.with_ymd_and_hms(2026, 11, 3, 14, 5, 0).unwrap()
}

/// An in-memory database with every migration and the seed data applied.
pub fn db() -> rusqlite::Connection {
    let mut conn = crate::db::open(Path::new(":memory:")).unwrap();
    let ms = crate::db::migrations(now().to_utc());
    crate::db::prepare(&mut conn, &ms, Path::new("unused"), now()).unwrap();
    conn
}

/// A monthly Salary payday rule into `account` from 2026-11-15, expecting `net` each time (PAY-1).
/// Smart goal dates need one (GOL-6).
pub fn payday(conn: &mut rusqlite::Connection, account: i64, net: i64) -> i64 {
    use crate::domain::{money::Money, recurrence::Frequency};
    use crate::services::recurring::{self, RecurringMode, RecurringRuleFields};
    use crate::services::transactions::{TransactionFields, TransactionKind};
    let transaction = TransactionFields {
        kind: TransactionKind::Income,
        amount: Money(net),
        date: "2026-11-15".into(),
        account_id: account,
        to_account_id: None,
        category_id: Some(1), // Salary, from the seed
        payee: None,
        payment_method_id: None,
        note: None,
    };
    let rule = RecurringRuleFields {
        transaction,
        frequency: Frequency::Monthly,
        interval: 1,
        end_date: None,
        max_occurrences: None,
        mode: RecurringMode::Confirm,
        is_payday: true,
    };
    recurring::create(conn, rule, now()).unwrap().id
}

/// A folder under the system temp dir, wiped when created and when dropped.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(name: &str) -> Self {
        let path = std::env::temp_dir()
            .join("jobertsvault-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
