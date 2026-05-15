use crate::config;
use crate::error::{AppError, Result};
use crate::registry::TaskRegistry;
use crate::storage;
use crate::types::{file_label_key, AppConfig, QuickActionKind, Source, SourceKind, Task};
use crate::watcher::IgnoreHashes;
use crate::{spawn_source_scan_and_watcher, WatcherSlots};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Emitter, Manager, State};

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
    let source = find_source_by_id(&state, &task.source_id)?;
    let new_hash = storage::toggle_task(&task.source_file, task.line_number)?;
    state.ignore_hashes.register(new_hash);
    state.registry.write().unwrap().refresh_file(&source, &task.source_file)?;
    Ok(())
}

/// Replace the text of an existing task and optionally move it to a different
/// quadrant section.
///
/// - `change_quadrant = false` → rewrite the line in place, preserving indent,
///   bullet style, checkbox state and trailing whitespace byte-for-byte.
/// - `change_quadrant = true` → the caller wants the task moved to
///   `new_quadrant` (which may be `None` for the unsorted bucket). If the
///   target is the same as the task's current quadrant we still take the
///   in-place path; if it differs, the original line is removed and a fresh
///   `- [ ] <text>` is appended under the target quadrant's `## ` header.
///
/// We need the explicit boolean because `Option<Option<T>>` is indistinguishable
/// from a single `Option<T>` once it crosses Tauri/serde IPC (both `None` and
/// `Some(None)` serialize to `null`).
#[tauri::command]
pub fn update_task(
    state: State<'_, AppState>,
    task_id: String,
    new_text: String,
    change_quadrant: bool,
    new_quadrant: Option<crate::types::Quadrant>,
) -> Result<()> {
    let task = {
        let reg = state.registry.read().unwrap();
        reg.get(&task_id).cloned().ok_or_else(|| AppError::TaskNotFound(task_id.clone()))?
    };
    let source = find_source_by_id(&state, &task.source_id)?;

    let quadrant_changing = change_quadrant && new_quadrant != task.quadrant;

    if !quadrant_changing {
        let new_hash = storage::update_task_text(&task.source_file, task.line_number, &new_text)?;
        state.ignore_hashes.register(new_hash);
        state.registry.write().unwrap().refresh_file(&source, &task.source_file)?;
        return Ok(());
    }

    // Cross-quadrant move within the same file: delete the original line,
    // then re-append under the target quadrant's header. The append path
    // reuses the same append_task_to_quadrant logic that add_task uses, so
    // header auto-creation honours the user's `auto_create_quadrant_headers`
    // preference exactly like new-task creation does.
    let auto_create_headers = state.config.read().unwrap().auto_create_quadrant_headers;
    let hash_after_delete = storage::remove_task_line(&task.source_file, task.line_number)?;
    state.ignore_hashes.register(hash_after_delete);
    let final_hash = storage::append_task_to_quadrant(
        &task.source_file,
        &new_text,
        new_quadrant,
        auto_create_headers,
    )?;
    state.ignore_hashes.register(final_hash);
    state.registry.write().unwrap().refresh_file(&source, &task.source_file)?;
    Ok(())
}

/// Append a task to a source. If `source_id` is omitted, falls back to
/// `default_source_id`. Folder sources append to `inbox_file`; File sources
/// append to the source file itself.
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

#[tauri::command]
pub fn list_sources(state: State<'_, AppState>) -> Result<Vec<Source>> {
    Ok(state.config.read().unwrap().sources.clone())
}

/// Add a new source. `kind` is "folder" or "file". `path` must exist and be
/// the right kind. Duplicate paths are rejected.
#[tauri::command]
pub fn add_source(
    state: State<'_, AppState>,
    watcher_slots: State<'_, WatcherSlots>,
    app: AppHandle,
    path: PathBuf,
    kind: SourceKind,
    label: Option<String>,
    project_root: Option<PathBuf>,
) -> Result<Source> {
    // Validate path against the requested kind.
    let meta = std::fs::metadata(&path)
        .map_err(|_| AppError::InvalidSourcePath(path.to_string_lossy().to_string()))?;
    match kind {
        SourceKind::Folder if !meta.is_dir() => {
            return Err(AppError::InvalidSourcePath(path.to_string_lossy().to_string()));
        }
        SourceKind::File if !meta.is_file() => {
            return Err(AppError::InvalidSourcePath(path.to_string_lossy().to_string()));
        }
        _ => {}
    }

    // dunce::canonicalize avoids Windows verbatim prefix (\\?\) unless the
    // path is long-enough to actually require it. Falls back to the original
    // when canonicalize fails (rare for an existing path).
    let canonical = dunce::canonicalize(&path).unwrap_or(path.clone());
    let id = Source::id_for(&canonical);

    // Duplicate check on id (== canonical path hash).
    {
        let cfg = state.config.read().unwrap();
        if cfg.sources.iter().any(|s| s.id == id) {
            return Err(AppError::DuplicateSource(canonical.to_string_lossy().to_string()));
        }
    }

    // Default label from project root's folder name — matches what "Open in
    // VS Code" / "Open terminal" will jump to, so the source label and the
    // shell-action target stay consistent. For a Folder source that's the
    // folder name; for a File source it's the parent folder name.
    let resolved_label = label.filter(|l| !l.trim().is_empty()).or_else(|| {
        let root_for_label = project_root.clone().unwrap_or_else(|| match kind {
            SourceKind::Folder => canonical.clone(),
            SourceKind::File => canonical
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| canonical.clone()),
        });
        root_for_label
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
    });

    let source = Source {
        id: id.clone(),
        path: canonical,
        kind,
        label: resolved_label,
        project_root,
        color: None,
    };

    // Persist + start watcher.
    {
        let mut cfg = state.config.write().unwrap();
        cfg.sources.push(source.clone());
        if cfg.default_source_id.is_none() {
            cfg.default_source_id = Some(id.clone());
        }
        config::save_to(&state.config_path, &cfg)?;
    }
    spawn_source_scan_and_watcher(
        source.clone(),
        app.clone(),
        state.registry.clone(),
        state.ignore_hashes.clone(),
        watcher_slots.inner().clone(),
    );
    // Mirror into hub if configured. Failures don't block the add — the
    // user can fix the hub setup later and re-run "Resync hub".
    {
        let src_for_hub = source.clone();
        try_hub(&state, |hub| {
            crate::hub::create_mirror(hub, &src_for_hub).map(|_| ())
        });
    }
    let _ = app.emit("sources-changed", ());
    Ok(source)
}

#[tauri::command]
pub fn remove_source(
    state: State<'_, AppState>,
    watcher_slots: State<'_, WatcherSlots>,
    app: AppHandle,
    source_id: String,
) -> Result<()> {
    // Drop watcher first so it stops before the registry is mutated.
    {
        let mut slots = watcher_slots.lock().unwrap();
        slots.remove(&source_id);
    }
    // Remove from config.
    let removed = {
        let mut cfg = state.config.write().unwrap();
        let idx = cfg
            .sources
            .iter()
            .position(|s| s.id == source_id)
            .ok_or_else(|| AppError::SourceNotFound(source_id.clone()))?;
        let removed = cfg.sources.remove(idx);
        if cfg.default_source_id.as_deref() == Some(&source_id) {
            cfg.default_source_id = cfg.sources.first().map(|s| s.id.clone());
        }
        config::save_to(&state.config_path, &cfg)?;
        removed
    };
    // Drop tasks belonging to this source by full rebuild of registry from
    // the remaining sources (simplest correct option; sources count is small).
    let remaining = state.config.read().unwrap().sources.clone();
    state.registry.write().unwrap().rebuild_from_sources(&remaining)?;
    // Clear this source's hub mirror so AI / scripts don't see a dead link.
    {
        let removed_for_hub = removed.clone();
        try_hub(&state, |hub| crate::hub::remove_mirror(hub, &removed_for_hub));
    }
    let _ = app.emit("sources-changed", ());
    let _ = app.emit("tasks-updated", ());
    Ok(())
}

/// Update the editable fields on a source: label and project_root.
#[tauri::command]
pub fn update_source(
    state: State<'_, AppState>,
    app: AppHandle,
    source_id: String,
    label: Option<String>,
    project_root: Option<PathBuf>,
    color: Option<String>,
) -> Result<Source> {
    // Snapshot the source's pre-update form so we can clean up its old
    // hub mirror (which is keyed by the old label).
    let previous = find_source_by_id(&state, &source_id)?;

    let mut cfg = state.config.write().unwrap();
    let src = cfg
        .sources
        .iter_mut()
        .find(|s| s.id == source_id)
        .ok_or_else(|| AppError::SourceNotFound(source_id.clone()))?;
    src.label = label;
    src.project_root = project_root;
    // Accept only hex colors (`#rgb` / `#rrggbb` / `#rrggbbaa`); silently
    // drop anything else so a corrupt config can't smuggle CSS into the UI.
    src.color = color.filter(|c| {
        let bytes = c.as_bytes();
        matches!(bytes.len(), 4 | 7 | 9)
            && bytes[0] == b'#'
            && bytes[1..].iter().all(|b| b.is_ascii_hexdigit())
    });
    let updated = src.clone();
    config::save_to(&state.config_path, &cfg)?;
    drop(cfg);

    // Re-mirror under the new label, then drop the old name if it differs.
    {
        let prev = previous.clone();
        let new = updated.clone();
        try_hub(&state, |hub| {
            crate::hub::create_mirror(hub, &new).map(|_| ())?;
            let old_path = crate::hub::mirror_path_for(hub, &prev);
            let new_path = crate::hub::mirror_path_for(hub, &new);
            if old_path != new_path {
                let _ = crate::hub::remove_mirror(hub, &prev);
            }
            Ok(())
        });
    }
    let _ = app.emit("sources-changed", ());
    Ok(updated)
}

/// Replace the source ordering with the given full list of ids.
/// All current source ids must appear in `ordered_ids` — partial reorders
/// are rejected so a stale frontend can't accidentally drop sources.
#[tauri::command]
pub fn reorder_sources(
    state: State<'_, AppState>,
    app: AppHandle,
    ordered_ids: Vec<String>,
) -> Result<()> {
    let mut cfg = state.config.write().unwrap();
    let index_of = |id: &str| ordered_ids.iter().position(|x| x == id);
    if cfg.sources.iter().any(|s| index_of(&s.id).is_none())
        || ordered_ids.len() != cfg.sources.len()
    {
        return Err(AppError::CommandFailed(
            "reorder list does not match current sources".into(),
        ));
    }
    cfg.sources.sort_by_key(|s| index_of(&s.id).unwrap());
    config::save_to(&state.config_path, &cfg)?;
    drop(cfg);
    let _ = app.emit("sources-changed", ());
    Ok(())
}

#[tauri::command]
pub fn set_default_source(
    state: State<'_, AppState>,
    app: AppHandle,
    source_id: Option<String>,
) -> Result<()> {
    let mut cfg = state.config.write().unwrap();
    if let Some(ref id) = source_id {
        if !cfg.sources.iter().any(|s| &s.id == id) {
            return Err(AppError::SourceNotFound(id.clone()));
        }
    }
    cfg.default_source_id = source_id;
    config::save_to(&state.config_path, &cfg)?;
    drop(cfg);
    let _ = app.emit("sources-changed", ());
    Ok(())
}

/// Set or clear the display label for a single file inside a source.
/// Pass `label = None` (or an empty string after trim) to remove the override.
#[tauri::command]
pub fn set_file_label(
    state: State<'_, AppState>,
    app: AppHandle,
    file_path: PathBuf,
    label: Option<String>,
) -> Result<()> {
    let key = file_label_key(&file_path);
    let mut cfg = state.config.write().unwrap();
    match label {
        Some(l) if !l.trim().is_empty() => {
            cfg.file_labels.insert(key, l.trim().to_string());
        }
        _ => {
            cfg.file_labels.remove(&key);
        }
    }
    config::save_to(&state.config_path, &cfg)?;
    drop(cfg);
    let _ = app.emit("sources-changed", ());
    Ok(())
}

/// Spawn `code <path>` for the source's effective project root.
/// On Windows the binary is `code.cmd` (a npm-style shim); on other platforms
/// it's plain `code`. Returns `CommandFailed` with a hint if `code` is not on PATH.
#[tauri::command]
pub fn open_in_vscode(state: State<'_, AppState>, source_id: String) -> Result<()> {
    let source = find_source_by_id(&state, &source_id)?;
    let target = source.effective_project_root();
    crate::shell::open_vscode(&target)
}

/// Open a terminal at the source's effective project root. Tries platform-
/// specific terminals in order of preference; the first one that spawns wins.
#[tauri::command]
pub fn open_in_terminal(state: State<'_, AppState>, source_id: String) -> Result<()> {
    let source = find_source_by_id(&state, &source_id)?;
    let target = source.effective_project_root();
    crate::shell::open_terminal(&target)
}

/// Reveal the source's path in the OS file manager.
#[tauri::command]
pub fn reveal_source(state: State<'_, AppState>, source_id: String) -> Result<()> {
    let source = find_source_by_id(&state, &source_id)?;
    crate::shell::reveal_in_explorer(&source.path)
}

/// Open a fresh Claude Code session at the source's effective project root.
#[tauri::command]
pub fn open_in_claude_code(state: State<'_, AppState>, source_id: String) -> Result<()> {
    let source = find_source_by_id(&state, &source_id)?;
    let target = source.effective_project_root();
    crate::shell::open_claude_code(&target)
}

/// Dynamic dispatch — front-end picks a `QuickActionKind` and the backend
/// routes to the matching launcher. Lets us add new kinds without breaking
/// the IPC surface.
#[tauri::command]
pub fn run_quick_action(
    state: State<'_, AppState>,
    source_id: String,
    kind: QuickActionKind,
) -> Result<()> {
    let source = find_source_by_id(&state, &source_id)?;
    // Reveal points at the source's own path (file or folder); the others
    // jump to the effective project root configured for shell launchers.
    match kind {
        QuickActionKind::Vscode => {
            crate::shell::open_vscode(&source.effective_project_root())
        }
        QuickActionKind::Terminal => {
            crate::shell::open_terminal(&source.effective_project_root())
        }
        QuickActionKind::ClaudeCode => {
            crate::shell::open_claude_code(&source.effective_project_root())
        }
        QuickActionKind::Reveal => crate::shell::reveal_in_explorer(&source.path),
    }
}

/// Point the hub mirror at `path` (or clear it with `None`). When set,
/// every existing source is immediately mirrored into it. The previous
/// hub (if any) keeps its entries — they're just stale links, the user
/// can delete the folder themselves.
#[tauri::command]
pub fn set_hub_folder(
    state: State<'_, AppState>,
    app: AppHandle,
    path: Option<PathBuf>,
) -> Result<()> {
    // Canonicalise so saved value matches what mirror_path_for produces.
    let canonical = match path {
        Some(p) => Some(dunce::canonicalize(&p).unwrap_or(p)),
        None => None,
    };
    let sources_snapshot = {
        let mut cfg = state.config.write().unwrap();
        cfg.hub_folder = canonical.clone();
        config::save_to(&state.config_path, &cfg)?;
        cfg.sources.clone()
    };
    if let Some(hub) = &canonical {
        crate::hub::sync_all(hub, &sources_snapshot)?;
    }
    let _ = app.emit("sources-changed", ());
    Ok(())
}

/// Open the hub folder in one of the quick-action targets (VS Code,
/// terminal, Claude Code). Errors if the hub isn't configured.
#[tauri::command]
pub fn open_hub(state: State<'_, AppState>, kind: QuickActionKind) -> Result<()> {
    let hub = state
        .config
        .read()
        .unwrap()
        .hub_folder
        .clone()
        .ok_or_else(|| AppError::CommandFailed("hub folder not configured".into()))?;
    match kind {
        QuickActionKind::Vscode => crate::shell::open_vscode(&hub),
        QuickActionKind::Terminal => crate::shell::open_terminal(&hub),
        QuickActionKind::ClaudeCode => crate::shell::open_claude_code(&hub),
        QuickActionKind::Reveal => crate::shell::reveal_in_explorer(&hub),
    }
}

/// Resync every source into the configured hub. Used after a manual
/// "Repair hub" click or when mirrors get out of date because the user
/// renamed labels while the hub was unset.
#[tauri::command]
pub fn resync_hub(state: State<'_, AppState>, app: AppHandle) -> Result<()> {
    let cfg = state.config.read().unwrap().clone();
    if let Some(hub) = &cfg.hub_folder {
        crate::hub::sync_all(hub, &cfg.sources)?;
        let _ = app.emit("sources-changed", ());
    }
    Ok(())
}

/// Replace the list of quick actions shown on every source header.
/// Order in the vec is preserved — that's the display order.
#[tauri::command]
pub fn set_enabled_quick_actions(
    state: State<'_, AppState>,
    app: AppHandle,
    actions: Vec<QuickActionKind>,
) -> Result<()> {
    {
        let mut cfg = state.config.write().unwrap();
        cfg.enabled_quick_actions = actions;
        config::save_to(&state.config_path, &cfg)?;
    }
    let _ = app.emit("sources-changed", ());
    Ok(())
}

/// Open an arbitrary URL with the OS default handler. Used by inline-markdown
/// link rendering inside task text — the WebView can't navigate externally.
#[tauri::command]
pub fn open_url(url: String) -> Result<()> {
    crate::shell::open_url(&url)
}

/// Toggle the main window's always-on-top behaviour and persist the
/// choice. Applies immediately so the user sees the change without
/// having to restart.
#[tauri::command]
pub fn set_always_on_top(
    state: State<'_, AppState>,
    app: AppHandle,
    on: bool,
) -> Result<()> {
    {
        let mut cfg = state.config.write().unwrap();
        cfg.always_on_top = on;
        config::save_to(&state.config_path, &cfg)?;
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_always_on_top(on);
    }
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

fn find_source_by_id(state: &AppState, source_id: &str) -> Result<Source> {
    state
        .config
        .read()
        .unwrap()
        .sources
        .iter()
        .find(|s| s.id == source_id)
        .cloned()
        .ok_or_else(|| AppError::SourceNotFound(source_id.to_string()))
}

/// Run a fallible hub-sync operation and swallow the error. We don't
/// want a junction-failure (e.g. cross-volume) to make `add_source`
/// itself fail — the source is added regardless, the user just needs to
/// fix the hub setup. The error surfaces via the next `resync_hub`.
fn try_hub<F: FnOnce(&std::path::Path) -> Result<()>>(state: &AppState, run: F) {
    let hub = state.config.read().unwrap().hub_folder.clone();
    if let Some(hub) = hub {
        let _ = run(&hub);
    }
}
