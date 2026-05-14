mod commands;
mod config;
mod error;
mod parser;
mod registry;
mod shell;
mod storage;
mod types;
mod watcher;

use crate::commands::AppState;
use crate::registry::TaskRegistry;
use crate::types::Source;
use crate::watcher::{start_watching_source, IgnoreHashes, WatchEvent, WatcherHandle};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

/// One watcher per source. Removing an entry drops the `WatcherHandle`, which
/// stops the underlying notify backend cleanly.
pub type WatcherSlots = Arc<Mutex<HashMap<String, WatcherHandle>>>;

/// Spawn an initial scan + watcher for one source. Replaces any existing entry
/// in `slots` for the same source id.
pub(crate) fn spawn_source_scan_and_watcher(
    source: Source,
    app: AppHandle,
    registry: Arc<RwLock<TaskRegistry>>,
    ignore: IgnoreHashes,
    slots: WatcherSlots,
) {
    std::thread::spawn(move || {
        {
            let mut reg = registry.write().unwrap();
            reg.rebuild_source(&source);
        }
        let _ = app.emit("tasks-updated", ());

        let app_for_cb = app.clone();
        let registry_for_cb = registry.clone();
        let src_for_cb = source.clone();
        let handle = start_watching_source(&source, ignore, move |ev| {
            match ev {
                WatchEvent::Changed(p) | WatchEvent::Deleted(p) => {
                    let mut reg = registry_for_cb.write().unwrap();
                    let _ = reg.refresh_file(&src_for_cb, &p);
                }
            }
            let _ = app_for_cb.emit("tasks-updated", ());
        });
        if let Ok(h) = handle {
            slots.lock().unwrap().insert(source.id.clone(), h);
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // ----- Config: load or default. load_from also normalises legacy
            // verbatim paths (\\?\...) — persist the cleaned form so the JSON
            // on disk matches what the UI / shell launchers see.
            let app_config_dir = app.path().app_config_dir().expect("config dir");
            let config_path = config::config_file(&app_config_dir);
            let cfg = config::load_from(&config_path).unwrap_or_default();
            let _ = config::save_to(&config_path, &cfg);

            // ----- Registry + watcher slots
            let registry = Arc::new(RwLock::new(TaskRegistry::new()));
            let ignore_hashes = IgnoreHashes::new();
            let state = AppState {
                registry: registry.clone(),
                config: Arc::new(RwLock::new(cfg.clone())),
                ignore_hashes: ignore_hashes.clone(),
                config_path: config_path.clone(),
            };
            app.manage(state);

            let watcher_slots: WatcherSlots = Arc::new(Mutex::new(HashMap::new()));
            app.manage(watcher_slots.clone());

            // Kick off scan + watcher for every configured source.
            for source in cfg.sources.iter().cloned() {
                spawn_source_scan_and_watcher(
                    source,
                    app.handle().clone(),
                    registry.clone(),
                    ignore_hashes.clone(),
                    watcher_slots.clone(),
                );
            }

            // ----- System tray
            let show_item = MenuItem::with_id(app, "show", "Show window", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "Hide window", true, None::<&str>)?;
            let manage_sources_item = MenuItem::with_id(app, "manage_sources", "Manage sources…", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &hide_item, &manage_sources_item, &quit_item])?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => toggle_window(app, true),
                    "hide" => toggle_window(app, false),
                    "manage_sources" => {
                        toggle_window(app, true);
                        let _ = app.emit("request-manage-sources", ());
                    }
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

            // Intercept window close → hide to tray.
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
            commands::list_sources,
            commands::add_source,
            commands::remove_source,
            commands::update_source,
            commands::set_default_source,
            commands::set_file_label,
            commands::open_in_vscode,
            commands::open_in_terminal,
            commands::open_url,
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
