use tauri::Manager;

use state::AppState;

pub mod commands;
pub mod db;
pub mod domain;
pub mod error;
pub mod log;
pub mod repo;
pub mod services;
pub mod state;
#[cfg(test)]
mod test_support;
pub mod updater;

/// 1440 × 900, or 1280 × 800 zoomed to 8/9 when that doesn't fit the monitor's usable area
/// (ADR 0008, ARCHITECTURE §11). Runs before the window is shown.
fn fit_window(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let Some(monitor) = window.current_monitor()? else {
        return Ok(());
    };
    let mut work = monitor.work_area().size;
    // Debug builds can pretend to be on a smaller screen, to see the fallback:
    // JV_WORK_AREA=1600x860 npm run tauri dev
    if cfg!(debug_assertions)
        && let Some((w, h)) = std::env::var("JV_WORK_AREA").ok().and_then(|v| {
            let (w, h) = v.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
    {
        work = tauri::PhysicalSize::new(w, h);
    }
    // Created hidden at 1440 × 900, so its outer size is what that takes on this monitor.
    let outer = window.outer_size()?;
    let fit = domain::window::window_fit((outer.width, outer.height), (work.width, work.height));
    if fit != domain::window::FULL {
        // Zoom first: if the resize then fails, the zoom goes back, so the page is never cut off.
        window.set_zoom(fit.zoom)?;
        if let Err(err) = window.set_size(tauri::LogicalSize::new(fit.width, fit.height)) {
            window.set_zoom(domain::window::FULL.zoom)?;
            return Err(err);
        }
        window.center()?;
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        // Only Rust uses it: the frontend gets no updater permissions (ARCHITECTURE §10).
        .plugin(tauri_plugin_updater::Builder::new().build())
        // The frontend may use only its open and save pickers (capabilities/default.json).
        .plugin(tauri_plugin_dialog::init())
        .manage(updater::Pending::default())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let now = chrono::Local::now();
            app.manage(AppState::open(data_dir, &db::migrations(now.to_utc()), now));
            // Setup goes on only once the backup thread holds the connection, so every screen
            // waits for today's backup rather than reading the state from before it.
            let handle = app.handle().clone();
            let (locked, holding) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                handle.state::<AppState>().back_up_daily(now, || {
                    let _ = locked.send(());
                });
            });
            let _ = holding.recv();
            // The window starts hidden and is shown once startup is done, even if it failed
            // (ARCHITECTURE §6).
            if let Some(window) = app.get_webview_window("main") {
                // Best effort: if it fails, the window stays 1440 × 900 and the log says why.
                if let Err(err) = fit_window(&window) {
                    let log_path = &app.state::<AppState>().log_path;
                    log::record(
                        log_path,
                        &error::AppError::Io {
                            message: format!("window size: {err}"),
                        },
                    );
                }
                window.show()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_startup_status,
            commands::system::open_data_folder,
            commands::system::get_app_version,
            commands::updates::check_update,
            commands::updates::install_update,
            commands::period::get_period,
            commands::dashboard::get_dashboard,
            commands::reports::get_timeseries,
            commands::reports::get_category_breakdown,
            commands::reports::get_comparison,
            commands::backup::backup_now,
            commands::backup::open_backup_folder,
            commands::backup::inspect_backup,
            commands::backup::restore_backup,
            commands::backup::export_csv,
            commands::backup::get_backup_status,
            commands::backup::set_backup_folder,
            commands::backup::set_auto_backup,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::accounts::list_accounts,
            commands::accounts::get_account,
            commands::accounts::create_account,
            commands::accounts::update_account,
            commands::accounts::reorder_accounts,
            commands::accounts::archive_account,
            commands::accounts::unarchive_account,
            commands::accounts::delete_account,
            commands::categories::list_categories,
            commands::categories::create_category,
            commands::categories::update_category,
            commands::categories::reorder_categories,
            commands::categories::archive_category,
            commands::categories::unarchive_category,
            commands::categories::delete_category,
            commands::categories::merge_categories,
            commands::payment_methods::list_payment_methods,
            commands::payment_methods::create_payment_method,
            commands::payment_methods::update_payment_method,
            commands::payment_methods::reorder_payment_methods,
            commands::payment_methods::archive_payment_method,
            commands::payment_methods::unarchive_payment_method,
            commands::payment_methods::delete_payment_method,
            commands::transactions::list_transactions,
            commands::transactions::get_transaction,
            commands::transactions::create_transaction,
            commands::transactions::update_transaction,
            commands::transactions::delete_transaction,
            commands::transactions::get_transaction_defaults,
            commands::transactions::get_pay_bill_defaults,
            commands::transactions::suggest_payees,
        ])
        .build(tauri::generate_context!())
        .expect("error while building the app")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event
                && let Some(state) = app.try_state::<AppState>()
            {
                state.close();
            }
        });
}
