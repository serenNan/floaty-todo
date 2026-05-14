use crate::error::Result;
use crate::registry::{best_effort_canonical, is_markdown_target};
use crate::types::{hash_content, ContentHash, Source, SourceKind};
use notify::{RecommendedWatcher, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, FileIdMap};
use std::collections::HashSet;
use std::path::PathBuf;
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

/// Start watching for a single source. Folder sources watch recursively; File
/// sources watch the parent dir and the callback filters events down to the
/// target file. The returned `WatcherHandle` keeps the OS-level watcher alive
/// until dropped.
pub fn start_watching_source<F>(
    source: &Source,
    ignore: IgnoreHashes,
    on_event: F,
) -> Result<WatcherHandle>
where
    F: Fn(WatchEvent) + Send + 'static,
{
    let (watch_root, filter_file): (PathBuf, Option<PathBuf>) = match source.kind {
        SourceKind::Folder => (source.path.clone(), None),
        SourceKind::File => {
            // notify only watches dirs reliably across platforms; watch the
            // parent and filter inside the callback.
            let parent = source
                .path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| source.path.clone());
            (parent, Some(best_effort_canonical(&source.path)))
        }
    };

    let cb = move |res: DebounceEventResult| {
        let events = match res { Ok(e) => e, Err(_) => return };
        for ev in events {
            for path in ev.event.paths {
                if !is_markdown_target(&path) { continue; }

                // File source filter: only react to the configured file.
                if let Some(target) = &filter_file {
                    if &best_effort_canonical(&path) != target {
                        continue;
                    }
                }

                if !path.exists() {
                    on_event(WatchEvent::Deleted(path));
                    continue;
                }
                let bytes = match std::fs::read(&path) { Ok(b) => b, Err(_) => continue };
                let h = hash_content(&bytes);
                if ignore.check_and_remove(&h) { continue; }
                on_event(WatchEvent::Changed(path));
            }
        }
    };

    let recursive = matches!(source.kind, SourceKind::Folder);
    let mut debouncer = new_debouncer(Duration::from_millis(200), None, cb)?;
    let mode = if recursive {
        notify::RecursiveMode::Recursive
    } else {
        notify::RecursiveMode::NonRecursive
    };
    debouncer.watcher().watch(&watch_root, mode)?;
    Ok(WatcherHandle { _debouncer: debouncer })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::Path;
    use std::sync::mpsc::channel;
    use std::time::Duration;
    use tempfile::TempDir;

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
    fn ignore_hashes_register_and_consume() {
        let ig = IgnoreHashes::new();
        let h = [42u8; 32];
        ig.register(h);
        assert!(ig.check_and_remove(&h));
        assert!(!ig.check_and_remove(&h));
    }

    #[test]
    fn folder_source_detects_md_change() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("a.md");
        std::fs::write(&p, "- [ ] x\n").unwrap();
        let (tx, rx) = channel();
        let ig = IgnoreHashes::new();
        let _h = start_watching_source(&folder_source(d.path()), ig, move |ev| { let _ = tx.send(ev); }).unwrap();

        std::thread::sleep(Duration::from_millis(300));
        let mut f = std::fs::OpenOptions::new().append(true).open(&p).unwrap();
        f.write_all(b"- [ ] y\n").unwrap();
        drop(f);

        let ev = rx.recv_timeout(Duration::from_secs(3)).expect("expected event");
        match ev { WatchEvent::Changed(_) => {}, other => panic!("unexpected: {:?}", other) }
    }

    #[test]
    fn folder_source_respects_registered_hash() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("a.md");
        std::fs::write(&p, b"- [ ] x\n").unwrap();
        let (tx, rx) = channel();
        let ig = IgnoreHashes::new();
        let ig2 = ig.clone();
        let _h = start_watching_source(&folder_source(d.path()), ig, move |ev| { let _ = tx.send(ev); }).unwrap();

        std::thread::sleep(Duration::from_millis(300));
        let new_bytes = b"- [x] x\n";
        ig2.register(hash_content(new_bytes));
        std::fs::write(&p, new_bytes).unwrap();

        assert!(rx.recv_timeout(Duration::from_secs(1)).is_err());
    }

    #[test]
    fn folder_source_ignores_non_markdown() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("notes.txt");
        std::fs::write(&p, "x").unwrap();
        let (tx, rx) = channel();
        let ig = IgnoreHashes::new();
        let _h = start_watching_source(&folder_source(d.path()), ig, move |ev| { let _ = tx.send(ev); }).unwrap();

        std::thread::sleep(Duration::from_millis(300));
        std::fs::write(&p, "y").unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(1)).is_err());
    }

    #[test]
    fn file_source_only_fires_for_target_file() {
        let d = TempDir::new().unwrap();
        let target = d.path().join("todo.md");
        let sibling = d.path().join("other.md");
        std::fs::write(&target, "- [ ] x\n").unwrap();
        std::fs::write(&sibling, "- [ ] y\n").unwrap();

        let (tx, rx) = channel();
        let ig = IgnoreHashes::new();
        let _h = start_watching_source(&file_source(&target), ig, move |ev| { let _ = tx.send(ev); }).unwrap();

        std::thread::sleep(Duration::from_millis(300));

        // Edit sibling first — should NOT fire.
        std::fs::write(&sibling, "- [ ] y2\n").unwrap();
        assert!(rx.recv_timeout(Duration::from_millis(500)).is_err());

        // Edit target — SHOULD fire.
        std::fs::write(&target, "- [ ] x2\n").unwrap();
        let ev = rx.recv_timeout(Duration::from_secs(3)).expect("expected event for target");
        match ev { WatchEvent::Changed(_) => {}, other => panic!("unexpected: {:?}", other) }
    }
}
