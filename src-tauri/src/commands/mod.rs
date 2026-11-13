pub mod accounts;
pub mod backup;
pub mod budgets;
pub mod categories;
pub mod dashboard;
pub mod goals;
pub mod loans;
pub mod payment_methods;
pub mod period;
pub mod recurring;
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
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        // Tauri loads every capability file in the folder, and any written into the config.
        let files: Vec<String> = fs::read_dir(root.join("capabilities"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| {
                [".json", ".json5", ".toml"]
                    .iter()
                    .any(|e| name.ends_with(e))
            })
            .collect();
        assert_eq!(files, ["default.json"]);
        let read = |path: &str| -> serde_json::Value {
            serde_json::from_str(&fs::read_to_string(root.join(path)).unwrap()).unwrap()
        };
        assert!(read("tauri.conf.json")["app"]["security"]["capabilities"].is_null());
        let mut capability = read("capabilities/default.json");
        capability.as_object_mut().unwrap().remove("description");
        assert_eq!(
            capability,
            serde_json::json!({
                "$schema": "../gen/schemas/desktop-schema.json",
                "identifier": "default",
                "windows": ["main"],
                "permissions": ["core:window:default", "dialog:allow-open", "dialog:allow-save"]
            })
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
