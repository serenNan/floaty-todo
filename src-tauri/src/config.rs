use crate::error::Result;
use crate::types::AppConfig;
use std::path::{Path, PathBuf};

/// Determine the OS-specific config file path. Tauri normally provides this via
/// `app_handle.path().app_config_dir()`, but tests need a pure function.
pub fn load_from(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = std::fs::read_to_string(path)?;
    match serde_json::from_str(&raw) {
        Ok(c) => Ok(c),
        Err(_) => Ok(AppConfig::default()), // tolerant: corrupt config -> defaults
    }
}

pub fn save_to(path: &Path, config: &AppConfig) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(config)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn config_file(app_config_dir: &Path) -> PathBuf {
    app_config_dir.join("config.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_missing_returns_default() {
        let d = TempDir::new().unwrap();
        let cfg = load_from(&d.path().join("nope.json")).unwrap();
        assert_eq!(cfg, AppConfig::default());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("config.json");
        let mut cfg = AppConfig::default();
        cfg.vault_path = Some(d.path().join("vault"));
        cfg.always_on_top = false;
        save_to(&p, &cfg).unwrap();
        let got = load_from(&p).unwrap();
        assert_eq!(got, cfg);
    }

    #[test]
    fn corrupt_json_falls_back_to_default() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("config.json");
        std::fs::write(&p, "{not json").unwrap();
        let cfg = load_from(&p).unwrap();
        assert_eq!(cfg, AppConfig::default());
    }
}
