use tauri::Manager;

use state::AppState;

pub mod commands;
pub mod db;
pub mod domain;
pub mod error;
pub mod log;
pub mod services;
pub mod state;
#[cfg(test)]
mod test_support;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let now = chrono::Local::now();
            app.manage(AppState::open(data_dir, &db::migrations(now.to_utc()), now));
            // The window starts hidden and is shown once startup is done, even if it failed
            // (ARCHITECTURE §6).
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_startup_status,
            commands::system::open_data_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the app");
}
