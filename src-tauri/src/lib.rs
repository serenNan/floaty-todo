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
use crate::watcher::{start_watching, IgnoreHashes, WatchEvent, WatcherHandle};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

pub type WatcherSlot = Arc<Mutex<Option<WatcherHandle>>>;

/// Rebuild the registry from `vault` and start watching it. Replaces any
/// existing watcher in `slot` (the old `WatcherHandle` is dropped, which stops
/// the underlying notify backend). Runs the scan + watcher start on a
/// background thread so the caller is not blocked.
pub(crate) fn spawn_vault_scan_and_watcher(
    vault: PathBuf,
    app: AppHandle,
    registry: Arc<RwLock<TaskRegistry>>,
    ignore: IgnoreHashes,
    slot: WatcherSlot,
) {
    std::thread::spawn(move || {
        {
            let mut reg = registry.write().unwrap();
            let _ = reg.rebuild_from_vault(&vault);
        }
        let _ = app.emit("tasks-updated", ());

        let app_for_cb = app.clone();
        let registry_for_cb = registry.clone();
        let handle = start_watching(&vault, ignore, move |ev| {
            match ev {
                WatchEvent::Changed(p) | WatchEvent::Deleted(p) => {
                    let mut reg = registry_for_cb.write().unwrap();
                    let _ = reg.refresh_file(&p);
                }
            }
            let _ = app_for_cb.emit("tasks-updated", ());
        });
        if let Ok(h) = handle {
            // Replacing Some(old) drops the previous WatcherHandle, which in
            // turn drops the notify Debouncer — old watcher stops cleanly.
            *slot.lock().unwrap() = Some(h);
        }
    });
}

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

            // Hold the watcher handle so it lives as long as the app, and
            // can be replaced when the user switches vault at runtime.
            let watcher_slot: WatcherSlot = Arc::new(Mutex::new(None));
            app.manage(watcher_slot.clone());

            // If vault is already configured, kick off initial scan + watcher.
            if let Some(vault) = cfg.vault_path.clone() {
                spawn_vault_scan_and_watcher(
                    vault,
                    app.handle().clone(),
                    registry.clone(),
                    ignore_hashes.clone(),
                    watcher_slot.clone(),
                );
            }

            // ----- System tray
            let show_item = MenuItem::with_id(app, "show", "Show window", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "Hide window", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                // Tauri 2 defaults to opening the menu on left-click; we want
                // left-click to toggle the window and right-click to open the
                // menu (Windows-conventional tray behaviour).
                .show_menu_on_left_click(false)
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
                            if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.unminimize();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // Intercept window close → hide to tray instead of exit. User must
            // pick "Quit" from the tray menu (or kill the process) to exit.
            if let Some(w) = app.get_webview_window("main") {
                let w_clone = w.clone();
                w.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w_clone.hide();
                    }
                });
            }

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
