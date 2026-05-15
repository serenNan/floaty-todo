use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Quadrant {
    UrgentImportant,
    NotUrgentImportant,
    UrgentNotImportant,
    NotUrgentNotImportant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub text: String,
    pub completed: bool,
    pub source_file: PathBuf,
    pub line_number: usize, // 1-indexed
    pub indent: usize,
    /// Which `Source` this task belongs to (for UI grouping and per-source actions).
    pub source_id: String,
    #[serde(default)]
    pub quadrant: Option<Quadrant>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Folder,
    File,
}

/// One of the built-in quick-action launchers shown on a source's header.
/// Future custom actions can be added without churning the enum by introducing
/// a `Custom { command, args }` variant.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum QuickActionKind {
    Vscode,
    Terminal,
    ClaudeCode,
    /// Open the source's path in the OS file manager (Explorer, Finder,
    /// or `xdg-open`). Operates on the path itself for folders, on the
    /// containing directory for files.
    Reveal,
}

fn default_true() -> bool { true }

/// Defaults shipped to brand-new users — Reveal + VS Code + terminal.
/// User toggles them in Settings → Quick actions.
pub fn default_quick_actions() -> Vec<QuickActionKind> {
    vec![
        QuickActionKind::Reveal,
        QuickActionKind::Vscode,
        QuickActionKind::Terminal,
    ]
}

/// A user-configured task source — either a folder (recursive `.md` scan)
/// or a single `.md` file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Source {
    /// Stable id derived from `path` (8-byte hex of SHA-256). Used by the
    /// frontend to refer to a source after the user renames its label.
    pub id: String,
    pub path: PathBuf,
    pub kind: SourceKind,
    /// Optional display name. `None` → derive from `path` filename.
    pub label: Option<String>,
    /// Where to "Open in VS Code" / "Open terminal here" jumps to.
    /// `None` → default to `path` (Folder) or `path.parent()` (File).
    pub project_root: Option<PathBuf>,
    /// Optional accent color (CSS hex, e.g. "#ef4444"). The UI uses it as
    /// a left-edge stripe + soft header tint so users can scan the source
    /// list visually. `None` = no tint.
    #[serde(default)]
    pub color: Option<String>,
}

impl Source {
    /// Compute the stable id for a given absolute path. The path is run
    /// through `dunce::simplified` first so a verbatim form (`\\?\D:\...`)
    /// and its friendly form (`D:\...`) hash to the same id.
    pub fn id_for(path: &std::path::Path) -> String {
        let cleaned = dunce::simplified(path);
        hex::encode(&hash_content(cleaned.to_string_lossy().as_bytes())[..8])
    }

    /// Effective project root for shell actions, applying the default.
    pub fn effective_project_root(&self) -> PathBuf {
        if let Some(p) = &self.project_root {
            return p.clone();
        }
        match self.kind {
            SourceKind::Folder => self.path.clone(),
            SourceKind::File => self
                .path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| self.path.clone()),
        }
    }

}

/// 两个全局快捷键的 accelerator 字符串。`None` = 该键未绑定/已禁用。
/// 字符串是 Tauri accelerator 语法（如 "CmdOrCtrl+Shift+T"）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HotkeyConfig {
    pub toggle: Option<String>,
    pub quick_add: Option<String>,
}

impl HotkeyConfig {
    /// serde `default` 用 —— 旧 config.json 没有 `hotkeys` 字段时落到这里。
    pub fn defaults() -> Self {
        HotkeyConfig {
            toggle: Some("CmdOrCtrl+Shift+T".into()),
            quick_add: Some("CmdOrCtrl+Shift+A".into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    /// User-configured task sources. Replaces the v0.1 single `vault_path`.
    #[serde(default)]
    pub sources: Vec<Source>,
    /// Which source receives `add_task` from the QuickAdd input.
    /// `None` → frontend must pick one explicitly.
    #[serde(default)]
    pub default_source_id: Option<String>,
    /// Filename appended inside a Folder source's root when adding from QuickAdd.
    /// For File sources, `add_task` appends to the source file itself.
    pub inbox_file: String,
    pub always_on_top: bool,
    /// User-defined display labels keyed by file path (string form of the
    /// canonicalised, dunce-simplified absolute path). Lets the UI rename
    /// individual files inside a Folder source — two `todo.md` files in
    /// different sub-folders can be told apart at a glance.
    #[serde(default)]
    pub file_labels: HashMap<String, String>,
    /// Which quick-action buttons to render on each source header, in order.
    /// Empty = no action buttons. Persisted so the user only configures once.
    #[serde(default = "default_quick_actions")]
    pub enabled_quick_actions: Vec<QuickActionKind>,
    /// Central folder where every configured source is mirrored — file
    /// sources via hard link, folder sources via directory junction
    /// (Windows). `None` = feature off (default). When set, AI tools and
    /// scripts can find every project's TODO inside one folder instead
    /// of crawling each repo individually.
    #[serde(default)]
    pub hub_folder: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub auto_create_quadrant_headers: bool,
    /// 全局快捷键绑定。`#[serde(default)]` 保证旧 config 能加载。
    #[serde(default = "HotkeyConfig::defaults")]
    pub hotkeys: HotkeyConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            default_source_id: None,
            inbox_file: "inbox.md".into(),
            always_on_top: true,
            file_labels: HashMap::new(),
            enabled_quick_actions: default_quick_actions(),
            hub_folder: None,
            auto_create_quadrant_headers: true,
            hotkeys: HotkeyConfig::defaults(),
        }
    }
}

/// Map a file path to the key used in `AppConfig::file_labels`. Always
/// goes through `dunce::simplified` so verbatim and friendly forms agree.
pub fn file_label_key(path: &std::path::Path) -> String {
    dunce::simplified(path).to_string_lossy().to_string()
}

/// 32-byte SHA-256 of file contents — used for watcher loop prevention.
pub type ContentHash = [u8; 32];

pub fn hash_content(bytes: &[u8]) -> ContentHash {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadrant_serializes_as_snake_case() {
        let v = serde_json::to_string(&Quadrant::UrgentImportant).unwrap();
        assert_eq!(v, "\"urgent_important\"");
        let v = serde_json::to_string(&Quadrant::NotUrgentNotImportant).unwrap();
        assert_eq!(v, "\"not_urgent_not_important\"");
    }

    #[test]
    fn task_quadrant_serializes_when_set() {
        let t = Task {
            id: "abc".into(),
            text: "hi".into(),
            completed: false,
            source_file: std::path::PathBuf::from("/x.md"),
            line_number: 1,
            indent: 0,
            source_id: "s".into(),
            quadrant: Some(Quadrant::UrgentImportant),
        };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("\"quadrant\":\"urgent_important\""));
    }

    #[test]
    fn task_quadrant_deserializes_missing_as_none() {
        let json = r#"{"id":"a","text":"hi","completed":false,"source_file":"/x.md","line_number":1,"indent":0,"source_id":"s"}"#;
        let t: Task = serde_json::from_str(json).unwrap();
        assert!(t.quadrant.is_none());
    }

    #[test]
    fn config_defaults_auto_create_headers_to_true() {
        let c = AppConfig::default();
        assert!(c.auto_create_quadrant_headers);
    }

    #[test]
    fn config_deserializes_missing_auto_create_as_true() {
        let json = r#"{"sources":[],"inbox_file":"inbox.md","always_on_top":true}"#;
        let c: AppConfig = serde_json::from_str(json).unwrap();
        assert!(c.auto_create_quadrant_headers);
    }

    #[test]
    fn hotkey_config_has_sensible_defaults() {
        let h = HotkeyConfig::defaults();
        assert_eq!(h.toggle.as_deref(), Some("CmdOrCtrl+Shift+T"));
        assert_eq!(h.quick_add.as_deref(), Some("CmdOrCtrl+Shift+A"));
    }

    #[test]
    fn config_defaults_include_hotkeys() {
        let c = AppConfig::default();
        assert_eq!(c.hotkeys.toggle.as_deref(), Some("CmdOrCtrl+Shift+T"));
    }

    #[test]
    fn config_deserializes_missing_hotkeys_as_defaults() {
        let json = r#"{"sources":[],"inbox_file":"inbox.md","always_on_top":true}"#;
        let c: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(c.hotkeys.toggle.as_deref(), Some("CmdOrCtrl+Shift+T"));
        assert_eq!(c.hotkeys.quick_add.as_deref(), Some("CmdOrCtrl+Shift+A"));
    }

    #[test]
    fn hotkey_config_roundtrips_with_cleared_key() {
        let h = HotkeyConfig { toggle: Some("Ctrl+Alt+X".into()), quick_add: None };
        let json = serde_json::to_string(&h).unwrap();
        let back: HotkeyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.toggle, h.toggle);
        assert_eq!(back.quick_add, None);
    }
}
