use crate::config;
use crate::error::{AppError, Result};
use crate::registry::TaskRegistry;
use crate::storage;
use crate::types::{AppConfig, Source, SourceKind, Task};
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

/// Append a task to a source. If `source_id` is omitted, falls back to
/// `default_source_id`. Folder sources append to `inbox_file`; File sources
/// append to the source file itself.
#[tauri::command]
pub fn add_task(
    state: State<'_, AppState>,
    text: String,
    source_id: Option<String>,
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
    let new_hash = storage::append_task(&target_file, &text)?;
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

    let canonical = path.canonicalize().unwrap_or(path.clone());
    let id = Source::id_for(&canonical);

    // Duplicate check on id (== canonical path hash).
    {
        let cfg = state.config.read().unwrap();
        if cfg.sources.iter().any(|s| s.id == id) {
            return Err(AppError::DuplicateSource(canonical.to_string_lossy().to_string()));
        }
    }

    let source = Source {
        id: id.clone(),
        path: canonical,
        kind,
        label,
        project_root,
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
    let _ = app.emit("sources-changed", ());
    let _ = app.emit("tasks-updated", ());
    let _ = removed;
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
) -> Result<Source> {
    let mut cfg = state.config.write().unwrap();
    let src = cfg
        .sources
        .iter_mut()
        .find(|s| s.id == source_id)
        .ok_or_else(|| AppError::SourceNotFound(source_id.clone()))?;
    src.label = label;
    src.project_root = project_root;
    let updated = src.clone();
    config::save_to(&state.config_path, &cfg)?;
    drop(cfg);
    let _ = app.emit("sources-changed", ());
    Ok(updated)
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
