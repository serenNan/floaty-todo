use std::str::FromStr;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::commands::AppState;
use crate::types::HotkeyConfig;

/// 单个键的注册结果。`ok` = 成功（含「故意清空」）；失败时 `error` 带原因。
#[derive(Debug, serde::Serialize)]
pub struct KeyOutcome {
    pub ok: bool,
    /// 本次尝试注册的 accelerator；清空（解绑）时为 None。
    pub accelerator: Option<String>,
    pub error: Option<String>,
}

/// `apply` 的返回 —— 每个键一个结果，给前端按键反馈。
#[derive(Debug, serde::Serialize)]
pub struct ApplyResult {
    pub toggle: KeyOutcome,
    pub quick_add: KeyOutcome,
}

/// 判断一次触发的快捷键是否等于配置里的某个 accelerator。
pub(crate) fn matches_accel(fired: &Shortcut, accel: &Option<String>) -> bool {
    accel
        .as_deref()
        .and_then(|a| Shortcut::from_str(a).ok())
        .map(|parsed| &parsed == fired)
        .unwrap_or(false)
}

/// 启动时注册全部快捷键。单个失败只记日志 + emit 警告事件，不中断启动。
pub fn register_all(app: &AppHandle, cfg: &HotkeyConfig) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    for (label, accel) in [("toggle", &cfg.toggle), ("quick_add", &cfg.quick_add)] {
        if let Some(a) = accel {
            if let Err(e) = gs.register(a.as_str()) {
                eprintln!("[hotkeys] failed to register {label} = {a}: {e}");
                let _ = app.emit("hotkey-register-failed", a.clone());
            }
        }
    }
}

/// 注销 `old` 的某键、注册 `new` 的同键。失败则回滚到 `old` 的绑定。
fn apply_one(app: &AppHandle, old: &Option<String>, new: &Option<String>) -> KeyOutcome {
    let gs = app.global_shortcut();
    if let Some(o) = old {
        let _ = gs.unregister(o.as_str());
    }
    match new {
        None => KeyOutcome { ok: true, accelerator: None, error: None },
        Some(n) => match gs.register(n.as_str()) {
            Ok(()) => KeyOutcome { ok: true, accelerator: Some(n.clone()), error: None },
            Err(e) => {
                // 回滚：注册失败就把旧绑定恢复回去，避免「新的没绑、旧的也没了」。
                if let Some(o) = old {
                    let _ = gs.register(o.as_str());
                }
                KeyOutcome { ok: false, accelerator: Some(n.clone()), error: Some(e.to_string()) }
            }
        },
    }
}

/// 把 `old` 的注册切换到 `new`，逐键返回结果。
pub fn apply(app: &AppHandle, old: &HotkeyConfig, new: &HotkeyConfig) -> ApplyResult {
    ApplyResult {
        toggle: apply_one(app, &old.toggle, &new.toggle),
        quick_add: apply_one(app, &old.quick_add, &new.quick_add),
    }
}

/// 插件 handler 调进来：把触发的快捷键路由到对应动作。
pub fn dispatch(app: &AppHandle, shortcut: &Shortcut) {
    let cfg = {
        let state = app.state::<AppState>();
        let guard = state.config.read().unwrap();
        guard.hotkeys.clone()
    };
    if matches_accel(shortcut, &cfg.toggle) {
        crate::toggle_main_window(app);
    } else if matches_accel(shortcut, &cfg.quick_add) {
        trigger_quick_add(app);
    }
}

/// quick-add 快捷键：窗口隐藏则先显示，再 emit 事件让前端弹 QuickAdd 模态。
fn trigger_quick_add(app: &AppHandle) {
    let was_visible = app
        .get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    if !was_visible {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.show();
            let _ = w.unminimize();
            let _ = w.set_focus();
        }
    }
    let _ = app.emit(
        "trigger-quick-add",
        serde_json::json!({ "wasHidden": !was_visible }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_accel_true_for_equal_accelerator() {
        let fired = Shortcut::from_str("Ctrl+Shift+T").unwrap();
        assert!(matches_accel(&fired, &Some("Ctrl+Shift+T".to_string())));
    }

    #[test]
    fn matches_accel_false_for_different_accelerator() {
        let fired = Shortcut::from_str("Ctrl+Shift+T").unwrap();
        assert!(!matches_accel(&fired, &Some("Ctrl+Shift+A".to_string())));
    }

    #[test]
    fn matches_accel_false_for_none() {
        let fired = Shortcut::from_str("Ctrl+Shift+T").unwrap();
        assert!(!matches_accel(&fired, &None));
    }

    #[test]
    fn matches_accel_false_for_garbage_string() {
        let fired = Shortcut::from_str("Ctrl+Shift+T").unwrap();
        assert!(!matches_accel(&fired, &Some("not-a-real-accel".to_string())));
    }
}
