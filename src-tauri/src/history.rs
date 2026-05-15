use crate::error::{AppError, Result};
use crate::storage;
use crate::types::Quadrant;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskLineState {
    pub done: bool,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quadrant: Option<Quadrant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LineSnapshot {
    pub line: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<TaskLineState>,
    pub raw: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffSummary {
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HistoryAction {
    Toggle {
        task_id: String,
        before: LineSnapshot,
        after: LineSnapshot,
    },
    Edit {
        task_id: String,
        before: LineSnapshot,
        after: LineSnapshot,
    },
    Add {
        after: LineSnapshot,
    },
    Move {
        task_id: String,
        before: LineSnapshot,
        after: LineSnapshot,
    },
    Delete {
        task_id: String,
        before: LineSnapshot,
    },
    ExternalEdit {
        diff_summary: DiffSummary,
        size_bytes_delta: i64,
        note: String,
    },
}

impl HistoryAction {
    pub fn is_external(&self) -> bool {
        matches!(self, HistoryAction::ExternalEdit { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryEvent {
    pub id: String,
    pub ts: String,
    pub source_id: String,
    pub file: PathBuf,
    #[serde(flatten)]
    pub action: HistoryAction,
}

fn current_ts() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

impl HistoryEvent {
    pub fn new(source_id: String, file: PathBuf, action: HistoryAction) -> Self {
        Self {
            id: ulid::Ulid::new().to_string(),
            ts: current_ts(),
            source_id,
            file,
            action,
        }
    }
}

/// Throttle window for merging consecutive external edits on the same file.
/// Matches the spec: "500ms 内同文件多次外部编辑合并为一条事件".
const EXTERNAL_MERGE_WINDOW: Duration = Duration::from_millis(500);

pub struct HistoryStore {
    file_path: PathBuf,
    cursor_path: PathBuf,
    events: Vec<HistoryEvent>,
    cursor: usize,
    /// For each watched file, the id of the most recent ExternalEdit event,
    /// used to merge subsequent edits inside the throttle window. Keyed by
    /// event id (not index) so truncation can't corrupt the lookup.
    external_last: HashMap<PathBuf, String>,
    /// Wall-clock timestamp of the last external edit per file (in-process
    /// only — not persisted; restarts always start a fresh throttle window).
    external_last_at: HashMap<PathBuf, Instant>,
    /// Last-known content for every watched file. Used to compute the
    /// `diff_summary` carried by ExternalEdit events. Updated on every
    /// successful write (via `record_file_snapshot`) and on every external
    /// edit we observe.
    snapshots: HashMap<PathBuf, Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpDirection {
    Undo,
    Redo,
}

#[derive(Debug, Clone)]
pub struct JumpPlan {
    pub direction: JumpDirection,
    pub events: Vec<HistoryEvent>,
    pub external_count: usize,
}

impl HistoryStore {
    pub fn open(hub: Option<&Path>, app_data: &Path) -> Result<Self> {
        let dir = hub.unwrap_or(app_data);
        fs::create_dir_all(dir)?;
        let file_path = dir.join(".floaty-history.jsonl");
        let cursor_path = dir.join(".floaty-history.cursor");
        let events = load_events(&file_path)?;
        let cursor = load_cursor(&cursor_path, &events)?;
        // Rebuild the per-file "last external event" index from history so
        // restart-time merging stays consistent (only the throttle window
        // itself is in-process state).
        let mut external_last = HashMap::new();
        for event in events.iter() {
            if event.action.is_external() {
                external_last.insert(canonical_key(&event.file), event.id.clone());
            }
        }
        Ok(Self {
            file_path,
            cursor_path,
            events,
            cursor,
            external_last,
            external_last_at: HashMap::new(),
            snapshots: HashMap::new(),
        })
    }

    /// Record the new content of `path` after a successful write. The next
    /// external edit on this path will diff against this snapshot. Callers
    /// should invoke this after their atomic_write in `storage::*`. Path is
    /// canonicalised so callers from registry/watcher/walkdir all share the
    /// same map key even if they hand in slightly different path shapes
    /// (drive-letter case, `\\?\` prefix, junction-through-hub vs source).
    pub fn record_file_snapshot(&mut self, path: &Path, bytes: Vec<u8>) {
        self.snapshots.insert(canonical_key(path), bytes);
    }

    pub fn push(&mut self, event: HistoryEvent) -> Result<()> {
        if self.cursor < self.events.len() {
            // Truncating the redo branch — full rewrite for atomicity.
            let truncated_ids: Vec<String> = self.events[self.cursor..]
                .iter()
                .map(|e| e.id.clone())
                .collect();
            self.events.truncate(self.cursor);
            self.events.push(event);
            self.cursor = self.events.len();
            self.persist_events()?;
            // Prune external_last entries that pointed to truncated events.
            self.external_last.retain(|_, id| !truncated_ids.contains(id));
        } else {
            // Pure append — fast path that doesn't rewrite the whole file.
            self.events.push(event);
            self.cursor = self.events.len();
            self.append_event(self.events.last().unwrap())?;
        }
        self.persist_cursor()?;
        Ok(())
    }

    /// Push a new ExternalEdit for `file` against `new_bytes`, or merge into
    /// an existing recent event if we're inside the throttle window. The diff
    /// summary is computed from the cached previous snapshot — first edit
    /// after startup reports `added = new line count`, which is honest.
    pub fn push_external_edit(
        &mut self,
        source_id: String,
        file: PathBuf,
        new_bytes: Vec<u8>,
    ) -> Result<()> {
        let key = canonical_key(&file);
        // Belt-and-suspenders against phantom external edits. Even if the
        // watcher's IgnoreHashes guard misses (e.g. notify fires multiple
        // events for the same write and only the first consumes the hash),
        // here we check the file content directly against the snapshot we
        // recorded at write time. Identical → nothing actually changed
        // outside the app, skip emit.
        if let Some(prev) = self.snapshots.get(&key) {
            if prev.as_slice() == new_bytes.as_slice() {
                return Ok(());
            }
        } else {
            // No baseline yet — file appeared after startup scan. Establish
            // baseline silently so the NEXT real edit reports a clean diff,
            // instead of emitting a misleading "added = entire file" event.
            self.snapshots.insert(key, new_bytes);
            return Ok(());
        }
        let prev = self.snapshots.get(&key).map(|v| v.as_slice()).unwrap_or(b"");
        let summary = approx_diff(prev, &new_bytes);
        let size_delta = new_bytes.len() as i64 - prev.len() as i64;
        // Snapshot the new content before we try to merge — even if the merge
        // path early-exits, future edits should diff against this new state.
        self.snapshots.insert(key.clone(), new_bytes);

        let now = Instant::now();
        let in_window = self
            .external_last_at
            .get(&key)
            .map(|prev_at| now.duration_since(*prev_at) < EXTERNAL_MERGE_WINDOW)
            .unwrap_or(false);

        if in_window {
            if let Some(existing_id) = self.external_last.get(&key).cloned() {
                if let Some(event) = self.events.iter_mut().find(|e| e.id == existing_id) {
                    if let HistoryAction::ExternalEdit {
                        diff_summary,
                        size_bytes_delta,
                        ..
                    } = &mut event.action
                    {
                        diff_summary.added += summary.added;
                        diff_summary.removed += summary.removed;
                        diff_summary.modified += summary.modified;
                        *size_bytes_delta += size_delta;
                    }
                    event.ts = current_ts();
                    self.external_last_at.insert(key, now);
                    // Merge mutates an existing line in the JSONL — must rewrite.
                    self.persist_events()?;
                    return Ok(());
                }
            }
        }

        // Either no recent event, or throttle expired, or the previous event
        // was truncated by an interceding push. Emit a fresh one.
        let event = HistoryEvent::new(
            source_id,
            file,
            HistoryAction::ExternalEdit {
                diff_summary: summary,
                size_bytes_delta: size_delta,
                note: "File changed outside Floaty Todo".to_string(),
            },
        );
        let event_id = event.id.clone();
        self.push(event)?;
        self.external_last.insert(key.clone(), event_id);
        self.external_last_at.insert(key, now);
        Ok(())
    }

    /// Read-only inspection: the next event a UI-driven undo would target.
    /// Caller validates the operation, then calls `commit_undo` to advance.
    pub fn peek_undo(&self) -> Option<HistoryEvent> {
        let idx = self.events[..self.cursor]
            .iter()
            .rposition(|event| !event.action.is_external())?;
        self.events.get(idx).cloned()
    }

    /// Move the cursor to just before `event_id`. Cursor adjustment also
    /// skips over any external events sitting between the cursor and that
    /// event (they remain in the timeline but are no longer "in the future").
    pub fn commit_undo(&mut self, event_id: &str) -> Result<()> {
        let idx = self
            .events
            .iter()
            .position(|event| event.id == event_id)
            .ok_or_else(|| AppError::CommandFailed("history event not found".into()))?;
        self.cursor = idx;
        self.persist_cursor()?;
        Ok(())
    }

    /// Read-only inspection of the next redo candidate.
    pub fn peek_redo(&self) -> Option<HistoryEvent> {
        let relative_idx = self.events[self.cursor..]
            .iter()
            .position(|event| !event.action.is_external())?;
        self.events.get(self.cursor + relative_idx).cloned()
    }

    /// Move the cursor to just after `event_id`.
    pub fn commit_redo(&mut self, event_id: &str) -> Result<()> {
        let idx = self
            .events
            .iter()
            .position(|event| event.id == event_id)
            .ok_or_else(|| AppError::CommandFailed("history event not found".into()))?;
        self.cursor = idx + 1;
        self.persist_cursor()?;
        Ok(())
    }

    pub fn list(&self, limit: usize, before_id: Option<&str>) -> Vec<HistoryEvent> {
        let end = before_id
            .and_then(|id| self.events.iter().position(|e| e.id == id))
            .unwrap_or(self.events.len());
        self.events[..end].iter().rev().take(limit).cloned().collect()
    }

    pub fn cursor_id(&self) -> Option<&str> {
        self.cursor
            .checked_sub(1)
            .and_then(|idx| self.events.get(idx))
            .map(|event| event.id.as_str())
    }

    pub fn cursor_id_owned(&self) -> Option<String> {
        self.cursor_id().map(str::to_string)
    }

    pub fn reopen(&mut self, hub: Option<&Path>, app_data: &Path) -> Result<()> {
        *self = Self::open(hub, app_data)?;
        Ok(())
    }

    pub fn jump_plan(&self, event_id: &str) -> Option<JumpPlan> {
        let target_idx = self.events.iter().position(|event| event.id == event_id)?;
        let current_idx = self.cursor.checked_sub(1);
        if current_idx == Some(target_idx) {
            return Some(JumpPlan {
                direction: JumpDirection::Redo,
                events: Vec::new(),
                external_count: 0,
            });
        }

        let (direction, events) = match current_idx {
            Some(idx) if target_idx < idx => {
                let mut events = self.events[target_idx + 1..=idx].to_vec();
                events.reverse();
                (JumpDirection::Undo, events)
            }
            _ => {
                let start = self.cursor.min(self.events.len());
                (JumpDirection::Redo, self.events[start..=target_idx].to_vec())
            }
        };
        let external_count = events.iter().filter(|event| event.action.is_external()).count();
        Some(JumpPlan {
            direction,
            events,
            external_count,
        })
    }

    /// Atomic full rewrite of the JSONL file via tempfile + rename. Used when
    /// in-place mutation (truncate-redo, merge-external) makes append-only
    /// impossible. The rename is atomic on the same volume so a crash leaves
    /// either the old or new file intact, never half-written garbage.
    fn persist_events(&self) -> Result<()> {
        let parent = self
            .file_path
            .parent()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent dir"))?;
        fs::create_dir_all(parent)?;
        let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
        {
            let mut writer = std::io::BufWriter::new(tmp.as_file_mut());
            for event in &self.events {
                serde_json::to_writer(&mut writer, event)?;
                writer.write_all(b"\n")?;
            }
            writer.flush()?;
        }
        tmp.as_file().sync_all()?;
        tmp.persist(&self.file_path)
            .map_err(|e| AppError::Io(e.error))?;
        Ok(())
    }

    /// Append-only fast path for the common case (push at end, no truncation).
    /// Avoids rewriting megabytes of history every time you tick a checkbox.
    fn append_event(&self, event: &HistoryEvent) -> Result<()> {
        let parent = self
            .file_path
            .parent()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent dir"))?;
        fs::create_dir_all(parent)?;
        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.file_path)?;
        serde_json::to_writer(&mut file, event)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        Ok(())
    }

    /// Cursor file written via tempfile + rename. It's a tiny single-line
    /// file but the same atomicity principle applies — a half-written cursor
    /// after a crash would silently re-do/re-undo events on next startup.
    fn persist_cursor(&self) -> Result<()> {
        let parent = self
            .cursor_path
            .parent()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent dir"))?;
        fs::create_dir_all(parent)?;
        let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
        let id = self.cursor_id().unwrap_or("");
        tmp.as_file_mut().write_all(id.as_bytes())?;
        tmp.as_file().sync_all()?;
        tmp.persist(&self.cursor_path)
            .map_err(|e| AppError::Io(e.error))?;
        Ok(())
    }
}

/// Crude line-count diff. Modified is left as 0 — computing it properly
/// requires a real LCS / Myers diff which we don't need for a personal
/// activity log. (added/removed alone already tells the user "VS Code
/// changed X / removed Y" which is the actionable info.)
fn approx_diff(old: &[u8], new: &[u8]) -> DiffSummary {
    if old == new {
        return DiffSummary { added: 0, removed: 0, modified: 0 };
    }
    let count = |b: &[u8]| b.iter().filter(|c| **c == b'\n').count();
    let old_lines = count(old);
    let new_lines = count(new);
    DiffSummary {
        added: new_lines.saturating_sub(old_lines),
        removed: old_lines.saturating_sub(new_lines),
        modified: 0,
    }
}

/// Best-effort canonicalisation for using a file path as a HashMap key. We
/// don't care about the failure case (file deleted etc.) — we just need a
/// stable shape so writes from `commands` and watcher events from `lib.rs`
/// can find each other in the snapshot cache.
fn canonical_key(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn load_events(path: &Path) -> Result<Vec<HistoryEvent>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut events = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<HistoryEvent>(&line) {
            events.push(event);
        }
    }
    Ok(events)
}

fn load_cursor(path: &Path, events: &[HistoryEvent]) -> Result<usize> {
    if !path.exists() {
        return Ok(events.len());
    }
    let id = fs::read_to_string(path)?.trim().to_string();
    if id.is_empty() {
        return Ok(0);
    }
    Ok(events
        .iter()
        .position(|event| event.id == id)
        .map(|idx| idx + 1)
        .unwrap_or(events.len()))
}

pub fn apply_reverse(event: &HistoryEvent) -> Result<Option<crate::types::ContentHash>> {
    let result = match &event.action {
        HistoryAction::Toggle { before, after, .. } | HistoryAction::Edit { before, after, .. } => {
            Some(storage::replace_line_if_hash(
                &event.file,
                after.line,
                &after.hash,
                &before.raw,
            )?)
        }
        HistoryAction::Add { after } => Some(storage::remove_line_if_hash(
            &event.file,
            after.line,
            &after.hash,
        )?),
        HistoryAction::Move { before, after, .. } => {
            storage::remove_line_if_hash(&event.file, after.line, &after.hash)?;
            Some(storage::insert_line_at(&event.file, before.line, &before.raw)?)
        }
        HistoryAction::Delete { before, .. } => {
            Some(storage::insert_line_at(&event.file, before.line, &before.raw)?)
        }
        HistoryAction::ExternalEdit { .. } => None,
    };
    Ok(result)
}

pub fn apply_forward(event: &HistoryEvent) -> Result<Option<crate::types::ContentHash>> {
    let result = match &event.action {
        HistoryAction::Toggle { before, after, .. } | HistoryAction::Edit { before, after, .. } => {
            Some(storage::replace_line_if_hash(
                &event.file,
                before.line,
                &before.hash,
                &after.raw,
            )?)
        }
        HistoryAction::Add { after } => {
            Some(storage::insert_line_at(&event.file, after.line, &after.raw)?)
        }
        HistoryAction::Move { before, after, .. } => {
            storage::remove_line_if_hash(&event.file, before.line, &before.hash)?;
            Some(storage::insert_line_at(&event.file, after.line, &after.raw)?)
        }
        HistoryAction::Delete { before, .. } => Some(storage::remove_line_if_hash(
            &event.file,
            before.line,
            &before.hash,
        )?),
        HistoryAction::ExternalEdit { .. } => None,
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn line(line: usize, text: &str, done: bool) -> LineSnapshot {
        let marker = if done { "x" } else { " " };
        let raw = format!("- [{}] {}\n", marker, text);
        LineSnapshot {
            line,
            state: Some(TaskLineState {
                done,
                text: text.to_string(),
                quadrant: None,
            }),
            hash: format!("sha256:{}", hex::encode(crate::types::hash_content(raw.as_bytes()))),
            raw,
        }
    }

    fn event(id: &str, action: HistoryAction) -> HistoryEvent {
        HistoryEvent {
            id: id.to_string(),
            ts: "2026-05-15T00:00:00Z".to_string(),
            source_id: "source-a".to_string(),
            file: PathBuf::from("todo.md"),
            action,
        }
    }

    fn toggle(id: &str) -> HistoryEvent {
        event(
            id,
            HistoryAction::Toggle {
                task_id: "task-a".to_string(),
                before: line(1, "x", false),
                after: line(1, "x", true),
            },
        )
    }

    fn external(id: &str) -> HistoryEvent {
        event(
            id,
            HistoryAction::ExternalEdit {
                diff_summary: DiffSummary {
                    added: 1,
                    removed: 0,
                    modified: 0,
                },
                size_bytes_delta: 8,
                note: "external save".to_string(),
            },
        )
    }

    #[test]
    fn push_pop_undo_redo_moves_cursor_over_app_events() {
        let dir = TempDir::new().unwrap();
        let mut store = HistoryStore::open(None, dir.path()).unwrap();
        store.push(toggle("01")).unwrap();
        store.push(toggle("02")).unwrap();

        assert_eq!(store.cursor_id(), Some("02"));
        let next = store.peek_undo().unwrap();
        assert_eq!(next.id, "02");
        store.commit_undo(&next.id).unwrap();
        assert_eq!(store.cursor_id(), Some("01"));
        let next = store.peek_redo().unwrap();
        assert_eq!(next.id, "02");
        store.commit_redo(&next.id).unwrap();
        assert_eq!(store.cursor_id(), Some("02"));
    }

    #[test]
    fn push_after_undo_truncates_redo_branch() {
        let dir = TempDir::new().unwrap();
        let mut store = HistoryStore::open(None, dir.path()).unwrap();
        store.push(toggle("01")).unwrap();
        store.push(toggle("02")).unwrap();
        let next = store.peek_undo().unwrap();
        store.commit_undo(&next.id).unwrap();
        store.push(toggle("03")).unwrap();

        let ids: Vec<_> = store.list(10, None).into_iter().map(|e| e.id).collect();
        assert_eq!(ids, vec!["03", "01"]);
        assert!(store.peek_redo().is_none());
    }

    #[test]
    fn undo_redo_skips_external_edits_but_cursor_remains_timeline_based() {
        let dir = TempDir::new().unwrap();
        let mut store = HistoryStore::open(None, dir.path()).unwrap();
        store.push(toggle("01")).unwrap();
        store.push(external("02")).unwrap();
        store.push(toggle("03")).unwrap();

        let next = store.peek_undo().unwrap();
        assert_eq!(next.id, "03");
        store.commit_undo(&next.id).unwrap();
        // cursor lands at the position of the toggle we just undid (idx=2),
        // so cursor_id (events[cursor-1]) reports the external sandwiched
        // between the two toggles. "01" and "02_external" remain "applied".
        assert_eq!(store.cursor_id(), Some("02"));
        let next = store.peek_undo().unwrap();
        assert_eq!(next.id, "01");
        store.commit_undo(&next.id).unwrap();
        assert_eq!(store.cursor_id(), None);
        let next = store.peek_redo().unwrap();
        assert_eq!(next.id, "01");
        store.commit_redo(&next.id).unwrap();
        assert_eq!(store.cursor_id(), Some("01"));
    }

    #[test]
    fn open_loads_jsonl_and_cursor_and_skips_corrupt_lines() {
        let dir = TempDir::new().unwrap();
        let e1 = toggle("01");
        let e2 = toggle("02");
        std::fs::write(
            dir.path().join(".floaty-history.jsonl"),
            format!(
                "{}\nnot json\n{}\n",
                serde_json::to_string(&e1).unwrap(),
                serde_json::to_string(&e2).unwrap()
            ),
        )
        .unwrap();
        std::fs::write(dir.path().join(".floaty-history.cursor"), "01").unwrap();

        let store = HistoryStore::open(None, dir.path()).unwrap();
        assert_eq!(store.cursor_id(), Some("01"));
        let ids: Vec<_> = store.list(10, None).into_iter().map(|e| e.id).collect();
        assert_eq!(ids, vec!["02", "01"]);
    }

    #[test]
    fn apply_reverse_and_forward_toggle_are_hash_guarded() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("todo.md");
        std::fs::write(&file, "- [ ] x\n").unwrap();
        let mut event = toggle("01");
        event.file = file.clone();

        apply_forward(&event).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "- [x] x\n");

        apply_reverse(&event).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "- [ ] x\n");

        std::fs::write(&file, "- [ ] externally changed\n").unwrap();
        let err = apply_forward(&event).unwrap_err();
        assert!(matches!(err, crate::error::AppError::HistoryHashMismatch { .. }));
    }

    #[test]
    fn apply_reverse_add_removes_added_line_and_forward_reinserts_it() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("todo.md");
        std::fs::write(&file, "- [ ] existing\n- [ ] added\n").unwrap();
        let mut event = event(
            "01",
            HistoryAction::Add {
                after: line(2, "added", false),
            },
        );
        event.file = file.clone();

        apply_reverse(&event).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "- [ ] existing\n");

        apply_forward(&event).unwrap();
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "- [ ] existing\n- [ ] added\n"
        );
    }

    #[test]
    fn apply_forward_delete_removes_line_and_reverse_reinserts_it() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("todo.md");
        std::fs::write(&file, "- [ ] keep\n- [ ] gone\n").unwrap();
        let mut event = event(
            "01",
            HistoryAction::Delete {
                task_id: "task-a".to_string(),
                before: line(2, "gone", false),
            },
        );
        event.file = file.clone();

        apply_forward(&event).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "- [ ] keep\n");

        apply_reverse(&event).unwrap();
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "- [ ] keep\n- [ ] gone\n"
        );
    }

    #[test]
    fn apply_forward_delete_is_hash_guarded() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("todo.md");
        std::fs::write(&file, "- [ ] changed externally\n").unwrap();
        let mut event = event(
            "01",
            HistoryAction::Delete {
                task_id: "task-a".to_string(),
                before: line(1, "original text", false),
            },
        );
        event.file = file.clone();
        let err = apply_forward(&event).unwrap_err();
        assert!(matches!(err, crate::error::AppError::HistoryHashMismatch { .. }));
    }

    #[test]
    fn external_edits_within_throttle_window_merge_into_one_event() {
        let dir = TempDir::new().unwrap();
        let mut store = HistoryStore::open(None, dir.path()).unwrap();
        let file = PathBuf::from("todo.md");
        store.record_file_snapshot(&file, b"- [ ] a\n".to_vec());

        store
            .push_external_edit("src".into(), file.clone(), b"- [ ] a\n- [ ] b\n".to_vec())
            .unwrap();
        store
            .push_external_edit("src".into(), file.clone(), b"- [ ] a\n- [ ] b\n- [ ] c\n".to_vec())
            .unwrap();

        let events = store.list(10, None);
        assert_eq!(events.len(), 1, "second edit must merge into the first");
        if let HistoryAction::ExternalEdit { diff_summary, size_bytes_delta, .. } =
            &events[0].action
        {
            assert_eq!(diff_summary.added, 2, "two added lines total");
            assert!(*size_bytes_delta > 0);
        } else {
            panic!("expected ExternalEdit");
        }
    }

    #[test]
    fn external_edit_diff_summary_uses_recorded_snapshot() {
        let dir = TempDir::new().unwrap();
        let mut store = HistoryStore::open(None, dir.path()).unwrap();
        let file = PathBuf::from("todo.md");
        store.record_file_snapshot(&file, b"- [ ] a\n- [ ] b\n".to_vec());

        store
            .push_external_edit("src".into(), file.clone(), b"- [ ] a\n".to_vec())
            .unwrap();

        let events = store.list(10, None);
        if let HistoryAction::ExternalEdit { diff_summary, .. } = &events[0].action {
            assert_eq!(diff_summary.removed, 1);
            assert_eq!(diff_summary.added, 0);
        } else {
            panic!("expected ExternalEdit");
        }
    }

    #[test]
    fn external_edit_with_unchanged_content_is_dropped() {
        // Regression for "every App write produces a phantom external_edit":
        // when the watcher fires a duplicate notify event for our own write,
        // the file content matches the snapshot we recorded at write time,
        // and the event should be silently dropped.
        let dir = TempDir::new().unwrap();
        let mut store = HistoryStore::open(None, dir.path()).unwrap();
        let file = PathBuf::from("todo.md");
        store.record_file_snapshot(&file, b"- [x] done\n".to_vec());

        store
            .push_external_edit("src".into(), file.clone(), b"- [x] done\n".to_vec())
            .unwrap();
        store
            .push_external_edit("src".into(), file.clone(), b"- [x] done\n".to_vec())
            .unwrap();

        assert!(store.list(10, None).is_empty(), "no events for no-op saves");
    }

    #[test]
    fn first_external_edit_without_baseline_establishes_snapshot_silently() {
        let dir = TempDir::new().unwrap();
        let mut store = HistoryStore::open(None, dir.path()).unwrap();
        let file = PathBuf::from("todo.md");

        // No prior snapshot — first edit establishes baseline, no event.
        store
            .push_external_edit("src".into(), file.clone(), b"- [ ] x\n".to_vec())
            .unwrap();
        assert!(store.list(10, None).is_empty());

        // Real change against the now-established baseline → real event.
        store
            .push_external_edit("src".into(), file.clone(), b"- [ ] x\n- [ ] y\n".to_vec())
            .unwrap();
        let events = store.list(10, None);
        assert_eq!(events.len(), 1);
        if let HistoryAction::ExternalEdit { diff_summary, .. } = &events[0].action {
            assert_eq!(diff_summary.added, 1);
        } else {
            panic!("expected ExternalEdit");
        }
    }

    #[test]
    fn jump_plan_undo_redo_and_in_place() {
        let dir = TempDir::new().unwrap();
        let mut store = HistoryStore::open(None, dir.path()).unwrap();
        store.push(toggle("01")).unwrap();
        store.push(toggle("02")).unwrap();
        store.push(toggle("03")).unwrap();

        // jump to "01" from cursor at "03" → undo two events (in reverse: 03, 02).
        let plan = store.jump_plan("01").unwrap();
        assert_eq!(plan.direction, JumpDirection::Undo);
        let ids: Vec<_> = plan.events.iter().map(|e| e.id.clone()).collect();
        assert_eq!(ids, vec!["03", "02"]);

        // commit the undo to "01"
        store.commit_undo("02").unwrap();
        store.commit_undo("01").unwrap();
        // now cursor before "01" (= position 0); redo to "03" → events 01..=03 in forward order
        let plan = store.jump_plan("03").unwrap();
        assert_eq!(plan.direction, JumpDirection::Redo);
        let ids: Vec<_> = plan.events.iter().map(|e| e.id.clone()).collect();
        assert_eq!(ids, vec!["01", "02", "03"]);
    }
}
