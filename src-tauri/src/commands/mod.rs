pub mod accounts;
pub mod categories;
pub mod payment_methods;
pub mod settings;
pub mod system;

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

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
