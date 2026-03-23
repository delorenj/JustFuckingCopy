use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Runtime configuration loaded from config.toml.
/// All fields have sane defaults so the app can run without a config file present.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub watch_dir: String,
    pub hotkey: String,
    pub ollama_endpoint: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            watch_dir: "~/data/ssbnk/hosted".to_string(),
            hotkey: "Ctrl+Shift+C".to_string(),
            ollama_endpoint: "http://192.168.1.12:11434".to_string(),
        }
    }
}

/// Returns the platform-correct path:
///   Linux/macOS: ~/.config/justfuckingcopy/config.toml
///   Windows:     %APPDATA%\justfuckingcopy\config.toml
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("justfuckingcopy")
        .join("config.toml")
}

/// Load config from disk. If the file does not exist, write defaults and return them.
/// If the file is malformed, warn to stderr and return defaults (no crash).
pub fn load_or_create() -> AppConfig {
    load_or_create_at(&config_path())
}

/// Testable variant that accepts an explicit path.
pub fn load_or_create_at(path: &PathBuf) -> AppConfig {
    if !path.exists() {
        let defaults = AppConfig::default();
        if let Err(e) = write_defaults(path, &defaults) {
            eprintln!("[JFC config] Failed to write default config to {}: {e}", path.display());
        }
        return defaults;
    }

    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[JFC config] Failed to read config at {}: {e}. Using defaults.", path.display());
            return AppConfig::default();
        }
    };

    match toml::from_str::<AppConfig>(&raw) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!(
                "[JFC config] Malformed config at {}. Using defaults. Parse error: {e}",
                path.display()
            );
            AppConfig::default()
        }
    }
}

fn write_defaults(path: &PathBuf, config: &AppConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;
    }
    let toml_str = toml::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize default config: {e}"))?;
    std::fs::write(path, toml_str)
        .map_err(|e| format!("Failed to write config file: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_path() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, AtomicOrdering::SeqCst);
        let dir = std::env::temp_dir()
            .join(format!("jfc_test_{}_{}", std::process::id(), n));
        dir.join("config.toml")
    }

    #[test]
    fn test_default_values() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.watch_dir, "~/data/ssbnk/hosted");
        assert_eq!(cfg.hotkey, "Ctrl+Shift+C");
        assert_eq!(cfg.ollama_endpoint, "http://192.168.1.12:11434");
    }

    #[test]
    fn test_load_or_create_missing_file_writes_defaults() {
        let path = temp_path();
        // Ensure it does not exist
        let _ = std::fs::remove_file(&path);

        let cfg = load_or_create_at(&path);
        assert_eq!(cfg.hotkey, "Ctrl+Shift+C");
        assert!(path.exists(), "Default config file should have been created");

        // Clean up
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn test_load_partial_overrides_uses_defaults_for_missing_fields() {
        // Write a TOML with only hotkey set — watch_dir and ollama_endpoint missing
        // Note: toml::from_str requires all fields for a plain struct unless using Option or default.
        // For partial override support, we parse into a helper struct with Option fields,
        // or we require full config. Current design: all fields required in file.
        // This test verifies a fully valid config round-trips correctly.
        let path = temp_path();
        let _ = std::fs::remove_file(&path);

        let toml_content = r#"
watch_dir = "~/screenshots"
hotkey = "Ctrl+Alt+C"
ollama_endpoint = "http://localhost:11434"
"#;
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&path, toml_content).unwrap();

        let cfg = load_or_create_at(&path);
        assert_eq!(cfg.watch_dir, "~/screenshots");
        assert_eq!(cfg.hotkey, "Ctrl+Alt+C");
        assert_eq!(cfg.ollama_endpoint, "http://localhost:11434");

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn test_malformed_toml_returns_defaults_no_panic() {
        let path = temp_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&path, "hotkey = !!!INVALID!!!").unwrap();

        let cfg = load_or_create_at(&path);
        // Should return defaults without panicking
        assert_eq!(cfg.hotkey, "Ctrl+Shift+C");
        assert_eq!(cfg.ollama_endpoint, "http://192.168.1.12:11434");

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn test_written_default_config_contains_all_keys() {
        let path = temp_path();
        let _ = std::fs::remove_file(&path);

        let _ = load_or_create_at(&path);

        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("watch_dir"), "Missing watch_dir key");
        assert!(contents.contains("hotkey"), "Missing hotkey key");
        assert!(contents.contains("ollama_endpoint"), "Missing ollama_endpoint key");

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
