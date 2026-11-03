use tauri::Manager;

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

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use serde::Serialize;
    use ts_rs::TS;

    // Proves the bindings pipeline until Phase 1 adds real types. Delete this and
    // src/bindings/BindingsCheck.ts when the first real type is exported.
    #[derive(Serialize, TS)]
    #[serde(rename_all = "camelCase")]
    #[ts(export)]
    struct BindingsCheck {
        #[ts(type = "number")]
        minor_units: i64,
        posted_on: NaiveDate,
    }

    #[test]
    fn money_crosses_ipc_as_a_number_and_dates_as_strings() {
        let declaration = BindingsCheck::decl(&ts_rs::Config::default());
        assert!(declaration.contains("minorUnits: number"), "{declaration}");
        assert!(declaration.contains("postedOn: string"), "{declaration}");
    }
}
