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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub vault_path: Option<PathBuf>,
    pub inbox_file: String,
    pub always_on_top: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            vault_path: None,
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
