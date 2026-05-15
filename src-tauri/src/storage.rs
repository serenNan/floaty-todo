use crate::error::{AppError, Result};
use crate::history::{LineSnapshot, TaskLineState};
use crate::parser::parse_line;
use crate::types::{hash_content, ContentHash, Quadrant};
use std::io::Write;
use std::path::Path;

/// Detect file's line ending. Defaults to LF on empty / unknown.
fn detect_line_ending(content: &str) -> &'static str {
    if content.contains("\r\n") { "\r\n" } else { "\n" }
}

/// Write file atomically (temp file + rename). Returns content hash.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<ContentHash> {
    let dir = path.parent().ok_or_else(|| std::io::Error::new(
        std::io::ErrorKind::InvalidInput, "no parent dir"))?;
    std::fs::create_dir_all(dir)?;
    let tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.as_file().write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| AppError::Io(e.error))?;
    Ok(hash_content(bytes))
}

#[derive(Debug, Clone)]
pub struct ToggleResult {
    pub new_hash: ContentHash,
    pub before: LineSnapshot,
    pub after: LineSnapshot,
}

#[derive(Debug, Clone)]
pub struct UpdateTextResult {
    pub new_hash: ContentHash,
    pub before: LineSnapshot,
    pub after: LineSnapshot,
}

#[derive(Debug, Clone)]
pub struct AppendResult {
    pub new_hash: ContentHash,
    pub after: LineSnapshot,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RemoveResult {
    pub new_hash: ContentHash,
    pub removed: LineSnapshot,
}

#[derive(Debug, Clone)]
pub struct MoveResult {
    pub new_hash: ContentHash,
    pub before: LineSnapshot,
    pub after: LineSnapshot,
}

pub fn hash_line(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(hash_content(bytes)))
}

fn count_newlines(s: &str) -> usize {
    s.as_bytes().iter().filter(|b| **b == b'\n').count()
}

fn snapshot_task_line(
    line_number: usize,
    raw_line: &str,
    quadrant: Option<Quadrant>,
) -> Result<LineSnapshot> {
    let stripped = raw_line.trim_end_matches(['\r', '\n']);
    let parsed = parse_line(stripped).ok_or_else(|| AppError::NotATaskLine {
        path: "<memory>".to_string(),
        line: line_number,
    })?;
    Ok(LineSnapshot {
        line: line_number,
        state: Some(TaskLineState {
            done: parsed.completed,
            text: parsed.text,
            quadrant,
        }),
        raw: raw_line.to_string(),
        hash: hash_line(raw_line.as_bytes()),
    })
}

fn split_lines(raw: &str) -> Vec<String> {
    raw.split_inclusive('\n').map(String::from).collect()
}

/// Toggle the checkbox on `line_number` (1-indexed) in `path`.
/// Returns the new content hash for watcher loop prevention.
pub fn toggle_task(path: &Path, line_number: usize) -> Result<ToggleResult> {
    let raw = std::fs::read_to_string(path)?;
    let eol = detect_line_ending(&raw);
    let mut lines: Vec<String> = split_lines(&raw);

    let idx = line_number.checked_sub(1).ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(), line: line_number,
    })?;
    let line = lines.get(idx).cloned().ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(), line: line_number,
    })?;
    let before = snapshot_task_line(line_number, &line, None)?;

    // Trim line ending for parsing
    let stripped = line.trim_end_matches(['\r', '\n']);
    let trailing = &line[stripped.len()..];

    let parsed = parse_line(stripped).ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(), line: line_number,
    })?;

    let new_marker = if parsed.completed { ' ' } else { 'x' };
    // Replace only the bracket content; keep everything else byte-identical.
    let new_line = replace_first_bracket(stripped, new_marker);
    lines[idx] = format!("{}{}", new_line, trailing);
    let after = snapshot_task_line(line_number, &lines[idx], None)?;

    let new_content: String = lines.concat();
    let _ = eol; // line endings preserved per-line; no normalization
    let new_hash = atomic_write(path, new_content.as_bytes())?;
    Ok(ToggleResult {
        new_hash,
        before,
        after,
    })
}

/// Replace `[ ]` or `[x]` or `[X]` with `[<m>]` — only the FIRST occurrence.
fn replace_first_bracket(line: &str, marker: char) -> String {
    let bytes = line.as_bytes();
    for i in 0..bytes.len().saturating_sub(2) {
        if bytes[i] == b'[' && bytes[i + 2] == b']'
            && matches!(bytes[i + 1], b' ' | b'x' | b'X') {
            let mut s = String::with_capacity(line.len());
            s.push_str(&line[..i + 1]);
            s.push(marker);
            s.push_str(&line[i + 2..]);
            return s;
        }
    }
    line.to_string()
}

/// Replace the text part of the task on `line_number`, preserving the
/// original indent, bullet style, checkbox state and trailing whitespace
/// byte-for-byte. New text is trimmed and rejected if empty or contains
/// a newline (task lines must stay single-line).
pub fn update_task_text(path: &Path, line_number: usize, new_text: &str) -> Result<UpdateTextResult> {
    let trimmed = new_text.trim();
    if trimmed.is_empty() {
        return Err(AppError::CommandFailed("task text cannot be empty".into()));
    }
    if trimmed.contains('\n') || trimmed.contains('\r') {
        return Err(AppError::CommandFailed("task text cannot contain newlines".into()));
    }

    let raw = std::fs::read_to_string(path)?;
    let mut lines: Vec<String> = split_lines(&raw);
    let idx = line_number.checked_sub(1).ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(), line: line_number,
    })?;
    let line = lines.get(idx).cloned().ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(), line: line_number,
    })?;
    let before = snapshot_task_line(line_number, &line, None)?;

    let stripped = line.trim_end_matches(['\r', '\n']);
    let trailing = &line[stripped.len()..];

    let prefix_len = task_prefix_len(stripped).ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(), line: line_number,
    })?;
    let prefix = &stripped[..prefix_len];
    lines[idx] = format!("{}{}{}", prefix, trimmed, trailing);
    let after = snapshot_task_line(line_number, &lines[idx], None)?;

    let new_content: String = lines.concat();
    let new_hash = atomic_write(path, new_content.as_bytes())?;
    Ok(UpdateTextResult {
        new_hash,
        before,
        after,
    })
}

/// Length in bytes of the task "prefix" (indent + bullet + checkbox + spaces),
/// up to and including the spaces after `]`. Returns None if the line is not a
/// task line. Mirrors `parser::TASK_REGEX` but operates byte-wise so callers
/// can keep the prefix verbatim.
fn task_prefix_len(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') { i += 1; }
    if i >= bytes.len() || !matches!(bytes[i], b'-' | b'*' | b'+') { return None; }
    i += 1;
    let after_bullet = i;
    while i < bytes.len() && bytes[i] == b' ' { i += 1; }
    if i == after_bullet { return None; }
    if i + 2 >= bytes.len() { return None; }
    if bytes[i] != b'[' || bytes[i + 2] != b']' { return None; }
    if !matches!(bytes[i + 1], b' ' | b'x' | b'X') { return None; }
    i += 3;
    let after_bracket = i;
    while i < bytes.len() && bytes[i] == b' ' { i += 1; }
    if i == after_bracket { return None; }
    Some(i)
}

/// Delete the task on `line_number` (1-indexed) from `path`. Verifies the
/// target line still parses as a task — otherwise the registry is stale and
/// we'd be deleting random content. Returns the new content hash.
#[allow(dead_code)]
pub fn remove_task_line(path: &Path, line_number: usize) -> Result<RemoveResult> {
    let raw = std::fs::read_to_string(path)?;
    let mut lines: Vec<String> = split_lines(&raw);
    let idx = line_number.checked_sub(1).ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(), line: line_number,
    })?;
    let line = lines.get(idx).cloned().ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(), line: line_number,
    })?;
    let stripped = line.trim_end_matches(['\r', '\n']);
    parse_line(stripped).ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(), line: line_number,
    })?;
    let removed = snapshot_task_line(line_number, &line, None)?;
    lines.remove(idx);
    let new_content: String = lines.concat();
    let new_hash = atomic_write(path, new_content.as_bytes())?;
    Ok(RemoveResult { new_hash, removed })
}

/// Append `- [ ] <text>` to file. Creates file (with parent dirs) if missing.
/// Returns new content hash.
pub fn append_task(path: &Path, text: &str) -> Result<AppendResult> {
    let trimmed = text.trim();

    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let (new_content, after) = append_plain_content(existing, trimmed)?;
    let new_hash = atomic_write(path, new_content.as_bytes())?;
    Ok(AppendResult { new_hash, after })
}

fn append_plain_content(existing: String, trimmed: &str) -> Result<(String, LineSnapshot)> {
    let new_line = format!("- [ ] {}\n", trimmed);
    let needs_leading_newline = !existing.is_empty() && !existing.ends_with('\n');
    let mut prefix = existing;
    if needs_leading_newline {
        prefix.push('\n');
    }
    if prefix.is_empty() {
        prefix.push_str("# Inbox\n\n");
    }
    let line_number = count_newlines(&prefix) + 1;
    let after = snapshot_task_line(line_number, &new_line, None)?;
    let mut new_content = prefix;
    new_content.push_str(&new_line);
    Ok((new_content, after))
}

/// CN/emoji label written when auto-creating a quadrant header. Matches the
/// todo skill template so future re-parses match the same Quadrant.
fn quadrant_header_label(q: Quadrant) -> &'static str {
    match q {
        Quadrant::UrgentImportant => "🔴 紧急+重要",
        Quadrant::NotUrgentImportant => "🟡 重要不紧急",
        Quadrant::UrgentNotImportant => "🟠 紧急不重要",
        Quadrant::NotUrgentNotImportant => "🟢 不紧急不重要",
    }
}

/// Find the byte index *just after* the last task / non-empty line inside
/// the section that begins on `header_line_idx`. Returns the insertion point
/// (in the original `content`) where a new `- [ ] task\n` should go.
fn find_section_insertion_point(content: &str, header_line_idx: usize) -> usize {
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    if header_line_idx >= lines.len() {
        return content.len();
    }
    let header_level = lines[header_line_idx]
        .trim_start()
        .bytes()
        .take_while(|b| *b == b'#')
        .count();

    let mut byte_offset: usize = lines[..=header_line_idx].iter().map(|l| l.len()).sum();
    let mut last_content_end = byte_offset;
    for line in &lines[header_line_idx + 1..] {
        let trimmed = line.trim_start();
        let next_level = trimmed.bytes().take_while(|b| *b == b'#').count();
        let is_header = next_level > 0
            && next_level <= header_level
            && trimmed.as_bytes().get(next_level) == Some(&b' ');
        if is_header {
            return last_content_end;
        }
        if !line.trim().is_empty() {
            last_content_end = byte_offset + line.len();
        }
        byte_offset += line.len();
    }
    last_content_end
}

/// Append a task into the section matching `quadrant`. `None` falls back to
/// the plain `append_task` (file end). `auto_create_header=true` adds a new
/// `## <emoji> <name>` block at EOF when the requested quadrant is absent.
pub fn append_task_to_quadrant(
    path: &Path,
    text: &str,
    quadrant: Option<Quadrant>,
    auto_create_header: bool,
) -> Result<AppendResult> {
    let trimmed = text.trim();
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let (new_content, after) =
        append_task_to_quadrant_content(existing, trimmed, quadrant, auto_create_header)?;
    let new_hash = atomic_write(path, new_content.as_bytes())?;
    Ok(AppendResult { new_hash, after })
}

fn append_task_to_quadrant_content(
    existing: String,
    trimmed: &str,
    quadrant: Option<Quadrant>,
    auto_create_header: bool,
) -> Result<(String, LineSnapshot)> {
    let q = match quadrant {
        None => return append_plain_content(existing, trimmed),
        Some(q) => q,
    };

    let mut header_line_idx: Option<usize> = None;
    for (i, line) in existing.split_inclusive('\n').enumerate() {
        let stripped = line.trim_end_matches(['\r', '\n']);
        if let Some(caps) = crate::parser::header_regex().captures(stripped) {
            if crate::parser::detect_quadrant_pub(caps.get(2).unwrap().as_str()) == Some(q) {
                header_line_idx = Some(i);
                break;
            }
        }
    }

    let (new_content, after) = match header_line_idx {
        Some(idx) => {
            let insert_at = find_section_insertion_point(&existing, idx);
            let mut s = String::with_capacity(existing.len() + trimmed.len() + 8);
            s.push_str(&existing[..insert_at]);
            if !s.ends_with('\n') {
                s.push('\n');
            }
            let line_number = count_newlines(&s) + 1;
            let new_line = format!("- [ ] {}\n", trimmed);
            s.push_str(&new_line);
            s.push_str(&existing[insert_at..]);
            (s, snapshot_task_line(line_number, &new_line, Some(q))?)
        }
        None => {
            if !auto_create_header {
                return Err(AppError::QuadrantHeaderMissing(q));
            }
            let mut s = existing.clone();
            if !s.is_empty() && !s.ends_with('\n') {
                s.push('\n');
            }
            if !s.ends_with("\n\n") && !s.is_empty() {
                s.push('\n');
            }
            s.push_str("## ");
            s.push_str(quadrant_header_label(q));
            s.push_str("\n\n- [ ] ");
            let line_number = count_newlines(&s) + 1;
            let new_line = format!("- [ ] {}\n", trimmed);
            s.push_str(trimmed);
            s.push('\n');
            (s, snapshot_task_line(line_number, &new_line, Some(q))?)
        }
    };

    Ok((new_content, after))
}

pub fn move_task_to_quadrant(
    path: &Path,
    line_number: usize,
    new_text: &str,
    new_quadrant: Option<Quadrant>,
    auto_create_header: bool,
) -> Result<MoveResult> {
    let trimmed = new_text.trim();
    if trimmed.is_empty() {
        return Err(AppError::CommandFailed("task text cannot be empty".into()));
    }
    if trimmed.contains('\n') || trimmed.contains('\r') {
        return Err(AppError::CommandFailed("task text cannot contain newlines".into()));
    }

    let raw = std::fs::read_to_string(path)?;
    let mut lines: Vec<String> = split_lines(&raw);
    let idx = line_number.checked_sub(1).ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(),
        line: line_number,
    })?;
    let line = lines.get(idx).cloned().ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(),
        line: line_number,
    })?;
    let stripped = line.trim_end_matches(['\r', '\n']);
    parse_line(stripped).ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(),
        line: line_number,
    })?;
    let before = snapshot_task_line(line_number, &line, None)?;
    lines.remove(idx);
    let without_line: String = lines.concat();
    let (new_content, after) =
        append_task_to_quadrant_content(without_line, trimmed, new_quadrant, auto_create_header)?;
    let new_hash = atomic_write(path, new_content.as_bytes())?;
    Ok(MoveResult {
        new_hash,
        before,
        after,
    })
}

pub fn replace_line_if_hash(
    path: &Path,
    line_number: usize,
    expected_hash: &str,
    replacement_line: &str,
) -> Result<ContentHash> {
    let raw = std::fs::read_to_string(path)?;
    let mut lines: Vec<String> = split_lines(&raw);
    let idx = line_number.checked_sub(1).ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(),
        line: line_number,
    })?;
    let line = lines.get(idx).cloned().ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(),
        line: line_number,
    })?;
    if hash_line(line.as_bytes()) != expected_hash {
        return Err(AppError::HistoryHashMismatch {
            event_id: String::new(),
            file: path.display().to_string(),
            line: line_number,
        });
    }
    lines[idx] = replacement_line.to_string();
    let new_content: String = lines.concat();
    atomic_write(path, new_content.as_bytes())
}

pub fn remove_line_if_hash(
    path: &Path,
    line_number: usize,
    expected_hash: &str,
) -> Result<ContentHash> {
    let raw = std::fs::read_to_string(path)?;
    let mut lines: Vec<String> = split_lines(&raw);
    let idx = line_number.checked_sub(1).ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(),
        line: line_number,
    })?;
    let line = lines.get(idx).cloned().ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(),
        line: line_number,
    })?;
    if hash_line(line.as_bytes()) != expected_hash {
        return Err(AppError::HistoryHashMismatch {
            event_id: String::new(),
            file: path.display().to_string(),
            line: line_number,
        });
    }
    lines.remove(idx);
    let new_content: String = lines.concat();
    atomic_write(path, new_content.as_bytes())
}

pub fn insert_line_at(path: &Path, line_number: usize, raw_line: &str) -> Result<ContentHash> {
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<String> = split_lines(&raw);
    let idx = line_number.checked_sub(1).ok_or_else(|| {
        AppError::CommandFailed("line numbers are 1-indexed".into())
    })?;
    if idx > lines.len() {
        return Err(AppError::NotATaskLine {
            path: path.display().to_string(),
            line: line_number,
        });
    }
    lines.insert(idx, raw_line.to_string());
    let new_content: String = lines.concat();
    atomic_write(path, new_content.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
        let p = dir.path().join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    #[test]
    fn toggle_unchecked_to_checked() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "- [ ] hello\n- [ ] world\n");
        toggle_task(&p, 1).unwrap();
        let got = std::fs::read_to_string(&p).unwrap();
        assert_eq!(got, "- [x] hello\n- [ ] world\n");
    }

    #[test]
    fn toggle_checked_to_unchecked() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "- [x] hello\n");
        toggle_task(&p, 1).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "- [ ] hello\n");
    }

    #[test]
    fn toggle_preserves_other_lines_byte_for_byte() {
        let d = TempDir::new().unwrap();
        let original = "# h\n\n- [ ] one\n- [ ] two\nrandom\n";
        let p = write(&d, "a.md", original);
        toggle_task(&p, 4).unwrap();
        let got = std::fs::read_to_string(&p).unwrap();
        assert_eq!(got, "# h\n\n- [ ] one\n- [x] two\nrandom\n");
    }

    #[test]
    fn toggle_preserves_crlf_line_endings() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "- [ ] one\r\n- [ ] two\r\n");
        toggle_task(&p, 1).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "- [x] one\r\n- [ ] two\r\n");
    }

    #[test]
    fn toggle_returns_new_hash() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "- [ ] x\n");
        let h = toggle_task(&p, 1).unwrap();
        let actual = hash_content(std::fs::read(&p).unwrap().as_slice());
        assert_eq!(h.new_hash, actual);
    }

    #[test]
    fn toggle_non_task_line_errors() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "# heading\n");
        assert!(toggle_task(&p, 1).is_err());
    }

    #[test]
    fn append_to_existing_file() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "inbox.md", "# Inbox\n\n- [ ] one\n");
        append_task(&p, "two").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "# Inbox\n\n- [ ] one\n- [ ] two\n");
    }

    #[test]
    fn append_creates_file_with_header() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("inbox.md");
        append_task(&p, "first").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "# Inbox\n\n- [ ] first\n");
    }

    #[test]
    fn append_handles_missing_trailing_newline() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "i.md", "- [ ] one"); // no \n
        append_task(&p, "two").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "- [ ] one\n- [ ] two\n");
    }

    #[test]
    fn update_replaces_only_text() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "- [ ] hello world\n- [ ] keep me\n");
        update_task_text(&p, 1, "goodbye world").unwrap();
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            "- [ ] goodbye world\n- [ ] keep me\n"
        );
    }

    #[test]
    fn update_preserves_indent_and_completed_state() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "    * [x] indented done task\n");
        update_task_text(&p, 1, "new content").unwrap();
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            "    * [x] new content\n"
        );
    }

    #[test]
    fn update_preserves_crlf_line_endings() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "- [ ] one\r\n- [ ] two\r\n");
        update_task_text(&p, 2, "TWO!").unwrap();
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            "- [ ] one\r\n- [ ] TWO!\r\n"
        );
    }

    #[test]
    fn update_rejects_empty_and_multiline() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "- [ ] hi\n");
        assert!(update_task_text(&p, 1, "   ").is_err());
        assert!(update_task_text(&p, 1, "a\nb").is_err());
        // File untouched.
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "- [ ] hi\n");
    }

    #[test]
    fn update_non_task_line_errors() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "# heading\n");
        assert!(update_task_text(&p, 1, "anything").is_err());
    }

    use crate::types::Quadrant;

    #[test]
    fn append_to_existing_quadrant_inserts_before_next_header() {
        let d = TempDir::new().unwrap();
        let original = "## 🔴 Urgent\n- [ ] a\n\n## 🟡 Important\n- [ ] b\n";
        let p = write(&d, "q.md", original);
        append_task_to_quadrant(&p, "new", Some(Quadrant::UrgentImportant), true).unwrap();
        let got = std::fs::read_to_string(&p).unwrap();
        assert_eq!(
            got,
            "## 🔴 Urgent\n- [ ] a\n- [ ] new\n\n## 🟡 Important\n- [ ] b\n"
        );
    }

    #[test]
    fn append_to_last_quadrant_appends_at_eof() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "q.md", "## 🟢 Later\n- [ ] x\n");
        append_task_to_quadrant(&p, "y", Some(Quadrant::NotUrgentNotImportant), true).unwrap();
        let got = std::fs::read_to_string(&p).unwrap();
        assert_eq!(got, "## 🟢 Later\n- [ ] x\n- [ ] y\n");
    }

    #[test]
    fn append_to_missing_quadrant_creates_header_when_allowed() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "q.md", "# Notes\n- [ ] keep\n");
        append_task_to_quadrant(&p, "n", Some(Quadrant::UrgentImportant), true).unwrap();
        let got = std::fs::read_to_string(&p).unwrap();
        assert_eq!(
            got,
            "# Notes\n- [ ] keep\n\n## 🔴 紧急+重要\n\n- [ ] n\n"
        );
    }

    #[test]
    fn append_to_missing_quadrant_errors_when_disallowed() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "q.md", "# Notes\n");
        let err = append_task_to_quadrant(&p, "n", Some(Quadrant::UrgentImportant), false);
        assert!(matches!(err, Err(crate::error::AppError::QuadrantHeaderMissing(_))));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "# Notes\n");
    }

    #[test]
    fn append_quadrant_none_falls_back_to_append_task() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "q.md", "- [ ] one\n");
        append_task_to_quadrant(&p, "two", None, true).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "- [ ] one\n- [ ] two\n");
    }

    #[test]
    fn append_to_quadrant_handles_file_not_ending_in_newline() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "q.md", "- [ ] tail"); // no \n
        append_task_to_quadrant(&p, "z", Some(Quadrant::UrgentImportant), true).unwrap();
        let got = std::fs::read_to_string(&p).unwrap();
        assert_eq!(got, "- [ ] tail\n\n## 🔴 紧急+重要\n\n- [ ] z\n");
    }

    #[test]
    fn toggle_returns_before_after_snapshots_for_history() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "- [ ] hello\n");
        let result = toggle_task(&p, 1).unwrap();

        assert_eq!(result.before.line, 1);
        assert_eq!(result.before.raw, "- [ ] hello\n");
        assert_eq!(result.before.state.as_ref().unwrap().done, false);
        assert_eq!(result.after.raw, "- [x] hello\n");
        assert_eq!(result.after.state.as_ref().unwrap().done, true);
        assert_eq!(result.new_hash, hash_content(std::fs::read(&p).unwrap().as_slice()));
    }

    #[test]
    fn append_to_quadrant_returns_inserted_line_snapshot() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "q.md", "## 🔴 Urgent\n- [ ] a\n\n## 🟡 Important\n- [ ] b\n");
        let result = append_task_to_quadrant(&p, "new", Some(Quadrant::UrgentImportant), true).unwrap();

        assert_eq!(result.after.line, 3);
        assert_eq!(result.after.raw, "- [ ] new\n");
        assert_eq!(result.after.state.as_ref().unwrap().text, "new");
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            "## 🔴 Urgent\n- [ ] a\n- [ ] new\n\n## 🟡 Important\n- [ ] b\n"
        );
    }

    #[test]
    fn replace_line_if_hash_rejects_mismatched_current_line() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "- [ ] one\n");
        let expected_hash = hash_line("- [ ] different\n".as_bytes());

        let err = replace_line_if_hash(&p, 1, &expected_hash, "- [x] one\n").unwrap_err();

        assert!(matches!(err, crate::error::AppError::HistoryHashMismatch { .. }));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "- [ ] one\n");
    }

    #[test]
    fn move_to_missing_quadrant_without_auto_create_leaves_file_untouched() {
        let d = TempDir::new().unwrap();
        let original = "## 🔴 Urgent\n- [ ] keep\n";
        let p = write(&d, "q.md", original);

        let err = move_task_to_quadrant(
            &p,
            2,
            "keep",
            Some(Quadrant::NotUrgentImportant),
            false,
        )
        .unwrap_err();

        assert!(matches!(err, crate::error::AppError::QuadrantHeaderMissing(_)));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), original);
    }

    #[test]
    fn move_to_existing_quadrant_returns_before_after_snapshots() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "q.md", "## 🔴 Urgent\n- [x] keep\n\n## 🟡 Later\n- [ ] other\n");

        let result = move_task_to_quadrant(
            &p,
            2,
            "keep edited",
            Some(Quadrant::NotUrgentImportant),
            true,
        )
        .unwrap();

        assert_eq!(result.before.raw, "- [x] keep\n");
        assert_eq!(result.after.line, 5);
        assert_eq!(result.after.raw, "- [ ] keep edited\n");
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            "## 🔴 Urgent\n\n## 🟡 Later\n- [ ] other\n- [ ] keep edited\n"
        );
    }
}
