# Floaty Todo —— 一个轻量级 Obsidian Todo 桌面悬浮窗

一个常驻系统托盘的桌面应用，读取你指定文件夹下的 `.md` 文件，把所有 `- [ ]` 任务聚合成一个可勾选的待办列表。文件即数据，AI 可直接读写，应用监听变化自动刷新。

> 本文档基于初版 PLAN.pdf（2026-05-14 微信收到），结合开放问题决策与若干补充设计点修订。后续以本文为准。

---

## 一、设计哲学

1. **文件即数据库**：不引入私有格式、不引入私有存储。markdown 文件就是真相之源 (single source of truth)。
2. **应用是视图层**：应用本身不存储任务数据，只负责扫描、解析、展示、回写。
3. **AI 友好**：AI 用任何方式改 markdown（手改、Claude Code、未来的 todo skill），应用都能感知并刷新。
4. **可扩展但不过度设计**：当前只支持 `- [ ]` 简单格式，但解析层留好接口，未来加 Obsidian Tasks 格式（emoji 元数据）不需要重构。
5. **轻量优先**：启动 < 1 秒、内存 < 100MB、安装包 < 10MB。
6. **绝不破坏 line_number 稳定性**：任何会让任务行漂移的操作（跨文件移动、自动归档插段）都不做。修改一律行内原地改。

### 非目标

- ❌ 不做云同步（Obsidian 自己有，或用户用 OneDrive/iCloud/Syncthing）
- ❌ 不做完整 markdown 渲染（只渲染任务行）
- ❌ 不做项目管理/看板/甘特图等重型功能
- ❌ 不取代 Obsidian（应用本身不编辑笔记内容，只编辑 checkbox 状态）

---

## 二、技术栈

| 层 | 选型 | 理由 |
| --- | --- | --- |
| 框架 | **Tauri 2.x** | 原生应用、包小（~5MB）、启动快、Rust 后端可以做高性能文件监听 |
| 前端 | **Vue 3 + TypeScript** | 用户熟、生态成熟、Composition API 写起来清爽 |
| 状态管理 | **Pinia** | Vue 官方推荐、轻量 |
| UI 库 | **不引入 UI 库，自己写** | 任务列表 UI 简单，避免引入 Element/Antd 等臃肿依赖 |
| 样式 | **CSS Variables + 原生 CSS** | 不引入 Tailwind 避免学习成本 |
| 构建 | **Vite** | Tauri 默认搭配 |
| 后端语言 | **Rust** | Tauri 默认 |
| 文件监听 | **notify crate** | Rust 生态最稳的跨平台 fs watcher |
| Markdown 解析 | **手写正则** | 只解析 `- [ ]` / `- [x]`，引入完整 markdown parser 是杀鸡用牛刀 |

### 技术栈决策记录

- **Vue vs Svelte 5 / SolidJS**：Svelte 5（runes）和 SolidJS 编译时无 vDOM、运行时更小，对"轻量悬浮窗"目标其实更贴合。但 PDF 已定 Vue（用户熟），且 Vue 3 + Composition API 对这个量级（< 200 行 UI）完全够用，不为换而换。如果未来包体积或冷启动成为瓶颈，再考虑切换。
- **Pinia vs 纯 composables**：Pinia 对单 store 应用稍重，但用户已熟 + DevTools 调试方便，保留。
- **Tailwind 不引入**：UI 极简（< 10 个组件），CSS Variables 已足够主题化。

---

## 三、系统架构

```
┌─────────────────────────────────────────────────────────┐
│                  Vue 前端 (WebView)                       │
│  ┌────────────┐  ┌────────────┐  ┌──────────────────┐   │
│  │  TaskList  │  │  QuickAdd  │  │     Settings     │   │
│  └─────┬──────┘  └─────┬──────┘  └────────┬─────────┘   │
│        │               │                    │             │
│        └───────────┬───┴──────────────────┘             │
│                    ▼                                      │
│              ┌──────────┐                                 │
│              │  Pinia   │  ← stores/tasks.ts              │
│              │  Store   │    stores/settings.ts           │
│              └────┬─────┘                                 │
│                   │ invoke() / event listen               │
└───────────────────┼──────────────────────────────────────┘
                    │ Tauri IPC bridge
┌───────────────────┼──────────────────────────────────────┐
│                   ▼                  Rust 后端             │
│         ┌──────────────────┐                              │
│         │   Commands API   │  ← src-tauri/src/commands.rs │
│         └────────┬─────────┘                              │
│                  │                                          │
│       ┌──────────────┼───────────────┬─────────────┐       │
│       ▼              ▼               ▼             ▼       │
│   ┌──────┐      ┌─────────┐     ┌──────────┐  ┌──────────┐│
│   │Parser│      │ Storage │     │ Watcher  │  │ Registry ││
│   │      │      │         │     │          │  │(任务索引) ││
│   └──────┘      └────┬────┘     └────┬─────┘  └──────────┘│
│                      │                │                    │
│                      ▼                ▼                    │
│              ┌─────────────────────────┐                  │
│              │ 文件系统：vault 文件夹    │                  │
│              │ *.md 文件 + sidecar     │                  │
│              └─────────────────────────┘                  │
└──────────────────────────────────────────────────────────┘
                            ▲
                            │ 直接读写
                ┌───────────┴────────────┐
                │  Claude Code / Codex   │
                │  (未来：todo skill)     │
                └────────────────────────┘
```

---

## 四、目录结构

```
floaty-todo/
├── src/                          # Vue 前端
│   ├── main.ts
│   ├── App.vue
│   ├── components/
│   │   ├── TaskList.vue          # 主任务列表
│   │   ├── TaskItem.vue          # 单条任务行
│   │   ├── QuickAdd.vue          # 快速添加输入框
│   │   ├── Settings.vue          # 设置面板
│   │   └── EmptyState.vue        # 首次启动 / 无 vault 引导卡片
│   ├── stores/
│   │   ├── tasks.ts              # 任务列表状态
│   │   └── settings.ts           # 用户配置
│   ├── services/
│   │   └── tauri-api.ts          # 封装所有 invoke 调用
│   ├── types/
│   │   └── task.ts               # TS 类型定义
│   └── styles/
│       └── main.css
│
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── main.rs               # 入口，注册 commands、托盘、快捷键
│   │   ├── commands.rs           # Tauri commands（前端调用入口）
│   │   ├── parser.rs             # markdown 解析（行 → Task）
│   │   ├── storage.rs            # 文件读写（含精准行级回写）
│   │   ├── watcher.rs            # 文件变化监听 + 防回环
│   │   ├── registry.rs           # 任务索引（内存 cache）
│   │   ├── sidecar.rs            # sidecar 元数据管理（completed_at 等）
│   │   ├── config.rs             # 配置读写
│   │   ├── types.rs              # Rust 数据结构定义
│   │   └── error.rs              # 统一错误类型
│   ├── tauri.conf.json
│   ├── Cargo.toml
│   └── icons/
│
├── package.json
├── vite.config.ts
├── tsconfig.json
├── README.md
└── PLAN.md                       # 本文档
```

---

## 五、核心数据模型

### Task（Rust 端）

```rust
// src-tauri/src/types.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,                  // hash(file_path + line_number)，稳定标识
    pub text: String,                // 任务文本（不含 "- [ ] " 前缀）
    pub completed: bool,
    pub source_file: PathBuf,        // 绝对路径
    pub line_number: usize,          // 1-indexed，方便定位
    pub indent: usize,               // 缩进空格数，用于显示层级
    pub completed_at: Option<String>,// ISO 8601，从 sidecar 来
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub vault_path: Option<PathBuf>, // 用户指定的扫描文件夹（首启时为空）
    pub inbox_file: String,          // 默认 "inbox.md"，快速添加目标
    pub hotkey_toggle: String,       // 默认 "Ctrl+Shift+T"
    pub hotkey_quick_add: String,    // 默认 "Ctrl+Shift+A"
    pub always_on_top: bool,         // 默认 true
    pub show_completed_today: bool,  // 默认 true（划线灰显）
    pub show_archived: bool,         // 默认 false（隐藏昨天之前完成的）
}
```

### Sidecar 元数据

`.floaty-todo.json` 存放在 vault 根目录（应用唯一会写的非 `.md` 文件）：

```json
{
  "version": 1,
  "tasks": {
    "<task_id>": {
      "completed_at": "2026-05-14T10:23:00+08:00"
    }
  }
}
```

**为什么用 sidecar？** 简单 `- [x]` 格式不带时间戳，要支持"当天完成划线变灰、隔天归档"必须额外存。放 sidecar 不污染笔记。AI 改文件后应用通过 watcher 检测 `[ ] → [x]` 变化时自动写入 sidecar。

**容错：** sidecar 丢了不致命——所有已完成任务被视为"很久之前完成"，按 `show_archived` 设置决定显示。下次任意任务勾选会自动重建 sidecar。

---

## 六、模块职责详解

### 1. `parser.rs` —— Markdown 任务解析

**职责**：把一个 `.md` 文件解析成 `Vec<Task>`。

**核心正则**（v0.1 只支持简单格式）：

```rust
// 匹配 "- [ ] xxx" 或 "- [x] xxx" 或 "* [ ] xxx" 等
static TASK_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\s*)[-*+]\s+\[([ xX])\]\s+(.+)$").unwrap()
});
```

**接口：**

```rust
pub fn parse_file(path: &Path) -> Result<Vec<Task>>;
pub fn parse_line(line: &str, line_num: usize) -> Option<ParsedTask>;
```

**编码处理：** UTF-8 假设；带 BOM 自动剥；行尾保持原文件风格（CRLF/LF 不强转）。解析失败的文件跳过 + 记日志，不 panic。

**扩展点：** 未来支持 Obsidian Tasks 的 emoji 元数据时，在 `parse_line` 内追加解析逻辑，外部接口不变。

### 2. `storage.rs` —— 精准回写

**关键约束：绝对不能整个文件 rewrite，必须按行号只改目标那一行。**

```rust
pub fn toggle_task(file: &Path, line_num: usize) -> Result<()>;
pub fn append_task(file: &Path, text: &str) -> Result<()>;          // 追加到文件末尾
pub fn update_task_text(file: &Path, line_num: usize, new_text: &str) -> Result<()>;
```

**写入策略：** 读全文 → 改目标行 → 写临时文件 → atomic rename。避免并发读到半完成状态。

**防回环：** 写入前算新内容 SHA-256，注册到 watcher 的"忽略 hash 集合"。watcher 收到事件时若文件 hash 命中集合则丢弃事件并清除该 hash。比单纯时间窗口可靠（HDD fsync 可能慢于 500ms）。

### 3. `watcher.rs` —— 文件监听

**职责：** 监听 vault 文件夹下所有 `.md` 文件的变化，触发重新解析。

```rust
pub fn start_watching(vault_path: &Path, tx: Sender<WatchEvent>);

pub enum WatchEvent {
    FileChanged(PathBuf),
    FileCreated(PathBuf),
    FileDeleted(PathBuf),
}
```

**实现细节：**

- 用 `notify::RecommendedWatcher` 监听
- debounce 200ms 避免频繁刷新
- 通过 Tauri 的 `app.emit()` 通知前端刷新
- **忽略名单**：`.obsidian/`、`.git/`、`.trash/`、`*.swp`、`*~`、`*.tmp`、Office 锁文件 `~$*`
- **防回环**：维护 `HashSet<ContentHash>`，storage 写入注册的 hash 命中时丢弃

### 4. `registry.rs` —— 内存索引

**职责：** 保持一份内存里的任务列表（避免每次前端请求都重新扫描整个文件夹）。

```rust
pub struct TaskRegistry {
    tasks: HashMap<String, Task>,             // id → Task
    by_file: HashMap<PathBuf, Vec<String>>,   // 文件 → 任务 id 列表
}

impl TaskRegistry {
    pub fn rebuild_from_vault(&mut self, vault_path: &Path) -> Result<()>;
    pub fn refresh_file(&mut self, file: &Path) -> Result<()>;
    pub fn all_tasks(&self) -> Vec<&Task>;
    pub fn pending_tasks(&self) -> Vec<&Task>;
}
```

**并发：** 用 `Arc<RwLock<TaskRegistry>>` 在 commands / watcher 间共享。

**异步启动：** `rebuild_from_vault` 必须在后台线程跑——主窗口先显示骨架屏，扫描完成再 `emit("tasks-ready")`。1000+ 文件的 vault 同步扫描会卡启动。

### 5. `commands.rs` —— 前端调用入口

```rust
#[tauri::command] async fn get_tasks() -> Result<Vec<Task>>;
#[tauri::command] async fn toggle_task(task_id: String) -> Result<()>;
#[tauri::command] async fn add_task(text: String) -> Result<()>;
#[tauri::command] async fn get_config() -> Result<AppConfig>;
#[tauri::command] async fn update_config(config: AppConfig) -> Result<()>;
#[tauri::command] async fn pick_vault_folder() -> Result<PathBuf>;       // 弹原生文件夹选择
#[tauri::command] async fn reveal_in_obsidian(task_id: String) -> Result<()>; // obsidian:// URL
```

### 6. `main.rs` —— 应用入口

负责：

- 初始化 Tauri app
- 注册 commands
- 创建托盘图标和菜单
- 注册全局快捷键（用 `tauri-plugin-global-shortcut`）
- 启动 watcher 线程
- 配置主窗口（无边框、always-on-top、初始隐藏）

---

## 七、关键交互流程

### 流程 A：启动应用

1. 读取 `config.json`（不存在则用默认值，`vault_path = None`）
2. 若 `vault_path` 为空 → 主窗口显示 `EmptyState` 卡片（"选择你的 Obsidian vault 文件夹"按钮）
3. 否则启动 watcher 监听 vault_path
4. 异步 `registry.rebuild_from_vault()`（前端显示骨架屏）
5. 注册全局快捷键
6. 创建托盘图标
7. 主窗口默认隐藏（按快捷键或点托盘才显示）

### 流程 B：用户勾选任务

```
前端 TaskItem 点击勾选
  → invoke("toggle_task", { id })
  → commands::toggle_task
    → registry 查 id 对应的 file + line_number
    → storage::toggle_task(file, line)            // 算新内容 hash → 注册到 watcher 忽略集
    → registry.refresh_file(file)                  // 刷新内存
    → sidecar 记录 completed_at（若是完成）
    → app.emit("tasks-updated")                    // 通知前端刷新
```

### 流程 C：AI 在外部改了 markdown

```
Claude Code 修改 vault/work.md，添加 "- [ ] 新任务"
  → 系统触发文件 modified 事件
  → watcher 收到（hash 不在忽略集内）
  → debounce 200ms
  → 触发 registry.refresh_file(file)
  → app.emit("tasks-updated")
  → 前端 store 重新拉取 tasks
  → UI 自动出现新任务
```

### 流程 D：快速添加任务

```
用户按 Ctrl+Shift+A
  → 全局快捷键回调
  → app.emit("show-quick-add") 或直接显示窗口
  → 前端 QuickAdd 组件聚焦输入框
  → 用户输入 "修复登入 bug" 回车
  → invoke("add_task", { text })
  → storage::append_task(inbox_file, text)
    → 若 inbox.md 不存在，自动创建（首行 "# Inbox"）
  → watcher 触发刷新（流程 C）
```

### 流程 E：完成任务的"当天划线 / 隔天隐藏"

```
用户勾选任务 A（或 AI 改成 [x]）
  → sidecar 记录 { task_id_A: completed_at: now }
  → 前端渲染时：
      if !completed                                 → 正常显示
      if completed && completed_at 是今天           → 划线 + 灰色
      if completed && completed_at 不是今天         → 隐藏（show_archived=true 时显示）
      if completed && 没有 completed_at（旧任务）   → 视为很久之前完成，按 show_archived 处理
```

---

## 八、UI 设计

### 主窗口（默认 380×600，无边框，置顶可切换）

```
┌────────────────────────────────────┐
│ Floaty Todo            [─]   [✕]  │ ← 自定义标题栏，[─] 隐藏到托盘
├────────────────────────────────────┤
│ [快速添加任务，回车确认...]    [+]  │ ← 顶部常驻输入框
├────────────────────────────────────┤
│ 📄 work.md                         │ ← 按来源文件分组
│   ☐ 修复登入页面 bug               │
│   ☐ 重构 video player 组件         │
│   ☑ 写周报                         │ ← 当天完成，划线灰显
│                                    │
│ 📄 personal.md                     │
│   ☐ 买咖啡豆                       │
│   ☐ 回邮件给小王                   │
│                                    │
├────────────────────────────────────┤
│ 6 todo · 1 done today    ⚙  ↻    │ ← 底栏：统计 + 设置 + 手动刷新
└────────────────────────────────────┘
```

**自定义标题栏拖动：** 用 `data-tauri-drag-region` 属性挂在标题栏 div 上。

### 首启空状态

```
┌────────────────────────────────────┐
│ Floaty Todo            [─]   [✕]  │
├────────────────────────────────────┤
│                                    │
│         👋 欢迎使用 Floaty Todo     │
│                                    │
│   选择一个 Obsidian vault 文件夹    │
│   应用会自动扫描其中所有 .md 任务   │
│                                    │
│        [  选择文件夹...  ]          │
│                                    │
└────────────────────────────────────┘
```

### 托盘菜单

```
显示窗口
快速添加任务...
─────────────
打开 vault 文件夹
设置...
─────────────
退出
```

---

## 九、迭代里程碑

### v0.1 —— 能跑（目标：1 个周末）

- [ ] Tauri + Vue 工程初始化
- [ ] 配置 vault 路径（首启 EmptyState 引导）
- [ ] 扫描 + 解析 `- [ ]` 任务（异步）
- [ ] 任务列表显示（不分组，纯平铺）
- [ ] 点击勾选/取消勾选（精准回写 + content hash 防回环）
- [ ] 文件变化自动刷新（watcher 跑通 + 忽略名单）
- [ ] 系统托盘 + 窗口显示/隐藏

### v0.2 —— 顺手（1-2 周）

- [ ] 全局快捷键（呼出窗口 / 快速添加）
- [ ] 窗口置顶切换
- [ ] 快速添加输入框（写入 inbox.md，不存在则创建）
- [ ] sidecar 元数据 + 当天完成划线灰显 + 隔天默认隐藏
- [ ] 按来源文件分组显示
- [ ] 自定义标题栏（无原生边框 + 拖动区）

### v0.3 —— 配置化（持续）

- [ ] 完整设置面板（快捷键自定义、置顶切换、显示选项）
- [ ] 搜索/过滤
- [ ] 点击任务 → 在 Obsidian 中打开来源文件（`obsidian://` URL scheme）
- [ ] 子任务缩进显示
- [ ] 父任务勾选时弹 toast 提示"是否一并勾选 N 个子任务"
- [ ] Obsidian Tasks emoji 元数据**只读识别**（不破坏文本，识别 ⏫ / 📅 等并显示标签）

### v1.0 —— 完整（再之后）

- [ ] 单实例多 vault 切换（托盘菜单选）
- [ ] 任务排序自定义
- [ ] 主题（深色/浅色/跟随系统）
- [ ] Obsidian Tasks emoji 完整支持（截止日期排序、优先级显示、循环任务）
- [ ] 显示已归档面板（设置开关 → 看历史完成）

### v2.0 —— AI 强化（可选）

- [ ] 配套 Claude Code skill（todo skill）
- [ ] 可选的 MCP server 模式
- [ ] 任务自动分类建议

---

## 十、AI 集成约定

应用本身**不内嵌 AI**。AI 通过直接读写文件夹下的 `.md` 来管理任务。

### 约定的 todo skill 规范（给未来的 skill 留接口）

未来写 Claude Code skill 时，建议遵守这些规则（应用会按这套规则正确解析）：

1. **添加任务**：追加 `- [ ] 任务文本` 到目标文件末尾（或指定 section 下）
2. **完成任务**：把对应行的 `[ ]` 改成 `[x]`，**不要改动文本其他部分**
3. **删除任务**：删除整行（包括换行符）
4. **修改任务文本**：保持 `- [ ]` 前缀不变，只改文本部分
5. **缩进语义**：缩进的 `- [ ]` 是子任务（v0.3 起支持显示）
6. **不要改的事情**：不要给任务加 emoji 元数据（v0.1 / v0.2 还不支持解析），不要修改文件标题等非任务内容
7. **不要跨文件移动任务**：会破坏 line_number 稳定性，sidecar 会找不到 completed_at

skill 操作完成后应用会自动通过 watcher 感知刷新，不需要任何额外通信。

---

## 十一、工程实践注意点

### 并发与一致性

- watcher 在独立线程，通过 channel 把事件发到主 runtime
- registry 用 `Arc<RwLock<TaskRegistry>>` 保护
- 写文件用临时文件 + atomic rename，避免并发读到半完成状态

### 错误处理

- 所有 IO 错误统一走 `error.rs` 的 `AppError`
- 前端通过 `Result<T, String>` 拿到错误消息，弹 toast 提示
- 文件解析失败（编码问题等）不要 panic，跳过该文件并记日志

### 跨平台路径

- 全部用 `PathBuf`，永远不字符串拼接路径
- WSL 用户：vault 路径用 Windows 风格（如 `D:\Obsidian\Vault`），不要走 `\\wsl$\...`（监听不稳定）

### 防回环写入（修订版）

- storage 写入前算 SHA-256，注册到 `watcher.ignore_hashes: HashSet<[u8; 32]>`
- watcher 收到事件时计算文件 hash，命中则丢弃事件 + 从 set 移除该 hash
- 比时间窗口（500ms）可靠：HDD/网络盘 fsync 可能更慢，且不依赖时钟

### Watcher 忽略名单

```
.obsidian/        # Obsidian 自己的配置 + 缓存
.git/             # git 元数据
.trash/           # Obsidian 删除的文件
*.swp, *~         # vim / nano 临时文件
*.tmp             # 通用临时文件
~$*               # MS Office 锁文件（用户偶尔在 vault 里开 docx）
.floaty-todo.json # 自家 sidecar
```

### 文件编码

- 假设 UTF-8。带 BOM 时读取剥掉，写出去**不带 BOM**
- 行尾：检测原文件用 CRLF 或 LF，按原风格回写（避免无谓 git diff）

### 性能预算

- 单个 vault 假设 < 1000 个 `.md` 文件、< 10000 个任务，全量内存索引完全够用
- 文件变化只重新解析变化的那个文件，不全量重扫
- 启动扫描必须异步——主窗口先显示骨架屏
- 前端虚拟滚动暂不需要

### Tauri 2 注意

- 全局快捷键用 `tauri-plugin-global-shortcut`（v2 已稳定）
- 托盘用 `TrayIconBuilder`（v2 内置）
- 自定义标题栏：HTML 元素加 `data-tauri-drag-region` 即可拖动；按钮记得 `data-tauri-drag-region="false"` 排除

---

## 十二、给 Claude Code 的启动 Prompt（v0.1 骨架）

```
我想用 Tauri 2 + Vue 3 + TypeScript 做一个桌面待办应用，详细规划见 PLAN.md。

请按以下步骤帮我搭建 v0.1 骨架：

1. 初始化 Tauri 工程（npm create tauri-app）
   - 前端用 Vue + TypeScript + Vite
   - 包名 floaty-todo

2. 按照 PLAN.md 第四节"目录结构"创建所有目录和空文件

3. 实现 src-tauri/src/parser.rs：
   - 用 regex crate 解析 `- [ ] xxx` 和 `- [x] xxx`
   - 暴露 parse_file(path) -> Vec<Task>
   - 写单元测试（解析正常行、忽略非任务行、处理缩进、UTF-8 BOM）

4. 实现 src-tauri/src/storage.rs：
   - toggle_task(file, line_num)：只修改指定行的 [ ] ↔ [x]
   - append_task(file, text)：追加 - [ ] text 到文件末尾，文件不存在则创建
   - 写入返回新内容 SHA-256（供 watcher 防回环用）
   - 行尾保持原文件风格
   - 写单元测试

5. 实现 src-tauri/src/registry.rs：
   - 内存维护 HashMap<String, Task>
   - rebuild_from_vault(vault_path) 异步扫描所有 .md 文件
   - refresh_file(file) 单文件刷新

6. 在 src-tauri/src/commands.rs 实现：
   - get_tasks、toggle_task、add_task、pick_vault_folder

7. 前端：
   - Pinia store: stores/tasks.ts（getTasks、toggleTask、addTask）
   - 组件：TaskList.vue（列表）、TaskItem.vue（单项）、EmptyState.vue（无 vault 引导）
   - App.vue 把它们组装起来
   - 暂不做样式，能跑就行

8. 跑起来测试：手动放一个 test.md 到某文件夹，在应用里看到任务、能勾选、能添加

后续步骤（v0.2）：watcher、托盘、全局快捷键、sidecar，等 v0.1 跑通再说。

每完成一步告诉我进展，遇到选择我没说清楚的地方直接问我。
```

---

## 十三、开放问题决策（基于初版 PDF 第 13 节）

### Q1：归档机制

**决策：sidecar 标记 + 显示策略，不动 markdown**

- `[x]` 永远留在原文件不挪位
- sidecar 记 `completed_at`
- UI 渲染：
  - 今天完成 → 划线灰显
  - 今天之前 → 默认隐藏（`show_archived` 设置可切显示）
  - 缺 `completed_at`（兼容旧任务/sidecar 丢失）→ 视为"很久之前"，按 `show_archived` 处理

**理由：** 任何"跨文件移动行"或"自动加 ## Archived section"都会破坏 line_number 稳定性，且违背"应用不污染笔记内容"的克制原则。

### Q2：子任务的完成传播

**决策：v0.1 不传播；v0.3 起父勾子时弹 toast 询问**

- v0.1：单击只改本行，行为完全可预测
- v0.3：勾父任务时弹 toast"是否一并勾选 N 个子任务"，避免静默批量写
- **子→父反向传播彻底不做**："父任务"经常是分组标题（如 `- [ ] 这周计划`），自动勾选会很怪

### Q3：多 vault

**决策：v0.1 单 vault；v1.0 单实例切换（不并行）**

- 悬浮窗本质是"专注当下"，同屏多 vault 反而分散
- 切换走托盘菜单 / 设置下拉，watcher 重建 + registry 重扫即可
- 不做"同时打开多个 vault"——会引入 tab UI、跨 vault 排序、多套快捷键等复杂度

### Q4：Obsidian Tasks emoji 元数据优先级

**决策：v0.3 只读识别；v1.0 完整支持；应用自身添加始终用简单 `- [ ]`**

- 很多 Obsidian 用户已用 Tasks 插件，不解析会让任务文本里出现一堆 ⏫ 📅 2026-05-20 显示很丑
- v0.3 只识别 + 显示成标签徽章，不修改原文本
- v1.0 完整支持（按截止日期排序、优先级颜色、循环任务自动复刻）
- 应用快速添加始终用 `- [ ]`——保持"应用不污染笔记"的克制

---

## 十四、补充设计点（PDF 未覆盖）

| 点 | 决策 |
| --- | --- |
| **首次启动引导** | 主窗口显示 `EmptyState` 卡片 + "选择文件夹"按钮，不弹原生对话框（避免突兀） |
| **inbox.md 不存在** | `add_task` 自动创建，首行写入 `# Inbox`（一行注释，不打扰用户） |
| **防回环机制** | 用 content SHA-256 替代时间窗口（HDD fsync 可能 > 500ms） |
| **watcher 忽略名单** | `.obsidian/`、`.git/`、`.trash/`、`*.swp`、`*~`、`*.tmp`、`~$*`、`.floaty-todo.json` |
| **文件编码** | UTF-8；带 BOM 自动剥；行尾保持原风格（CRLF/LF 不强转，避免 git diff 噪声） |
| **启动性能** | `rebuild_from_vault` 后台异步，前端先显示骨架屏，扫完 emit `tasks-ready` |
| **窗口拖动** | `data-tauri-drag-region` 挂在标题栏 div；按钮加 `="false"` 排除 |
| **sidecar 与 git** | `.floaty-todo.json` 可加进 `.gitignore`（用户决定，应用不强制）；冲突时以本地为准——sidecar 不是真相之源，markdown 才是 |

---

整理完成。准备好就动手吧 🚀
