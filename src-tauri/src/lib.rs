use tauri::Manager;

pub mod error;
pub mod log;
pub mod state;
#[cfg(test)]
mod test_support;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // The window starts hidden and is shown once startup is done (ARCHITECTURE §6).
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the app");
}
