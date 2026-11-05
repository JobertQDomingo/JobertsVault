//! Checks for and installs updates through tauri-plugin-updater (ARCHITECTURE §11.1). It's
//! Tauri-specific, so it sits beside `log` rather than in `services` (ARCHITECTURE §4.1).

use std::error::Error as _;
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::error::AppError;

/// How long the check waits before the app starts without it (UPD-2).
const CHECK_TIMEOUT: Duration = Duration::from_secs(3);
/// A download has no overall limit, but one that stops sending data for this long has failed,
/// rather than leaving the startup screen on "Updating…" forever.
const STALL_TIMEOUT: Duration = Duration::from_secs(30);

/// The update the last check found, so Update now installs exactly what it announced.
#[derive(Default)]
pub struct Pending(Mutex<Option<Update>>);

/// The newer version announced by this repository's latest GitHub release, or `None` when this
/// copy is up to date. Debug builds never connect, so they always get `None` (UPD-7).
pub async fn check<R: Runtime>(app: &AppHandle<R>) -> Result<Option<String>, AppError> {
    if cfg!(debug_assertions) {
        return Ok(None);
    }
    let failed = |err: tauri_plugin_updater::Error| {
        update_error(&format!("Couldn't check for updates: {}.", sentence(&err)))
    };
    let found = app
        .updater_builder()
        .timeout(CHECK_TIMEOUT)
        .configure_client(|client| client.read_timeout(STALL_TIMEOUT))
        .build()
        .map_err(failed)?
        .check()
        .await
        .map_err(failed)?;
    let version = found.as_ref().map(|update| update.version.clone());
    *pending(app) = found;
    Ok(version)
}

/// Downloads the update the last check found, checks its signature and its signed version
/// against the public key in tauri.conf.json (UPD-4), then runs the installer. On Windows the
/// plugin ends this process itself, and the installer opens the new version.
pub async fn install<R: Runtime>(app: &AppHandle<R>) -> Result<(), AppError> {
    if cfg!(debug_assertions) {
        return Err(update_error(
            "Development builds don't install updates. Nothing was changed.",
        ));
    }
    let mut update = pending(app)
        .take()
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
    // Not reached on Windows, where the plugin has already exited.
    app.restart()
}

fn pending<R: Runtime>(app: &AppHandle<R>) -> std::sync::MutexGuard<'_, Option<Update>> {
    app.state::<Pending>()
        .inner()
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn update_error(message: &str) -> AppError {
    AppError::Update {
        message: message.into(),
    }
}

/// The updater's error and its causes, without a final full stop so ours can follow. A network
/// error's own text is only "error sending request for url (…)"; the cause says why.
fn sentence(err: &tauri_plugin_updater::Error) -> String {
    let mut text = err.to_string();
    let mut cause = err.source();
    while let Some(inner) = cause {
        text = format!("{text}: {inner}");
        cause = inner.source();
    }
    text.trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_updaters_full_stop_is_dropped() {
        let err = tauri_plugin_updater::Error::EmptyEndpoints;
        assert_eq!(sentence(&err), "Updater does not have any endpoints set");
    }

    #[test]
    fn the_causes_are_kept() {
        // Shaped like reqwest's error: its own text, and the reason only as its source.
        #[derive(Debug)]
        struct Sending(std::io::Error);
        impl std::fmt::Display for Sending {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("error sending request for url (https://github.com/)")
            }
        }
        impl std::error::Error for Sending {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }
        let timed_out = std::io::Error::new(std::io::ErrorKind::TimedOut, "operation timed out.");
        let err = tauri_plugin_updater::Error::Io(std::io::Error::other(Sending(timed_out)));
        assert_eq!(
            sentence(&err),
            "error sending request for url (https://github.com/): operation timed out"
        );
    }
}
