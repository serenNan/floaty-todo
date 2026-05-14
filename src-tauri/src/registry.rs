use crate::error::Result;
use crate::parser;
use crate::types::{Source, SourceKind, Task};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Default)]
pub struct TaskRegistry {
    tasks: HashMap<String, Task>,
    /// (source_id, canonical_file_path) → list of task ids in that file.
    /// Keyed by `(source_id, path)` so the same file appearing under two
    /// different sources (rare but possible — overlapping folder + file)
    /// is handled independently.
    by_file: HashMap<(String, PathBuf), Vec<String>>,
}

impl TaskRegistry {
    pub fn new() -> Self { Self::default() }

    /// Full rebuild from the configured sources. Replaces all stored state.
    pub fn rebuild_from_sources(&mut self, sources: &[Source]) -> Result<()> {
        self.tasks.clear();
        self.by_file.clear();
        for src in sources {
            self.rebuild_source(src);
        }
        Ok(())
    }

    /// Scan / re-scan a single source. Removes any existing entries belonging
    /// to that source first, then re-parses.
    pub fn rebuild_source(&mut self, src: &Source) {
        // Drop everything belonging to this source.
        let keys: Vec<_> = self
            .by_file
            .keys()
            .filter(|(sid, _)| sid == &src.id)
            .cloned()
            .collect();
        for k in &keys {
            if let Some(ids) = self.by_file.remove(k) {
                for id in ids {
                    self.tasks.remove(&id);
                }
            }
        }

        if !src.path.exists() {
            return;
        }
        match src.kind {
            SourceKind::File => {
                if is_markdown_target(&src.path) {
                    let _ = self.refresh_file(src, &src.path);
                }
            }
            SourceKind::Folder => {
                for entry in WalkDir::new(&src.path).into_iter().filter_map(|e| e.ok()) {
                    if !entry.file_type().is_file() {
                        continue;
                    }
                    let path = entry.path();
                    if !is_markdown_target(path) {
                        continue;
                    }
                    let _ = self.refresh_file(src, path);
                }
            }
        }
    }

    /// Re-parse a single file within a known source. For File sources, the
    /// file must equal `src.path`. For Folder sources, the file must be
    /// inside (and a markdown target).
    pub fn refresh_file(&mut self, src: &Source, file: &Path) -> Result<()> {
        // Skip files that don't belong to this source.
        if !file_belongs_to_source(src, file) {
            return Ok(());
        }

        let canonical = best_effort_canonical(file);
        let key = (src.id.clone(), canonical.clone());

        // Remove stale entries.
        if let Some(old_ids) = self.by_file.remove(&key) {
            for id in old_ids {
                self.tasks.remove(&id);
            }
        }

        if !file.exists() {
            return Ok(());
        }

        let parsed = parser::parse_file(file, &src.id)?;
        let ids: Vec<String> = parsed.iter().map(|t| t.id.clone()).collect();
        for t in parsed {
            self.tasks.insert(t.id.clone(), t);
        }
        self.by_file.insert(key, ids);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Task> { self.tasks.get(id) }

    pub fn all_tasks(&self) -> Vec<Task> {
        let mut v: Vec<Task> = self.tasks.values().cloned().collect();
        v.sort_by(|a, b| {
            a.source_id
                .cmp(&b.source_id)
                .then(a.source_file.cmp(&b.source_file))
                .then(a.line_number.cmp(&b.line_number))
        });
        v
    }
}

/// True if `file` is a valid target for `src` (matches kind + ignore rules).
fn file_belongs_to_source(src: &Source, file: &Path) -> bool {
    if !is_markdown_target(file) {
        return false;
    }
    match src.kind {
        SourceKind::File => {
            // Compare canonicalized paths so symlinks and case-mismatch don't bite.
            best_effort_canonical(file) == best_effort_canonical(&src.path)
        }
        SourceKind::Folder => {
            let src_canon = best_effort_canonical(&src.path);
            let file_canon = best_effort_canonical(file);
            file_canon.starts_with(&src_canon)
        }
    }
}

/// Build a stable canonical path even when the file no longer exists.
/// Uses `dunce::canonicalize` so Windows verbatim prefixes (`\\?\`) are
/// stripped — keeps paths friendly for prompts / VS Code title bars.
pub fn best_effort_canonical(path: &Path) -> PathBuf {
    if let Ok(c) = dunce::canonicalize(path) {
        return c;
    }
    if let Some(parent) = path.parent() {
        if let Ok(cp) = dunce::canonicalize(parent) {
            if let Some(name) = path.file_name() {
                return cp.join(name);
            }
        }
    }
    dunce::simplified(path).to_path_buf()
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

    fn folder_source(path: &Path) -> Source {
        Source {
            id: Source::id_for(path),
            path: path.to_path_buf(),
            kind: SourceKind::Folder,
            label: None,
            project_root: None,
        }
    }

    fn file_source(path: &Path) -> Source {
        Source {
            id: Source::id_for(path),
            path: path.to_path_buf(),
            kind: SourceKind::File,
            label: None,
            project_root: None,
        }
    }

    #[test]
    fn folder_source_collects_tasks() {
        let d = TempDir::new().unwrap();
        touch(d.path(), "a.md", "- [ ] one\n- [x] done\n");
        touch(d.path(), "sub/b.md", "- [ ] two\n");
        let mut r = TaskRegistry::new();
        r.rebuild_from_sources(&[folder_source(d.path())]).unwrap();
        assert_eq!(r.all_tasks().len(), 3);
    }

    #[test]
    fn folder_source_skips_ignored_dirs() {
        let d = TempDir::new().unwrap();
        touch(d.path(), "a.md", "- [ ] keep\n");
        touch(d.path(), ".obsidian/x.md", "- [ ] skip\n");
        touch(d.path(), ".git/y.md", "- [ ] skip\n");
        touch(d.path(), "node_modules/z.md", "- [ ] skip\n");
        let mut r = TaskRegistry::new();
        r.rebuild_from_sources(&[folder_source(d.path())]).unwrap();
        let texts: Vec<_> = r.all_tasks().iter().map(|t| t.text.clone()).collect();
        assert_eq!(texts, vec!["keep"]);
    }

    #[test]
    fn file_source_collects_only_target_file() {
        let d = TempDir::new().unwrap();
        let p = touch(d.path(), "todo.md", "- [ ] a\n- [ ] b\n");
        touch(d.path(), "other.md", "- [ ] should not show\n");
        let mut r = TaskRegistry::new();
        r.rebuild_from_sources(&[file_source(&p)]).unwrap();
        let texts: Vec<_> = r.all_tasks().iter().map(|t| t.text.clone()).collect();
        assert_eq!(texts, vec!["a", "b"]);
    }

    #[test]
    fn multi_source_aggregates() {
        let d = TempDir::new().unwrap();
        touch(d.path(), "vault/a.md", "- [ ] one\n");
        let single = touch(d.path(), "project/todo.md", "- [ ] two\n");
        let mut r = TaskRegistry::new();
        r.rebuild_from_sources(&[
            folder_source(&d.path().join("vault")),
            file_source(&single),
        ]).unwrap();
        let texts: Vec<_> = r.all_tasks().iter().map(|t| t.text.clone()).collect();
        // Ordering is by source_id (hash of path), so we test as a set.
        let set: std::collections::HashSet<_> = texts.into_iter().collect();
        assert!(set.contains("one"));
        assert!(set.contains("two"));
    }

    #[test]
    fn refresh_file_replaces_entries_within_source() {
        let d = TempDir::new().unwrap();
        let p = touch(d.path(), "a.md", "- [ ] old\n");
        let src = folder_source(d.path());
        let mut r = TaskRegistry::new();
        r.rebuild_from_sources(&[src.clone()]).unwrap();
        assert_eq!(r.all_tasks().len(), 1);

        std::fs::write(&p, "- [ ] new1\n- [ ] new2\n").unwrap();
        r.refresh_file(&src, &p).unwrap();
        let names: Vec<_> = r.all_tasks().iter().map(|t| t.text.clone()).collect();
        assert_eq!(names, vec!["new1", "new2"]);
    }

    #[test]
    fn refresh_file_handles_deletion() {
        let d = TempDir::new().unwrap();
        let p = touch(d.path(), "a.md", "- [ ] x\n");
        let src = folder_source(d.path());
        let mut r = TaskRegistry::new();
        r.rebuild_from_sources(&[src.clone()]).unwrap();
        std::fs::remove_file(&p).unwrap();
        r.refresh_file(&src, &p).unwrap();
        assert_eq!(r.all_tasks().len(), 0);
    }

    #[test]
    fn file_source_ignores_sibling_changes() {
        let d = TempDir::new().unwrap();
        let target = touch(d.path(), "todo.md", "- [ ] keep\n");
        let sibling = touch(d.path(), "other.md", "- [ ] noise\n");
        let src = file_source(&target);
        let mut r = TaskRegistry::new();
        r.rebuild_from_sources(&[src.clone()]).unwrap();

        // Pretend sibling changed — refresh_file must be a no-op.
        r.refresh_file(&src, &sibling).unwrap();
        let texts: Vec<_> = r.all_tasks().iter().map(|t| t.text.clone()).collect();
        assert_eq!(texts, vec!["keep"]);
    }
}
