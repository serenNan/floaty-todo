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
        // Remove stale entries for this file.
        // canonicalize() fails when the file is already deleted, so fall back
        // to parent-dir canonicalization + filename to produce a stable key.
        let canonical = best_effort_canonical(file);
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

/// Build a stable canonical path even when the file no longer exists.
/// Tries full canonicalize first; if that fails (file deleted), canonicalizes
/// the parent directory and appends the filename.
fn best_effort_canonical(path: &Path) -> PathBuf {
    if let Ok(c) = path.canonicalize() {
        return c;
    }
    // File may have been deleted — canonicalize the parent instead.
    if let Some(parent) = path.parent() {
        if let Ok(cp) = parent.canonicalize() {
            if let Some(name) = path.file_name() {
                return cp.join(name);
            }
        }
    }
    path.to_path_buf()
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
