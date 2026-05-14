use crate::error::{AppError, Result};
use crate::parser::parse_line;
use crate::types::{hash_content, ContentHash};
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
}
