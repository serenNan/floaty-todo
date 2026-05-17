# 全局快捷键 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 给 Floaty Todo 加两个可自定义的全局快捷键 —— 一个呼出/隐藏主窗口，一个直接弹快速添加。

**Architecture:** Rust 端用 `tauri-plugin-global-shortcut` v2 注册快捷键，插件 handler 把按键事件路由到 `hotkeys.rs`；键位字符串存进 `AppConfig.hotkeys`。前端在设置页用「按键录制」方式自定义，通过 `set_hotkeys` 命令重新注册并持久化。quick-add 快捷键通过 `trigger-quick-add` 事件让前端弹出已有的 QuickAdd 模态。

**Tech Stack:** Tauri 2 / Rust、`tauri-plugin-global-shortcut` v2、Vue 3 + TypeScript + Pinia、Vitest（本计划新引入，前端首个测试框架）。

设计文档：`docs/superpowers/specs/2026-05-15-global-hotkey-design.md`

---

## 文件结构

| 文件 | 创建/修改 | 职责 |
|---|---|---|
| `vitest.config.ts` | 创建 | Vitest 配置（独立于 `vite.config.ts`，不碰 Tauri dev 配置） |
| `package.json` | 修改 | 加 `vitest` devDependency + `test` script |
| `src/utils/hotkey.ts` | 创建 | `translateKeyEvent` 纯函数：`KeyboardEvent` → Tauri accelerator 字符串 |
| `src/utils/hotkey.test.ts` | 创建 | `translateKeyEvent` 单测 |
| `src-tauri/src/types.rs` | 修改 | 新增 `HotkeyConfig`；`AppConfig` 加 `hotkeys` 字段 |
| `src-tauri/Cargo.toml` | 修改 | 加 `tauri-plugin-global-shortcut = "2"` |
| `src-tauri/src/hotkeys.rs` | 创建 | 快捷键注册/注销/路由：`register_all` / `apply` / `dispatch` / `matches_accel` |
| `src-tauri/src/lib.rs` | 修改 | `mod hotkeys`、插件 init、抽 `toggle_main_window`、setup 里调 `register_all` |
| `src-tauri/src/commands.rs` | 修改 | 新增 `set_hotkeys` 命令 |
| `src/types/task.ts` | 修改 | `HotkeyConfig` / `KeyOutcome` / `ApplyResult` 接口；`AppConfig` 加 `hotkeys` |
| `src/services/tauri-api.ts` | 修改 | `setHotkeys` 封装 + `onTriggerQuickAdd` / `onHotkeyRegisterFailed` 监听 |
| `src/stores/settings.ts` | 修改 | `hotkeys` computed + `setHotkeys` action |
| `src/App.vue` | 修改 | 监听 `trigger-quick-add` / `hotkey-register-failed` |
| `src/views/SettingsView.vue` | 修改 | 新增「快捷键」设置区 + 录制交互 |
| `src/i18n/locales/en.ts` / `zh.ts` | 修改 | 快捷键相关文案 |

**与 spec 的一处偏差（已确认）：** spec「数据模型」一节提到新增 `AppError::HotkeyRegistrationFailed`。实际设计里 `set_hotkeys` 返回结构化的 `ApplyResult`（每个键带 `ok` / `error`），属于成功返回值而非 `Err`，注册失败不会走 `AppError` 通道。按 YAGNI 不引入这个不会被用到的错误变体。spec 其余内容全部覆盖。

---

## Task 1: 搭建 Vitest

**Files:**
- Create: `vitest.config.ts`
- Modify: `package.json`

- [ ] **Step 1: 安装 vitest**

Run: `npm install -D vitest`
Expected: `package.json` 的 `devDependencies` 出现 `vitest`，无报错。

- [ ] **Step 2: 创建 Vitest 配置**

Create `vitest.config.ts`：

```ts
import { defineConfig } from 'vitest/config';

// 独立配置，不复用 vite.config.ts —— 那份是 Tauri dev/build 专用的。
export default defineConfig({
  test: {
    include: ['src/**/*.test.ts'],
    environment: 'node',
  },
});
```

- [ ] **Step 3: 加 test script**

修改 `package.json` 的 `scripts`，在 `"tauri": "tauri"` 后加一行：

```json
  "scripts": {
    "dev": "vite",
    "build": "vue-tsc --noEmit && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "test": "vitest run"
  },
```

- [ ] **Step 4: 验证 vitest 能跑（暂无测试文件）**

Run: `npx vitest run`
Expected: 输出 `No test files found`（退出码可能非 0，正常 —— 下个任务就会有测试文件）。

- [ ] **Step 5: Commit**

```bash
git add package.json package-lock.json vitest.config.ts
git commit -m "chore: add vitest as the frontend test runner"
```

---

## Task 2: `translateKeyEvent` 纯函数（TDD）

把浏览器 `KeyboardEvent` 翻译成 Tauri accelerator 字符串（如 `"Ctrl+Shift+T"`）。无主键（只按修饰键）返回 `null`。

**Files:**
- Create: `src/utils/hotkey.ts`
- Test: `src/utils/hotkey.test.ts`

- [ ] **Step 1: 写失败的测试**

Create `src/utils/hotkey.test.ts`：

```ts
import { describe, it, expect } from 'vitest';
import { translateKeyEvent, type KeyEventLike } from './hotkey';

/** 构造一个 KeyEventLike，修饰键默认全 false。 */
function ev(partial: Partial<KeyEventLike>): KeyEventLike {
  return {
    key: '',
    code: '',
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    metaKey: false,
    ...partial,
  };
}

describe('translateKeyEvent', () => {
  it('translates a single letter with one modifier', () => {
    expect(translateKeyEvent(ev({ key: 't', code: 'KeyT', ctrlKey: true }))).toBe('Ctrl+T');
  });

  it('translates multiple modifiers in a fixed Ctrl+Shift+Alt+Super order', () => {
    const e = ev({ key: 't', code: 'KeyT', altKey: true, ctrlKey: true, shiftKey: true });
    expect(translateKeyEvent(e)).toBe('Ctrl+Shift+Alt+T');
  });

  it('returns null for a modifier-only event (no main key yet)', () => {
    expect(translateKeyEvent(ev({ key: 'Control', code: 'ControlLeft', ctrlKey: true }))).toBeNull();
    expect(translateKeyEvent(ev({ key: 'Shift', code: 'ShiftLeft', shiftKey: true }))).toBeNull();
  });

  it('translates digit and function keys', () => {
    expect(translateKeyEvent(ev({ key: '1', code: 'Digit1', ctrlKey: true }))).toBe('Ctrl+1');
    expect(translateKeyEvent(ev({ key: 'F5', code: 'F5', altKey: true }))).toBe('Alt+F5');
  });

  it('returns null for Escape (used to cancel recording, never a binding)', () => {
    expect(translateKeyEvent(ev({ key: 'Escape', code: 'Escape', ctrlKey: true }))).toBeNull();
  });

  it('returns null for an unsupported main key', () => {
    expect(translateKeyEvent(ev({ key: 'Dead', code: 'IntlBackslash', ctrlKey: true }))).toBeNull();
  });

  it('maps meta key to Super', () => {
    expect(translateKeyEvent(ev({ key: 'k', code: 'KeyK', metaKey: true }))).toBe('Super+K');
  });
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `npx vitest run src/utils/hotkey.test.ts`
Expected: FAIL —— `Failed to resolve import "./hotkey"`（文件还没创建）。

- [ ] **Step 3: 写实现**

Create `src/utils/hotkey.ts`：

```ts
/**
 * translateKeyEvent 只读取 KeyboardEvent 的这几个字段。抽成接口让单测能
 * 直接构造普通对象，不必造真实 DOM 事件。
 */
export interface KeyEventLike {
  key: string;
  code: string;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  metaKey: boolean;
}

/** 修饰键自身的 `key` 值 —— 单独按这些键不构成一个合法绑定。 */
const MODIFIER_KEYS = new Set(['Control', 'Shift', 'Alt', 'Meta']);

/** 少数有名字的非字母数字主键，映射到 Tauri accelerator 接受的 token。 */
const NAMED_KEYS: Record<string, string> = {
  Space: 'Space',
  Enter: 'Enter',
  Tab: 'Tab',
  Backspace: 'Backspace',
  Delete: 'Delete',
  Home: 'Home',
  End: 'End',
  PageUp: 'PageUp',
  PageDown: 'PageDown',
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
};

/** 把 `event.code` 翻成 Tauri accelerator 主键 token，不支持的返回 null。 */
function mainKeyToken(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3); // KeyT -> T
  if (/^Digit[0-9]$/.test(code)) return code.slice(5); // Digit1 -> 1
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code; // F5 -> F5
  return NAMED_KEYS[code] ?? null;
}

/**
 * 把一次 keydown 翻译成 Tauri accelerator 字符串（如 "Ctrl+Shift+T"）。
 * 返回 null 表示这次事件没有合法主键 —— 用户只按着修饰键，或主键不受支持，
 * 录制态应继续等下一次按键。
 */
export function translateKeyEvent(e: KeyEventLike): string | null {
  if (MODIFIER_KEYS.has(e.key)) return null;
  const main = mainKeyToken(e.code);
  if (!main) return null;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.shiftKey) parts.push('Shift');
  if (e.altKey) parts.push('Alt');
  if (e.metaKey) parts.push('Super');
  parts.push(main);
  return parts.join('+');
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `npx vitest run src/utils/hotkey.test.ts`
Expected: PASS，7 个测试全绿。

- [ ] **Step 5: Commit**

```bash
git add src/utils/hotkey.ts src/utils/hotkey.test.ts
git commit -m "feat: add translateKeyEvent — keyboard event to Tauri accelerator"
```

---

## Task 3: Rust 数据模型 —— `HotkeyConfig`（TDD）

**Files:**
- Modify: `src-tauri/src/types.rs`

- [ ] **Step 1: 写失败的测试**

在 `src-tauri/src/types.rs` 末尾的 `mod tests` 里，`config_deserializes_missing_auto_create_as_true` 之后加：

```rust
    #[test]
    fn hotkey_config_has_sensible_defaults() {
        let h = HotkeyConfig::defaults();
        assert_eq!(h.toggle.as_deref(), Some("CmdOrCtrl+Shift+T"));
        assert_eq!(h.quick_add.as_deref(), Some("CmdOrCtrl+Shift+A"));
    }

    #[test]
    fn config_defaults_include_hotkeys() {
        let c = AppConfig::default();
        assert_eq!(c.hotkeys.toggle.as_deref(), Some("CmdOrCtrl+Shift+T"));
    }

    #[test]
    fn config_deserializes_missing_hotkeys_as_defaults() {
        let json = r#"{"sources":[],"inbox_file":"inbox.md","always_on_top":true}"#;
        let c: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(c.hotkeys.toggle.as_deref(), Some("CmdOrCtrl+Shift+T"));
        assert_eq!(c.hotkeys.quick_add.as_deref(), Some("CmdOrCtrl+Shift+A"));
    }

    #[test]
    fn hotkey_config_roundtrips_with_cleared_key() {
        let h = HotkeyConfig { toggle: Some("Ctrl+Alt+X".into()), quick_add: None };
        let json = serde_json::to_string(&h).unwrap();
        let back: HotkeyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.toggle, h.toggle);
        assert_eq!(back.quick_add, None);
    }
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml hotkey`
Expected: FAIL —— 编译错误 `cannot find type HotkeyConfig`。

- [ ] **Step 3: 写实现**

在 `src-tauri/src/types.rs` 中，`AppConfig` 定义（`pub struct AppConfig`）之前插入 `HotkeyConfig`：

```rust
/// 两个全局快捷键的 accelerator 字符串。`None` = 该键未绑定/已禁用。
/// 字符串是 Tauri accelerator 语法（如 "CmdOrCtrl+Shift+T"）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HotkeyConfig {
    pub toggle: Option<String>,
    pub quick_add: Option<String>,
}

impl HotkeyConfig {
    /// serde `default` 用 —— 旧 config.json 没有 `hotkeys` 字段时落到这里。
    pub fn defaults() -> Self {
        HotkeyConfig {
            toggle: Some("CmdOrCtrl+Shift+T".into()),
            quick_add: Some("CmdOrCtrl+Shift+A".into()),
        }
    }
}
```

在 `AppConfig` 结构体里，`auto_create_quadrant_headers` 字段之后加：

```rust
    /// 全局快捷键绑定。`#[serde(default)]` 保证旧 config 能加载。
    #[serde(default = "HotkeyConfig::defaults")]
    pub hotkeys: HotkeyConfig,
```

在 `impl Default for AppConfig` 的结构体字面量里，`auto_create_quadrant_headers: true,` 之后加：

```rust
            hotkeys: HotkeyConfig::defaults(),
```

- [ ] **Step 4: 修复 `config.rs` 测试里手写的 `AppConfig` 字面量**

`src-tauri/src/config.rs` 的 `load_strips_verbatim_prefix_and_remaps_default_id` 测试里有一处手写的 `AppConfig { ... }` 字面量（`auto_create_quadrant_headers: true,` 那行）。在它之后补一行：

```rust
            hotkeys: crate::types::HotkeyConfig::defaults(),
```

- [ ] **Step 5: 跑测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS —— 新增 4 个 hotkey 测试 + 原有 config/types 测试全绿。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/types.rs src-tauri/src/config.rs
git commit -m "feat: add HotkeyConfig to AppConfig with serde-default fallback"
```

---

## Task 4: `hotkeys.rs` 模块 + `lib.rs` 集成（TDD）

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/hotkeys.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 加依赖**

`src-tauri/Cargo.toml` 的 `[dependencies]` 末尾（`ulid = "1"` 之后）加：

```toml
tauri-plugin-global-shortcut = "2"
```

- [ ] **Step 2: 写失败的测试（`matches_accel` 纯函数）**

Create `src-tauri/src/hotkeys.rs`，先只放测试 + 待实现的函数签名占位会编译不过，所以这一步直接写完整文件骨架 + 测试。创建 `src-tauri/src/hotkeys.rs`：

```rust
use std::str::FromStr;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

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
```

- [ ] **Step 3: 在 `lib.rs` 注册模块**

`src-tauri/src/lib.rs` 顶部的 `mod` 列表里，按字母序在 `mod history;` 之后加：

```rust
mod hotkeys;
```

- [ ] **Step 4: 跑测试确认通过（这一步同时验证依赖能编译）**

Run: `cargo test --manifest-path src-tauri/Cargo.toml matches_accel`
Expected: PASS —— 4 个 `matches_accel` 测试全绿。若 `Shortcut::from_str` 报 trait 未引入，确认 `use std::str::FromStr;` 在文件顶部。

- [ ] **Step 5: 抽出 `toggle_main_window`**

在 `src-tauri/src/lib.rs` 末尾、现有 `fn toggle_window` 之后，加共享函数：

```rust
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
```

- [ ] **Step 6: 托盘左键点击改用 `toggle_main_window`**

在 `src-tauri/src/lib.rs` 中，把现有的 `.on_tray_icon_event` 闭包整体替换为：

```rust
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
```

- [ ] **Step 7: 注册 global-shortcut 插件**

在 `src-tauri/src/lib.rs` 的 `tauri::Builder::default()` 链上，把 `.plugin(tauri_plugin_dialog::init())` 改成两个 plugin 调用：

```rust
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
```

- [ ] **Step 8: 启动时注册快捷键**

在 `src-tauri/src/lib.rs` 的 `.setup(|app| { ... })` 闭包里，history 窗口预创建的那个 `if app.get_webview_window("history").is_none() { ... }` 块之后、`Ok(())` 之前，加：

```rust
            // 注册全局快捷键。单个失败不影响启动（register_all 内部已 emit
            // hotkey-register-failed，前端会 toast 提示）。
            hotkeys::register_all(&app.handle().clone(), &cfg.hotkeys);
```

- [ ] **Step 9: 编译验证**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过，无 error。`hotkeys.rs` 里 `apply` / `ApplyResult` / `KeyOutcome` 此时还没被调用，可能有 `dead_code` warning —— 正常，Task 5 会用到。

- [ ] **Step 10: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/hotkeys.rs src-tauri/src/lib.rs
git commit -m "feat: register global shortcuts via tauri-plugin-global-shortcut"
```

---

## Task 5: `set_hotkeys` 命令

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 加 `set_hotkeys` 命令**

在 `src-tauri/src/commands.rs` 顶部的 `use crate::types::{...}` 一行里加上 `HotkeyConfig`：

```rust
use crate::types::{file_label_key, AppConfig, HotkeyConfig, QuickActionKind, Source, SourceKind, Task};
```

在文件末尾、`hide_window` 命令之后加：

```rust
/// 重新设置两个全局快捷键。先注销旧键再注册新键；某键注册失败会回滚到旧
/// 绑定。只把成功注册的键写进 config —— 失败的键保留旧值，配置和实际注册
/// 状态始终一致。返回逐键结果给前端做 toast 反馈。
#[tauri::command]
pub fn set_hotkeys(
    state: State<'_, AppState>,
    app: AppHandle,
    toggle: Option<String>,
    quick_add: Option<String>,
) -> Result<crate::hotkeys::ApplyResult> {
    let old = { state.config.read().unwrap().hotkeys.clone() };
    let new = HotkeyConfig { toggle, quick_add };
    let result = crate::hotkeys::apply(&app, &old, &new);

    let mut persisted = old.clone();
    if result.toggle.ok {
        persisted.toggle = new.toggle.clone();
    }
    if result.quick_add.ok {
        persisted.quick_add = new.quick_add.clone();
    }
    {
        let mut cfg = state.config.write().unwrap();
        cfg.hotkeys = persisted;
        config::save_to(&state.config_path, &cfg)?;
    }
    Ok(result)
}
```

- [ ] **Step 2: 注册到 invoke_handler**

在 `src-tauri/src/lib.rs` 的 `tauri::generate_handler![ ... ]` 列表里，`commands::hide_window,` 之后加：

```rust
            commands::set_hotkeys,
```

- [ ] **Step 3: 编译验证**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过，无 error，`hotkeys.rs` 的 `dead_code` warning 消失。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add set_hotkeys command with per-key rollback"
```

---

## Task 6: 前端 API + 类型 + settings store

**Files:**
- Modify: `src/types/task.ts`
- Modify: `src/services/tauri-api.ts`
- Modify: `src/stores/settings.ts`

- [ ] **Step 1: 加 TS 类型**

在 `src/types/task.ts` 的 `AppConfig` 接口定义**之前**加：

```ts
export interface HotkeyConfig {
  /// Tauri accelerator string, or null when the key is unbound.
  toggle: string | null;
  quick_add: string | null;
}

/// Result of one key's registration attempt, returned by `set_hotkeys`.
export interface KeyOutcome {
  ok: boolean;
  accelerator: string | null;
  error: string | null;
}

export interface ApplyResult {
  toggle: KeyOutcome;
  quick_add: KeyOutcome;
}
```

在 `AppConfig` 接口里，`auto_create_quadrant_headers: boolean;` 之后加：

```ts
  hotkeys: HotkeyConfig;
```

- [ ] **Step 2: 加 api 封装 + 事件监听**

在 `src/services/tauri-api.ts` 顶部的类型 import 里加 `ApplyResult`：

```ts
import type { Task, AppConfig, Source, SourceKind, QuickActionKind, Quadrant, ApplyResult } from '../types/task';
```

在 `api` 对象里，`setAlwaysOnTop` 那一行之后加：

```ts
  /// Re-register both global hotkeys. Pass the full pair every time —
  /// the backend persists the whole HotkeyConfig. `null` = unbind that key.
  setHotkeys: (toggle: string | null, quickAdd: string | null) =>
    invoke<ApplyResult>('set_hotkeys', { toggle, quickAdd }),
```

在 `api` 对象末尾的事件监听区，`onHistorySeenChanged` 之后加：

```ts
  onTriggerQuickAdd: (cb: (wasHidden: boolean) => void): Promise<UnlistenFn> =>
    listen<{ wasHidden: boolean }>('trigger-quick-add', e => cb(e.payload.wasHidden)),
  onHotkeyRegisterFailed: (cb: (accelerator: string) => void): Promise<UnlistenFn> =>
    listen<string>('hotkey-register-failed', e => cb(e.payload)),
```

- [ ] **Step 3: settings store 加 `hotkeys` + `setHotkeys`**

在 `src/stores/settings.ts` 顶部的类型 import 里加 `HotkeyConfig` 和 `ApplyResult`：

```ts
import type { AppConfig, ApplyResult, HotkeyConfig, QuickActionKind, Source, SourceKind } from '../types/task';
```

在 store 内，`hubFolder` computed 之后加：

```ts
  const hotkeys = computed<HotkeyConfig>(
    () => config.value?.hotkeys ?? { toggle: null, quick_add: null },
  );
```

在 `resyncHub` action 之后加：

```ts
  /// Re-register hotkeys, reload config so the `hotkeys` computed reflects
  /// what actually persisted, and hand the per-key result back to the caller
  /// (SettingsView decides the toast since it knows which key changed).
  async function setHotkeys(
    toggle: string | null,
    quickAdd: string | null,
  ): Promise<ApplyResult> {
    const result = await api.setHotkeys(toggle, quickAdd);
    await load();
    return result;
  }
```

在 store 的 `return { ... }` 里，`hubFolder,` 之后加 `hotkeys,`，`resyncHub,` 之后加 `setHotkeys,`。

- [ ] **Step 4: 类型检查**

Run: `npx vue-tsc --noEmit`
Expected: 无 error。

- [ ] **Step 5: Commit**

```bash
git add src/types/task.ts src/services/tauri-api.ts src/stores/settings.ts
git commit -m "feat: add hotkeys to frontend config types, api, and settings store"
```

---

## Task 7: `App.vue` 监听 `trigger-quick-add` / `hotkey-register-failed`

**Files:**
- Modify: `src/App.vue`

- [ ] **Step 1: 加 import**

在 `src/App.vue` 的 `<script setup>` 里，现有 import 之后加：

```ts
import { openQuickAdd } from './composables/useQuickAdd';
import { toast } from './composables/useToast';
import { useI18n } from 'vue-i18n';
```

并在 `const settings = useSettingsStore();` 等声明附近加：

```ts
const { t } = useI18n();
```

- [ ] **Step 2: 在 `onMounted` 里加两个监听器**

在 `src/App.vue` 的 `onMounted` 内，`onSourceScanFinished` 监听器注册之后加：

```ts
  unlisteners.push(await api.onTriggerQuickAdd(async (wasHidden) => {
    // 全局 quick-add 快捷键触发：选默认源（没有默认就第一个），弹 QuickAdd。
    const sourceId = settings.defaultSourceId ?? settings.sources[0]?.id ?? null;
    if (!sourceId) {
      toast.info(t('toast.addSourceFirst'));
      return;
    }
    const result = await openQuickAdd({ sourceId });
    if (result) {
      await tasks.add(result.text, result.sourceId, result.quadrant);
      // 窗口本是被快捷键临时呼出的 —— 存完任务退回隐藏。取消(Esc)则不动。
      if (wasHidden) await api.hideWindow();
    }
  }));
  unlisteners.push(await api.onHotkeyRegisterFailed((accelerator) => {
    toast.warning(t('toast.hotkeyRegisterFailed', { accel: accelerator }));
  }));
```

- [ ] **Step 3: 类型检查**

Run: `npx vue-tsc --noEmit`
Expected: 无 error（`toast.addSourceFirst` / `toast.hotkeyRegisterFailed` 的 i18n key 在 Task 8 添加；vue-tsc 不校验 i18n key 是否存在，所以这一步能过）。

- [ ] **Step 4: Commit**

```bash
git add src/App.vue
git commit -m "feat: open QuickAdd on the global quick-add hotkey"
```

---

## Task 8: `SettingsView.vue`「快捷键」设置区 + i18n

**Files:**
- Modify: `src/i18n/locales/en.ts`
- Modify: `src/i18n/locales/zh.ts`
- Modify: `src/views/SettingsView.vue`

- [ ] **Step 1: 加 i18n 文案（en）**

在 `src/i18n/locales/en.ts` 中，`settings.sections` 对象里 `language: 'Language',` 之后加：

```ts
      hotkeys: 'Global hotkeys',
```

在 `settings` 对象里，`behavior` 相关键之后（`auto_create_quadrant_headers_help` 之后）加：

```ts
    hotkeys: {
      hint: 'System-wide shortcuts. Click a binding to record a new key combo, Esc to cancel.',
      toggle: 'Show / hide window',
      quickAdd: 'Quick add task',
      record: 'Click to record',
      recording: 'Press a key combo…',
      clear: 'Clear',
      unbound: 'Not set',
      updated: 'Hotkey updated',
    },
```

在 `toast` 对象里（找到 `quickActionsUpdated` 所在的对象），加：

```ts
    addSourceFirst: 'Add a task source first',
    hotkeyRegisterFailed: '{accel} could not be registered — it may be in use by another app',
```

- [ ] **Step 2: 加 i18n 文案（zh）**

在 `src/i18n/locales/zh.ts` 中，`settings.sections` 对象里 `language: '语言',`（或对应行）之后加：

```ts
      hotkeys: '全局快捷键',
```

在 `settings` 对象里，`auto_create_quadrant_headers_help` 对应行之后加：

```ts
    hotkeys: {
      hint: '系统级快捷键。点击某个绑定可录制新组合键，Esc 取消。',
      toggle: '显示 / 隐藏窗口',
      quickAdd: '快速添加任务',
      record: '点击录制',
      recording: '请按下组合键…',
      clear: '清空',
      unbound: '未设置',
      updated: '已更新快捷键',
    },
```

在 `toast` 对象里加：

```ts
    addSourceFirst: '请先添加一个任务源',
    hotkeyRegisterFailed: '{accel} 注册失败，可能已被其它程序占用',
```

- [ ] **Step 3: SettingsView 加 script 逻辑**

在 `src/views/SettingsView.vue` 的 `<script setup>` 里，现有 import 之后加：

```ts
import { translateKeyEvent } from '../utils/hotkey';
import { toast } from '../composables/useToast';
```

确认 `ref` / `onUnmounted` 已从 `vue` import；若没有则补上。然后在 script 内（已有 `const settings = useSettingsStore();` 之类声明附近）加：

```ts
type HotkeyKey = 'toggle' | 'quick_add';

/// 当前正在录制哪个键；null = 没在录制。
const recordingKey = ref<HotkeyKey | null>(null);

/// 把 accelerator 字符串显示得好看点："Ctrl+Shift+T" -> "Ctrl + Shift + T"。
function displayAccel(accel: string | null): string {
  return accel ? accel.replace(/\+/g, ' + ') : '';
}

function onRecordKey(e: KeyboardEvent) {
  e.preventDefault();
  e.stopPropagation();
  if (e.key === 'Escape') {
    stopRecording();
    return;
  }
  const accel = translateKeyEvent(e);
  if (!accel) return; // 只按了修饰键 / 不支持的主键 —— 继续等
  const key = recordingKey.value;
  stopRecording();
  if (key) void commitHotkey(key, accel);
}

function startRecording(key: HotkeyKey) {
  recordingKey.value = key;
  window.addEventListener('keydown', onRecordKey, { capture: true });
}

function stopRecording() {
  recordingKey.value = null;
  window.removeEventListener('keydown', onRecordKey, { capture: true });
}

/// 提交一个键的新值（accel=null 表示清空解绑），按结果 toast。
async function commitHotkey(key: HotkeyKey, accel: string | null) {
  const hk = settings.hotkeys;
  const toggle = key === 'toggle' ? accel : hk.toggle;
  const quickAdd = key === 'quick_add' ? accel : hk.quick_add;
  try {
    const result = await settings.setHotkeys(toggle, quickAdd);
    const outcome = key === 'toggle' ? result.toggle : result.quick_add;
    if (outcome.ok) {
      toast.success(t('settings.hotkeys.updated'));
    } else {
      toast.error(t('toast.hotkeyRegisterFailed', { accel: accel ?? '' }));
    }
  } catch (e) {
    toast.error(errorMessage(e));
  }
}

function clearHotkey(key: HotkeyKey) {
  void commitHotkey(key, null);
}

onUnmounted(() => {
  // 录制中途离开设置页时把 window 监听器摘掉。
  if (recordingKey.value) stopRecording();
});
```

注意：`errorMessage` 若 SettingsView 还没 import，从 `'../utils/errors'` 引入；`t` 应该已通过现有的 `useI18n()` 拿到（文件里别处已在用 `t(...)`）。若文件里没有现成的 `onUnmounted` import，记得在 `from 'vue'` 里补上。

- [ ] **Step 4: SettingsView 加「快捷键」设置区模板**

在 `src/views/SettingsView.vue` 的 `<template>` 里，语言那个 `<section>`（`t('settings.sections.language')`）的 `</section>` 之后、行为 `<section>` 之前，插入：

```html
      <!-- Global hotkeys -->
      <section class="section">
        <h3>{{ t('settings.sections.hotkeys') }}</h3>
        <p class="muted hint">{{ t('settings.hotkeys.hint') }}</p>
        <div class="row">
          <span class="row-label">{{ t('settings.hotkeys.toggle') }}</span>
          <div class="hotkey-controls">
            <button
              class="hotkey-bind"
              :class="{ recording: recordingKey === 'toggle' }"
              @click="recordingKey === 'toggle' ? stopRecording() : startRecording('toggle')"
            >
              {{ recordingKey === 'toggle'
                ? t('settings.hotkeys.recording')
                : (settings.hotkeys.toggle
                    ? displayAccel(settings.hotkeys.toggle)
                    : t('settings.hotkeys.record')) }}
            </button>
            <button
              v-if="settings.hotkeys.toggle && recordingKey !== 'toggle'"
              class="hotkey-clear"
              :title="t('settings.hotkeys.clear')"
              @click="clearHotkey('toggle')"
            >✕</button>
          </div>
        </div>
        <div class="row">
          <span class="row-label">{{ t('settings.hotkeys.quickAdd') }}</span>
          <div class="hotkey-controls">
            <button
              class="hotkey-bind"
              :class="{ recording: recordingKey === 'quick_add' }"
              @click="recordingKey === 'quick_add' ? stopRecording() : startRecording('quick_add')"
            >
              {{ recordingKey === 'quick_add'
                ? t('settings.hotkeys.recording')
                : (settings.hotkeys.quick_add
                    ? displayAccel(settings.hotkeys.quick_add)
                    : t('settings.hotkeys.record')) }}
            </button>
            <button
              v-if="settings.hotkeys.quick_add && recordingKey !== 'quick_add'"
              class="hotkey-clear"
              :title="t('settings.hotkeys.clear')"
              @click="clearHotkey('quick_add')"
            >✕</button>
          </div>
        </div>
      </section>
```

- [ ] **Step 5: SettingsView 加样式**

在 `src/views/SettingsView.vue` 的 `<style scoped>` 末尾加：

```css
.hotkey-controls {
  display: flex;
  align-items: center;
  gap: 6px;
}
.hotkey-bind {
  min-width: 140px;
  padding: 5px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-elevated);
  color: var(--text);
  font-size: 0.8rem;
  cursor: pointer;
}
.hotkey-bind:hover {
  border-color: var(--accent);
}
.hotkey-bind.recording {
  border-color: var(--accent);
  color: var(--accent);
}
.hotkey-clear {
  width: 24px;
  height: 24px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-elevated);
  color: var(--text-muted);
  cursor: pointer;
  line-height: 1;
}
.hotkey-clear:hover {
  color: var(--danger);
  border-color: var(--danger);
}
```

注意：若 `--bg-elevated` / `--danger` / `--text-muted` 等变量名在本项目 CSS 里不存在，用 `src/styles/main.css` 里实际定义的同义变量替换（参考 SettingsView 现有样式块用的是哪些变量）。

- [ ] **Step 6: 类型检查 + 测试全跑**

Run: `npx vue-tsc --noEmit`
Expected: 无 error。

Run: `npx vitest run`
Expected: PASS（`hotkey.test.ts` 全绿）。

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS。

- [ ] **Step 7: Commit**

```bash
git add src/i18n/locales/en.ts src/i18n/locales/zh.ts src/views/SettingsView.vue
git commit -m "feat: add global hotkey settings section with key recording"
```

---

## 手动验证（实现完成后）

全局快捷键注册依赖 OS，无法单测。跑 `npm run tauri dev` 后逐项验证：

1. **默认键生效** —— 按 `Ctrl+Shift+T` 切换主窗口显示/隐藏；按 `Ctrl+Shift+A` 弹出 QuickAdd 模态。
2. **quick-add 退回行为** —— 窗口隐藏时按 `Ctrl+Shift+A`，输入任务回车 → 窗口自动隐回；窗口本就开着时按 → 存完保持可见。Esc 取消 → 窗口留在原状态（被临时呼出的会留个空窗口，已知行为）。
3. **没有 source** —— 删光所有 source，按 `Ctrl+Shift+A` → 看到「请先添加一个任务源」toast，不弹模态。
4. **设置页录制** —— 进设置 →「全局快捷键」区 → 点 toggle 绑定按钮 → 显示「请按下组合键…」→ 按 `Ctrl+Alt+J` → toast「已更新快捷键」→ 新键生效，旧键失效。
5. **冲突回退** —— 录制一个明显被占用的组合（如 `Ctrl+Shift+T` 若已被别的程序占）→ 看到 error toast，按钮回退显示旧值。
6. **清空** —— 点某键的 ✕ → 该键解绑、按它无反应；按钮显示「点击录制」。
7. **持久化** —— 改完键、关 app、重开 → 配置保留，新键仍生效。
8. **旧 config 兼容** —— 临时把 `config.json` 里的 `hotkeys` 字段删掉再启动 → app 正常，快捷键落到默认值。

## Self-Review 记录

- **Spec 覆盖**：架构（Task 4/5）、数据模型 `HotkeyConfig`（Task 3）、交互流程 A toggle（Task 4 `toggle_main_window` + `dispatch`）、流程 B quick-add（Task 4 `trigger_quick_add` + Task 7）、流程 C 设置页录制（Task 2 + Task 8）、错误处理 rollback（Task 5 `apply_one`）/ 启动失败 emit（Task 4 `register_all`）/ 前端 toast（Task 7、Task 8）、测试 `translateKeyEvent`（Task 2）+ config serde（Task 3）—— 全部有对应任务。`AppError::HotkeyRegistrationFailed` 见开头「与 spec 的偏差」说明，有意不做。
- **类型一致性**：`HotkeyConfig` 字段 `toggle` / `quick_add`（Rust snake_case）与 TS `HotkeyConfig.toggle` / `quick_add` 一致；`ApplyResult` / `KeyOutcome` Rust 与 TS 字段对齐；`set_hotkeys` 命令参数 `quick_add`（Rust）↔ `quickAdd`（JS invoke，Tauri 自动转换）；`trigger-quick-add` 事件 payload `{ wasHidden }` 在 Rust emit（`hotkeys.rs`）与 TS listen（`tauri-api.ts`）两侧一致。
- **占位符**：无 TBD / TODO；每个代码步骤都给了完整代码。
