pub mod accounts;
pub mod categories;
pub mod dashboard;
pub mod payment_methods;
pub mod period;
pub mod reports;
pub mod settings;
pub mod system;
pub mod transactions;
pub mod updates;

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    // No fs, shell or http plugin, and of the dialog plugin only its pickers (ARCHITECTURE §10).
    #[test]
    fn the_frontend_gets_only_the_window_and_the_file_pickers() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities");
        // Tauri loads every file here, so a second one would add to these.
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        let capability: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("default.json")).unwrap()).unwrap();
        assert_eq!(
            capability["permissions"],
            serde_json::json!([
                "core:window:default",
                "dialog:allow-open",
                "dialog:allow-save"
            ])
        );
    }

    // A plain #[tauri::command] runs on the main thread and freezes the window (ARCHITECTURE §4.3).
    #[test]
    fn every_command_runs_off_the_main_thread() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands");
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let source = fs::read_to_string(&path).unwrap();
            for line in source
                .lines()
                .filter(|line| line.trim_start().starts_with("#[tauri::command"))
            {
                assert!(line.contains("async"), "{}: {line}", path.display());
            }
        }
    }
}
