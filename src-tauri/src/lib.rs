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
use std::sync::{Arc, Mutex, RwLock};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

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

            // Hold the watcher handle so it lives as long as the app.
            let watcher_slot: Arc<Mutex<Option<WatcherHandle>>> = Arc::new(Mutex::new(None));
            app.manage(watcher_slot.clone());

            // If vault is configured, start initial scan + watcher.
            if let Some(vault) = cfg.vault_path.clone() {
                let app_handle = app.handle().clone();
                let registry_clone = registry.clone();
                let ignore_clone = ignore_hashes.clone();
                let watcher_slot_clone = watcher_slot.clone();
                std::thread::spawn(move || {
                    {
                        let mut reg = registry_clone.write().unwrap();
                        let _ = reg.rebuild_from_vault(&vault);
                    }
                    let _ = app_handle.emit("tasks-updated", ());

                    let app_for_cb = app_handle.clone();
                    let registry_for_cb = registry_clone.clone();
                    let handle = start_watching(&vault, ignore_clone, move |ev| {
                        match ev {
                            WatchEvent::Changed(p) | WatchEvent::Deleted(p) => {
                                let mut reg = registry_for_cb.write().unwrap();
                                let _ = reg.refresh_file(&p);
                            }
                        }
                        let _ = app_for_cb.emit("tasks-updated", ());
                    });
                    if let Ok(h) = handle {
                        *watcher_slot_clone.lock().unwrap() = Some(h);
                    }
                });
            }

            // ----- System tray
            let show_item = MenuItem::with_id(app, "show", "Show window", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "Hide window", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
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
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

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
