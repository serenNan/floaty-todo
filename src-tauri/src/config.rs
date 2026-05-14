use crate::error::Result;
use crate::types::{AppConfig, Source};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Determine the OS-specific config file path. Tauri normally provides this via
/// `app_handle.path().app_config_dir()`, but tests need a pure function.
pub fn load_from(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = std::fs::read_to_string(path)?;
    let mut cfg: AppConfig = match serde_json::from_str(&raw) {
        Ok(c) => c,
        Err(_) => return Ok(AppConfig::default()), // tolerant: corrupt -> defaults
    };
    normalize_paths(&mut cfg);
    Ok(cfg)
}

/// Strip Windows verbatim prefixes (`\\?\`) from every stored source path
/// and recompute ids so they line up with the freshly normalized form.
/// Idempotent — running on an already-clean config is a no-op.
fn normalize_paths(cfg: &mut AppConfig) {
    let mut id_remap: HashMap<String, String> = HashMap::new();
    for s in &mut cfg.sources {
        let cleaned = dunce::simplified(&s.path).to_path_buf();
        let cleaned_root = s
            .project_root
            .as_ref()
            .map(|p| dunce::simplified(p).to_path_buf());
        let new_id = Source::id_for(&cleaned);
        if new_id != s.id {
            id_remap.insert(s.id.clone(), new_id.clone());
        }
        s.path = cleaned;
        s.project_root = cleaned_root;
        s.id = new_id;
    }
    if let Some(d) = cfg.default_source_id.as_ref() {
        if let Some(new_id) = id_remap.get(d) {
            cfg.default_source_id = Some(new_id.clone());
        }
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
        use crate::types::{Source, SourceKind};
        let d = TempDir::new().unwrap();
        let p = d.path().join("config.json");
        let vault = d.path().join("vault");
        let mut cfg = AppConfig::default();
        cfg.sources.push(Source {
            id: Source::id_for(&vault),
            path: vault,
            kind: SourceKind::Folder,
            label: Some("Vault".into()),
            project_root: None,
        });
        cfg.default_source_id = Some(cfg.sources[0].id.clone());
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

    #[test]
    #[cfg(windows)]
    fn load_strips_verbatim_prefix_and_remaps_default_id() {
        use crate::types::{Source, SourceKind};
        let d = TempDir::new().unwrap();
        let p = d.path().join("config.json");

        // Hand-craft a legacy config with a \\?\ prefixed path + matching id.
        let verbatim = PathBuf::from(r"\\?\D:\Projects\WishTalk");
        let legacy_id_input = verbatim.to_string_lossy().to_string();
        let legacy_id = hex::encode(
            &crate::types::hash_content(legacy_id_input.as_bytes())[..8],
        );
        let cfg = AppConfig {
            sources: vec![Source {
                id: legacy_id.clone(),
                path: verbatim.clone(),
                kind: SourceKind::Folder,
                label: None,
                project_root: Some(verbatim.clone()),
            }],
            default_source_id: Some(legacy_id.clone()),
            inbox_file: "inbox.md".into(),
            always_on_top: true,
            file_labels: HashMap::new(),
        };
        save_to(&p, &cfg).unwrap();

        let got = load_from(&p).unwrap();
        let cleaned = PathBuf::from(r"D:\Projects\WishTalk");
        assert_eq!(got.sources[0].path, cleaned);
        assert_eq!(got.sources[0].project_root, Some(cleaned.clone()));
        // id must follow the cleaned path
        assert_eq!(got.sources[0].id, Source::id_for(&cleaned));
        assert_eq!(got.default_source_id, Some(got.sources[0].id.clone()));
        // Idempotency: a second load is a no-op
        let again = load_from(&p).unwrap();
        assert_eq!(again, got);
    }
}
