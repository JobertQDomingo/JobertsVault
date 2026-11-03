use std::path::Path;
use std::process::Command;

use crate::error::AppError;

/// Opens Explorer at `dir`. The path never comes from the frontend (ARCHITECTURE §6).
pub fn open_folder(dir: &Path) -> Result<(), AppError> {
    // Not waited on: Explorer exits with 1 even when it opens the folder.
    Command::new("explorer").arg(dir).spawn()?;
    Ok(())
}
