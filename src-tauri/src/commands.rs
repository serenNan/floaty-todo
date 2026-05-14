use crate::config;
use crate::error::{AppError, Result};
use crate::registry::TaskRegistry;
use crate::storage;
use crate::types::{AppConfig, Task};
use crate::watcher::IgnoreHashes;
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

/// Set the vault path. Caller (Vue side) should call `pickVaultFolder` first
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
