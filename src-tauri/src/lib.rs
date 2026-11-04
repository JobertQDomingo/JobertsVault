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
            commands::period::get_period,
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
