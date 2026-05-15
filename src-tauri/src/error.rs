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
    #[error("history hash mismatch for {file}:{line} ({event_id})")]
    HistoryHashMismatch {
        event_id: String,
        file: String,
        line: usize,
    },
    #[allow(dead_code)]
    #[error("history source file missing")]
    HistoryFileMissing,
    #[allow(dead_code)]
    #[error("history storage disabled")]
    HistoryDisabled,
    #[error("external edit in undo range (count={count})")]
    ExternalInUndoRange { count: usize },
}

impl AppError {
    /// Stable machine-readable code for the frontend. Lives separately from
    /// the user-facing `Display` so we can refactor messages without breaking
    /// `e.code === 'X'` checks.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Io(_) => "IO",
            AppError::Json(_) => "JSON",
            AppError::Watcher(_) => "WATCHER",
            AppError::NoSources => "NO_SOURCES",
            AppError::SourceNotFound(_) => "SOURCE_NOT_FOUND",
            AppError::DuplicateSource(_) => "DUPLICATE_SOURCE",
            AppError::InvalidSourcePath(_) => "INVALID_SOURCE_PATH",
            AppError::TaskNotFound(_) => "TASK_NOT_FOUND",
            AppError::NotATaskLine { .. } => "NOT_A_TASK_LINE",
            AppError::CommandFailed(_) => "COMMAND_FAILED",
            AppError::QuadrantHeaderMissing(_) => "QUADRANT_HEADER_MISSING",
            AppError::HistoryHashMismatch { .. } => "HISTORY_HASH_MISMATCH",
            AppError::HistoryFileMissing => "HISTORY_FILE_MISSING",
            AppError::HistoryDisabled => "HISTORY_DISABLED",
            AppError::ExternalInUndoRange { .. } => "EXTERNAL_IN_UNDO_RANGE",
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

// Tauri commands need a Serialize-friendly error. We emit an object with
// `code`, `message`, and optional structured fields so the frontend can
// match on `e.code` without parsing the human-readable message.
impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = s.serialize_map(None)?;
        map.serialize_entry("code", self.code())?;
        map.serialize_entry("message", &self.to_string())?;
        if let AppError::ExternalInUndoRange { count } = self {
            map.serialize_entry("count", count)?;
        }
        if let AppError::HistoryHashMismatch { event_id, file, line } = self {
            map.serialize_entry("event_id", event_id)?;
            map.serialize_entry("file", file)?;
            map.serialize_entry("line", line)?;
        }
        map.end()
    }
}
