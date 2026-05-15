use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("watcher error: {0}")]
    Watcher(#[from] notify::Error),
    #[error("no sources configured")]
    NoSources,
    #[error("source not found: {0}")]
    SourceNotFound(String),
    #[error("source already exists: {0}")]
    DuplicateSource(String),
    #[error("invalid source path: {0}")]
    InvalidSourcePath(String),
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("line {line} in {path} is not a task line")]
    NotATaskLine { path: String, line: usize },
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("quadrant header missing for {0:?}")]
    QuadrantHeaderMissing(crate::types::Quadrant),
}

pub type Result<T> = std::result::Result<T, AppError>;

// Tauri commands need a Serialize-friendly error
impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
