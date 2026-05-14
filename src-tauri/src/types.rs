use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Folder,
    File,
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
}

impl Source {
    /// Compute the stable id for a given absolute path.
    pub fn id_for(path: &std::path::Path) -> String {
        hex::encode(&hash_content(path.to_string_lossy().as_bytes())[..8])
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

    /// Display label, falling back to the filename of `path`.
    pub fn effective_label(&self) -> String {
        if let Some(l) = &self.label {
            if !l.trim().is_empty() {
                return l.clone();
            }
        }
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.path.to_string_lossy().to_string())
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
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            default_source_id: None,
            inbox_file: "inbox.md".into(),
            always_on_top: true,
        }
    }
}

/// 32-byte SHA-256 of file contents — used for watcher loop prevention.
pub type ContentHash = [u8; 32];

pub fn hash_content(bytes: &[u8]) -> ContentHash {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().into()
}
