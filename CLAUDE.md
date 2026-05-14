# Floaty Todo — 项目说明

## 技术栈

- **前端：** Vue 3 + TypeScript、Vite 6、Pinia、vue-i18n（en/zh）
- **后端：** Tauri 2（Rust）、`tauri-plugin-dialog`（文件选择器）
- **包管理：** npm
- **Identifier：** `com.serendipity.floaty-todo`

## 目录结构

```
src/              # Vue 前端
src-tauri/        # Rust 后端
  src/lib.rs      # App 初始化、托盘、watcher 调度 + invoke_handler 注册
  src/main.rs     # 入口（调用 floaty_todo_lib::run()）
  src/commands.rs # Tauri IPC 命令 + AppState（registry/config/watcher 粘合层）
  src/types.rs    # Task / AppConfig / ContentHash
  src/error.rs    # AppError（thiserror）+ Tauri 用的 Serialize 实现
  src/parser.rs   # Markdown 任务解析（parse_line / parse_file）
  src/storage.rs  # 原子文件写入：toggle_task / append_task
  src/config.rs   # AppConfig 加载/保存（JSON，容错）
  src/registry.rs # 内存 TaskRegistry（按 source 扫描 + 按文件刷新）
  src/watcher.rs  # 防抖 fs watcher（每个 source 一个）+ IgnoreHashes 防回写循环
  src/shell.rs    # 外部进程启动器（VS Code / 终端），含平台级联兜底
  src/hub.rs      # Hub 目录通过 hard-link / NTFS junction 做镜像
  tauri.conf.json # App 配置（productName / identifier / devUrl / window 340×520 transparent decorations:false skipTaskbar alwaysOnTop）
  Cargo.toml      # Rust 依赖（crate 名 floaty-todo，lib 名 floaty_todo_lib）
```

## 数据模型

多 source 聚合（v0.2）：用户可配置 **N 个任务源**，每个是：
- **Folder source** — 递归扫描 `path` 下所有 `.md`
- **File source** — 单个 `.md` 文件（watcher 监听父目录，按文件名过滤）

每个 `Source` 有 `id`（canonical 路径的 8 字节 hex sha256）、`path`、`kind`、可选 `label`、可选 `project_root`（被 `open_in_vscode` / `open_in_terminal` 使用；默认值：Folder → `path`，File → `path.parent()`）、可选 `color`（hex `#xxx[xxx[xx]]`，Rust `update_source` 正则校验；UI 用作卡片左条 + header 色相）。

任务通过 `Task.source_id` 关联到 source。registry 用 `(source_id, canonical_path)` 做键，所以同一个文件出现在两个 source 下也互相独立。

**Hub 目录（可选）：** `AppConfig.hub_folder` 指向一个中心目录，所有 source 镜像到这里 —— File source 用 `std::fs::hard_link`（同 inode = 真双向同步），Folder source 在 Windows 上用 NTFS junction（`cmd mklink /J`），其它平台用 POSIX symlink。Hub 侧文件名从 source label 派生（已清洗）。只支持同卷；跨卷镜像会失败并报错，但不影响 source CRUD。

## Rust 模块

| 模块 | 职责 |
|---|---|
| `commands` | `AppState` + Tauri 命令：`get_tasks` / `toggle_task` / `update_task` / `add_task`、source CRUD（`list_sources` / `add_source` / `remove_source` / `update_source` / `reorder_sources` / `set_default_source`）、单文件 label 覆盖（`set_file_label`）、快捷动作（`open_in_vscode` / `open_in_terminal` / `open_in_claude_code` / `run_quick_action` / `set_enabled_quick_actions` / `open_url`）、hub（`set_hub_folder` / `resync_hub`）、窗口控制。`add_source` 在调用者没传 label 时会从 `project_root` 的目录名推断默认 label；source CRUD 也会调 `hub` 维护镜像项 |
| `hub` | Hub 镜像引擎：`mirror_path_for(hub, source)` 派生清洗后的 link 名；`create_mirror` / `remove_mirror` / `sync_all` 管理每个镜像项（幂等，会清理孤儿）；File source 用 `std::fs::hard_link`，Folder source 在 Windows 上用 NTFS junction，其它平台用 POSIX symlink |
| `shell` | 副作用启动器：`open_vscode(path)`、`open_terminal(path)`（Windows：wt → pwsh → powershell；macOS：`open -a Terminal`；Linux：x-terminal-emulator → gnome-terminal → konsole → xterm）、`open_claude_code(path)`（Windows：`wt -d <p> -- powershell -NoExit -Command "& '%USERPROFILE%\.local\bin\claude.exe' --dangerously-skip-permissions"` —— 用绝对路径是因为 cmd 的 PATH 一般没有 `~/.local/bin`；macOS：osascript 调起 Terminal.app；Linux：terminal-emulator `-e claude`）、`reveal_in_explorer(path)`（Windows：`explorer.exe /select,<file>` 处理文件；macOS：`open -R` 处理文件；Linux：`xdg-open` 父目录）、`open_url(url)`（系统默认浏览器） |
| `registry` | `TaskRegistry` 以 `(source_id, canonical_path)` 为键；`rebuild_from_sources` / `rebuild_source` / `refresh_file(source, file)` |
| `watcher` | `start_watching_source`（Folder = 递归；File = 监听父目录 + 文件名过滤）+ `IgnoreHashes` 防写回循环；每个 source 在 `WatcherSlots: Arc<Mutex<HashMap<source_id, WatcherHandle>>>` 里一个 `WatcherHandle` |
| `storage` | `toggle_task` / `update_task_text` / `append_task` —— 通过 `tempfile::NamedTempFile` 原子写。`update_task_text` 逐字节保留行前的缩进 / bullet / checkbox 前缀；拒绝空文本与多行输入 |
| `config` | `load_from` / `save_to` / `config_file` —— JSON，损坏容忍 |
| `parser` | `parse_line` / `parse_file(path, source_id)` —— 正则，稳定的 SHA-256 任务 ID |
| `types` | `Task`（带 `source_id`）、`Source` / `SourceKind`（Folder / File）、`QuickActionKind`（Vscode / Terminal / ClaudeCode / Reveal）、`AppConfig`（`sources` + `default_source_id` + `file_labels` + `enabled_quick_actions` + `hub_folder`）、`ContentHash`、`file_label_key()`、`default_quick_actions()` |
| `error` | `AppError`（Io / Json / Watcher / NoSources / SourceNotFound / DuplicateSource / InvalidSourcePath / TaskNotFound / NotATaskLine / CommandFailed） |

## 前端模块

| 模块 | 职责 |
|---|---|
| `src/types/task.ts` | `Task` / `Source` / `SourceKind` / `AppConfig` TS 接口（与 Rust 对齐） |
| `src/services/tauri-api.ts` | `api` 对象 —— 封装 `invoke` 命令 + 对话框选择器（`pickFolder` / `pickMarkdownFile`）+ 事件监听器（`tasks-updated` / `sources-changed` / `request-manage-sources`） |
| `src/stores/tasks.ts` | `useTaskStore` —— `tasks` / `sortedTasks` / `loading` / `error`；`refresh` / `silentRefresh` / `toggle` / `update(id, text)` / `add(text, sourceId?)` |
| `src/stores/settings.ts` | `useSettingsStore` —— `config` / `sources` / `hasSources` / `defaultSourceId` / `fileLabels` / `enabledQuickActions` / `alwaysOnTop` / `hubFolder` / `scanningSourceIds`；CRUD：`addSource` / `removeSource` / `updateSource` / `setDefaultSource` / `reorderSources` / `setFileLabel` / `setEnabledQuickActions` / `setAlwaysOnTop` / `toggleAlwaysOnTop` / `setHubFolder` / `resyncHub`；选择器 `pickAndAddFolder` / `pickAndAddFile` / `pickAndSetHubFolder`；`markScanning(id, on)` 切换扫描状态 |
| `src/main.ts` | App 入口 —— 装配 `createPinia()` + i18n 后挂载 `App` |
| `src/i18n/` | `vue-i18n` 配置 + `locales/en.ts` / `locales/zh.ts`；`setLocale()` 持久化到 localStorage `floaty.locale` 并同步 `<html lang>` |
| `src/composables/useTheme.ts` | 主题 composable —— `currentTheme` / `effectiveTheme` / `setTheme`；localStorage `floaty.theme`，监听系统配色 media query |
| `src/composables/useConfirm.ts` | 单例 `confirm({ title, message, danger, … }) → Promise<boolean>`，应用内确认弹窗 |
| `src/composables/useSourceDrag.ts` | Pointer-events 版 source 拖拽状态（Tauri WebView2 不发 HTML5 native `dragover`，所以绕开 `draggable="true"`）。模块级 `draggedSourceId` / `dropTargetSourceId` ref + `startSourceDrag({ e, sourceId, onClick, onDrop })` 入口：挂 document 级 pointermove/up，通过 `elementFromPoint` 查目标元素再向上爬 DOM 找 `[data-source-id]` |
| `src/composables/useTaskEditor.ts` | 单例 `editTask(task) → Promise<string \| null>` API，驱动任务文本编辑模态；返回 null 表示取消 / 无改动 |
| `src/components/TaskEditorDialog.vue` | 由 `useTaskEditor` 驱动的 Teleport 模态：显示「文件·行号」副标题、单行输入；Enter 保存 / Esc 取消；在 App 根处挂一次，与 `<ConfirmDialog />` 同级 |
| `src/composables/useCollapse.ts` | 计数器风格的全局触发器 —— `collapseAll()` / `expandAll()` 自增 token；组件内 `bindCollapse(setter)` 监听这两个值，让 SourceGroup / FileGroup 翻转自身的 `collapsed` ref |
| `src/components/ConfirmDialog.vue` | 由 `useConfirm` 驱动的 Teleport 模态；点遮罩 / Esc 取消，confirm 按钮 focus-trap，危险动作有红色变体 |
| `src/utils/inline-md.ts` | 零依赖 inline-only Markdown 解析器 → `InlineSegment[]`（text / code / bold / italic / strike / link）；`TaskItem` 用它安全渲染任务文本，不走 v-html |
| `src/utils/colors.ts` | source 强调色配套：`SOURCE_COLORS`（9 色 Tailwind-500 调色板）+ `safeHexColor(c)` hex 校验（与 Rust 端规则对齐，防止任意 CSS 注入） |
| `src/views/SettingsView.vue` | 全屏设置页 —— 外观（主题分段控件）、语言（locale 下拉）、快捷动作（按类型开关）、Hub 目录（选择 / 重同步 / 更改 / 关闭）、Sources（卡片 + ⎘ / ▷ / 📝 / 🗑 + 内嵌编辑器）、关于；emit `back` |
| `src/components/SourceGroup.vue` | 可折叠的单 source 组：点 header 任意处切换折叠；header 带 `data-source-id` 供拖拽目标识别。Header 含 caret + 类型 emoji（折叠时翻转）+ label + 默认徽章 + 扫描旋转图标 + 数量 + 可拖排序的品牌色 `QuickActionIcon` 按钮 + ⚙ 设置按钮（单击切换内嵌编辑器：label / project_root / set-default / remove）+ ⋮⋮ grip 拖拽手柄（**只做拖拽**，单击无动作；通过 `useSourceDrag` 重排 source）。Folder source 按 `source_file` 分桶渲染嵌套 `FileGroup`（任务数 > 50 时初始全部折叠）；File source 直接渲染 TaskItem。订阅 `useCollapse` 响应全局「Collapse all」 |
| `src/components/icons/QuickActionIcon.vue` | 三个快捷动作的品牌色 inline SVG（VS Code / Terminal / Claude Code）；零依赖，适配深浅色 |
| `src/components/icons/Icon.vue` | 中心化 SVG 图标库；`name: IconName` 字符串字面量联合 → 21 个 Lucide 风格的图标（settings / refresh / chevron-* / pencil / folder / file / trash / sun / moon / monitor / arrow-left / loader / more-horizontal / grip-vertical / collapse-all / expand-all 等）；非品牌标记的所有图标都来自这里 |
| `src/components/FileGroup.vue` | `SourceGroup` 内的单文件子组：独立可折叠、悬停才显示的 ✎ 重命名按钮、内嵌重命名输入（Enter / Esc / ↺ 重置）；没自定义 label 时落到文件在 source 内的相对路径 |
| `src/components/TaskList.vue` | 分组任务视图（按 config 顺序逐个渲染 `SourceGroup`）；QuickAdd 输入框 + 单任务 source 下拉；footer 左下角 ⚙ Settings + 计数 + ↻ 刷新 |
| `src/components/EmptyState.vue` | 首次进入落地页：📁 Folder / 📄 File 选择器按钮 + 左下角 ⚙ Settings 角标按钮 |

## 构建命令

```powershell
npm run tauri dev    # 开发模式（Vite + cargo run）
npm run tauri build  # 生产构建
```

## 关键注意点

- `src-tauri/src/main.rs` 调的是 `floaty_todo_lib::run()` —— lib crate 名是 `floaty_todo_lib`（下划线，不是连字符）
- Dev URL 是 `http://localhost:1420`（在 `tauri.conf.json` 配置）
- `node_modules/` 和 `src-tauri/target/` 已 gitignore
