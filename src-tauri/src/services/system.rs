use std::path::Path;
use std::process::Command;

use crate::error::AppError;

/// Opens Explorer at `dir`. The path never comes from the frontend (ARCHITECTURE §6).
pub fn open_folder(dir: &Path) -> Result<(), AppError> {
    // Explorer opens Documents for a missing folder, which would look like success.
    if !dir.is_dir() {
        return Err(AppError::Io {
            message: format!("The data folder doesn't exist: {}", dir.display()),
        });
    }
    // Not waited on: Explorer exits with 1 even when it opens the folder.
    Command::new("explorer").arg(dir).spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn a_missing_folder_is_an_error_instead_of_opening_documents() {
        let dir = TempDir::new("open-missing");
        let missing = dir.path().join("com.jobertsvault.app");
        assert!(matches!(open_folder(&missing), Err(AppError::Io { .. })));
    }
}
