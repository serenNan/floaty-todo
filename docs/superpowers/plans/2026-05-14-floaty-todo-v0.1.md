# Floaty Todo v0.1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build v0.1 of Floaty Todo — a Tauri 2 + Vue 3 desktop app that scans a folder for `- [ ]` markdown tasks, displays them in a flat list, supports click-to-toggle with precise line-level write-back, auto-refreshes on external file changes, and lives in the system tray.

**Architecture:** Rust backend (parser → storage → registry → watcher) exposed via Tauri commands; Vue 3 frontend with Pinia store consumes commands and listens for `tasks-updated` events. Loop prevention via SHA-256 content hash registered with watcher before each write. No sidecar yet (v0.2). No global hotkeys yet (v0.2).

**Tech Stack:** Tauri 2.x, Rust (regex, notify, sha2, serde, tokio, thiserror, walkdir, tempfile for tests), Vue 3 + TypeScript + Pinia + Vite, native CSS.

**Reference:** See `PLAN.md` in repo root for full design rationale and decisions.

**Testing strategy:** TDD for all Rust modules using `cargo test` with `tempfile` for filesystem isolation. Vue components verified manually in v0.1 (no Vitest setup until v0.2 when QuickAdd window adds enough surface area to justify it).

**Working directory:** `D:\Projects\Floaty-todo` (PowerShell on Windows 11). Commands shown in PowerShell-friendly form.

---

## Task 1: Project Scaffold

**Files:**
- Create: `package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/lib.rs`, `src-tauri/src/main.rs`, `src/main.ts`, `src/App.vue`, `.gitignore`

- [ ] **Step 1: Initialize Tauri 2 + Vue + TS project (non-interactive)**

Run from `D:\Projects\Floaty-todo`:

```powershell
npm create tauri-app@latest -- --name floaty-todo --identifier com.serendipity.floaty-todo --template vue-ts --manager npm -y
```

Expected: project files created in `floaty-todo/` subfolder. **Move** everything from `floaty-todo/` up to repo root, then delete the empty `floaty-todo/` folder. Keep existing `PLAN.md` and `docs/` untouched.

```powershell
Move-Item floaty-todo\* . -Force
Move-Item floaty-todo\.gitignore . -Force -ErrorAction SilentlyContinue
Remove-Item floaty-todo -Recurse -Force
```

- [ ] **Step 2: Install npm deps**

```powershell
npm install
npm install pinia
```

Expected: `node_modules/` created, no errors.

- [ ] **Step 3: Smoke test the scaffold**

```powershell
npm run tauri dev
```

Expected: a window opens showing the default Tauri+Vue template. Close the window (Ctrl+C in the terminal). If this fails, fix before continuing — usually missing Rust toolchain (`rustup default stable`) or WebView2 (Win10 only; Win11 has it built in).

- [ ] **Step 4: Commit scaffold**

```powershell
git add .
git commit -m "chore: scaffold Tauri 2 + Vue 3 + TS project"
```

---

## Task 2: Rust Foundations (types + error + deps)

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/types.rs`, `src-tauri/src/error.rs`

- [ ] **Step 1: Add Rust dependencies to `src-tauri/Cargo.toml`**

Under `[dependencies]` add (or merge with existing tauri/serde lines):

```toml
regex = "1.10"
notify = "6.1"
notify-debouncer-full = "0.3"
sha2 = "0.10"
walkdir = "2.5"
once_cell = "1.19"
thiserror = "1.0"
tokio = { version = "1", features = ["sync", "rt-multi-thread", "macros"] }
tauri-plugin-dialog = "2"

[dev-dependencies]
tempfile = "3.10"
```

- [ ] **Step 2: Create `src-tauri/src/error.rs`**

```rust
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
    #[error("config not initialized: vault_path is None")]
    NoVault,
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("line {line} in {path} is not a task line")]
    NotATaskLine { path: String, line: usize },
}

pub type Result<T> = std::result::Result<T, AppError>;

// Tauri commands need a Serialize-friendly error
impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
```

- [ ] **Step 3: Create `src-tauri/src/types.rs`**

```rust
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
```

- [ ] **Step 4: Wire modules in `src-tauri/src/lib.rs`**

At the top of `lib.rs`, before the `pub fn run()`:

```rust
mod error;
mod types;
```

- [ ] **Step 5: Build and commit**

```powershell
cd src-tauri
cargo build
cd ..
```

Expected: builds without warnings (or only unused-import warnings, fine for now).

```powershell
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/src/error.rs src-tauri/src/types.rs
git commit -m "feat: add error types, Task/AppConfig models, content hashing"
```

---

## Task 3: parser.rs — Markdown Task Parser (TDD)

**Files:**
- Create: `src-tauri/src/parser.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add module declaration to `lib.rs`**

Add `mod parser;` near the other `mod` lines.

- [ ] **Step 2: Write failing tests in `src-tauri/src/parser.rs`**

```rust
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

pub fn parse_file(path: &Path) -> Result<Vec<Task>> {
    let raw = std::fs::read(path)?;
    // Strip UTF-8 BOM if present
    let content = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) { &raw[3..] } else { &raw[..] };
    let text = String::from_utf8_lossy(content);

    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
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
        let tasks = parse_file(f.path()).unwrap();
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
        let tasks = parse_file(f.path()).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].text, "bom task");
        assert_eq!(tasks[0].line_number, 1);
    }

    #[test]
    fn stable_id_for_same_file_and_line() {
        let f = write_tmp("- [ ] x\n");
        let a = parse_file(f.path()).unwrap();
        let b = parse_file(f.path()).unwrap();
        assert_eq!(a[0].id, b[0].id);
    }
}
```

Add `hex = "0.4"` to `Cargo.toml` `[dependencies]`.

- [ ] **Step 3: Run tests — expect FAIL on the `hex` dep first build, then PASS after `cargo build`**

```powershell
cd src-tauri
cargo test --lib parser
cd ..
```

Expected: 10 passed, 0 failed.

- [ ] **Step 4: Commit**

```powershell
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/src/parser.rs
git commit -m "feat(parser): regex-based markdown task parsing with BOM handling"
```

---

## Task 4: storage.rs — Precise Line-Level Write-Back (TDD)

**Files:**
- Create: `src-tauri/src/storage.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `mod storage;` to `lib.rs`**

- [ ] **Step 2: Write `src-tauri/src/storage.rs` with tests**

```rust
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
```

- [ ] **Step 3: Run tests**

```powershell
cd src-tauri
cargo test --lib storage
cd ..
```

Expected: 9 passed.

- [ ] **Step 4: Commit**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/storage.rs
git commit -m "feat(storage): atomic line-level toggle and append with content hash"
```

---

## Task 5: config.rs — Persistent App Config (TDD)

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `mod config;` to `lib.rs`**

- [ ] **Step 2: Write `src-tauri/src/config.rs`**

```rust
use crate::error::Result;
use crate::types::AppConfig;
use std::path::{Path, PathBuf};

/// Determine the OS-specific config file path. Tauri normally provides this via
/// `app_handle.path().app_config_dir()`, but tests need a pure function.
pub fn load_from(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = std::fs::read_to_string(path)?;
    match serde_json::from_str(&raw) {
        Ok(c) => Ok(c),
        Err(_) => Ok(AppConfig::default()), // tolerant: corrupt config -> defaults
    }
}

pub fn save_to(path: &Path, config: &AppConfig) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(config)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn config_file(app_config_dir: &Path) -> PathBuf {
    app_config_dir.join("config.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_missing_returns_default() {
        let d = TempDir::new().unwrap();
        let cfg = load_from(&d.path().join("nope.json")).unwrap();
        assert_eq!(cfg, AppConfig::default());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("config.json");
        let mut cfg = AppConfig::default();
        cfg.vault_path = Some(d.path().join("vault"));
        cfg.always_on_top = false;
        save_to(&p, &cfg).unwrap();
        let got = load_from(&p).unwrap();
        assert_eq!(got, cfg);
    }

    #[test]
    fn corrupt_json_falls_back_to_default() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("config.json");
        std::fs::write(&p, "{not json").unwrap();
        let cfg = load_from(&p).unwrap();
        assert_eq!(cfg, AppConfig::default());
    }
}
```

- [ ] **Step 3: Run tests**

```powershell
cd src-tauri
cargo test --lib config
cd ..
```

Expected: 3 passed.

- [ ] **Step 4: Commit**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/config.rs
git commit -m "feat(config): persistent AppConfig with corrupt-tolerant load"
```

---

## Task 6: registry.rs — In-Memory Task Index (TDD)

**Files:**
- Create: `src-tauri/src/registry.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `mod registry;` to `lib.rs`**

- [ ] **Step 2: Write `src-tauri/src/registry.rs`**

```rust
use crate::error::Result;
use crate::parser;
use crate::types::Task;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Default)]
pub struct TaskRegistry {
    tasks: HashMap<String, Task>,
    by_file: HashMap<PathBuf, Vec<String>>,
}

impl TaskRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn rebuild_from_vault(&mut self, vault: &Path) -> Result<()> {
        self.tasks.clear();
        self.by_file.clear();
        if !vault.exists() { return Ok(()); }
        for entry in WalkDir::new(vault).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() { continue; }
            let path = entry.path();
            if !is_markdown_target(path) { continue; }
            let _ = self.refresh_file(path); // skip files that fail to parse
        }
        Ok(())
    }

    pub fn refresh_file(&mut self, file: &Path) -> Result<()> {
        // Remove stale entries for this file
        let canonical = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
        if let Some(old_ids) = self.by_file.remove(&canonical) {
            for id in old_ids { self.tasks.remove(&id); }
        }
        if !file.exists() { return Ok(()); }

        let parsed = parser::parse_file(file)?;
        let ids: Vec<String> = parsed.iter().map(|t| t.id.clone()).collect();
        for t in parsed { self.tasks.insert(t.id.clone(), t); }
        self.by_file.insert(canonical, ids);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Task> { self.tasks.get(id) }

    pub fn all_tasks(&self) -> Vec<Task> {
        let mut v: Vec<Task> = self.tasks.values().cloned().collect();
        v.sort_by(|a, b| a.source_file.cmp(&b.source_file)
            .then(a.line_number.cmp(&b.line_number)));
        v
    }
}

/// Filter: only `.md`/`.markdown`, skip ignore list.
pub fn is_markdown_target(path: &Path) -> bool {
    let ext_ok = path.extension().and_then(|e| e.to_str())
        .map(|s| matches!(s.to_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false);
    if !ext_ok { return false; }
    is_not_ignored(path)
}

pub fn is_not_ignored(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.starts_with('~') || name.ends_with('~') || name.ends_with(".swp") || name.ends_with(".tmp") {
        return false;
    }
    if name == ".floaty-todo.json" { return false; }
    for comp in path.components() {
        if let std::path::Component::Normal(seg) = comp {
            let s = seg.to_string_lossy();
            if matches!(s.as_ref(), ".obsidian" | ".git" | ".trash" | "node_modules") {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn touch(dir: &Path, rel: &str, content: &str) -> PathBuf {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    #[test]
    fn rebuild_collects_tasks_across_files() {
        let d = TempDir::new().unwrap();
        touch(d.path(), "a.md", "- [ ] one\n- [x] done\n");
        touch(d.path(), "sub/b.md", "- [ ] two\n");
        let mut r = TaskRegistry::new();
        r.rebuild_from_vault(d.path()).unwrap();
        assert_eq!(r.all_tasks().len(), 3);
    }

    #[test]
    fn rebuild_skips_obsidian_and_git_dirs() {
        let d = TempDir::new().unwrap();
        touch(d.path(), "a.md", "- [ ] keep\n");
        touch(d.path(), ".obsidian/x.md", "- [ ] skip\n");
        touch(d.path(), ".git/y.md", "- [ ] skip\n");
        touch(d.path(), "node_modules/z.md", "- [ ] skip\n");
        let mut r = TaskRegistry::new();
        r.rebuild_from_vault(d.path()).unwrap();
        let all: Vec<_> = r.all_tasks().iter().map(|t| t.text.clone()).collect();
        assert_eq!(all, vec!["keep"]);
    }

    #[test]
    fn refresh_file_replaces_old_entries() {
        let d = TempDir::new().unwrap();
        let p = touch(d.path(), "a.md", "- [ ] old\n");
        let mut r = TaskRegistry::new();
        r.rebuild_from_vault(d.path()).unwrap();
        assert_eq!(r.all_tasks().len(), 1);

        // Rewrite file with different content
        std::fs::write(&p, "- [ ] new1\n- [ ] new2\n").unwrap();
        r.refresh_file(&p).unwrap();
        let names: Vec<_> = r.all_tasks().iter().map(|t| t.text.clone()).collect();
        assert_eq!(names, vec!["new1", "new2"]);
    }

    #[test]
    fn refresh_file_handles_deletion() {
        let d = TempDir::new().unwrap();
        let p = touch(d.path(), "a.md", "- [ ] x\n");
        let mut r = TaskRegistry::new();
        r.rebuild_from_vault(d.path()).unwrap();
        std::fs::remove_file(&p).unwrap();
        r.refresh_file(&p).unwrap();
        assert_eq!(r.all_tasks().len(), 0);
    }
}
```

- [ ] **Step 3: Run tests**

```powershell
cd src-tauri
cargo test --lib registry
cd ..
```

Expected: 4 passed.

- [ ] **Step 4: Commit**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/registry.rs
git commit -m "feat(registry): in-memory task index with vault scan and ignore list"
```

---

## Task 7: watcher.rs — Filesystem Watcher with Loop Prevention (TDD)

**Files:**
- Create: `src-tauri/src/watcher.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `mod watcher;` to `lib.rs`**

- [ ] **Step 2: Write `src-tauri/src/watcher.rs`**

```rust
use crate::error::Result;
use crate::registry::is_markdown_target;
use crate::types::{hash_content, ContentHash};
use notify::RecommendedWatcher;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, FileIdMap};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum WatchEvent {
    Changed(PathBuf),
    Deleted(PathBuf),
}

/// Shared loop-prevention set. `storage` registers the new content hash before
/// writing; the watcher checks the file's current hash and discards events that
/// match (then removes the hash so subsequent legitimate edits are not muted).
#[derive(Default, Clone)]
pub struct IgnoreHashes(pub Arc<Mutex<HashSet<ContentHash>>>);

impl IgnoreHashes {
    pub fn new() -> Self { Self::default() }
    pub fn register(&self, h: ContentHash) { self.0.lock().unwrap().insert(h); }
    pub fn check_and_remove(&self, h: &ContentHash) -> bool {
        self.0.lock().unwrap().remove(h)
    }
}

pub struct WatcherHandle {
    _debouncer: Debouncer<RecommendedWatcher, FileIdMap>,
}

/// Start watching `vault`. Spawns a thread that reads debounced events and
/// invokes `on_event` for each non-ignored, markdown-relevant path change.
pub fn start_watching<F>(
    vault: &Path,
    ignore: IgnoreHashes,
    on_event: F,
) -> Result<WatcherHandle>
where
    F: Fn(WatchEvent) + Send + 'static,
{
    let cb = move |res: DebounceEventResult| {
        let events = match res { Ok(e) => e, Err(_) => return };
        for ev in events {
            for path in ev.paths {
                if !is_markdown_target(&path) { continue; }

                if !path.exists() {
                    on_event(WatchEvent::Deleted(path));
                    continue;
                }
                // Loop prevention: hash current content, skip if it matches
                // a hash registered by our own writer.
                let bytes = match std::fs::read(&path) { Ok(b) => b, Err(_) => continue };
                let h = hash_content(&bytes);
                if ignore.check_and_remove(&h) { continue; }
                on_event(WatchEvent::Changed(path));
            }
        }
    };

    let mut debouncer = new_debouncer(Duration::from_millis(200), None, cb)?;
    debouncer.watcher().watch(vault, notify::RecursiveMode::Recursive)?;
    Ok(WatcherHandle { _debouncer: debouncer })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::mpsc::channel;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn ignore_hashes_register_and_consume() {
        let ig = IgnoreHashes::new();
        let h = [42u8; 32];
        ig.register(h);
        assert!(ig.check_and_remove(&h));
        assert!(!ig.check_and_remove(&h)); // single-shot
    }

    #[test]
    fn detects_external_md_change() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("a.md");
        std::fs::write(&p, "- [ ] x\n").unwrap();
        let (tx, rx) = channel();
        let ig = IgnoreHashes::new();
        let _h = start_watching(d.path(), ig, move |ev| { let _ = tx.send(ev); }).unwrap();

        // Give watcher a beat
        std::thread::sleep(Duration::from_millis(300));
        let mut f = std::fs::OpenOptions::new().append(true).open(&p).unwrap();
        f.write_all(b"- [ ] y\n").unwrap();
        drop(f);

        let ev = rx.recv_timeout(Duration::from_secs(3)).expect("expected event");
        match ev { WatchEvent::Changed(_) => {}, other => panic!("unexpected: {:?}", other) }
    }

    #[test]
    fn ignores_change_with_registered_hash() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("a.md");
        let initial = b"- [ ] x\n";
        std::fs::write(&p, initial).unwrap();
        let (tx, rx) = channel();
        let ig = IgnoreHashes::new();
        let ig2 = ig.clone();
        let _h = start_watching(d.path(), ig, move |ev| { let _ = tx.send(ev); }).unwrap();

        std::thread::sleep(Duration::from_millis(300));
        // Pre-register the hash of the content we're about to write
        let new_bytes = b"- [x] x\n";
        ig2.register(hash_content(new_bytes));
        std::fs::write(&p, new_bytes).unwrap();

        // No event should arrive within 1s
        assert!(rx.recv_timeout(Duration::from_secs(1)).is_err());
    }

    #[test]
    fn ignores_non_markdown_files() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("notes.txt");
        std::fs::write(&p, "x").unwrap();
        let (tx, rx) = channel();
        let ig = IgnoreHashes::new();
        let _h = start_watching(d.path(), ig, move |ev| { let _ = tx.send(ev); }).unwrap();

        std::thread::sleep(Duration::from_millis(300));
        std::fs::write(&p, "y").unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(1)).is_err());
    }
}
```

- [ ] **Step 3: Run tests (slower — filesystem timing)**

```powershell
cd src-tauri
cargo test --lib watcher -- --test-threads=1
cd ..
```

Expected: 4 passed. (`--test-threads=1` avoids races on the shared tempdir watcher.)

- [ ] **Step 4: Commit**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/watcher.rs
git commit -m "feat(watcher): debounced fs watcher with content-hash loop prevention"
```

---

## Task 8: commands.rs + AppState (Tauri IPC layer)

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `mod commands;` to `lib.rs`**

- [ ] **Step 2: Write `src-tauri/src/commands.rs`**

```rust
use crate::config;
use crate::error::{AppError, Result};
use crate::registry::TaskRegistry;
use crate::storage;
use crate::types::{AppConfig, Task};
use crate::watcher::IgnoreHashes;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    pub registry: Arc<RwLock<TaskRegistry>>,
    pub config: Arc<RwLock<AppConfig>>,
    pub ignore_hashes: IgnoreHashes,
    pub config_path: PathBuf,
}

#[tauri::command]
pub fn get_tasks(state: State<'_, AppState>) -> Result<Vec<Task>> {
    Ok(state.registry.read().unwrap().all_tasks())
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Result<AppConfig> {
    Ok(state.config.read().unwrap().clone())
}

#[tauri::command]
pub fn update_config(state: State<'_, AppState>, new_config: AppConfig) -> Result<()> {
    *state.config.write().unwrap() = new_config.clone();
    config::save_to(&state.config_path, &new_config)?;
    Ok(())
}

#[tauri::command]
pub fn toggle_task(state: State<'_, AppState>, task_id: String) -> Result<()> {
    let task = {
        let reg = state.registry.read().unwrap();
        reg.get(&task_id).cloned().ok_or_else(|| AppError::TaskNotFound(task_id.clone()))?
    };
    let new_hash = storage::toggle_task(&task.source_file, task.line_number)?;
    state.ignore_hashes.register(new_hash);
    state.registry.write().unwrap().refresh_file(&task.source_file)?;
    Ok(())
}

#[tauri::command]
pub fn add_task(state: State<'_, AppState>, text: String) -> Result<()> {
    let cfg = state.config.read().unwrap().clone();
    let vault = cfg.vault_path.ok_or(AppError::NoVault)?;
    let inbox = vault.join(&cfg.inbox_file);
    let new_hash = storage::append_task(&inbox, &text)?;
    state.ignore_hashes.register(new_hash);
    state.registry.write().unwrap().refresh_file(&inbox)?;
    Ok(())
}

/// Set the vault path. Caller (Vue side) should call `pick_vault_folder` first
/// via the dialog plugin to get the path string.
#[tauri::command]
pub fn set_vault(state: State<'_, AppState>, app: AppHandle, path: PathBuf) -> Result<()> {
    {
        let mut cfg = state.config.write().unwrap();
        cfg.vault_path = Some(path.clone());
        config::save_to(&state.config_path, &cfg)?;
    }
    state.registry.write().unwrap().rebuild_from_vault(&path)?;
    let _ = app.emit("vault-changed", path.to_string_lossy().to_string());
    let _ = app.emit("tasks-updated", ());
    Ok(())
}

#[tauri::command]
pub fn show_window(app: AppHandle) -> Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
    Ok(())
}

#[tauri::command]
pub fn hide_window(app: AppHandle) -> Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    Ok(())
}
```

- [ ] **Step 3: Build to check compilation**

```powershell
cd src-tauri
cargo build
cd ..
```

Expected: builds. Likely warnings about `app.emit` requiring import — fix by adding `use tauri::Emitter;` if needed.

- [ ] **Step 4: Commit**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/commands.rs
git commit -m "feat(commands): Tauri IPC layer with AppState for tasks/config/vault"
```

---

## Task 9: Wire Up `lib.rs` — App Init, Tray, Watcher Bridge

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Replace `src-tauri/src/lib.rs` body**

```rust
mod commands;
mod config;
mod error;
mod parser;
mod registry;
mod storage;
mod types;
mod watcher;

use crate::commands::AppState;
use crate::registry::TaskRegistry;
use crate::types::AppConfig;
use crate::watcher::{start_watching, IgnoreHashes, WatchEvent, WatcherHandle};
use std::sync::{Arc, Mutex, RwLock};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // ----- Config: load or default
            let app_config_dir = app.path().app_config_dir().expect("config dir");
            let config_path = config::config_file(&app_config_dir);
            let cfg = config::load_from(&config_path).unwrap_or_default();

            // ----- Registry: rebuild if vault is set (in background)
            let registry = Arc::new(RwLock::new(TaskRegistry::new()));
            let ignore_hashes = IgnoreHashes::new();
            let state = AppState {
                registry: registry.clone(),
                config: Arc::new(RwLock::new(cfg.clone())),
                ignore_hashes: ignore_hashes.clone(),
                config_path: config_path.clone(),
            };
            app.manage(state);

            // Hold the watcher handle so it lives as long as the app.
            let watcher_slot: Arc<Mutex<Option<WatcherHandle>>> = Arc::new(Mutex::new(None));
            app.manage(watcher_slot.clone());

            // If vault is configured, start initial scan + watcher.
            if let Some(vault) = cfg.vault_path.clone() {
                let app_handle = app.handle().clone();
                let registry_clone = registry.clone();
                let ignore_clone = ignore_hashes.clone();
                let watcher_slot_clone = watcher_slot.clone();
                std::thread::spawn(move || {
                    {
                        let mut reg = registry_clone.write().unwrap();
                        let _ = reg.rebuild_from_vault(&vault);
                    }
                    let _ = app_handle.emit("tasks-updated", ());

                    let app_for_cb = app_handle.clone();
                    let registry_for_cb = registry_clone.clone();
                    let handle = start_watching(&vault, ignore_clone, move |ev| {
                        match ev {
                            WatchEvent::Changed(p) | WatchEvent::Deleted(p) => {
                                let mut reg = registry_for_cb.write().unwrap();
                                let _ = reg.refresh_file(&p);
                            }
                        }
                        let _ = app_for_cb.emit("tasks-updated", ());
                    });
                    if let Ok(h) = handle {
                        *watcher_slot_clone.lock().unwrap() = Some(h);
                    }
                });
            }

            // ----- System tray
            let show_item = MenuItem::with_id(app, "show", "Show window", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "Hide window", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => toggle_window(app, true),
                    "hide" => toggle_window(app, false),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // Hide window from taskbar but keep tray-only behavior optional;
            // for v0.1 we show the window on first launch so the user can pick a vault.
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_tasks,
            commands::get_config,
            commands::update_config,
            commands::toggle_task,
            commands::add_task,
            commands::set_vault,
            commands::show_window,
            commands::hide_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn toggle_window(app: &AppHandle, show: bool) {
    if let Some(w) = app.get_webview_window("main") {
        if show { let _ = w.show(); let _ = w.set_focus(); }
        else { let _ = w.hide(); }
    }
}
```

- [ ] **Step 2: Update `src-tauri/tauri.conf.json` window config**

Replace the `app.windows` array with:

```json
"windows": [
  {
    "label": "main",
    "title": "Floaty Todo",
    "width": 380,
    "height": 600,
    "resizable": true,
    "alwaysOnTop": true,
    "decorations": true,
    "visible": true,
    "skipTaskbar": false
  }
]
```

(For v0.1 we keep `decorations: true` so the user has a normal close/min button. The custom titlebar from PLAN section 8 is a v0.2 task.)

Also ensure the `tray-icon` feature is enabled on the `tauri` dep in `Cargo.toml`. The scaffold line will look like `tauri = { version = "2", features = [] }` — change it to:

```toml
tauri = { version = "2", features = ["tray-icon"] }
```

(Preserve any other features the scaffold added — just add `"tray-icon"` to the array.)

And ensure `src-tauri/capabilities/default.json` has the dialog plugin permission. Replace its `permissions` array with:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "default capability",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default"
  ]
}
```

(`core:default` already covers event/window/webview. `dialog:default` covers `open`. If a permission error appears at runtime when invoking a specific command, look up the exact permission name in the Tauri docs and add it explicitly.)

- [ ] **Step 3: Build**

```powershell
cd src-tauri
cargo build
cd ..
```

Expected: builds. If `Emitter` import warning appears, it's already used.

- [ ] **Step 4: Commit**

```powershell
git add src-tauri/src/lib.rs src-tauri/Cargo.toml src-tauri/tauri.conf.json src-tauri/capabilities/default.json
git commit -m "feat(app): wire commands, tray icon, watcher bridge"
```

---

## Task 10: Frontend — Types, API Service, Pinia Stores

**Files:**
- Create: `src/types/task.ts`, `src/services/tauri-api.ts`, `src/stores/tasks.ts`, `src/stores/settings.ts`
- Modify: `src/main.ts`

- [ ] **Step 1: Create `src/types/task.ts`**

```ts
export interface Task {
  id: string;
  text: string;
  completed: boolean;
  source_file: string;
  line_number: number;
  indent: number;
}

export interface AppConfig {
  vault_path: string | null;
  inbox_file: string;
  always_on_top: boolean;
}
```

- [ ] **Step 2: Create `src/services/tauri-api.ts`**

```ts
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type { Task, AppConfig } from '../types/task';

export const api = {
  getTasks: () => invoke<Task[]>('get_tasks'),
  getConfig: () => invoke<AppConfig>('get_config'),
  updateConfig: (cfg: AppConfig) => invoke<void>('update_config', { newConfig: cfg }),
  toggleTask: (taskId: string) => invoke<void>('toggle_task', { taskId }),
  addTask: (text: string) => invoke<void>('add_task', { text }),
  setVault: (path: string) => invoke<void>('set_vault', { path }),

  pickVaultFolder: async (): Promise<string | null> => {
    const sel = await open({ directory: true, multiple: false });
    return typeof sel === 'string' ? sel : null;
  },

  onTasksUpdated: (cb: () => void): Promise<UnlistenFn> =>
    listen('tasks-updated', cb),
};
```

Install the plugin npm package:

```powershell
npm install @tauri-apps/plugin-dialog
```

- [ ] **Step 3: Create `src/stores/tasks.ts`**

```ts
import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { Task } from '../types/task';
import { api } from '../services/tauri-api';

export const useTaskStore = defineStore('tasks', () => {
  const tasks = ref<Task[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function refresh() {
    loading.value = true;
    error.value = null;
    try {
      tasks.value = await api.getTasks();
    } catch (e: any) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function toggle(id: string) {
    try { await api.toggleTask(id); await refresh(); }
    catch (e: any) { error.value = String(e); }
  }

  async function add(text: string) {
    if (!text.trim()) return;
    try { await api.addTask(text.trim()); await refresh(); }
    catch (e: any) { error.value = String(e); }
  }

  return { tasks, loading, error, refresh, toggle, add };
});
```

- [ ] **Step 4: Create `src/stores/settings.ts`**

```ts
import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { AppConfig } from '../types/task';
import { api } from '../services/tauri-api';

export const useSettingsStore = defineStore('settings', () => {
  const config = ref<AppConfig | null>(null);

  async function load() { config.value = await api.getConfig(); }

  async function pickAndSetVault(): Promise<boolean> {
    const path = await api.pickVaultFolder();
    if (!path) return false;
    await api.setVault(path);
    await load();
    return true;
  }

  return { config, load, pickAndSetVault };
});
```

- [ ] **Step 5: Update `src/main.ts`**

```ts
import { createApp } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';

createApp(App).use(createPinia()).mount('#app');
```

- [ ] **Step 6: Commit**

```powershell
git add src/types src/services src/stores src/main.ts package.json package-lock.json
git commit -m "feat(frontend): types, tauri-api service, Pinia stores"
```

---

## Task 11: Frontend Components — EmptyState, TaskItem, TaskList, App

**Files:**
- Create: `src/components/EmptyState.vue`, `src/components/TaskItem.vue`, `src/components/TaskList.vue`
- Modify: `src/App.vue`
- Create: `src/styles/main.css`

- [ ] **Step 1: Create `src/components/EmptyState.vue`**

```vue
<script setup lang="ts">
import { useSettingsStore } from '../stores/settings';
import { useTaskStore } from '../stores/tasks';

const settings = useSettingsStore();
const tasks = useTaskStore();

async function pick() {
  const ok = await settings.pickAndSetVault();
  if (ok) await tasks.refresh();
}
</script>

<template>
  <div class="empty">
    <h2>👋 Welcome to Floaty Todo</h2>
    <p>Pick an Obsidian vault folder. The app will scan all <code>.md</code> tasks inside.</p>
    <button @click="pick">Choose folder…</button>
  </div>
</template>

<style scoped>
.empty { padding: 2rem; text-align: center; color: var(--fg-muted); }
.empty button { margin-top: 1rem; padding: 0.6rem 1.2rem; cursor: pointer; }
</style>
```

- [ ] **Step 2: Create `src/components/TaskItem.vue`**

```vue
<script setup lang="ts">
import type { Task } from '../types/task';
import { useTaskStore } from '../stores/tasks';

defineProps<{ task: Task }>();
const tasks = useTaskStore();
</script>

<template>
  <label class="row" :class="{ done: task.completed }" :style="{ paddingLeft: 8 + task.indent * 12 + 'px' }">
    <input type="checkbox" :checked="task.completed" @change="tasks.toggle(task.id)" />
    <span class="text">{{ task.text }}</span>
  </label>
</template>

<style scoped>
.row { display: flex; align-items: center; gap: 0.5rem; padding: 0.35rem 0.5rem; cursor: pointer; }
.row:hover { background: var(--bg-hover); }
.row.done .text { text-decoration: line-through; color: var(--fg-muted); }
.text { flex: 1; user-select: text; }
</style>
```

- [ ] **Step 3: Create `src/components/TaskList.vue`**

```vue
<script setup lang="ts">
import { ref } from 'vue';
import { useTaskStore } from '../stores/tasks';
import TaskItem from './TaskItem.vue';

const tasks = useTaskStore();
const newText = ref('');

async function submit() {
  if (!newText.value.trim()) return;
  await tasks.add(newText.value);
  newText.value = '';
}
</script>

<template>
  <div class="list">
    <form class="add-row" @submit.prevent="submit">
      <input v-model="newText" placeholder="Add task, Enter to confirm…" />
      <button type="submit">+</button>
    </form>

    <div v-if="tasks.loading" class="hint">Loading…</div>
    <div v-else-if="tasks.error" class="error">{{ tasks.error }}</div>
    <div v-else-if="tasks.tasks.length === 0" class="hint">No tasks yet.</div>
    <div v-else class="rows">
      <TaskItem v-for="t in tasks.tasks" :key="t.id" :task="t" />
    </div>

    <div class="footer">
      <span>{{ tasks.tasks.filter(t => !t.completed).length }} todo · {{ tasks.tasks.filter(t => t.completed).length }} done</span>
      <button @click="tasks.refresh">↻</button>
    </div>
  </div>
</template>

<style scoped>
.list { display: flex; flex-direction: column; height: 100vh; }
.add-row { display: flex; padding: 0.5rem; gap: 0.4rem; border-bottom: 1px solid var(--border); }
.add-row input { flex: 1; padding: 0.4rem; }
.rows { flex: 1; overflow-y: auto; }
.hint, .error { padding: 1rem; text-align: center; color: var(--fg-muted); }
.error { color: #c33; }
.footer { display: flex; justify-content: space-between; align-items: center; padding: 0.4rem 0.6rem; border-top: 1px solid var(--border); font-size: 0.85em; color: var(--fg-muted); }
</style>
```

- [ ] **Step 4: Replace `src/App.vue`**

```vue
<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue';
import { useSettingsStore } from './stores/settings';
import { useTaskStore } from './stores/tasks';
import { api } from './services/tauri-api';
import EmptyState from './components/EmptyState.vue';
import TaskList from './components/TaskList.vue';

const settings = useSettingsStore();
const tasks = useTaskStore();
const hasVault = computed(() => !!settings.config?.vault_path);

let unlisten: (() => void) | null = null;

onMounted(async () => {
  await settings.load();
  if (hasVault.value) await tasks.refresh();
  unlisten = await api.onTasksUpdated(() => { tasks.refresh(); });
});

onUnmounted(() => { unlisten?.(); });
</script>

<template>
  <main>
    <EmptyState v-if="!hasVault" />
    <TaskList v-else />
  </main>
</template>

<style>
@import './styles/main.css';
</style>
```

- [ ] **Step 5: Create `src/styles/main.css`**

```css
:root {
  --bg: #ffffff;
  --bg-hover: #f0f0f0;
  --fg: #222;
  --fg-muted: #888;
  --border: #e5e5e5;
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #1e1e1e;
    --bg-hover: #2a2a2a;
    --fg: #e8e8e8;
    --fg-muted: #888;
    --border: #333;
  }
}
* { box-sizing: border-box; }
html, body, #app, main { margin: 0; padding: 0; height: 100%; }
body { font-family: system-ui, -apple-system, "Segoe UI", sans-serif; background: var(--bg); color: var(--fg); font-size: 14px; }
input, button { font: inherit; color: inherit; background: var(--bg); border: 1px solid var(--border); border-radius: 4px; }
button { cursor: pointer; padding: 0.3rem 0.6rem; }
button:hover { background: var(--bg-hover); }
```

- [ ] **Step 6: Commit**

```powershell
git add src/components src/App.vue src/styles
git commit -m "feat(ui): EmptyState, TaskItem, TaskList, dark-mode-aware styles"
```

---

## Task 12: End-to-End Manual Verification

**Files:** none (test only)

- [ ] **Step 1: Build and launch dev mode**

```powershell
npm run tauri dev
```

Expected: window opens showing the Welcome / EmptyState card.

- [ ] **Step 2: Pick a test vault**

Create a test folder + file outside the project (e.g. `D:\tmp\floaty-test\`):

```powershell
New-Item -ItemType Directory -Force D:\tmp\floaty-test | Out-Null
@"
# Test
- [ ] Task one
- [x] Task two (done)
- [ ] Task three with `code`
"@ | Set-Content -Encoding UTF8 D:\tmp\floaty-test\test.md
```

In the app, click "Choose folder…" → select `D:\tmp\floaty-test`. Expected: 3 tasks appear, "Task two" shown struck-through.

- [ ] **Step 3: Verify toggle works (round trip to disk)**

Click the checkbox on "Task one". Expected: it becomes struck-through in UI; `Get-Content D:\tmp\floaty-test\test.md` shows `- [x] Task one`.

- [ ] **Step 4: Verify add works**

Type "buy milk" in the top input, press Enter. Expected: a new "buy milk" task appears under `inbox.md` (created automatically). Verify file exists: `Get-Content D:\tmp\floaty-test\inbox.md`.

- [ ] **Step 5: Verify external change auto-refresh**

In a separate editor, add `- [ ] external task` to `D:\tmp\floaty-test\test.md` and save. Expected: within ~1s the new task appears in the app without manual refresh.

- [ ] **Step 6: Verify loop prevention (no duplicate refresh)**

Open dev tools (right click → Inspect Element). Console should show no rapid-fire repeated refreshes when you click checkboxes (each click → at most one `tasks-updated` event).

- [ ] **Step 7: Verify tray**

Click the X to close the window. App should remain running (tray icon visible). Click tray icon → window reappears. Right-click tray → "Quit" exits.

- [ ] **Step 8: Verify ignore list**

```powershell
New-Item -ItemType Directory -Force D:\tmp\floaty-test\.obsidian | Out-Null
"- [ ] should not show" | Set-Content D:\tmp\floaty-test\.obsidian\noise.md
```

Trigger a refresh (toggle any task). Expected: "should not show" does NOT appear in the list.

- [ ] **Step 9: Cleanup test data**

```powershell
Remove-Item -Recurse -Force D:\tmp\floaty-test
```

- [ ] **Step 10: Final commit + tag**

```powershell
git status                    # should be clean
git log --oneline             # review commits
git tag v0.1.0
```

---

## Done Criteria

v0.1 ships when all of the following are true:

- All Rust unit tests pass: `cd src-tauri && cargo test`
- `npm run tauri dev` opens to EmptyState on first run; pick-folder flow works
- Toggling a checkbox writes the new state to disk and reflects back in UI
- External edits to `.md` files refresh the UI within ~1s
- Files in `.obsidian/`, `.git/`, `node_modules/` are ignored
- App persists across window-close (lives in tray); tray menu can show/hide/quit
- A clean `git log` shows one commit per task with conventional-commit style messages
