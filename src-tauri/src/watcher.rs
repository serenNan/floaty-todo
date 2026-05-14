use crate::error::Result;
use crate::registry::is_markdown_target;
use crate::types::{hash_content, ContentHash};
use notify::{RecommendedWatcher, Watcher};
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
            for path in ev.event.paths {
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
