# Quadrant View Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Group tasks inside each markdown source by Eisenhower quadrant inferred from header emoji (🔴🟡🟠🟢), with full UI support and a QuickAdd quadrant selector.

**Architecture:** Stateful single-pass scan in `parser.rs` infers `Option<Quadrant>` per task from the most recent header containing a quadrant emoji (child headers inherit). UI gains a `QuadrantGroup.vue` component slotted between `SourceGroup`/`FileGroup` and `TaskItem`. `storage::append_task_to_quadrant` writes new tasks into the target section, auto-creating headers when absent. No sidecar, no task-row mutation, line_number stability preserved for existing lines.

**Tech Stack:** Rust (regex / once_cell / serde / tempfile), Tauri 2 IPC, Vue 3 + TypeScript + Pinia, vue-i18n. Rust tested via `cargo test`; frontend validated via `npm run build` (vue-tsc + vite) and manual `npm run tauri dev`.

**Spec:** `docs/superpowers/specs/2026-05-15-quadrant-view-design.md`

---

## File Structure

### New
- `src/components/QuadrantGroup.vue` — single quadrant section (header + collapse + task list)

### Modified (Rust)
- `src-tauri/src/types.rs` — `Quadrant` enum, `Task.quadrant`, `AppConfig.auto_create_quadrant_headers`
- `src-tauri/src/error.rs` — `QuadrantHeaderMissing` variant
- `src-tauri/src/parser.rs` — stateful scan, header→quadrant inference
- `src-tauri/src/storage.rs` — `append_task_to_quadrant`
- `src-tauri/src/commands.rs` — `add_task` accepts `quadrant`

### Modified (Frontend)
- `src/types/task.ts` — mirror Rust types
- `src/services/tauri-api.ts` — `addTask` signature
- `src/stores/tasks.ts` — `add(text, sourceId?, quadrant?)`
- `src/stores/settings.ts` — surface `autoCreateQuadrantHeaders` + setter
- `src/components/SourceGroup.vue` — file source renders QuadrantGroup
- `src/components/FileGroup.vue` — folder source renders QuadrantGroup
- `src/components/TaskList.vue` — QuickAdd quadrant selector
- `src/i18n/locales/en.ts` + `src/i18n/locales/zh.ts` — quadrant names + settings copy
- `src/views/SettingsView.vue` — auto-create-headers toggle

---

## Task 1: Add `Quadrant` enum and `Task.quadrant` field

**Files:**
- Modify: `src-tauri/src/types.rs`

- [ ] **Step 1: Write the failing test**

Append to the `#[cfg(test)] mod tests` block in `src-tauri/src/types.rs` (create the mod block if it does not exist):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadrant_serializes_as_snake_case() {
        let v = serde_json::to_string(&Quadrant::UrgentImportant).unwrap();
        assert_eq!(v, "\"urgent_important\"");
        let v = serde_json::to_string(&Quadrant::NotUrgentNotImportant).unwrap();
        assert_eq!(v, "\"not_urgent_not_important\"");
    }

    #[test]
    fn task_quadrant_serializes_when_set() {
        let t = Task {
            id: "abc".into(),
            text: "hi".into(),
            completed: false,
            source_file: std::path::PathBuf::from("/x.md"),
            line_number: 1,
            indent: 0,
            source_id: "s".into(),
            quadrant: Some(Quadrant::UrgentImportant),
        };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("\"quadrant\":\"urgent_important\""));
    }

    #[test]
    fn task_quadrant_deserializes_missing_as_none() {
        let json = r#"{"id":"a","text":"hi","completed":false,"source_file":"/x.md","line_number":1,"indent":0,"source_id":"s"}"#;
        let t: Task = serde_json::from_str(json).unwrap();
        assert!(t.quadrant.is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml types::tests`
Expected: FAIL — `Quadrant` not found, `Task` missing `quadrant` field.

- [ ] **Step 3: Add `Quadrant` enum and `Task.quadrant`**

In `src-tauri/src/types.rs`, immediately after the existing `use` block at the top, add the enum:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Quadrant {
    UrgentImportant,
    NotUrgentImportant,
    UrgentNotImportant,
    NotUrgentNotImportant,
}
```

In the existing `Task` struct, append the new field after `source_id`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub text: String,
    pub completed: bool,
    pub source_file: PathBuf,
    pub line_number: usize,
    pub indent: usize,
    pub source_id: String,
    #[serde(default)]
    pub quadrant: Option<Quadrant>,
}
```

- [ ] **Step 4: Update parser.rs to keep compiling**

In `src-tauri/src/parser.rs`, in the existing `parse_file` function inside the loop that pushes `Task { … }`, append `quadrant: None,` to the struct literal so the file still compiles. (Real inference comes in Task 3.) The struct literal must now read:

```rust
tasks.push(Task {
    id,
    text: p.text,
    completed: p.completed,
    source_file: abs.clone(),
    line_number,
    indent: p.indent,
    source_id: source_id.to_string(),
    quadrant: None,
});
```

- [ ] **Step 5: Run all Rust tests to verify pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS — all existing tests + 3 new ones.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/types.rs src-tauri/src/parser.rs
git commit -m "feat(types): add Quadrant enum and Task.quadrant field"
```

---

## Task 2: Add `AppConfig.auto_create_quadrant_headers`

**Files:**
- Modify: `src-tauri/src/types.rs`

- [ ] **Step 1: Write the failing test**

Append to the same `#[cfg(test)] mod tests` in `types.rs`:

```rust
#[test]
fn config_defaults_auto_create_headers_to_true() {
    let c = AppConfig::default();
    assert!(c.auto_create_quadrant_headers);
}

#[test]
fn config_deserializes_missing_auto_create_as_true() {
    let json = r#"{"sources":[],"inbox_file":"inbox.md","always_on_top":true}"#;
    let c: AppConfig = serde_json::from_str(json).unwrap();
    assert!(c.auto_create_quadrant_headers);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml types::tests::config`
Expected: FAIL — field does not exist.

- [ ] **Step 3: Add field with default**

In `src-tauri/src/types.rs`, at the bottom of the existing `AppConfig` struct (after `hub_folder`), add:

```rust
    #[serde(default = "default_true")]
    pub auto_create_quadrant_headers: bool,
```

Add the helper function near `default_quick_actions`:

```rust
fn default_true() -> bool { true }
```

In `impl Default for AppConfig`, add the field to the `Self { … }` literal:

```rust
            auto_create_quadrant_headers: true,
```

- [ ] **Step 4: Run all Rust tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/types.rs
git commit -m "feat(config): add auto_create_quadrant_headers (default true)"
```

---

## Task 3: Parser infers quadrant from header emoji

**Files:**
- Modify: `src-tauri/src/parser.rs`

- [ ] **Step 1: Write the failing tests**

Append to `#[cfg(test)] mod tests` in `src-tauri/src/parser.rs`:

```rust
    #[test]
    fn parse_file_assigns_quadrant_from_header() {
        let f = write_tmp("## 🔴 Urgent+Important\n- [ ] a\n## 🟡 Important\n- [ ] b\n");
        let tasks = parse_file(f.path(), "s").unwrap();
        assert_eq!(tasks[0].quadrant, Some(crate::types::Quadrant::UrgentImportant));
        assert_eq!(tasks[1].quadrant, Some(crate::types::Quadrant::NotUrgentImportant));
    }

    #[test]
    fn parse_file_child_header_inherits_parent_quadrant() {
        let f = write_tmp("## 🔴 X\n### sub\n- [ ] a\n");
        let tasks = parse_file(f.path(), "s").unwrap();
        assert_eq!(tasks[0].quadrant, Some(crate::types::Quadrant::UrgentImportant));
    }

    #[test]
    fn parse_file_task_before_any_header_is_none() {
        let f = write_tmp("- [ ] a\n## 🔴 X\n- [ ] b\n");
        let tasks = parse_file(f.path(), "s").unwrap();
        assert_eq!(tasks[0].quadrant, None);
        assert_eq!(tasks[1].quadrant, Some(crate::types::Quadrant::UrgentImportant));
    }

    #[test]
    fn parse_file_multiple_same_quadrant_headers_merge() {
        let f = write_tmp("## 🔴 a\n- [ ] one\n## 🟡 b\n- [ ] two\n## 🔴 c\n- [ ] three\n");
        let tasks = parse_file(f.path(), "s").unwrap();
        assert_eq!(tasks[0].quadrant, Some(crate::types::Quadrant::UrgentImportant));
        assert_eq!(tasks[1].quadrant, Some(crate::types::Quadrant::NotUrgentImportant));
        assert_eq!(tasks[2].quadrant, Some(crate::types::Quadrant::UrgentImportant));
    }

    #[test]
    fn parse_file_recognises_any_header_level() {
        let f = write_tmp("# 🔴 H1\n- [ ] a\n###### 🟢 H6\n- [ ] b\n");
        let tasks = parse_file(f.path(), "s").unwrap();
        assert_eq!(tasks[0].quadrant, Some(crate::types::Quadrant::UrgentImportant));
        assert_eq!(tasks[1].quadrant, Some(crate::types::Quadrant::NotUrgentNotImportant));
    }

    #[test]
    fn parse_file_emoji_anywhere_in_header_text() {
        let f = write_tmp("## Today 🔴 urgent things\n- [ ] a\n");
        let tasks = parse_file(f.path(), "s").unwrap();
        assert_eq!(tasks[0].quadrant, Some(crate::types::Quadrant::UrgentImportant));
    }

    #[test]
    fn parse_file_mixed_emoji_picks_red_first() {
        let f = write_tmp("## 🟡 and 🔴 mixed\n- [ ] a\n");
        let tasks = parse_file(f.path(), "s").unwrap();
        assert_eq!(tasks[0].quadrant, Some(crate::types::Quadrant::UrgentImportant));
    }
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parser::tests`
Expected: FAIL — `parse_file` still always sets `quadrant: None`.

- [ ] **Step 3: Implement stateful scan**

In `src-tauri/src/parser.rs`, replace the `parse_file` function with the version below. Add a `HEADER_REGEX` static alongside `TASK_REGEX` and a private helper `detect_quadrant`. Resulting top of file:

```rust
use crate::error::Result;
use crate::types::{hash_content, Quadrant, Task};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;

static TASK_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\s*)[-*+]\s+\[([ xX])\]\s+(.+?)\s*$").unwrap()
});

static HEADER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s{0,3}(#{1,6})\s+(.+?)\s*$").unwrap()
});

/// Probe a header's text for a quadrant emoji. Priority order
/// 🔴 → 🟡 → 🟠 → 🟢 makes mixed-emoji headers deterministic.
fn detect_quadrant(header_text: &str) -> Option<Quadrant> {
    if header_text.contains('\u{1F534}') { return Some(Quadrant::UrgentImportant); }
    if header_text.contains('\u{1F7E1}') { return Some(Quadrant::NotUrgentImportant); }
    if header_text.contains('\u{1F7E0}') { return Some(Quadrant::UrgentNotImportant); }
    if header_text.contains('\u{1F7E2}') { return Some(Quadrant::NotUrgentNotImportant); }
    None
}
```

Replace `parse_file` body:

```rust
pub fn parse_file(path: &Path, source_id: &str) -> Result<Vec<Task>> {
    let raw = std::fs::read(path)?;
    let content = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) { &raw[3..] } else { &raw[..] };
    let text = String::from_utf8_lossy(content);

    let abs = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut tasks = Vec::new();
    let mut current_quadrant: Option<Quadrant> = None;

    for (i, line) in text.lines().enumerate() {
        let line_number = i + 1;
        if let Some(h) = HEADER_REGEX.captures(line) {
            if let Some(q) = detect_quadrant(h.get(2).unwrap().as_str()) {
                current_quadrant = Some(q);
            }
            continue;
        }
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
                quadrant: current_quadrant,
            });
        }
    }
    Ok(tasks)
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parser::tests`
Expected: PASS — all existing + 7 new quadrant tests.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/parser.rs
git commit -m "feat(parser): infer quadrant from markdown header emoji"
```

---

## Task 4: Add `QuadrantHeaderMissing` error variant

**Files:**
- Modify: `src-tauri/src/error.rs`

- [ ] **Step 1: Add the variant**

In `src-tauri/src/error.rs`, append a variant inside the `AppError` enum, after `CommandFailed`:

```rust
    #[error("quadrant header missing for {0:?}")]
    QuadrantHeaderMissing(crate::types::Quadrant),
```

- [ ] **Step 2: Run `cargo check` to verify it compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: success (no errors).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/error.rs
git commit -m "feat(error): add QuadrantHeaderMissing variant"
```

---

## Task 5: Storage — append task into a specific quadrant section

**Files:**
- Modify: `src-tauri/src/storage.rs`

- [ ] **Step 1: Write the failing tests**

Append to `#[cfg(test)] mod tests` in `storage.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml storage::tests::append_to`
Expected: FAIL — `append_task_to_quadrant` not defined.

- [ ] **Step 3: Implement the function**

In `src-tauri/src/storage.rs`, add to the top `use` block:

```rust
use crate::types::Quadrant;
```

Add at the end of the file (before `#[cfg(test)]`):

```rust
/// CN/emoji label written when auto-creating a quadrant header.
/// Matches the todo skill template so future re-parses match the same Quadrant.
fn quadrant_header_label(q: Quadrant) -> &'static str {
    match q {
        Quadrant::UrgentImportant => "🔴 紧急+重要",
        Quadrant::NotUrgentImportant => "🟡 重要不紧急",
        Quadrant::UrgentNotImportant => "🟠 紧急不重要",
        Quadrant::NotUrgentNotImportant => "🟢 不紧急不重要",
    }
}

/// Find the byte index *just after* the last task / non-empty line inside
/// the section that begins on `header_line_idx`. Returns the insertion index
/// (in the original `content`) where a new `- [ ] task\n` should go.
fn find_section_insertion_point(content: &str, header_line_idx: usize) -> usize {
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    if header_line_idx >= lines.len() {
        return content.len();
    }
    // Header level = leading `#` count.
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

    // Locate first header whose text contains the target quadrant emoji.
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
            // Ensure inserted line begins on its own line.
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
```

In `src-tauri/src/parser.rs`, expose the regex and detector as `pub(crate)` so `storage.rs` can reuse them:

```rust
pub(crate) fn header_regex() -> &'static Regex {
    &HEADER_REGEX
}

pub(crate) fn detect_quadrant_pub(header_text: &str) -> Option<Quadrant> {
    detect_quadrant(header_text)
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS — all existing + 6 new storage tests.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/storage.rs src-tauri/src/parser.rs
git commit -m "feat(storage): append_task_to_quadrant with auto-create header"
```

---

## Task 6: Wire `add_task` command to accept `quadrant`

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Update the command signature**

In `src-tauri/src/commands.rs`, find the existing `add_task` command. Replace its full body with:

```rust
#[tauri::command]
pub fn add_task(
    state: State<'_, AppState>,
    text: String,
    source_id: Option<String>,
    quadrant: Option<crate::types::Quadrant>,
) -> Result<()> {
    let cfg = state.config.read().unwrap().clone();
    if cfg.sources.is_empty() {
        return Err(AppError::NoSources);
    }
    let target_id = source_id.or(cfg.default_source_id.clone()).ok_or(AppError::NoSources)?;
    let source = cfg
        .sources
        .iter()
        .find(|s| s.id == target_id)
        .cloned()
        .ok_or_else(|| AppError::SourceNotFound(target_id.clone()))?;

    let target_file = match source.kind {
        SourceKind::Folder => source.path.join(&cfg.inbox_file),
        SourceKind::File => source.path.clone(),
    };
    let new_hash = storage::append_task_to_quadrant(
        &target_file,
        &text,
        quadrant,
        cfg.auto_create_quadrant_headers,
    )?;
    state.ignore_hashes.register(new_hash);
    state.registry.write().unwrap().refresh_file(&source, &target_file)?;
    Ok(())
}
```

- [ ] **Step 2: Verify the project still compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: success.

- [ ] **Step 3: Run all Rust tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(commands): add_task accepts optional quadrant"
```

---

## Task 7: Frontend mirror — TS types

**Files:**
- Modify: `src/types/task.ts`

- [ ] **Step 1: Read the existing file**

Open `src/types/task.ts` and inspect the current shape of `Task` and `AppConfig`. Identify where `source_id` ends in `Task` and where `hub_folder` ends in `AppConfig`.

- [ ] **Step 2: Add `Quadrant` and extend interfaces**

In `src/types/task.ts`, add:

```ts
export type Quadrant =
  | 'urgent_important'
  | 'not_urgent_important'
  | 'urgent_not_important'
  | 'not_urgent_not_important';
```

In the existing `Task` interface, add after `source_id`:

```ts
  quadrant: Quadrant | null;
```

In the existing `AppConfig` interface, add after `hub_folder`:

```ts
  auto_create_quadrant_headers: boolean;
```

- [ ] **Step 3: Run TypeScript check**

Run: `npm run build`
Expected: pass (vue-tsc will surface anywhere the new fields are required but unhandled — fix only call-sites that error; do not pre-emptively touch others).

If build errors mention `quadrant` missing in object literals, the offending file is doing a partial cast; leave it for the Vue tasks below. If unrelated build errors appear, restore the file and investigate.

- [ ] **Step 4: Commit**

```bash
git add src/types/task.ts
git commit -m "feat(types): mirror Quadrant + auto_create_quadrant_headers in TS"
```

---

## Task 8: Frontend API + store — `addTask` accepts `quadrant`

**Files:**
- Modify: `src/services/tauri-api.ts`, `src/stores/tasks.ts`

- [ ] **Step 1: Update API wrapper**

In `src/services/tauri-api.ts`, locate the existing `addTask` wrapper inside the `api` object. Replace its signature and body with:

```ts
  async addTask(text: string, sourceId?: string, quadrant?: Quadrant | null): Promise<void> {
    await invoke('add_task', {
      text,
      sourceId: sourceId ?? null,
      quadrant: quadrant ?? null,
    });
  },
```

Add the import at the top of the file if `Quadrant` is not already imported:

```ts
import type { Quadrant } from '../types/task';
```

- [ ] **Step 2: Update store**

In `src/stores/tasks.ts`, find the existing `add` action. Replace its signature and body with:

```ts
  async function add(text: string, sourceId?: string, quadrant?: Quadrant | null) {
    await api.addTask(text, sourceId, quadrant);
    await silentRefresh();
  }
```

Add the import at the top if missing:

```ts
import type { Quadrant } from '../types/task';
```

- [ ] **Step 3: Build to verify**

Run: `npm run build`
Expected: pass.

- [ ] **Step 4: Commit**

```bash
git add src/services/tauri-api.ts src/stores/tasks.ts
git commit -m "feat(api): addTask passes optional quadrant through to Rust"
```

---

## Task 9: Settings store — surface `autoCreateQuadrantHeaders`

**Files:**
- Modify: `src/stores/settings.ts`

- [ ] **Step 1: Add reactive + setter**

In `src/stores/settings.ts`, locate the `hubFolder` computed/ref pattern and add a parallel one after it:

```ts
  const autoCreateQuadrantHeaders = computed(
    () => config.value?.auto_create_quadrant_headers ?? true,
  );

  async function setAutoCreateQuadrantHeaders(on: boolean): Promise<void> {
    if (!config.value) return;
    const next = { ...config.value, auto_create_quadrant_headers: on };
    await api.updateConfig(next);
    config.value = next;
  }
```

In the return object at the bottom of the store, add `autoCreateQuadrantHeaders` and `setAutoCreateQuadrantHeaders`.

- [ ] **Step 2: Build to verify**

Run: `npm run build`
Expected: pass.

- [ ] **Step 3: Commit**

```bash
git add src/stores/settings.ts
git commit -m "feat(settings): expose autoCreateQuadrantHeaders + setter"
```

---

## Task 10: New component `QuadrantGroup.vue`

**Files:**
- Create: `src/components/QuadrantGroup.vue`

- [ ] **Step 1: Create the component**

Write `src/components/QuadrantGroup.vue` with this content:

```vue
<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { bindCollapse } from '../composables/useCollapse';
import type { Quadrant, Task } from '../types/task';
import TaskItem from './TaskItem.vue';

const props = defineProps<{
  quadrant: Quadrant | null;
  tasks: Task[];
}>();

const { t } = useI18n();

const collapsed = ref(false);
bindCollapse((v) => { collapsed.value = v; });

function emoji(q: Quadrant | null): string {
  switch (q) {
    case 'urgent_important': return '🔴';
    case 'not_urgent_important': return '🟡';
    case 'urgent_not_important': return '🟠';
    case 'not_urgent_not_important': return '🟢';
    default: return '⚪';
  }
}

function nameKey(q: Quadrant | null): string {
  switch (q) {
    case 'urgent_important': return 'quadrant.urgent_important';
    case 'not_urgent_important': return 'quadrant.not_urgent_important';
    case 'urgent_not_important': return 'quadrant.urgent_not_important';
    case 'not_urgent_not_important': return 'quadrant.not_urgent_not_important';
    default: return 'quadrant.unsorted';
  }
}
</script>

<template>
  <div v-if="tasks.length > 0" class="quadrant-group" :class="{ collapsed }">
    <button class="quadrant-header" @click="collapsed = !collapsed">
      <span class="caret">{{ collapsed ? '▶' : '▼' }}</span>
      <span class="emoji">{{ emoji(quadrant) }}</span>
      <span class="name">{{ t(nameKey(quadrant)) }}</span>
      <span class="count">{{ tasks.length }}</span>
    </button>
    <div v-show="!collapsed" class="quadrant-tasks">
      <TaskItem v-for="task in tasks" :key="task.id" :task="task" />
    </div>
  </div>
</template>

<style scoped>
.quadrant-group {
  margin: 0.25rem 0 0.5rem;
}
.quadrant-header {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  width: 100%;
  padding: 0.15rem 0.4rem;
  background: none;
  border: 0;
  font: inherit;
  color: var(--text-muted, #888);
  cursor: pointer;
  text-align: left;
}
.quadrant-header:hover { color: var(--text, #ddd); }
.quadrant-header .caret { width: 0.8em; font-size: 0.75em; }
.quadrant-header .emoji { font-size: 0.95em; }
.quadrant-header .name { flex: 1; font-size: 0.82em; }
.quadrant-header .count {
  font-variant-numeric: tabular-nums;
  font-size: 0.78em;
  opacity: 0.7;
}
.quadrant-tasks { padding-left: 0.6rem; }
</style>
```

- [ ] **Step 2: Build to verify**

Run: `npm run build`
Expected: pass (i18n keys not yet defined — vue-i18n falls back to key strings at runtime, no build error).

- [ ] **Step 3: Commit**

```bash
git add src/components/QuadrantGroup.vue
git commit -m "feat(ui): QuadrantGroup component (collapsible, count-aware)"
```

---

## Task 11: i18n strings for quadrants

**Files:**
- Modify: `src/i18n/locales/en.ts`, `src/i18n/locales/zh.ts`

- [ ] **Step 1: Add English strings**

In `src/i18n/locales/en.ts`, add to the top-level object (next to other top-level keys):

```ts
  quadrant: {
    urgent_important: 'Urgent · Important',
    not_urgent_important: 'Important · Not Urgent',
    urgent_not_important: 'Urgent · Not Important',
    not_urgent_not_important: 'Neither',
    unsorted: 'Unsorted',
  },
  settings: {
    // ... existing keys preserved
    auto_create_quadrant_headers: 'Auto-create quadrant headers',
    auto_create_quadrant_headers_help:
      'When you add a task to a quadrant whose header is missing, create the header automatically.',
  },
```

(If `settings` already exists, merge the two new keys into it instead of duplicating.)

- [ ] **Step 2: Add Chinese strings**

In `src/i18n/locales/zh.ts`, add:

```ts
  quadrant: {
    urgent_important: '紧急+重要',
    not_urgent_important: '重要不紧急',
    urgent_not_important: '紧急不重要',
    not_urgent_not_important: '不紧急不重要',
    unsorted: '未分类',
  },
  settings: {
    // ... existing keys preserved
    auto_create_quadrant_headers: '自动创建象限标题',
    auto_create_quadrant_headers_help:
      '向一个不存在的象限添加任务时，自动追加 `## 🔴 紧急+重要` 等标题。',
  },
```

- [ ] **Step 3: Build to verify**

Run: `npm run build`
Expected: pass.

- [ ] **Step 4: Commit**

```bash
git add src/i18n/locales/en.ts src/i18n/locales/zh.ts
git commit -m "feat(i18n): quadrant names + auto-create-header settings copy"
```

---

## Task 12: `SourceGroup.vue` — file source renders QuadrantGroup

**Files:**
- Modify: `src/components/SourceGroup.vue`

- [ ] **Step 1: Read the file**

Open `src/components/SourceGroup.vue`. Locate the branch that handles `kind === 'file'` and currently renders `TaskItem` directly from a `tasks` array.

- [ ] **Step 2: Add the grouping computed**

In the `<script setup>` block, add (next to other computed/utility code, before the template):

```ts
import QuadrantGroup from './QuadrantGroup.vue';
import type { Quadrant, Task } from '../types/task';

const QUADRANT_ORDER: (Quadrant | null)[] = [
  'urgent_important',
  'not_urgent_important',
  'urgent_not_important',
  'not_urgent_not_important',
  null,
];

function groupByQuadrant(tasks: Task[]): Array<{ quadrant: Quadrant | null; tasks: Task[] }> {
  const buckets = new Map<Quadrant | null, Task[]>();
  for (const q of QUADRANT_ORDER) buckets.set(q, []);
  for (const t of tasks) buckets.get(t.quadrant ?? null)!.push(t);
  return QUADRANT_ORDER
    .map((q) => ({ quadrant: q, tasks: buckets.get(q)! }))
    .filter((g) => g.tasks.length > 0);
}
```

- [ ] **Step 3: Switch file-source rendering**

In the template, replace the section that iterates `TaskItem` directly for file sources with:

```vue
<QuadrantGroup
  v-for="g in groupByQuadrant(tasksForSource)"
  :key="String(g.quadrant)"
  :quadrant="g.quadrant"
  :tasks="g.tasks"
/>
```

Where `tasksForSource` is the existing computed/prop array of tasks for this source.

- [ ] **Step 4: Build to verify**

Run: `npm run build`
Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add src/components/SourceGroup.vue
git commit -m "feat(ui): file source renders QuadrantGroup × N"
```

---

## Task 13: `FileGroup.vue` — folder source files render QuadrantGroup

**Files:**
- Modify: `src/components/FileGroup.vue`

- [ ] **Step 1: Read the file**

Open `src/components/FileGroup.vue`. Identify where it iterates `TaskItem` over its tasks prop.

- [ ] **Step 2: Reuse grouping logic**

In `<script setup>`, add identical grouping code (DRY would extract to a composable, but for one extra call site this is cheaper than a new module):

```ts
import QuadrantGroup from './QuadrantGroup.vue';
import type { Quadrant, Task } from '../types/task';

const QUADRANT_ORDER: (Quadrant | null)[] = [
  'urgent_important',
  'not_urgent_important',
  'urgent_not_important',
  'not_urgent_not_important',
  null,
];

function groupByQuadrant(tasks: Task[]): Array<{ quadrant: Quadrant | null; tasks: Task[] }> {
  const buckets = new Map<Quadrant | null, Task[]>();
  for (const q of QUADRANT_ORDER) buckets.set(q, []);
  for (const t of tasks) buckets.get(t.quadrant ?? null)!.push(t);
  return QUADRANT_ORDER
    .map((q) => ({ quadrant: q, tasks: buckets.get(q)! }))
    .filter((g) => g.tasks.length > 0);
}
```

- [ ] **Step 3: Switch template**

In the template, replace the `TaskItem v-for` over `tasks` with:

```vue
<QuadrantGroup
  v-for="g in groupByQuadrant(tasks)"
  :key="String(g.quadrant)"
  :quadrant="g.quadrant"
  :tasks="g.tasks"
/>
```

- [ ] **Step 4: Build to verify**

Run: `npm run build`
Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add src/components/FileGroup.vue
git commit -m "feat(ui): folder-source files render QuadrantGroup × N"
```

---

## Task 14: `TaskList.vue` — QuickAdd quadrant selector

**Files:**
- Modify: `src/components/TaskList.vue`

- [ ] **Step 1: Read the file**

Open `src/components/TaskList.vue` and find the QuickAdd form (input + add button + source selector).

- [ ] **Step 2: Add quadrant selector**

In `<script setup>`, add:

```ts
import type { Quadrant } from '../types/task';

const QUADRANT_BUTTONS: Array<{ q: Quadrant | null; emoji: string; tooltipKey: string }> = [
  { q: 'urgent_important', emoji: '🔴', tooltipKey: 'quadrant.urgent_important' },
  { q: 'not_urgent_important', emoji: '🟡', tooltipKey: 'quadrant.not_urgent_important' },
  { q: 'urgent_not_important', emoji: '🟠', tooltipKey: 'quadrant.urgent_not_important' },
  { q: 'not_urgent_not_important', emoji: '🟢', tooltipKey: 'quadrant.not_urgent_not_important' },
  { q: null, emoji: '⚪', tooltipKey: 'quadrant.unsorted' },
];

const LAST_QUADRANT_KEY = 'floaty.lastQuadrant';

function loadLastQuadrant(): Quadrant | null {
  const v = localStorage.getItem(LAST_QUADRANT_KEY);
  if (v === 'urgent_important' || v === 'not_urgent_important'
      || v === 'urgent_not_important' || v === 'not_urgent_not_important') {
    return v;
  }
  return null;
}

const selectedQuadrant = ref<Quadrant | null>(loadLastQuadrant());

function pickQuadrant(q: Quadrant | null) {
  selectedQuadrant.value = q;
  localStorage.setItem(LAST_QUADRANT_KEY, q ?? 'unsorted');
}
```

Modify the existing QuickAdd submit handler so that on submit it passes `selectedQuadrant.value` as the third arg of `tasks.add(text, sourceId, quadrant)`:

```ts
await tasks.add(text.value.trim(), selectedSourceId.value, selectedQuadrant.value);
```

(Exact variable names follow the existing file — keep them as-is.)

- [ ] **Step 3: Add buttons to template**

In the QuickAdd template row, after the existing source selector, add:

```vue
<div class="quadrant-picker">
  <button
    v-for="b in QUADRANT_BUTTONS"
    :key="String(b.q)"
    type="button"
    class="quadrant-btn"
    :class="{ active: selectedQuadrant === b.q }"
    :title="$t(b.tooltipKey)"
    @click="pickQuadrant(b.q)"
  >{{ b.emoji }}</button>
</div>
```

Add styles in the existing `<style scoped>` block:

```css
.quadrant-picker {
  display: inline-flex;
  gap: 0.1rem;
  margin-left: 0.3rem;
}
.quadrant-btn {
  background: none;
  border: 1px solid transparent;
  border-radius: 0.25rem;
  padding: 0.1rem 0.25rem;
  font-size: 0.95em;
  cursor: pointer;
  opacity: 0.55;
}
.quadrant-btn:hover { opacity: 0.85; }
.quadrant-btn.active {
  opacity: 1;
  border-color: var(--accent, #5af);
  background: var(--accent-bg, rgba(85, 170, 255, 0.12));
}
```

- [ ] **Step 4: Build to verify**

Run: `npm run build`
Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add src/components/TaskList.vue
git commit -m "feat(ui): QuickAdd quadrant picker + localStorage persistence"
```

---

## Task 15: Settings — auto-create-headers toggle + end-to-end manual verification

**Files:**
- Modify: `src/views/SettingsView.vue`

- [ ] **Step 1: Read the file**

Open `src/views/SettingsView.vue` and locate the existing "Appearance" or "Hub folder" sections to find the pattern for switch/toggle rows.

- [ ] **Step 2: Add a Behavior section**

In `<script setup>`, import and read from the settings store:

```ts
const settings = useSettingsStore();
// (existing imports / settings usage stays)
```

Add a new section to the template, structured like existing toggle rows:

```vue
<section class="settings-section">
  <h3>{{ $t('settings.behavior') }}</h3>
  <label class="settings-row">
    <input
      type="checkbox"
      :checked="settings.autoCreateQuadrantHeaders"
      @change="(e: Event) => settings.setAutoCreateQuadrantHeaders((e.target as HTMLInputElement).checked)"
    />
    <div class="settings-label">
      <span>{{ $t('settings.auto_create_quadrant_headers') }}</span>
      <small>{{ $t('settings.auto_create_quadrant_headers_help') }}</small>
    </div>
  </label>
</section>
```

- [ ] **Step 3: Add `settings.behavior` to i18n**

In `src/i18n/locales/en.ts`, inside the `settings` object, add: `behavior: 'Behavior',`
In `src/i18n/locales/zh.ts`, inside the `settings` object, add: `behavior: '行为',`

- [ ] **Step 4: Build to verify**

Run: `npm run build`
Expected: pass.

- [ ] **Step 5: Manual smoke test**

Run: `npm run tauri dev`

Then in the running app:

1. Add a file source pointing at the project's `D:\Projects\Floaty-todo\TODO.md`. Verify it renders 4 quadrants (🔴 / 🟡 / 🟠 / 🟢) with the expected counts (1 / 8 / 4 / 9) and **no** ⚪ Unsorted section.
2. In QuickAdd, type "smoke test", select 🟠, click Add. Verify the task appears under 🟠 and the file on disk has `- [ ] smoke test` inserted under `## 🟠 紧急不重要`.
3. Add another file source pointing at a fresh `.md` with no quadrant headers and a single `- [ ] hi` line. Verify everything renders inside ⚪ Unsorted.
4. With QuickAdd selecting 🔴 and the no-headers source as target, click Add. Verify a `## 🔴 紧急+重要` block is appended at EOF with the new task underneath.
5. Open Settings, turn off "Auto-create quadrant headers", repeat step 4 against another no-headers file. Verify the add fails (toast/console error containing `QuadrantHeaderMissing`).

Note any failures with reproduction steps. If all five pass, proceed.

- [ ] **Step 6: Commit**

```bash
git add src/views/SettingsView.vue src/i18n/locales/en.ts src/i18n/locales/zh.ts
git commit -m "feat(settings): toggle for auto_create_quadrant_headers"
```

---

## Done Criteria

- All 15 tasks committed
- `cargo test --manifest-path src-tauri/Cargo.toml` is green
- `npm run build` is clean (no vue-tsc errors)
- Manual smoke test scenarios in Task 15 all pass
- CHANGELOG.md has a section summarising the feature (add at the very end as a single edit, not per-task)
