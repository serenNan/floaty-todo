# 全局快捷键 — 设计文档

> v0.3 功能。让 Floaty Todo 真正「按一下就能记一条」：一个键呼出/隐藏主窗口，一个键直接弹快速添加。

## 摘要

新增两个可自定义的全局快捷键：

- **toggle 键**（默认 `CmdOrCtrl+Shift+T`）—— 呼出/隐藏主窗口
- **quick-add 键**（默认 `CmdOrCtrl+Shift+A`）—— 直接弹快速添加模态

基于 `tauri-plugin-global-shortcut` v2。键位存进 `AppConfig`，可在设置页用「按键录制」方式自定义，也可清空解绑。注册失败（撞键 / 非法）通过 toast 提示，不影响 app 启动。

## 架构

新增 Rust 模块 `src-tauri/src/hotkeys.rs`，单一职责：注册/注销全局快捷键 + 路由回调。

```
src-tauri/
  Cargo.toml      # + tauri-plugin-global-shortcut = "2"
  src/lib.rs      # 插件 init + setup 里调 hotkeys::register_all；抽出 toggle_main_window
  src/hotkeys.rs  # 新模块
  src/commands.rs # + set_hotkeys 命令
  src/types.rs    # AppConfig + HotkeyConfig
  src/config.rs   # serde default 兜底
  src/error.rs    # + HotkeyRegistrationFailed
src/
  utils/hotkey.ts        # translateKeyEvent 纯函数（带单测）
  services/tauri-api.ts  # + trigger-quick-add / hotkey-register-failed 事件监听 + setHotkeys 封装
  stores/settings.ts     # + hotkeys 状态 + setHotkeys action
  views/SettingsView.vue # + 「快捷键」设置区
  App.vue                # 监听 trigger-quick-add
```

`hotkeys.rs` 对外接口：

- `register_all(app: &AppHandle, cfg: &HotkeyConfig)` —— 启动时调，逐个注册。单个失败只记 `eprintln` 日志 + emit `hotkey-register-failed` 事件，不 panic。
- `apply(app: &AppHandle, cfg: &HotkeyConfig) -> ApplyResult` —— 先全注销再按新配置注册，返回每个键的注册结果给命令层。
- 内部回调：toggle 调共享的 `toggle_main_window`；quick-add 见交互流程 B。

`lib.rs` 现有托盘左键点击逻辑（可见且聚焦→隐藏，否则显示+聚焦+unminimize）抽成共享函数 `toggle_main_window(app)`，托盘和 toggle 快捷键都调它。

## 数据模型

`AppConfig` 新增一个嵌套字段：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub toggle: Option<String>,     // None = 未绑定 / 禁用
    pub quick_add: Option<String>,
}

impl HotkeyConfig {
    fn defaults() -> Self {
        HotkeyConfig {
            toggle: Some("CmdOrCtrl+Shift+T".into()),
            quick_add: Some("CmdOrCtrl+Shift+A".into()),
        }
    }
}

pub struct AppConfig {
    // ...现有字段...
    #[serde(default = "HotkeyConfig::defaults")]
    pub hotkeys: HotkeyConfig,
}
```

设计要点：

- `Option<String>` 支持「清空某个键 = 不绑定」。
- `#[serde(default = "HotkeyConfig::defaults")]` 保证旧 `config.json` 没有 `hotkeys` 字段时也能正常加载并落到默认值（沿用现有「损坏容忍」风格）。
- accelerator 用 `CmdOrCtrl` 跨平台写法，与 `shell.rs` 已照顾 mac/linux 的风格一致。

`AppError` 新增变体：

```rust
HotkeyRegistrationFailed { key: String, accelerator: String }
```

序列化为 `{code, message, key, accelerator}` 结构，前端用 `errorCode(e)` / `errorField(e, key)` 稳定匹配，与现有错误处理风格一致。

## 交互流程

### 流程 A：toggle 键（呼出/隐藏）

```
按 toggle 键 → hotkeys 回调
  → main 窗口 is_visible && is_focused ?
      是 → hide()
      否 → show() + unminimize() + set_focus()
```

复用抽出的 `toggle_main_window(app)`。

### 流程 B：quick-add 键（快速添加）

```
按 quick-add 键 → hotkeys 回调
  → was_visible = main 窗口 is_visible()
  → 不可见则 show() + set_focus()
  → emit("trigger-quick-add", { wasHidden: !was_visible })

前端 App.vue 监听 trigger-quick-add：
  → sourceId = settings.defaultSourceId ?? settings.sources[0]?.id
  → 没有任何 source → toast.info("先添加一个任务源") + 结束
  → openQuickAdd({ sourceId })
  → await 结果：
      result ≠ null（已保存）&& wasHidden → api.hideWindow()
      result = null（Esc 取消）           → 不动，窗口留在原状态
```

边界处理：

- **窗口被快捷键临时呼出 → Esc 取消**：窗口留着不隐回去。已知会留一个空窗口在屏上，按用户决定保留此行为。
- **QuickAdd 模态已开着时再按键**：`openQuickAdd` 单例会把旧 pending `resolve(null)` 再开新的，行为正常。
- quick-add 的 source 选择：模态内仍有 source 下拉，全局触发只是预选默认源，用户可在弹窗内改。

### 流程 C：设置页改快捷键

```
SettingsView「快捷键」区，每个键一个按钮显示当前绑定（如 "Ctrl + Shift + T"）
  → 点击 → 进入「按下快捷键…」录制态
  → 捕获下一个 keydown → translateKeyEvent() → accelerator 字符串
      纯修饰键（只按了 Ctrl 等）→ 返回 null，继续等
      Esc → 取消录制，保持原值
      合法组合 → invoke set_hotkeys
  → 每个键旁有「✕ 清空」按钮 → 设为 None（解绑）
```

`translateKeyEvent`（`src/utils/hotkey.ts`）把 JS `KeyboardEvent` 翻译成 Tauri accelerator 字符串：收集修饰键（Ctrl/Shift/Alt/Meta）按固定顺序排列 + 主键。无主键返回 `null`。

## 错误处理

- **`set_hotkeys` 命令**：先注销旧键 → 注册新键。某个键注册失败（accelerator 非法 / 被其它程序占用）→ **回滚**：重新注册该键的旧值，避免落到「新的没绑上、旧的也没了」。命令返回 `{toggle: Result, quick_add: Result}`（成功/失败 + accelerator）。
- **前端按结果反馈**：成功 → `toast.success`；失败 → `toast.error("Ctrl+Shift+T 注册失败，可能已被其它程序占用")`，设置页按钮回退显示旧值。
- **启动时 `register_all`**：某个键失败不影响 app 启动，记 `eprintln` 日志 + emit `hotkey-register-failed` 事件；主窗口监听后 `toast.warning` 提示「某快捷键未生效，请到设置里换一个」。
- **`translateKeyEvent`**：只有修饰键无主键 → 返回 `null`（录制态继续等）；主键非法 → 返回 `null`。
- **解绑（设为 None）**：注销该键，不再注册，不算错误。

## 测试

### 单元测试（重点）

- **`translateKeyEvent` 纯函数**（`src/utils/hotkey.ts`）—— 输入构造的 `KeyboardEvent`-like 对象，断言输出 accelerator 字符串。覆盖：单字母键、功能键、纯修饰键返回 `null`、多修饰键顺序归一化（固定 `Ctrl+Shift+Alt+` 顺序）、Esc 特判。
- **Rust：`AppConfig` serde round-trip** —— 旧 JSON（无 `hotkeys` 字段）能加载且 `hotkeys` 落到默认值；`HotkeyConfig` 序列化/反序列化。

### 手动验证

全局快捷键注册依赖 OS，无法单测：

1. 默认键启动 → 按 `Ctrl+Shift+T` 切换窗口、`Ctrl+Shift+A` 弹快速添加。
2. quick-add 后窗口按预期隐回（被临时呼出时）/ 保留（原本就开着）。
3. 设置页录制新键 → 生效；故意填被占用的组合 → 看到 error toast + 按钮回退。
4. 清空某键 → 该键失效；重启后配置持久。

## 实现顺序（TDD 友好）

1. `translateKeyEvent` + 单测
2. Rust 数据模型（`HotkeyConfig` / `AppConfig.hotkeys` / `AppError`）+ config serde 单测
3. `hotkeys.rs` 注册逻辑 + `lib.rs` 抽 `toggle_main_window` + 插件 init + `register_all`
4. `set_hotkeys` 命令 + invoke_handler 注册
5. 前端：`tauri-api.ts` 事件/封装 + `settings.ts` 状态 + `App.vue` 监听 `trigger-quick-add`
6. `SettingsView.vue` 「快捷键」设置区

## 设计上明确「不做」

- ❌ 多于两个的快捷键（如「打开历史窗口」全局键）—— 本轮只做 toggle + quick-add。
- ❌ 快捷键序列 / 和弦（chord）—— 只支持单组合键。
- ❌ 按平台分别配置不同键 —— 一份 accelerator 用 `CmdOrCtrl` 跨平台。
