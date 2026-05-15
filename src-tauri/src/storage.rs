use crate::error::{AppError, Result};
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

/// Toggle the checkbox on `line_number` (1-indexed) in `path`.
/// Returns the new content hash for watcher loop prevention.
pub fn toggle_task(path: &Path, line_number: usize) -> Result<ContentHash> {
    let raw = std::fs::read_to_string(path)?;
    let eol = detect_line_ending(&raw);
    let mut lines: Vec<String> = raw.split_inclusive(|c| c == '\n').map(String::from).collect();

    let idx = line_number.checked_sub(1).ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(), line: line_number,
    })?;
    let line = lines.get(idx).cloned().ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(), line: line_number,
    })?;

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

    let new_content: String = lines.concat();
    let _ = eol; // line endings preserved per-line; no normalization
    atomic_write(path, new_content.as_bytes())
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
pub fn update_task_text(path: &Path, line_number: usize, new_text: &str) -> Result<ContentHash> {
    let trimmed = new_text.trim();
    if trimmed.is_empty() {
        return Err(AppError::CommandFailed("task text cannot be empty".into()));
    }
    if trimmed.contains('\n') || trimmed.contains('\r') {
        return Err(AppError::CommandFailed("task text cannot contain newlines".into()));
    }

    let raw = std::fs::read_to_string(path)?;
    let mut lines: Vec<String> = raw.split_inclusive(|c| c == '\n').map(String::from).collect();
    let idx = line_number.checked_sub(1).ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(), line: line_number,
    })?;
    let line = lines.get(idx).cloned().ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(), line: line_number,
    })?;

    let stripped = line.trim_end_matches(['\r', '\n']);
    let trailing = &line[stripped.len()..];

    let prefix_len = task_prefix_len(stripped).ok_or_else(|| AppError::NotATaskLine {
        path: path.display().to_string(), line: line_number,
    })?;
    let prefix = &stripped[..prefix_len];
    lines[idx] = format!("{}{}{}", prefix, trimmed, trailing);

    let new_content: String = lines.concat();
    atomic_write(path, new_content.as_bytes())
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

/// Append `- [ ] <text>` to file. Creates file (with parent dirs) if missing.
/// Returns new content hash.
pub fn append_task(path: &Path, text: &str) -> Result<ContentHash> {
    let trimmed = text.trim();
    let new_line = format!("- [ ] {}\n", trimmed);

    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let needs_leading_newline = !existing.is_empty() && !existing.ends_with('\n');
    let mut new_content = existing;
    if needs_leading_newline {
        new_content.push('\n');
    }
    if new_content.is_empty() {
        new_content.push_str("# Inbox\n\n");
    }
    new_content.push_str(&new_line);
    atomic_write(path, new_content.as_bytes())
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
) -> Result<ContentHash> {
    let q = match quadrant {
        None => return append_task(path, text),
        Some(q) => q,
    };
    let trimmed = text.trim();
    let existing = std::fs::read_to_string(path).unwrap_or_default();

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

    let new_content = match header_line_idx {
        Some(idx) => {
            let insert_at = find_section_insertion_point(&existing, idx);
            let mut s = String::with_capacity(existing.len() + trimmed.len() + 8);
            s.push_str(&existing[..insert_at]);
            if !s.ends_with('\n') {
                s.push('\n');
            }
            s.push_str("- [ ] ");
            s.push_str(trimmed);
            s.push('\n');
            s.push_str(&existing[insert_at..]);
            s
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
            s.push_str(trimmed);
            s.push('\n');
            s
        }
    };

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
        assert_eq!(h, actual);
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
}
