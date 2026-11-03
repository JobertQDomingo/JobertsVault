use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, TimeZone};

/// The fixed time tests run at: 2026-11-03 14:05 local.
pub fn now() -> DateTime<Local> {
    Local.with_ymd_and_hms(2026, 11, 3, 14, 5, 0).unwrap()
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
