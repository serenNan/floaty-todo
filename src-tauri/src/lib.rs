mod commands;
mod config;
mod error;
mod history;
mod hotkeys;
mod hub;
mod parser;
mod registry;
mod shell;
mod storage;
mod types;
mod watcher;

use crate::commands::AppState;
use crate::history::HistoryStore;
use crate::registry::TaskRegistry;
use crate::types::{Source, SourceKind};
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

/// Read every .md file in `source` and record its current bytes into the
/// history store's per-file snapshot cache. Called once per source on
/// startup / add so the first watcher-detected external edit has a
/// baseline to diff against (instead of reporting `added = full line count`).
fn snapshot_files_in_source(source: &Source, history: &Arc<Mutex<HistoryStore>>) {
    let mut store = history.lock().unwrap();
    match source.kind {
        SourceKind::File => {
            if let Ok(bytes) = std::fs::read(&source.path) {
                store.record_file_snapshot(&source.path, bytes);
            }
        }
        SourceKind::Folder => {
            for entry in walkdir::WalkDir::new(&source.path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                if let Ok(bytes) = std::fs::read(path) {
                    store.record_file_snapshot(path, bytes);
                }
            }
        }
    }
}

/// Spawn an initial scan + watcher for one source. Replaces any existing entry
/// in `slots` for the same source id.
pub(crate) fn spawn_source_scan_and_watcher(
    source: Source,
    app: AppHandle,
    registry: Arc<RwLock<TaskRegistry>>,
    history: Arc<Mutex<HistoryStore>>,
    ignore: IgnoreHashes,
    slots: WatcherSlots,
) {
    std::thread::spawn(move || {
        // Tell the frontend a scan is in-flight so it can show a spinner /
        // disable the "Add source" button until we're done. Payload is the
        // source id — the UI only blocks the right card, not the whole app.
        let _ = app.emit("source-scan-started", source.id.clone());

        {
            let mut reg = registry.write().unwrap();
            reg.rebuild_source(&source);
        }
        // Establish baseline file snapshots so the first external_edit fired
        // by the watcher has something to diff against. Without this the
        // first edit would report `added = full line count` (everything new).
        snapshot_files_in_source(&source, &history);
        let _ = app.emit("tasks-updated", ());
        let _ = app.emit("source-scan-finished", source.id.clone());

        let app_for_cb = app.clone();
        let registry_for_cb = registry.clone();
        let history_for_cb = history.clone();
        let src_for_cb = source.clone();
        let handle = start_watching_source(&source, ignore, move |ev| {
            match ev {
                WatchEvent::Changed(p) => {
                    let mut reg = registry_for_cb.write().unwrap();
                    let _ = reg.refresh_file(&src_for_cb, &p);
                    // Read the new content here, in the watcher thread, so the
                    // diff_summary actually reflects what changed on disk
                    // (history.rs's push_external_edit diffs against its cached
                    // snapshot and updates the cache afterwards). Throttling +
                    // merge of consecutive edits within 500ms happens inside.
                    if let Ok(bytes) = std::fs::read(&p) {
                        let _ = history_for_cb.lock().unwrap().push_external_edit(
                            src_for_cb.id.clone(),
                            p.clone(),
                            bytes,
                        );
                        let _ = app_for_cb.emit("history-updated", ());
                    }
                }
                WatchEvent::Deleted(p) => {
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
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    // handler 对按下和松开都会触发，只处理按下。
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        hotkeys::dispatch(app, shortcut);
                    }
                })
                .build(),
        )
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
            let history = Arc::new(Mutex::new(HistoryStore::open(
                cfg.hub_folder.as_deref(),
                &app_config_dir,
            )?));
            let state = AppState {
                registry: registry.clone(),
                config: Arc::new(RwLock::new(cfg.clone())),
                history: history.clone(),
                ignore_hashes: ignore_hashes.clone(),
                config_path: config_path.clone(),
                app_data_dir: app_config_dir.clone(),
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
                    history.clone(),
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
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // Intercept window close → hide to tray, and sync always-on-top
            // from the persisted config (tauri.conf.json gives an initial
            // value, but the user may have toggled it off on the previous run).
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_always_on_top(cfg.always_on_top);
                let w_clone = w.clone();
                w.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w_clone.hide();
                    }
                });
            }

            // Pre-create the history window hidden so the first click on the
            // 🕒 button is just a show() — no WebView2 cold-start latency,
            // no "click once does nothing, need to click again" UX. The
            // CloseRequested handler turns its close button into hide-only
            // so the window survives for fast subsequent opens too.
            if app.get_webview_window("history").is_none() {
                if let Ok(w) = tauri::WebviewWindowBuilder::new(
                    app.handle(),
                    "history",
                    tauri::WebviewUrl::App("index.html".into()),
                )
                .title("Floaty Todo - History")
                .inner_size(680.0, 520.0)
                .resizable(true)
                .visible(false)
                .decorations(true)
                .always_on_top(false)
                .build()
                {
                    let w_clone = w.clone();
                    w.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            let _ = w_clone.hide();
                        }
                    });
                }
            }

            // 注册全局快捷键。单个失败不影响启动（register_all 内部已 emit
            // hotkey-register-failed，前端会 toast 提示）。
            hotkeys::register_all(&app.handle().clone(), &cfg.hotkeys);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_tasks,
            commands::get_config,
            commands::update_config,
            commands::toggle_task,
            commands::update_task,
            commands::add_task,
            commands::delete_task,
            commands::get_history,
            commands::get_history_cursor,
            commands::undo,
            commands::redo,
            commands::jump_to,
            commands::open_history_window,
            commands::list_sources,
            commands::add_source,
            commands::remove_source,
            commands::update_source,
            commands::reorder_sources,
            commands::set_default_source,
            commands::set_file_label,
            commands::open_in_vscode,
            commands::open_in_terminal,
            commands::open_in_claude_code,
            commands::reveal_source,
            commands::run_quick_action,
            commands::set_enabled_quick_actions,
            commands::set_hub_folder,
            commands::resync_hub,
            commands::open_hub,
            commands::open_url,
            commands::set_always_on_top,
            commands::show_window,
            commands::hide_window,
            commands::set_hotkeys,
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

/// 主窗口智能切换：可见且聚焦 → 隐藏，否则前置。托盘左键点击和 toggle
/// 全局快捷键共用。
pub fn toggle_main_window(app: &AppHandle) {
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
