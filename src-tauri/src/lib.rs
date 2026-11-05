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

/// 1440 × 900, or 1280 × 800 zoomed to 8/9 when that doesn't fit the monitor's usable area
/// (ADR 0008, ARCHITECTURE §11). Runs before the window is shown.
fn fit_window(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let Some(monitor) = window.current_monitor()? else {
        return Ok(());
    };
    let mut work = *monitor.work_area();
    // Debug builds can pretend to be on a smaller screen, to see the fallback:
    // JV_WORK_AREA=1366x728 npm run tauri dev
    if cfg!(debug_assertions)
        && let Some((w, h)) = std::env::var("JV_WORK_AREA")
            .ok()
            .and_then(|v| v.split_once('x').map(|(w, h)| (w.parse(), h.parse())))
            .and_then(|(w, h)| Some((w.ok()?, h.ok()?)))
    {
        work.size = tauri::PhysicalSize::new(w, h);
    }
    let (outer, inner) = (window.outer_size()?, window.inner_size()?);
    let fit = domain::window::window_fit(
        (work.size.width, work.size.height),
        (
            outer.width.saturating_sub(inner.width),
            outer.height.saturating_sub(inner.height),
        ),
        monitor.scale_factor(),
    );
    if fit != domain::window::FULL {
        window.set_size(tauri::LogicalSize::new(fit.width, fit.height))?;
        window.set_zoom(fit.zoom)?;
        window.center()?;
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let now = chrono::Local::now();
            app.manage(AppState::open(data_dir, &db::migrations(now.to_utc()), now));
            // The window starts hidden and is shown once startup is done, even if it failed
            // (ARCHITECTURE §6).
            if let Some(window) = app.get_webview_window("main") {
                // Best effort: if the monitor can't be read, it stays 1440 × 900.
                let _ = fit_window(&window);
                window.show()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_startup_status,
            commands::system::open_data_folder,
            commands::period::get_period,
            commands::dashboard::get_dashboard,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running the app");
}
