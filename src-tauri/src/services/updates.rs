use std::time::Duration;

use tauri::{AppHandle, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::error::AppError;

/// How long the check waits before the app starts without it (UPD-2).
const CHECK_TIMEOUT: Duration = Duration::from_secs(3);

/// The newer version announced by this repository's latest GitHub release, or `None` when this
/// copy is up to date. Debug builds never connect, so they always get `None` (UPD-7).
pub async fn check<R: Runtime>(app: &AppHandle<R>) -> Result<Option<String>, AppError> {
    if cfg!(debug_assertions) {
        return Ok(None);
    }
    Ok(find(app).await?.map(|update| update.version))
}

/// Downloads the update, checks its signature and its signed version against the public key in
/// tauri.conf.json (UPD-4), then runs the installer, which closes the app and opens the new one.
pub async fn install<R: Runtime>(app: &AppHandle<R>) -> Result<(), AppError> {
    if cfg!(debug_assertions) {
        return Err(update_error(
            "Development builds don't install updates. Nothing was changed.",
        ));
    }
    let mut update = find(app)
        .await?
        .ok_or_else(|| update_error("Jobert's Vault is already up to date."))?;
    // The 3-second limit is for the check. The builder's timeout also covers the download,
    // which would make every install on a slow connection fail.
    update.timeout = None;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|err| {
            update_error(&format!(
                "Couldn't install the update: {}. Nothing was changed.",
                sentence(&err)
            ))
        })?;
    app.restart()
}

async fn find<R: Runtime>(app: &AppHandle<R>) -> Result<Option<Update>, AppError> {
    let failed = |err: tauri_plugin_updater::Error| {
        update_error(&format!("Couldn't check for updates: {}.", sentence(&err)))
    };
    app.updater_builder()
        .timeout(CHECK_TIMEOUT)
        .build()
        .map_err(failed)?
        .check()
        .await
        .map_err(failed)
}

fn update_error(message: &str) -> AppError {
    AppError::Update {
        message: message.into(),
    }
}

/// The updater's error without its own full stop, so ours can follow it.
fn sentence(err: &tauri_plugin_updater::Error) -> String {
    err.to_string().trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_updaters_full_stop_is_dropped() {
        let err = tauri_plugin_updater::Error::EmptyEndpoints;
        assert_eq!(sentence(&err), "Updater does not have any endpoints set");
    }
}
