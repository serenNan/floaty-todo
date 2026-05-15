# 任务历史 + 撤销 + 时间线窗口 设计文档

**日期**：2026-05-15
**作者**：serenNan + Claude
**状态**：待审阅

## 1. 目标 / 非目标

### 目标

- 给 Floaty-todo 加 Ctrl+Z / Ctrl+Y 经典撤销重做（覆盖 toggle / edit / add / move quadrant 4 种 App 内动作）
- 提供独立**历史窗口**：左侧 Notion 风时间线（事件流），右侧 diff 详情 + 「跳到此处」回滚按钮
- 把外部编辑（VS Code / Obsidian 直接改 .md）也显示在时间线上，但**不可撤销**，提示用户走 git 回滚
- 操作历史文件持久化到 Hub 根目录（`<hub>/.floaty-history.jsonl`），自动随 Hub git 仓库被追踪

### 非目标

- 不做外部编辑的可撤销（diff 反向应用复杂度高，且 git 已经覆盖）
- 不做协作 / 多设备同步（这是单机工具）
- 不做无限期清理 / 归档策略（按估算 5 年 ~ 36MB，远未到瓶颈）
- 不做选择性多步撤销（仅按时间顺序栈式撤销）

## 2. 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                       Floaty-todo App                       │
│                                                             │
│  ┌─────────────────┐    ┌───────────────────────────────┐   │
│  │  storage.rs     │    │  history.rs    (新增)         │   │
│  │  ───────────    │───▶│  ───────────                  │   │
│  │  toggle_task    │    │  push_event(InAppEvent)       │   │
│  │  update_task    │    │  pop_undo() / pop_redo()      │   │
│  │  remove_task    │    │  jump_to(event_id)            │   │
│  │  append_task    │    │  load_events() / save_event() │   │
│  └─────────────────┘    └─────────┬─────────────────────┘   │
│                                   │                         │
│  ┌─────────────────┐              │                         │
│  │  watcher.rs     │──────────────┤                         │
│  │  ─────────────  │  external_edit                         │
│  │  (现有)         │  事件                                  │
│  └─────────────────┘              ▼                         │
│                            ┌─────────────┐                  │
│                            │ history.    │                  │
│                            │ jsonl       │  (append-only)   │
│                            └─────────────┘                  │
│                                   │                         │
│  ┌──────────────┐  ┌────────────────────────────┐           │
│  │ 主窗口       │  │ History Window (新增)       │           │
│  │ Ctrl+Z/Y     │  │ ─────────────────           │           │
│  │ footer 🕒按钮│  │ 左：event stream            │           │
│  │ Ctrl+H 打开  │  │ 右：diff + jump 按钮        │           │
│  └──────────────┘  └────────────────────────────┘           │
└─────────────────────────────────────────────────────────────┘
              │
              ▼
       <hub>/.floaty-history.jsonl
       （随 Hub git 仓库追踪 → 跨电脑可看历史）
```

## 3. 数据模型

### 3.1 事件 schema（JSONL，每行一个 JSON）

```jsonc
// 通用字段
{
  "id": "01HXYZ...",                  // ULID，方便时间排序+全局唯一
  "ts": "2026-05-15T14:32:01.123Z",  // RFC3339
  "kind": "toggle" | "edit" | "add" | "move" | "external_edit",
  "source_id": "6009f9444e1d25f2",   // 引用 AppConfig.sources
  "file": "D:\\Notes\\📅TODO.md",     // 绝对路径（同 source 也可能多个文件）
  // 以下字段按 kind 不同
}
```

**App 内事件（可撤销）：**

```jsonc
// toggle
{
  "kind":"toggle",
  "task_id":"a1b2c3d4",              // 同 parser.rs 算出的稳定 ID
  "line":42,
  "before":{"done":false,"text":"修临床方案"},
  "after":{"done":true,"text":"修临床方案"},
  "before_hash":"sha256:..."          // 原始整行（含缩进/bullet/checkbox/空白）的 SHA-256，撤销前比对，确保中间无外部改动
}

// edit (仅 text，不跨象限)
{
  "kind":"edit",
  "task_id":"a1b2c3d4",
  "line":42,
  "before":{"text":"修 bug"},
  "after":{"text":"修 OCR bug"},
  "before_hash":"sha256:..."
}

// add
{
  "kind":"add",
  "line":42,                          // 新增行的位置
  "after":{"done":false,"text":"新任务","quadrant":"q3"}
  // 撤销 = 按 (line, text_hash) 定位并删除
}

// move quadrant (跨象限)
{
  "kind":"move",
  "task_id":"a1b2c3d4",
  "before":{"line":42,"quadrant":"q1","text":"x"},
  "after":{"line":67,"quadrant":"q3","text":"x"},
  "before_hash":"sha256:..."
}
```

**外部编辑事件（仅时间线，不可撤销）：**

```jsonc
{
  "kind":"external_edit",
  "diff_summary":{"added":3,"removed":1,"modified":2},
  "size_bytes_delta":+87,
  "note":"watcher 在 14:15:42 检测到 file 变化，且不是 App 自己的写入"
}
```

### 3.2 存储位置

- 路径：`<hub_folder>/.floaty-history.jsonl`
- 仅当 `AppConfig.hub_folder` 被设置时启用历史记录
- **未设置 hub 时**：fall back 到 `%APPDATA%/com.serendipity.floaty-todo/history.jsonl`（保证功能可用）
- App 启动时通过 `hub_folder` 切换读取位置；切换 hub 时**不迁移旧记录**（避免数据丢失/重复），用户自行处理

### 3.3 撤销 cursor 持久化

- 文件：`<同目录>/.floaty-history.cursor`，内容：最后一次「未被撤销」的事件 ID
- 重启后从 cursor 加载 undo 栈（cursor 之后的事件 = 已撤销但可 redo）
- 任何新事件写入时：清空 cursor 之后的事件 + 追加新事件 + 更新 cursor

## 4. Rust 模块改动

### 4.1 新增 `src-tauri/src/history.rs`

```rust
pub struct HistoryEvent { /* 上节 schema */ }
pub enum EventKind { Toggle, Edit, Add, Move, ExternalEdit }

pub struct HistoryStore {
    file_path: PathBuf,              // .floaty-history.jsonl
    cursor_path: PathBuf,            // .floaty-history.cursor
    events: Vec<HistoryEvent>,       // 内存索引（启动时全量加载）
    cursor: usize,                   // 指向最后一个「生效」事件
}

impl HistoryStore {
    pub fn open(hub: Option<&Path>, app_data: &Path) -> Result<Self>;
    pub fn push(&mut self, event: HistoryEvent) -> Result<()>;     // 追加并清 cursor 之后
    pub fn pop_undo(&mut self) -> Option<HistoryEvent>;            // 返回需反向应用的事件
    pub fn pop_redo(&mut self) -> Option<HistoryEvent>;            // 重新前进
    pub fn jump_to(&mut self, event_id: &str) -> Vec<HistoryEvent>; // 返回区间内待撤销事件
    pub fn list(&self, limit: usize, before_id: Option<&str>) -> Vec<&HistoryEvent>;
}
```

存放在 `AppState` 中，用 `Arc<Mutex<HistoryStore>>` 包裹。

### 4.2 `storage.rs` 改动

每个写入函数加 `before` 信息回返，由 `commands.rs` 组装 HistoryEvent 后 `history_store.push(...)`：

```rust
// before
pub fn toggle_task(file: &Path, line: u32) -> Result<()>;

// after — 返回旧的 done 状态供组事件
pub fn toggle_task(file: &Path, line: u32) -> Result<ToggleResult> {
    // ToggleResult { before_done: bool, text: String, line_hash: String }
}
```

类似 `update_task_text` 返回 `before_text + hash`，`append_task` 返回行号，`remove_task_line` 配合 `append_task_to_quadrant` 时分别返回信息让上层组合成 move 事件。

### 4.3 `watcher.rs` 改动

watcher 现有逻辑里有 `IgnoreHashes` 防回写循环。**已经做了 App vs 外部的区分**：
- App 写入时：把新 hash 加入 `IgnoreHashes`，watcher 检测到忽略
- watcher 检测到变化但**不在** `IgnoreHashes` 里 = 外部编辑 → 新分支：调 `history_store.push(ExternalEdit { diff_summary })`

计算 diff_summary 用 `similar` crate（轻量）或纯 line-count（先简后精）。

### 4.4 `commands.rs` 新增 IPC

```rust
#[tauri::command] async fn get_history(limit: usize, before_id: Option<String>) -> Vec<HistoryEvent>;
#[tauri::command] async fn undo() -> Result<Option<HistoryEvent>>;       // 返回被撤销的事件用于 UI 反馈
#[tauri::command] async fn redo() -> Result<Option<HistoryEvent>>;
#[tauri::command] async fn jump_to(event_id: String, confirm_external: bool) -> Result<JumpResult>;
//                                                       ^ 若区间内有 external_edit，前端必须先弹窗拿 true 才执行
#[tauri::command] async fn open_history_window() -> Result<()>;
```

`JumpResult { undone_count: usize, skipped_external: usize }`。

### 4.5 `error.rs` 新增

```rust
HistoryHashMismatch { event_id, file, line }   // 撤销时 hash 不对
HistoryFileMissing                              // 源文件被删
HistoryDisabled                                 // hub 未设且 app_data 写不进
ExternalInUndoRange                             // jump_to 时检测到外部编辑且 confirm_external=false
```

## 5. Tauri 多窗口

`tauri.conf.json` 加第二个窗口配置：

```jsonc
{
  "windows": [
    { "label":"main", /* 现有配置 */ },
    {
      "label":"history",
      "title":"Floaty Todo — History",
      "width":680, "height":520,
      "resizable":true, "visible":false,        // 默认不显示，由 open_history_window 创建/显示
      "decorations":true, "alwaysOnTop":false   // 与主窗对比：有标题栏、不置顶
    }
  ]
}
```

`open_history_window` 命令：若 label="history" 的窗口已存在 → focus；不存在 → 用 Tauri builder 动态创建。

主窗口和历史窗口共享 `AppState`（同一进程同一 store）。

## 6. 前端组件 / 状态

### 6.1 新文件

| 路径 | 职责 |
|---|---|
| `src/types/history.ts` | `HistoryEvent` / `EventKind` 接口（与 Rust 对齐） |
| `src/stores/history.ts` | `useHistoryStore` —— `events` / `cursorIndex` / `loading`；`refresh` / `undo` / `redo` / `jumpTo`；订阅 `history-updated` Tauri 事件做实时同步 |
| `src/views/HistoryView.vue` | 历史窗口的根组件（仅在 history label 窗口渲染） |
| `src/components/history/EventList.vue` | 左侧事件流：按天分组、source 色块、外部编辑半透明、cursor 之后的事件灰显（已撤销） |
| `src/components/history/EventDetail.vue` | 右侧 diff 详情：用 `inline-md.ts` 渲染 before/after，highlight 差异 |
| `src/components/history/JumpButton.vue` | 「跳到此处」按钮 + 二次确认对话框（区间含外部编辑时） |

### 6.2 主窗口改动

| 文件 | 改动 |
|---|---|
| `src/components/TaskList.vue` | footer 加 🕒 按钮（调 `api.openHistoryWindow()`） |
| `src/main.ts` | 全局 `keydown` 监听 Ctrl+Z / Ctrl+Y / Ctrl+H |
| （v1 不改 SettingsView）— 历史功能默认开启，没有用户可配置项 | |

### 6.3 路由

App 启动时检查窗口 label（Tauri API），label="main" 渲染 `<App />`，label="history" 渲染 `<HistoryView />`。

## 7. 算法

### 7.1 单步撤销

```
event = history.pop_undo()
if event is None: return                          # 栈底
if event.kind == "external_edit": skip + 提示     # 实际不会进 undo 栈，仅时间线
verify hash(file[line]) == event.before_hash
if mismatch: 
    return Err(HashMismatch)                      # 严格拒绝
apply_reverse(event)                              # 调用 storage.rs 对应反向
emit "tasks-updated" + "history-updated"
```

### 7.2 跳到此处（双向）

```
target_idx = events.index(event_id)
if target_idx < cursor:                            # 向后跳 = 多步撤销
    range = events[target_idx+1 .. cursor]
    direction = "undo"
elif target_idx > cursor:                          # 向前跳 = 多步重做
    range = events[cursor+1 .. target_idx]
    direction = "redo"
else:
    return Ok(0)                                   # 已在此处

external_count = count(e.kind == "external_edit" for e in range)
if external_count > 0 and not confirm_external:
    return Err(ExternalInUndoRange { count, direction })

iter = range.reverse() if direction == "undo" else range
for e in iter:
    if e.kind == "external_edit": continue
    if direction == "undo": apply_reverse(e)        # hash 校验 before
    else:                   apply_forward(e)        # hash 校验也是 before（旧态）
    # hash mismatch 立即中断 + 提示
cursor = target_idx
```

部分失败语义：**遇到 hash mismatch 立即停**，返回「成功 N / 共 M 个」，保留之前的状态。

### 7.3 重做

```
event = history.pop_redo()                        # cursor + 1 位置
verify hash(file[line]) == event.before_hash      # 同样校验 before
apply_forward(event)
```

## 8. 外部编辑检测细节

`watcher.rs` 收到 fs event → 计算文件新 hash → 是否在 `IgnoreHashes`？

- **是** → 是 App 自己刚写的，忽略
- **否** → 外部编辑：
  1. 读旧内容（从内存 registry 或 git index 拿）
  2. 算 diff_summary（added / removed / modified lines）
  3. 调 `history_store.push(ExternalEdit { ... })`
  4. 触发 `tasks-updated` + `history-updated`

**节流**：500ms 内同文件多次外部编辑合并为一条事件（避免编辑器保存抖动产生多条）。

## 9. UI 流程

### 9.1 主窗口键位

| 快捷键 | 动作 |
|---|---|
| Ctrl+Z | undo |
| Ctrl+Y / Ctrl+Shift+Z | redo |
| Ctrl+H | toggle 历史窗口 |

撤销时主窗口 toast：「↶ 撤销：完成「修临床方案」」（2 秒消失）

### 9.2 历史窗口

打开后默认滚到「现在」位置（cursor 处）。左侧列表项：

```
[14:32] ☑ 完成「修临床方案」              [Notes]
[14:15] ✎ 编辑「论文修订标题」            [IDS]
[13:42] 📝 VS Code 修改 TODO.md (+3/-1)  [每日]  (半透明)
[13:01] + 新增「OCR 任务」                [Notes]
─── 昨天 ───
...
```

cursor 之后的事件（已撤销但可 redo）灰显 + `(已撤销)` 标签。

点击事件 → 右侧加载 diff 详情 + 「↶ 跳到此处」按钮（如果是 cursor 之后则按钮变 「↷ 重做到此处」）。

### 9.3 状态指示

主窗口 footer 增加：
- 🕒 按钮（点开历史窗口；hover 显示「最近：完成 X · 2 分钟前」）
- 如果当前有 redo 可用，🕒 旁边显示 ↷ 小图标

## 10. 边界与错误处理

| 场景 | 行为 |
|---|---|
| 撤销时文件不存在 | 弹 toast「源文件已删除，无法撤销」，事件保留在 history 但跳过 |
| 撤销时 hash mismatch | 严格拒绝 + 弹窗解释「该行已被外部修改」，提供「在历史窗口看 diff」按钮 |
| Hub 未设置 + AppData 也不可写 | 历史功能 disable，主窗口 footer 🕒 灰显，hover 提示「请配置 Hub 文件夹」 |
| history.jsonl 损坏（某行无法解析） | 跳过坏行 + log warn，继续加载后续 |
| 跨 source 撤销 | 不限制（事件按时间排序，反向应用时按事件内 source 路由） |
| 同 task 短时间多次 edit | 不合并（每次都是独立事件，撤销也分步） |
| App 启动时 history.jsonl 很大（10MB+） | 流式读取 + 内存只保留最近 N=10000 条索引；超出按需读 |

## 11. 测试

### 11.1 Rust 单元测试

- `history::push` / `pop_undo` / `pop_redo` 状态机
- `jump_to` 不同 cursor 位置 + 含/不含 external_edit 的区间
- hash mismatch 时撤销正确拒绝
- 损坏 jsonl 行的容错

### 11.2 集成测试

- toggle → undo → 验证文件回到原内容
- edit → undo → redo → 验证最终态等于 edit 后
- move quadrant → undo → 文件结构精确还原（行号 / header 顺序）
- external edit 后 undo 上一个 App 动作 → hash mismatch 正确拒绝

### 11.3 手动验证清单

- [ ] Ctrl+Z / Ctrl+Y / Ctrl+H 三个键位在主窗口与历史窗口都生效
- [ ] 历史窗口 source 色块与主窗口配色一致
- [ ] 外部编辑在 VS Code 改后 2 秒内出现在时间线
- [ ] 「跳到此处」遇外部编辑弹警告，确认后正确跳过
- [ ] Hub 切换后旧 history 不可见（不迁移）
- [ ] 历史窗口关闭后再打开，光标位置 / 滚动位置不重置

## 12. 开放问题

1. **diff_summary 精度**：用纯行数（快）还是 `similar` 算 word-level diff（更准但慢）？建议先纯行数，后续按需精化。
2. **历史窗口是否支持搜索**：v1 不做，v2 看反馈。
3. **多机协作场景**：如果用户从多台机器编辑同一 Hub（通过 git 同步），事件 ULID 全局唯一但 cursor 不同步，会出现「在 A 机撤销但 B 机不知道」。**v1 不处理**，cursor 留本机。

## 13. 实现顺序建议

1. **阶段 1（核心）**：`history.rs` + `storage.rs` 改造 + 主窗口 Ctrl+Z/Y。一周。
2. **阶段 2（UI）**：历史窗口 + EventList / EventDetail / JumpButton。一周。
3. **阶段 3（外部编辑）**：watcher.rs 接入 + diff_summary + 时间线显示。3 天。
4. **阶段 4（打磨）**：toast 反馈、错误处理、测试补全。3 天。

每个阶段独立可发布。

---

## 修订记录

- 2026-05-15：v0 初稿（serenNan + Claude）
