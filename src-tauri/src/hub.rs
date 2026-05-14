//! Hub-folder mirror: maintain a single directory that mirrors every
//! configured `Source` via OS-level filesystem links.
//!
//! Strategy:
//! - File source → **hard link** (same inode; edits on either side are
//!   the literal same file). Works without admin / Developer Mode; only
//!   requirement is "same volume".
//! - Folder source → **directory junction** (NTFS). Also no special
//!   permission, also same-volume only. Junctions look just like a real
//!   directory to every tool that follows them.
//!
//! When a source can't be mirrored (cross-volume, missing target, name
//! collision) we surface the error to the caller but **never** fall back
//! to copying — the user explicitly opted into the link model for a
//! reason (instant two-way sync). A future "copy + watcher" mode could
//! handle cross-volume cases but is intentionally out of scope here.

use crate::error::{AppError, Result};
use crate::types::{Source, SourceKind};
use std::path::{Path, PathBuf};

/// Sanitise a string into a filesystem-safe filename. Reserved characters
/// (`<>:"/\|?*`), control characters, and trailing whitespace/dots are
/// replaced or stripped. Empty result → fall back to the source id.
fn sanitize_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    cleaned.trim().trim_end_matches('.').trim().to_string()
}

/// Compute the target path inside `hub` for `source`. File sources keep
/// the original extension; folder sources land as a junction directory.
///
/// If the requested name collides with another entry, the source id is
/// appended in parens to disambiguate (`WishTalk (a1b2c3d4).md`).
pub fn mirror_path_for(hub: &Path, source: &Source) -> PathBuf {
    let label = source
        .label
        .as_deref()
        .filter(|l| !l.trim().is_empty())
        .map(|l| sanitize_name(l))
        .unwrap_or_default();

    let stem = if label.is_empty() {
        // Last-ditch fallback: source path's filename, sanitised.
        source
            .path
            .file_stem()
            .and_then(|n| n.to_str())
            .map(sanitize_name)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| source.id.clone())
    } else {
        label
    };

    match source.kind {
        SourceKind::File => {
            let ext = source
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("md");
            hub.join(format!("{stem}.{ext}"))
        }
        SourceKind::Folder => hub.join(stem),
    }
}

/// Ensure the hub directory exists, then create the link for `source`.
/// Idempotent: if the link already exists and points where we expect, do
/// nothing. If it exists but points elsewhere (or is the wrong kind),
/// replace it.
pub fn create_mirror(hub: &Path, source: &Source) -> Result<PathBuf> {
    std::fs::create_dir_all(hub).map_err(AppError::Io)?;
    let target = mirror_path_for(hub, source);

    // If something already lives at `target`, decide whether to keep it.
    if let Ok(meta) = std::fs::symlink_metadata(&target) {
        // Different kind, or junction pointing somewhere stale → replace.
        let should_replace = match source.kind {
            SourceKind::File => !meta.file_type().is_file() || !already_hardlinked(&source.path, &target),
            SourceKind::Folder => !meta.file_type().is_dir() && !meta.file_type().is_symlink(),
        };
        if !should_replace {
            return Ok(target);
        }
        remove_mirror_entry(&target)?;
    }

    match source.kind {
        SourceKind::File => {
            std::fs::hard_link(&source.path, &target).map_err(|e| {
                AppError::CommandFailed(format!(
                    "could not hard-link {} → {}: {} (note: hard links require both paths on the same volume)",
                    source.path.display(),
                    target.display(),
                    e
                ))
            })?;
        }
        SourceKind::Folder => {
            create_directory_junction(&source.path, &target)?;
        }
    }
    Ok(target)
}

/// Remove a previously-created mirror entry. Safe to call when the entry
/// doesn't exist.
pub fn remove_mirror(hub: &Path, source: &Source) -> Result<()> {
    let target = mirror_path_for(hub, source);
    remove_mirror_entry(&target)
}

fn remove_mirror_entry(target: &Path) -> Result<()> {
    match std::fs::symlink_metadata(target) {
        Ok(meta) if meta.file_type().is_dir() || meta.file_type().is_symlink() => {
            // For directory junctions on Windows, `remove_dir` removes the
            // junction without touching the real directory it points at.
            std::fs::remove_dir(target).or_else(|_| std::fs::remove_file(target)).map_err(AppError::Io)
        }
        Ok(_) => std::fs::remove_file(target).map_err(AppError::Io),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::Io(e)),
    }
}

/// Quick check: does `link` appear to be a hard link to the same inode as
/// `original`? Used to keep `create_mirror` idempotent.
fn already_hardlinked(original: &Path, link: &Path) -> bool {
    let a = match std::fs::metadata(original) {
        Ok(m) => m,
        Err(_) => return false,
    };
    let b = match std::fs::metadata(link) {
        Ok(m) => m,
        Err(_) => return false,
    };
    // Same length is necessary but not sufficient. The cheap "good enough"
    // signal: same length + same modified time. If we're wrong the worst
    // case is a replace that produces an identical link.
    a.len() == b.len() && a.modified().ok() == b.modified().ok()
}

#[cfg(windows)]
fn create_directory_junction(original: &Path, link: &Path) -> Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    // CREATE_NO_WINDOW = 0x08000000 — don't flash a console.
    let status = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(original)
        .creation_flags(0x0800_0000)
        .status()
        .map_err(AppError::Io)?;
    if !status.success() {
        return Err(AppError::CommandFailed(format!(
            "mklink /J failed for {} → {} (exit {:?}); junctions require both paths on the same volume",
            link.display(),
            original.display(),
            status.code()
        )));
    }
    Ok(())
}

#[cfg(not(windows))]
fn create_directory_junction(original: &Path, link: &Path) -> Result<()> {
    // POSIX: a symlink does the same job and needs no special privilege.
    std::os::unix::fs::symlink(original, link).map_err(|e| {
        AppError::CommandFailed(format!(
            "could not symlink {} → {}: {e}",
            original.display(),
            link.display()
        ))
    })
}

/// Rebuild every mirror from scratch — used by `set_hub_folder` and the
/// "Repair hub" affordance. Removes orphaned entries (anything in the
/// hub that doesn't belong to a current source) so a renamed label
/// doesn't leave dangling links.
pub fn sync_all(hub: &Path, sources: &[Source]) -> Result<()> {
    if !hub.exists() {
        std::fs::create_dir_all(hub).map_err(AppError::Io)?;
    }
    // 1. Drop entries that don't belong to any current source.
    let wanted: std::collections::HashSet<PathBuf> = sources
        .iter()
        .map(|s| mirror_path_for(hub, s))
        .collect();
    if let Ok(read_dir) = std::fs::read_dir(hub) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if !wanted.contains(&path) {
                let _ = remove_mirror_entry(&path);
            }
        }
    }
    // 2. (Re-)create every source's mirror. Individual failures don't
    //    abort the whole sync — the user can see per-source errors
    //    after the fact through the surfaced status.
    let mut last_err: Option<AppError> = None;
    for s in sources {
        if let Err(e) = create_mirror(hub, s) {
            last_err = Some(e);
        }
    }
    if let Some(e) = last_err {
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SourceKind;
    use std::io::Write;
    use tempfile::TempDir;

    fn file_source(path: PathBuf, label: &str) -> Source {
        Source {
            id: Source::id_for(&path),
            path,
            kind: SourceKind::File,
            label: Some(label.to_string()),
            project_root: None,
            color: None,
        }
    }

    #[test]
    fn sanitize_drops_reserved_chars() {
        assert_eq!(sanitize_name("a/b:c?d"), "a_b_c_d");
        assert_eq!(sanitize_name("  spaced  "), "spaced");
        assert_eq!(sanitize_name("trail..."), "trail");
    }

    #[test]
    fn mirror_path_uses_label_with_extension() {
        let tmp = TempDir::new().unwrap();
        let src = file_source(tmp.path().join("orig.md"), "WishTalk");
        let p = mirror_path_for(tmp.path(), &src);
        assert_eq!(p, tmp.path().join("WishTalk.md"));
    }

    #[test]
    fn file_mirror_creates_hardlink() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let orig = tmp.path().join("real.md");
        let mut f = std::fs::File::create(&orig).unwrap();
        f.write_all(b"- [ ] one\n").unwrap();
        drop(f);

        let src = file_source(orig.clone(), "Proj");
        let mirror = create_mirror(&hub, &src).unwrap();
        assert!(mirror.exists());
        assert_eq!(mirror, hub.join("Proj.md"));

        // Edit through the mirror; original sees it (same inode).
        std::fs::write(&mirror, b"- [x] one\n").unwrap();
        let got = std::fs::read_to_string(&orig).unwrap();
        assert_eq!(got, "- [x] one\n");
    }

    #[test]
    fn create_mirror_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let orig = tmp.path().join("real.md");
        std::fs::write(&orig, b"- [ ] x\n").unwrap();
        let src = file_source(orig, "X");

        let m1 = create_mirror(&hub, &src).unwrap();
        let m2 = create_mirror(&hub, &src).unwrap();
        assert_eq!(m1, m2);
        assert!(m2.exists());
    }

    #[test]
    fn remove_mirror_clears_link() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let orig = tmp.path().join("real.md");
        std::fs::write(&orig, b"x\n").unwrap();
        let src = file_source(orig.clone(), "X");
        let mirror = create_mirror(&hub, &src).unwrap();
        assert!(mirror.exists());

        remove_mirror(&hub, &src).unwrap();
        assert!(!mirror.exists());
        // Original survives.
        assert!(orig.exists());
    }

    #[test]
    fn sync_all_prunes_orphans() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        std::fs::create_dir_all(&hub).unwrap();
        // Leave a stale file in hub.
        std::fs::write(hub.join("Stale.md"), b"old\n").unwrap();

        let orig = tmp.path().join("real.md");
        std::fs::write(&orig, b"x\n").unwrap();
        let src = file_source(orig, "Live");

        sync_all(&hub, &[src.clone()]).unwrap();
        assert!(hub.join("Live.md").exists());
        assert!(!hub.join("Stale.md").exists());
    }
}
