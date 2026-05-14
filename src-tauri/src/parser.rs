use crate::error::Result;
use crate::types::{hash_content, Task};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;

static TASK_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\s*)[-*+]\s+\[([ xX])\]\s+(.+?)\s*$").unwrap()
});

pub struct ParsedTask {
    pub indent: usize,
    pub completed: bool,
    pub text: String,
}

pub fn parse_line(line: &str) -> Option<ParsedTask> {
    let caps = TASK_REGEX.captures(line)?;
    Some(ParsedTask {
        indent: caps.get(1).unwrap().as_str().chars().count(),
        completed: matches!(caps.get(2).unwrap().as_str(), "x" | "X"),
        text: caps.get(3).unwrap().as_str().to_string(),
    })
}

pub fn parse_file(path: &Path, source_id: &str) -> Result<Vec<Task>> {
    let raw = std::fs::read(path)?;
    // Strip UTF-8 BOM if present
    let content = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) { &raw[3..] } else { &raw[..] };
    let text = String::from_utf8_lossy(content);

    let abs = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut tasks = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line_number = i + 1;
        if let Some(p) = parse_line(line) {
            let id_input = format!("{}:{}", abs.display(), line_number);
            let id = hex::encode(&hash_content(id_input.as_bytes())[..8]);
            tasks.push(Task {
                id,
                text: p.text,
                completed: p.completed,
                source_file: abs.clone(),
                line_number,
                indent: p.indent,
                source_id: source_id.to_string(),
            });
        }
    }
    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn parse_unchecked_task() {
        let p = parse_line("- [ ] hello").unwrap();
        assert_eq!(p.text, "hello");
        assert!(!p.completed);
        assert_eq!(p.indent, 0);
    }

    #[test]
    fn parse_checked_task() {
        let p = parse_line("- [x] done").unwrap();
        assert!(p.completed);
        assert_eq!(p.text, "done");
    }

    #[test]
    fn parse_uppercase_x() {
        assert!(parse_line("- [X] done").unwrap().completed);
    }

    #[test]
    fn parse_alt_bullets() {
        assert!(parse_line("* [ ] a").is_some());
        assert!(parse_line("+ [ ] b").is_some());
    }

    #[test]
    fn parse_indent_in_spaces() {
        let p = parse_line("    - [ ] indented").unwrap();
        assert_eq!(p.indent, 4);
    }

    #[test]
    fn ignores_non_task_lines() {
        assert!(parse_line("# heading").is_none());
        assert!(parse_line("- not a task").is_none());
        assert!(parse_line("- [ ]no space after bracket").is_none());
        assert!(parse_line("").is_none());
    }

    #[test]
    fn trims_trailing_whitespace() {
        let p = parse_line("- [ ] hello   ").unwrap();
        assert_eq!(p.text, "hello");
    }

    fn write_tmp(content: &str) -> NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(".md").tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn parse_file_returns_tasks_with_line_numbers() {
        let f = write_tmp("# title\n- [ ] one\nrandom line\n- [x] two\n");
        let tasks = parse_file(f.path(), "test-src").unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].text, "one");
        assert_eq!(tasks[0].line_number, 2);
        assert_eq!(tasks[1].text, "two");
        assert_eq!(tasks[1].line_number, 4);
        assert!(tasks[1].completed);
    }

    #[test]
    fn parse_file_strips_utf8_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"- [ ] bom task\n");
        let mut f = tempfile::Builder::new().suffix(".md").tempfile().unwrap();
        f.write_all(&bytes).unwrap();
        let tasks = parse_file(f.path(), "test-src").unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].text, "bom task");
        assert_eq!(tasks[0].line_number, 1);
    }

    #[test]
    fn stable_id_for_same_file_and_line() {
        let f = write_tmp("- [ ] x\n");
        let a = parse_file(f.path(), "test-src").unwrap();
        let b = parse_file(f.path(), "test-src").unwrap();
        assert_eq!(a[0].id, b[0].id);
    }

    #[test]
    fn task_carries_source_id() {
        let f = write_tmp("- [ ] x\n");
        let tasks = parse_file(f.path(), "my-source").unwrap();
        assert_eq!(tasks[0].source_id, "my-source");
    }
}
