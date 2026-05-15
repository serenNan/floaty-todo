# 变更日志

## 2026-05-15 搭建 Vitest 前端测试框架

- `npm install -D vitest`：添加 vitest 作为前端单测运行器（项目首个前端测试框架）
- 新增 `vitest.config.ts`：独立配置（不复用 vite.config.ts），test include
  pattern `src/**/*.test.ts`，环境 node
- `package.json` 加 `"test": "vitest run"`：可通过 `npm test` 触发
- 为「全局快捷键」功能的 `translateKeyEvent` 单测做准备

## 2026-05-15 新增 CONTRIBUTING.md 与项目级 todo skill

- 新增 `CONTRIBUTING.md`：开发流程与 Claude Code skill 约定（todo skill
  内置说明、superpowers 插件安装、各 skill 用途、brainstorm→计划→TDD→
  验证的开发流程、代码风格）
- `.claude/skills/todo/`：内置 todo skill 实体，随仓库版本管理
- `.claude/skills/README.md`：目录说明，指向 CONTRIBUTING.md
- `CLAUDE.md` 顶部加 CONTRIBUTING.md 指针

## 2026-05-15 重写 README

- 把默认 Tauri 脚手架 README 替换为完整项目介绍（定位、功能特性、
  技术栈、快速开始、项目结构、任务格式说明）

## 2026-05-15 象限「+」按钮改为常驻显示

- `QuadrantGroup` header 的「+」按钮从 hover 才淡入改成一直可见
  （去掉 `opacity: 0` 和 `:hover/:focus-within` 显示规则），hover 仍
  保留 accent 高亮反馈

## 2026-05-15 每个象限独立的「+」入口

- `QuadrantGroup` header 加 hover 显示的「+」按钮（`<span role="button">`
  实现，因为外层 header 已经是 `<button>`，嵌套 button 是 invalid HTML）。
  点击 → `openQuickAdd({ sourceId, quadrant: <本象限> })`，QuickAdd 弹窗
  以**当前象限**为初始选中，不再总是 fallback 到「不紧急不重要」
- `QuickAddOptions` 加可选 `quadrant?: Quadrant | null`：`undefined` /
  缺失 = "使用 dialog 默认值"（保持原先 source header 「+」入口行为），
  显式 `null` = "锁定到 unsorted"
- `QuickAddDialog` 打开时 `q === undefined ? DEFAULT_QUADRANT : q`，
  正确区分 undefined / null 两种语义
- `SourceGroup` / `FileGroup` 给嵌套的 `QuadrantGroup` 透传
  `:source-id="source.id"`
- TODO.md：勾掉「大文件夹首次扫描进度」（已发版）

## 2026-05-15 colored source 在深色主题下 accent 一致性

- `SourceGroup.vue` 给 `.group.colored` 加一层 7% src-color body tint，
  并把 header tint 从 10%/18% 提到 16%/26%。原因：深色 `--surface` 是
  rgba 半透明，accent 左条只在 header 区域有 tint 衬托时看起来正常，
  到展开后的 body 区域就被周围半透明深色面板"稀释"得偏暗，stripe 在
  header / body 两段亮度对不齐。统一 tint 之后两段视觉连续
- `.gitignore` 加 `.superpowers/`（superpowers skill 本地缓存目录）

## 2026-05-15 feat：toast 气泡反馈系统 + 历史按钮未读外部修改红点

- **toast 核心**：新增 `useToast.ts` 模块级单例 + `ToastContainer.vue` 全局
  组件，4 个变体（success 2s 绿 / info 3s 灰 / warning 4s 琥珀+深字 / error 6s 红），
  整块色填充 + 进场弹性 translateY + 离场 scale fade + `.toast-move` FLIP 给兄弟
  平滑滑位；`MAX_VISIBLE = 3`，hover 暂停剩余计时，× 悬浮可见。容器
  `bottom: 52px` 压在 footer 上方居中，`align-items: center` 让宽度自适应内容
  的 toast 各自居中堆叠；toast 本身 `inline-flex` 无 `min-width` —— 短消息窄
  气泡、长消息撑到 `max-width: 100%`
- **挂载**：主窗口 `App.vue` + 历史窗口 `HistoryView.vue` 两个 root 各挂一次
  （per-window 队列；触发动作的窗口出自己的反馈）
- **接入点**：`stores/tasks.ts` (toggle / update / add) / `stores/settings.ts`
  (source CRUD / hub / quick actions) / `stores/history.ts` (undo / redo /
  jumpTo) 全部成功/失败 toast；i18n 走模块级 `i18n.global.t`（Pinia setup 不能
  用 `useI18n()`，store 在 `main.ts` 模块级实例化，没有 component context）
- **i18n**：`toast.*` namespace 加 21 个 key（en/zh 双语），`taskCompleted /
  taskUncompleted / taskMoved` 等带 `{text}` `{quadrant}` 占位
- **历史按钮未读红点**：`useHistoryStore` 扩展 `lastSeenAt`（localStorage
  `floaty.history.lastSeenAt`）/ `unseenExternal` computed / `markSeen()` /
  `syncLastSeenFromStorage()`；`HistoryView.vue` `onMounted` 之后立即
  `markSeen()` + 前端 `emit('history-seen-changed')` 广播；主窗口 `App.vue`
  listen 后调 `syncLastSeenFromStorage()` → `unseenExternal` 重新算 → 0 →
  红点消失。`TaskList.vue` `.history-btn::before` 左上红点 + 既有 `::after`
  右上绿点（hasRedo）位置对称可同时显示
- **Tauri 跨窗口事件**：`tauri-api.ts` 加 `emitHistorySeen()` / `onHistorySeenChanged()`；
  Tauri 2 前端 `@tauri-apps/api/event.emit` 默认跨窗口广播，**Rust 后端无改动**

## 2026-05-15 docs：toast 气泡系统 + 历史未读徽章 设计文档 + 实现计划

- 新增 `docs/superpowers/specs/2026-05-15-toast-notifications-design.md`：
  toast 单例 composable + 全局组件、4 个变体（success 2s / info 3s /
  warning 4s / error 6s）、`bottom: 52px` 居中、max 3 同时可见、hover 暂停、
  跨窗口（主 + history）独立 toast；外部修改静默 + 历史按钮挂未读红点
  徽章（localStorage `floaty.history.lastSeenAt` + `unseenExternal` computed
  + 跨窗口 emit `history-seen-changed`）
- 新增 `docs/superpowers/plans/2026-05-15-toast-notifications.md`：
  9 个 task 的实现计划，每个 task 列出 file 路径 + 完整代码块 + 手动验证步骤

## 2026-05-15 style：图标 polish + 修 phantom VS Code tab

- **图标**：历史按钮 `◷` → `🕒`；三处 `Icon name="settings"`
  （`TaskList` footer / `SourceGroup` header / `EmptyState` 角标）
  统一换成 `⚙️` emoji；`Icon.vue` 不再保留临时加的 `clock`
- **➕ 添加按钮**：去掉 accent 底色和边框，只保留白色十字，hover 继承
  base `.icon-btn:hover` 的 source-tint 反馈
- **Action-row hover 统一**：所有 `.icon-btn:hover` 改用
  `var(--src-color, var(--accent))` 做背景/边框 tint（之前 brand 按钮
  用自身品牌色 tint，导致 ➕ / VS Code / Terminal 等并排 hover 颜色
  不一致）。Brand 按钮前景仍由 `QuickActionIcon` scoped style 控制，
  品牌色辨识不丢
- **Press-and-hold cursor**：brand 按钮的 `cursor: grab` 改为按住 220ms
  后才切到 `grabbing`（pointerdown timer + `.pressing` class +
  pointerup/leave/dragstart/dragend 清理），避免 hover 看到 grab 误导
  用户以为按钮是 drag-only
- **修 phantom VS Code tab**：删除 `shell.rs` 里的
  `open_vscode_returns_helpful_error_when_missing` 测试 —— 它在
  `cargo test` 时**真的执行** `code.cmd C:\definitely-does-not-exist`，
  VS Code 会创建一个名为 `definitely-does-not-exist` 的脏空 buffer 并
  持久化进工作区会话，用户每次开同一个工作区都会看到这个无法摆脱的
  幻影 tab，关窗时弹「是否保存」对话框

## 2026-05-15 feat：任务历史 + 撤销 + 时间线窗口（实现）

- **后端**：新增 `src-tauri/src/history.rs`（`HistoryStore` + 事件 schema +
  peek/commit 模式 + 双向 jump_plan + atomic persist + 文件快照缓存 +
  500ms 外部编辑合并窗口）；改造 `storage.rs` 让 toggle/update/append/move
  返回 before/after `LineSnapshot`，并新增 `replace_line_if_hash` /
  `remove_line_if_hash` / `insert_line_at` 给撤销路径用；`commands.rs`
  新增 `get_history` / `get_history_cursor` / `undo` / `redo` / `jump_to` /
  `open_history_window` 5 个 IPC + 每个写入路径补 `record_snapshot`；
  `error.rs` 新增 `HistoryHashMismatch` / `HistoryFileMissing` /
  `HistoryDisabled` / `ExternalInUndoRange{count}` 并改 Serialize 为
  `{code, message, ...}` 结构化错误
- **前端**：新增 `HistoryView.vue` 历史窗口（左 timeline + 右 diff + 跳转）、
  `stores/history.ts` / `types/history.ts` / `utils/errors.ts`；`main.ts`
  按窗口 label 路由 App vs HistoryView，挂全局 Ctrl+Z / Ctrl+Y /
  Ctrl+Shift+Z / Ctrl+H 键位；`TaskList.vue` footer 加 🕒 历史按钮（带
  redo 可用小红点）；所有 store 错误展示从 `String(e)` 换成
  `errorMessage(e)` 防止结构化错误退化成 `[object Object]`
- **窗口体验**：`lib.rs` setup 在 app 启动时**预创建** history 窗口为隐藏，
  关闭按钮拦截走 `hide()`；`main.ts` 全局 `import './styles/main.css'`
  让 history 窗口也拿到 theme 变量（修复整个窗口白屏）；`open_history_window`
  简化为 show + setFocus —— 解决「点一下没反应、点两下才开」
- **review 修复**：① watcher 单回调内 `HashSet<PathBuf>` dedupe 路径，
  防 notify 多事件让 `IgnoreHashes` 漏出去产生 phantom 外部修改 ②
  `history.rs::push_external_edit` 兜底内容等同就丢，且首次无 baseline
  时静默建立而不 emit「整文件 added」误报 ③ history 文件持久化走 tempfile
  + rename 原子重写，常规 push 走 append-only 快速路径 ④ 撤销 cursor 改
  peek/commit 模式，apply 失败时 cursor 一动不动无漂移 ⑤ HistoryView 用
  `useConfirm` 取代 `window.confirm`，区间含外部编辑时弹危险确认 ⑥
  所有路径用 `dunce::canonicalize` 做 HashMap key
- **其它**：Codex 顺手做的 QuadrantGroup collapse 持久化（每个 source +
  file + quadrant 一个 localStorage key）；`stores/settings.addSource`
  移除 race-prone 的 `markScanning(true)`，统一由 backend
  `source-scan-started` / `-finished` 事件驱动 spinner

## 2026-05-15 docs：任务历史 + 撤销 + 时间线窗口 设计文档

- 新增 `docs/superpowers/specs/2026-05-15-task-history-undo-design.md`：
  13 节设计文档，含目标、架构图、事件 JSONL schema、Rust 模块改动
  （新增 `history.rs` / 改造 `storage.rs` / 接入 `watcher.rs` 外部编辑分支）、
  Tauri 多窗口（主窗 + 历史窗）、前端组件（`HistoryView` + `EventList` +
  `EventDetail` + `JumpButton`）、撤销 / 重做 / 双向 jump-to 算法、Ctrl+Z /
  Ctrl+Y / Ctrl+H 键位、边界处理、测试与 4 阶段实现顺序
- 决策摘要：① 分两层覆盖（App 内动作可撤销，外部编辑只读时间线，回滚走 git）
  ② JSONL 不限期保留 ③ 历史文件落 Hub 根 `.floaty-history.jsonl` 随 git 走
  ④ 时间线窗口用左流右 diff 双栏（Notion 风）⑤ cursor 持久化跨会话 Ctrl+Z

## 2026-05-15 source 拖拽重排 off-by-one 修复 + UI 细节打磨

- `SourceGroup.onDotsPointerDown.onDrop` 里 `splice(srcIdx, 1)` 后再算
  `tgtIdx` 会让向下拖时目标下标少 1，导致"第 1 个挪到第 3 个"实际落到
  第 2 个。修复：在 splice 之前先抓 `tgtIdx`，向上向下都用同一条
  `splice(tgtIdx, 0, src)` 语义；同时把 srcIdx/tgtIdx 双重 < 0 校验合并
- `TaskItem` 搜索命中高亮从 accent 半透明改成纯黄底 + 加粗深灰字，密集
  列表里也能一眼看到
- `QuadrantGroup` 接入全局 `searchQuery` —— 搜索激活时强制展开，跟
  SourceGroup / FileGroup 一致，避免命中藏在折叠的象限里看不到
- `TaskList` 顶部「添加源」下拉改为纯 click 切换，去掉之前 180ms 悬停
  延时（鼠标稍偏一下就误触 / 误闭，体感跳脱）
- `SourceGroup` header 上的 per-source 象限计数条整行移除：每个
  QuadrantGroup header 自己显示数字，再来一份就是重复信息
- TODO.md 标注本轮 dev-test + 任务搜索完成；zh.ts 补 `source.expandQuadrants`
  / `collapseQuadrants` 两条 i18n（之前漏配，按钮 hover title 显示 key 字面量）

## 2026-05-15 source 内置「+ 添加任务」按钮 + 跨象限编辑 + 任务搜索

- TaskList 顶部的搜索栏式任务输入框退役：每个 SourceGroup header 的 actions
  区开头加了一颗 accent 色调的「+」按钮，点开新的 `QuickAddDialog`（teleport
  + promise 单例，跟 `useConfirm`/`useTaskEditor` 同模式）。弹窗里选 source
  下拉 + 4+1 象限 chip + 文本输入；象限默认每次重置为 🟢「不紧急不重要」，
  避免上次的选择隐式继承
- 顶部行简化为 `.toolbar` — 搜索框 + 添加源菜单 + 折叠/展开 三件套；旧的
  source-select / 5 象限按钮 / 任务 input 全部移除（i18n 也清掉了对应 key）
- `TaskEditorDialog` 同步加上同款象限 chip 行 + source 颜色圆点 + label 头条；
  返回类型从 `string | null` 变成 `{ text, quadrant } | null`
- Rust `update_task` 命令新增 `change_quadrant: bool` + `new_quadrant:
  Option<Quadrant>` 参数：象限没变 → 走原 `update_task_text`（缩进 / bullet /
  checkbox 字节级保留）；象限变了 → `storage::remove_task_line` 删原行 +
  `append_task_to_quadrant` 在目标 `##` header 下追加，遵守 `auto_create_quadrant_headers`
- `storage::remove_task_line(path, line)` 新增：原子写、删行前用 `parse_line`
  二次校验目标行确实是任务行，避免 registry 过期时误删非任务内容
- 任务行级搜索 (linter / 用户协作完成): TaskList 顶部 search input → provide
  `searchQuery`；TaskItem 在每段 inline-md segment 内部分词高亮匹配；FileGroup
  / SourceGroup 在搜索激活时强制展开（独立于用户折叠状态）；FileGroup 计数升级
  为 per-quadrant emoji 细分，跟 SourceGroup 风格对齐

## 2026-05-15 hover tint 跟随 source 强调色

- TaskItem `.row:hover` + FileGroup `.head:hover` 的背景换成
  `color-mix(in srgb, var(--src-color, var(--accent)) 14%, transparent)`，
  让 hover 与所在 source 卡片的 `--src-color` 保持同色系；未设强调色
  的 source 自动 fallback 到 `--accent`（中性灰）
- 旧实现用 `--surface-strong`（0.92 alpha 近实色），叠在卡片半透明
  body 上变成厚重色块，深色主题下完成行尤其突兀
- 顺带去掉 `.row` 的 `border-bottom: 1px solid transparent` 与 hover
  时的 `border-bottom-color` 切换，hover 不再多出一条分割线

## 2026-05-15 source 内四象限分组 (Eisenhower view)

每个 markdown source 内部按艾森豪威尔矩阵分四组渲染 — `## 🔴 紧急+重要` /
`## 🟡 重要不紧急` / `## 🟠 紧急不重要` / `## 🟢 不紧急不重要` 任意层级 header
都识别, 子标题继承父标题象限, 任务上方没出现过 quadrant header → ⚪ 未分类。
零 sidecar、不动任务行 emoji 元数据 — 跟 PLAN.md 第十节的 AI 集成约定一致,
跟 `/todo` skill 写出来的文件结构天然兼容。

合并自 `feature/quadrant-view` (15 commits, merge `7692051`)。设计文档见
`docs/superpowers/specs/2026-05-15-quadrant-view-design.md`, 实施计划见
`docs/superpowers/plans/2026-05-15-quadrant-view.md`。

### 后端
- `types.rs`: `Quadrant` enum (`urgent_important` / `not_urgent_important` /
  `urgent_not_important` / `not_urgent_not_important`, snake_case 序列化),
  `Task.quadrant: Option<Quadrant>` (serde default → None), `AppConfig.
  auto_create_quadrant_headers: bool` (默认 true)
- `parser.rs`: stateful scan — 任意级别 header 文字含 🔴🟡🟠🟢 之一就更新
  `current_quadrant`, 否则保持 (子标题继承); 任务行打当前 quadrant; 7 个新
  单测覆盖嵌套继承、多次出现合并、emoji 任意位置、混合 emoji 取第一个等
- `storage.rs`: `append_task_to_quadrant(path, text, quadrant, auto_create)`
  — 找到对应 quadrant header 在 section 末尾(下一个同级 header 前)插入;
  header 不存在 + auto_create 则在 EOF 追加 `## <emoji> <中文名>` 块。`None`
  fallback 到原 `append_task`。6 个新单测
- `commands.rs`: `add_task` 接受可选 `quadrant: Option<Quadrant>`, 调用
  `append_task_to_quadrant` 并传入 config 的 `auto_create_quadrant_headers`
- `error.rs`: 新增 `QuadrantHeaderMissing(Quadrant)` 变体

### 前端
- `src/components/QuadrantGroup.vue` (新): 折叠分组 (复用 `useCollapse` 响应
  全局 collapse-all), emoji + 中文名 + count badge; `tasks.length === 0` →
  整组不渲染 (空象限自动消失)
- `SourceGroup.vue` / `FileGroup.vue`: file source 直接 `QuadrantGroup × ≤5`,
  folder source 在 `FileGroup` 内嵌 `QuadrantGroup × ≤5`; 渲染顺序固定
  🔴 → 🟡 → 🟠 → 🟢 → ⚪
- `TaskList.vue`: QuickAdd 加 5 个 emoji 按钮选择器, 选中态高亮; 选择持久化
  到 localStorage `floaty.lastQuadrant`, 首次启动默认 ⚪
- `services/tauri-api.ts` + `stores/tasks.ts`: `addTask` / `add` 接受第三个
  可选参数 `quadrant`, 后端 None fallback 到原行为
- `stores/settings.ts`: `autoCreateQuadrantHeaders` computed + setter
- `SettingsView.vue`: 新 "Behavior" section, 暴露 auto-create-headers 开关
- i18n: `quadrant.*` (5 个名称) + `settings.behavior` + `settings.
  auto_create_quadrant_headers{,_help}` (en/zh)

### Merge 整理
- `SourceGroup.vue` import 区两边都加东西的冲突, 共存解决
- `SettingsView.vue` 清掉 `safeHexColor` dead import (`0315e12` commit 引入
  但未使用; vue-tsc strict 模式在 merge 后才暴露)

## 2026-05-15 source 自定义强调色

- `Source` 加 `color: Option<String>` 字段（hex 形式 `#xxx[xxx[xx]]`），
  Rust `update_source` 在保存时正则校验，非 hex 一律置 `None`，防止
  被改坏的 config 把任意 CSS 灌进 UI
- `src/utils/colors.ts` 新文件：9 色 Tailwind-500 调色板 + `safeHexColor`
  校验（与 Rust 侧规则同步）
- SourceGroup 卡片读 `--src-color`：4px inset 左条（不挤位）+ 10% header
  色相，hover 升到 18%；卡片 border 也向源色微调。同一套色板出现在
  in-header 编辑器与 Settings 页源卡编辑器
- 实时预览：in-header 编辑器打开时卡片立即反映 draft 色，按保存才落盘，
  取消还原
- TS api/store 透传 `color`；`source.fields.color` / `colorNone` 两条
  i18n（zh + en）
- 4 处 Rust 测试 helper（config/hub/registry/watcher.rs）的 `Source`
  字面量补 `color: None` 以匹配新字段

## 2026-05-15 修复 task 行 label padding 区误勾复选框

- `TaskItem.vue`：在 `<label class="row">` 上加 `@click.self.prevent`，
  拦截真正落在 label 自身 padding/gap 空白上的点击，阻止 HTML 隐式
  「label 转发 click 到包裹的 `<input>`」翻转 checkbox。`.self` 修饰
  符让 .text / checkbox 子元素冒泡上来的 click 不受影响。展开 source
  group 后立刻点 task 时高频触发：行的 `fadeIn translateY` 动画让指针
  常落在 .text 即将渲染到的位置之上的 padding 区域，恰好命中 label

## 2026-05-15 应用内任务 & source 编辑器 + UI 打磨

- `update_task` Tauri 命令 + `storage::update_task_text`（原子写、
  逐字节保留行前缀、拒绝空文本 / 多行）。`storage::tests` 新增 5 个单测
- 新 `useTaskEditor` composable + `TaskEditorDialog` 模态，在 App 根
  与 `<ConfirmDialog />` 同级挂载。单击任务即可编辑；带实时 inline-md
  预览；textarea 内支持 Ctrl+B/I/E/D/K 快捷键；打开时光标落在末尾
  （避免无意中全选擦掉）
- 在 `SourceGroup` header 内重新引入 source 编辑器（label /
  project_root / set-default / remove）。header 上新增一个 ⚙ 按钮
  切换它。Enter 保存、Esc 取消
- 把原本的 `⋯` 按钮换成 Lucide `grip-vertical` 六点图标。grip 现在
  是**只拖拽**的手柄 —— 单击不再跳转到 Settings；编辑请用旁边那个
  ⚙ 按钮
- 把 `collapse-all` / `expand-all` 切换按钮从 footer 移到 add-row
  的右端。图标换成 Lucide `chevrons-down-up` / `chevrons-up-down`；
  颜色跟随 `var(--text)`
- `+` 加 source 菜单改成 hover 自动展开（180ms 延迟关闭，让鼠标可以
  跨过 4px 缝隙）；保留 click 作为触屏 / 键盘 fallback
- 给 source / file / footer 的计数上色：todo 用琥珀（`--count-todo`），
  done 用绿（`--count-done`）；去掉 done 计数后面的 `✓`，颜色本身
  已经传达了语义
- 新加的 `SourceGroup` 首次渲染默认折叠
- 修 Claude Code 启动器（`shell::open_claude_code`）：运行时解析
  绝对路径 `%USERPROFILE%\.local\bin\claude.exe` 通过 PowerShell
  调用，不再依赖 `claude.cmd` 出现在 cmd.exe 的 PATH 里（通常不在）

## 2026-05-15 象限视图设计稿

- `docs/superpowers/specs/2026-05-15-quadrant-view-design.md`（新）：
  source 内按艾森豪威尔象限分组的 brainstorming 输出。象限通过任意
  层级 markdown 标题前的 emoji（🔴 / 🟡 / 🟠 / 🟢）识别；子标题没带
  emoji 时继承父标题的象限。不引入 sidecar、不改写任务行 —— 保住
  line_number 稳定性和 PLAN.md「markdown 是唯一事实源」原则。实现前
  先走 writing-plans

## 2026-05-15 通过 ⋯ 按钮拖拽重排整个 source（pointer events）

每个 source header 最右侧的 `⋯` 按钮同时承担两种行为：**单击打开设置**，
**按住拖到另一个 source header 上即重排**。`SourceGroup` 内的行内 source
编辑器整段移除 —— 编辑改走已有的 Settings 页面。

HTML5 native drag-and-drop 在 Tauri 2 的 WebView2 里完全不可用：
`dragstart` 会触发，但 `dragover` 始终不会派发到页面元素，整段拖拽
鼠标都卡在「禁止」光标。改用 pointer events 重写，在任何环境都能跑。

- `src-tauri/src/commands.rs`（+ `lib.rs` 注册）：新 `reorder_sources(ordered_ids)`
  Tauri 命令。拒绝不完整的列表，避免前端状态过期时意外丢 source
- `src/composables/useSourceDrag.ts`（新）：`startSourceDrag({ e,
  sourceId, onClick, onDrop })` —— 挂 document 级 pointermove/up；
  5px 阈值区分点击与拖动；目标通过 `elementFromPoint` 取顶层元素后
  向上爬 DOM 找带 `[data-source-id]` 的祖先
- `src/components/SourceGroup.vue`：dots 按钮换成 `@pointerdown` →
  `startSourceDrag(...)`；header 带 `data-source-id`。行内 source
  编辑器及相关状态（`editing` / `labelDraft` / `rootDraft` + 处理函数
  + 死 CSS）全部删除；source 编辑统一走 Settings 页
- `src/components/TaskList.vue`：把 `SourceGroup` 新加的 `open-settings`
  事件冒泡到 App 层的视图切换器
- `src/services/tauri-api.ts` + `src/stores/settings.ts`：为新命令
  添加封装（`api.reorderSources` / `settings.reorderSources`）

## 2026-05-15 reveal 图标改成粗实心文件夹（原本是描边 + 放大镜）

描边文件夹 + 放大镜 overlay 在 14px 下看着乱，更像「搜索」而不是「在
文件管理器打开」。换成单一实心文件夹 + 顶部一条柔白高光线 —— 轮廓
更干净，保留原本的黄色品牌色，动作语义更直白（「就是打开文件夹」）。

- `src/components/icons/QuickActionIcon.vue`：重写 `reveal` SVG ——
  一条 fill path 画文件夹主体 + 一条 35% 不透明度的白色 stroke 线
  做翻折边提示；删掉放大镜的 `<circle>` 和 `<line>`

## 2026-05-15 默认窗口宽度 380 → 760，项目自带 TODO.md

- `src-tauri/tauri.conf.json`：窗口初始宽度 380 → 760（高度仍是 600）；
  `minWidth` 保留 320 不变，用户照样能缩
- `TODO.md`（新）：项目自家 backlog，用的就是 Floaty Todo 自己解析的
  `- [ ]` 格式 —— 把这个项目当 File source 添加进去就能直接 dogfooding。
  按优先级分组（now / v0.3 / v1.0 / v2.0）、已知小坑、明确不做的事
  （PLAN.md 的 non-goals 也搬过来一份）

## 2026-05-15 折叠/展开全部、整行 header 可点、拖排序快捷按钮、reveal-in-explorer

一次性打磨四个 UX 毛刺：

1. **折叠 / 展开全部** —— 在 Settings 旁边新增 footer 按钮，通过全局
   trigger 一次切换所有 source + 所有 file group
2. **整行 header 点击切换** —— 以前必须点中那个小箭头；现在 source
   或 file header 任意处都行。动作按钮会 stopPropagation，避免点
   VS Code 又顺便把分组折叠掉
3. **拖拽重排快捷按钮** —— 抓 source header 上任意一个按钮拖到另一个
   按钮上，会改写 `enabled_quick_actions`（这是全局列表，所有 source
   一起跟着重排）
4. **「在文件管理器中打开」** —— 新 `QuickActionKind::Reveal` 用
   Explorer / Finder / xdg-open 打开 source 路径；每个 source header
   和 footer 的 hub 集群都加上一个对应的黄色文件夹按钮。默认
   `enabled_quick_actions` 改成 `reveal + vscode + terminal`，开箱即可
   发现这个能力

### 后端
- `types.rs`：`QuickActionKind::Reveal` 变体；`default_quick_actions()`
  改成 `[Reveal, Vscode, Terminal]`
- `shell.rs::reveal_in_explorer(path)` —— Windows 走 `explorer.exe`
  （传文件时用 `/select,<file>` 在父目录里高亮），macOS 文件用
  `open -R` / 目录用 `open`，Linux 用 `xdg-open` 打开父目录
- `commands.rs`：新 `reveal_source(source_id)`；`run_quick_action` 和
  `open_hub` 把 `Reveal` 派到 `reveal_in_explorer`
- `lib.rs`：注册 `reveal_source`

### 前端
- `src/composables/useCollapse.ts`（新）：`collapseAll()` /
  `expandAll()` 自增计数器 ref；`bindCollapse(setter)` 在组件内把
  本地 `collapsed` ref 绑到这两个 token
- `src/components/icons/Icon.vue`：新增 `collapse-all` / `expand-all`
  图标（chevron + 中心线）
- `src/components/icons/QuickActionIcon.vue`：新增 `reveal` SVG ——
  开口文件夹 + 内嵌放大镜；黄色品牌色 `#f59e0b`
- `src/components/TaskList.vue`：footer 在 Settings 旁加 `collapse/expand-all`
  按钮；hub 配置后，footer 品牌色右侧集群在 hub-vscode + hub-claude
  之前加 hub-reveal
- `src/components/SourceGroup.vue`：
  - header click 切换 `collapsed`（caret 退化成装饰 span）；动作集群
    和编辑面板用 `@click.stop` 防止冒泡到切换逻辑
  - 订阅 `bindCollapse`
  - 每个 `.icon-btn.brand` 标记 `draggable="true"`；dragstart/over/leave/
    drop/end 跟踪 `dragKind` + `dropTargetKind`；drop 时 splice 启用
    列表并调用 `setEnabledQuickActions`；CSS 让正在拖的按钮变成 0.4
    透明度，落点按钮加 accent 描边
- `src/components/FileGroup.vue`：同样的 header-click + bindCollapse
  处理；编辑器用 `@click.stop`
- `src/views/SettingsView.vue`：`ALL_QUICK_ACTIONS` 把 `reveal` 排到
  最上面；每张 source 卡片加 reveal 按钮；`revealSource(s)` 调
  `runQuickAction('reveal')`
- `src/i18n/locales/{en,zh}.ts`：`source.reveal`、
  `settings.sources.reveal`、`hub.reveal`、`tasks.collapseAll` /
  `expandAll` 字符串

## 2026-05-14 TaskList footer 加 hub 快捷入口（VS Code / Claude Code）

配置了 hub 文件夹后，footer 多出两个品牌色按钮，直接打开 hub —— 一个
VS Code，一个新开的 Claude Code 会话。位置在计数和 pin 按钮之间，跟
原本的 source 快捷按钮挨着同一个右侧集群。

- `src-tauri/src/commands.rs`：新 `open_hub(kind: QuickActionKind)` ——
  从状态读 `hub_folder`，派到对应的 `shell::open_*`；hub 没配置时
  返回错误
- `src-tauri/src/lib.rs`：注册命令
- `src/services/tauri-api.ts`：`openHub(kind)`
- `src/components/TaskList.vue`：两个新 footer 按钮，依赖
  `settings.hubFolder` 显隐；用 `<QuickActionIcon>` 渲染保持视觉一致；
  `.footer-btn.brand` 复用 `SourceGroup` 的 `color-mix(currentColor …)`
  hover ring
- `src/i18n/locales/{en,zh}.ts`：`hub.openVscode` / `hub.openClaudeCode`
  tooltip 字符串

## 2026-05-14 source 类型图标换成真实 emoji，展开/折叠时切换

把每个 source header 上原本的描边文件夹 / 文件 SVG 换成真彩 emoji，
folder + file 字符还要根据展开状态翻转，让图标本身就传达状态。

- `src/components/SourceGroup.vue`：新 `kindEmoji` computed ——
  folder：📁 折叠 / 📂 展开；file：📄 折叠 / 📝 展开；模板用
  `<span class="kind-icon">` 渲染，CSS 钉死 Segoe UI Emoji /
  Apple Color Emoji / Noto Color Emoji 字体保证彩色显示
- `src/views/SettingsView.vue`：source 卡片 `src-icon` 跟进新风格
  （这里没展开/折叠态，所以只用 📁 / 📄）

## 2026-05-14 add-row 的 "+" 改成加 source（原本是加任务）

视觉上 "+" 紧挨着 source 下拉框，用户预期就是加 *source*。任务用
文本框 + Enter 加就够了，按钮重新分配用途。

- `src/components/TaskList.vue`：把表单 submit 按钮替换成独立的
  `type="button"` "+"，包在 `.add-source-wrap` 里；点击弹一个挂在
  按钮下方的小下拉菜单（Folder / File）
- 点击外部 + Esc 关闭菜单；弹出动画跟 ConfirmDialog 一致
- 加任务：直接在输入框按 Enter（placeholder 现在写「…（Enter）」/
  「…（回车）」，affordance 一眼能看见）
- i18n：新 `tasks.addSourceTitle`（新按钮的 title）；更新
  `tasks.addPlaceholder` 带上 Enter 提示

## 2026-05-14 pin 按钮换成 U+1F4CC 图钉 emoji

之前的卡通 SVG 钉子在 Windows 14px 下不像图钉（描边 + ellipse 看着
更像气球）。换成真 📌 emoji —— Segoe UI Emoji 渲染成正确的彩色，
关闭态通过 CSS `filter: grayscale(0.85) opacity(0.55)` 去饱和。激活
态加一个 `rotate(-12deg)` 让两种状态一眼能区分。

- `src/components/TaskList.vue`：pin 按钮渲染 `<span class="pin-emoji">📌</span>`
  代替 `<Icon name="pin" />`；新 `.pin-emoji` 规则锁死彩色 emoji 字体，
  切换时 filter/transform 平滑动画
- `src/components/icons/Icon.vue`：把不再使用的 `pin` 和 `pin-off`
  case 从 union 和模板里拿掉 —— 让中心化图标库保持精简

## 2026-05-14 hub 文件夹 —— 用硬链接 / junction 镜像所有 source

加入一个可选的「hub 文件夹」，把所有配置过的 source 通过 OS 级
文件系统链接镜像过来。AI 工具和 shell 脚本可以从一个地方驱动所有
项目的 TODO，不用一个个 crawl 仓库。双向同步零延迟，因为没拷贝 ——
两端真的是同一个 inode（File source → hard link）或同一个目录
（Folder source → NTFS junction / POSIX symlink）。

### 后端
- `src-tauri/src/hub.rs`（新）：纯文件系统模块 —— `mirror_path_for`
  从 source 清洗后的 label 派生 hub 侧文件名，`create_mirror` /
  `remove_mirror` / `sync_all` 管理单个镜像项 + 整体重同步时清理
  孤儿；跨平台：文件用 `std::fs::hard_link`，文件夹在 Windows 用
  `cmd mklink /J`，其它平台用 POSIX symlink
- `types.rs`：`AppConfig` 加 `hub_folder: Option<PathBuf>`，配
  `#[serde(default)]` 让旧配置无缝迁移
- `commands.rs`：新 `set_hub_folder(path?)`（变更时整体重同步）和
  `resync_hub()`（手动修复）；`add_source` / `remove_source` /
  `update_source` 在主效果完成后通过一个吞错误的 helper 调 `hub`，
  避免 junction 失败阻塞 source CRUD
- `lib.rs`：注册这两个命令；声明 `mod hub`
- `config.rs`：测试更新到新字段
- 44 个单测通过（新增 5 个 hub 测试：名称清洗、镜像路径推导、
  hard-link 创建 + 写入透传、幂等重建、删除、`sync_all` 孤儿清理）

### 前端
- `types/task.ts`：`AppConfig.hub_folder: string | null`
- `services/tauri-api.ts`：`setHubFolder(path?)`、`resyncHub()`
- `stores/settings.ts`：`hubFolder` computed；`setHubFolder` /
  `resyncHub` / `pickAndSetHubFolder` helper
- `views/SettingsView.vue`：在 Quick actions 和 Sources 之间新增
  「Hub folder」section —— 已配置时显示路径 + `Resync` / `Change` /
  `Disable` 按钮，未配置时显示「Choose folder…」CTA；错误就地展示
- `i18n/locales/{en,zh}.ts`：`settings.sections.hub` +
  `settings.hub.*` 字符串

### 取舍
- 仅同卷支持（硬链接 + NTFS junction 不能跨卷）。跨卷 source 镜像
  失败时给出可操作的错误信息，但 source 本身还是会被加进来
- Hub 侧 label 冲突是已知边界情况 —— 当前谁先镜像谁占名字；未来
  可以加 `(source_id)` 后缀消歧

## 2026-05-14 pin 图标改成图钉红，不再被静音色吞掉

pin 图标之前继承 footer 的静音文字颜色，激活态几乎看不见。现在改成
两套主题都用 #ef4444 —— 激活时是饱和的图钉红，浮窗状态是「半红半静音」
混合，让切换一目了然又不丢 affordance。

- `src/components/TaskList.vue`：`.pin-btn` 颜色规则 —— `.active`
  用 #ef4444 给图标 / 边框 / 软背景，hover 加深到约 22% mix；
  `:not(.active)` 用 55% 混入红色的静音色，hover 直接切回纯红

## 2026-05-14 统一卡通 SVG 图标库 —— 取代所有 ASCII / emoji 字符

应用里所有图标按钮现在统一通过一个 `<Icon name="…" />` 组件渲染，
不再让 ⚙ / ↻ / ▾ / 📁 / 📄 / 🗑 / 📝 / ✎ / ↺ / ← / ⟳ / ☀ / 🌙 /
🖥 这种字符散落在模板里。风格刻意做得粗壮、亲切（Lucide 风格描边，
1.9 px 笔触，圆角端点），保证一组图标在 12 px 也能读成一体。

- `src/components/icons/Icon.vue`（新）：中心化组件；`name: IconName`
  （字符串字面量联合）+ `size`；20 个图标 —— `pin` / `pin-off` /
  `settings` / `refresh` / `plus` / `chevron-down` / `chevron-right` /
  `more-horizontal` / `pencil` / `rotate-ccw` / `folder` / `file` /
  `trash` / `sun` / `moon` / `monitor` / `arrow-left` / `loader` /
  `check` / `x`；pin 的两个变体更粗壮（激活时是带高光的填充倾斜
  图钉，浮窗态是更倾斜的描边）
- `src/components/TaskList.vue`：替换掉 `⚙ ↻ +` 以及老的 inline pin
  SVG；新加共享 `.icon-only` footer 按钮修饰类 flex 居中 28×26 px 图标
- `src/components/SourceGroup.vue`：caret（`▾/▸`）、类型图标（`📁/📄`）、
  扫描旋转图标（`⟳`）、`⋯` 按钮，以及行内编辑器的文件夹选择按钮全
  换成 `<Icon>`；品牌色快捷按钮保持 `<QuickActionIcon>`
- `src/components/FileGroup.vue`：caret + ✎ 重命名 + ↺ 重置全换成 `<Icon>`
- `src/views/SettingsView.vue`：返回箭头、主题分段控件（sun / moon /
  monitor）、source toolbar（folder / file 带文字 label）、单卡 source
  动作（vscode/terminal 用 `QuickActionIcon`，pencil / trash 用 `Icon`）、
  内嵌编辑器的文件夹选择
- `src/components/EmptyState.vue`：folder / file 选择按钮 + 右下角设置
  齿轮全换 `<Icon>`
- `src/i18n/locales/{en,zh}.ts`：剥掉 `empty.add*` 和
  `settings.sources.add*` 字符串里的 emoji 前缀 —— 视觉交给图标组件,
  文字只剩 label

## 2026-05-14 TaskList footer 加 always-on-top 的 pin / unpin 切换

之前 app 是永久 always-on-top。如果用户想切换到别的窗口专心做事，
不用再手动把 Floaty 拽开了 —— 直接在 footer 按钮切。

- `src-tauri/src/commands.rs`：新 `set_always_on_top(on: bool)`
  命令 —— 把 flag 写回 config + 调 `window.set_always_on_top(on)`
  让改动立即生效
- `src-tauri/src/lib.rs`：setup 在加载后对主窗口执行
  `set_always_on_top(cfg.always_on_top)`，每次启动都用持久化值覆盖
  `tauri.conf.json` 的初值
- `src/services/tauri-api.ts`：`setAlwaysOnTop(on)`
- `src/stores/settings.ts`：`alwaysOnTop` computed + `setAlwaysOnTop` /
  `toggleAlwaysOnTop` helper
- `src/components/TaskList.vue`：footer 在计数和 ↻ 刷新之间塞一个
  28px pin 按钮；inline SVG 画图钉，两个状态 —— 锁定时是 accent 色
  实心、浮窗时是描边 + 倾斜
- `src/i18n/locales/{en,zh}.ts`：`window.pin` / `window.unpin` title

## 2026-05-14 快捷按钮换成品牌色 SVG

把快捷按钮上的占位 Unicode 字符（⎘ / ▷ / ◆）换成真正的品牌色 inline
SVG，用户一眼能认出每个动作。

- `src/components/icons/QuickActionIcon.vue`（新）：单个 Vue 组件，
  `kind` prop，三个 inline SVG 取自 simple-icons（CC0）—— VS Code 折角
  V 标志（#0098FF）、通用终端窗口加 `>_` 提示符（#4DAA7F）、Anthropic
  八角 sparkle（#D97757）；暗色模式 CSS 把每个色微调亮一点适配毛玻璃
- `src/components/SourceGroup.vue`：source header 每个按钮渲染
  `<QuickActionIcon>`（原本是硬编码字符）；按钮加 `.brand` 修饰类 ——
  hover ring 用图标自身颜色通过 `color-mix(currentColor 10% / 30%)`
  实现，让图标本身依然清晰可读
- `src/views/SettingsView.vue`：Quick actions section 也做同样替换，
  让 checkbox 列表中的品牌图标一致
- 不引入运行时依赖 —— 所有 SVG 都 inline，零网络请求

## 2026-05-14 可配置快捷动作（+ Claude Code）+ 响应式扫描 UX

两个相关的打磨项一起做：

1. **每个 source header 的快捷按钮现在用户可配置。** 内建集是
   VS Code / Terminal / Claude Code；用户在 Settings → Quick actions
   勾选要显示哪些。保存列表的顺序就是每个 source 上的显示顺序
2. **添加大文件夹 source 不再卡 UI。** 扫描已经是后台线程（这条
   一直如此），现在还会 emit `source-scan-started` /
   `source-scan-finished` 事件；对应 source 的 header 显示一个旋转
   ⟳ 徽章 + body 显示「扫描中…」一行；任务数 > 50 的 source 内部
   FileGroup 默认全部折叠渲染，DOM 保持小巧，用户展开某个文件再
   渲染。单文件 source 也不再额外套一层 FileGroup —— source header
   *就是* 文件 header

### 后端
- `types.rs`：新 `QuickActionKind` 枚举（`Vscode` / `Terminal` /
  `ClaudeCode`）；`AppConfig` 加 `enabled_quick_actions: Vec<…>`，
  通过 `#[serde(default …)]` 注入 `default_quick_actions()`（VS Code
  + Terminal）
- `shell.rs`：新 `open_claude_code(path)` —— Windows 级联
  `wt.exe -d <p> -- cmd /k claude.cmd` → 裸 `wt.exe -d <p>` →
  `cmd /c start cmd /k claude.cmd`；macOS 走 `osascript` → Terminal.app；
  Linux 用跟 `open_terminal` 同样的终端级联，外加 `-e claude`
- `commands.rs`：`open_in_claude_code(source_id)`、
  `run_quick_action(source_id, kind)`（按 `QuickActionKind` 动态派发）、
  `set_enabled_quick_actions(actions)`；三个都接进 `invoke_handler!`
- `lib.rs::spawn_source_scan_and_watcher`：在 `rebuild_source` 之前
  emit `source-scan-started`，之后 emit `source-scan-finished`，
  payload 是 source id
- `config.rs`：旧的 `load_strips_verbatim_prefix_and_remaps_default_id`
  测试更新到新的 `enabled_quick_actions` 字段
- 38 个单测通过

### 前端
- `types/task.ts`：`QuickActionKind = 'vscode' | 'terminal' | 'claude_code'`；
  `AppConfig.enabled_quick_actions: QuickActionKind[]`
- `services/tauri-api.ts`：`openInClaudeCode`、`runQuickAction`、
  `setEnabledQuickActions`、`onSourceScanStarted` / `onSourceScanFinished`
- `stores/settings.ts`：`enabledQuickActions` computed、
  `scanningSourceIds: ref<Set<string>>`、`isScanning` computed、
  `markScanning(id, on)` helper、`setEnabledQuickActions`；`addSource`
  立刻把自己标成 scanning，避免事件 race
- `App.vue`：订阅 scan-started / scan-finished 转发给
  `settings.markScanning`
- `components/SourceGroup.vue`：
  - `settings.enabledQuickActions` 每个条目渲染一个图标按钮，通过
    `api.runQuickAction` 派发
  - 扫描中显示旋转 ⟳ + 「Scanning files…」
  - `BIG_SOURCE_TASK_THRESHOLD = 50` —— 给所有 FileGroup 传
    `initial-collapsed`，避免大 source 一次性渲染数千 TaskItem 节点
  - 文件类 source 跳过 FileGroup 包装层，直接渲染 TaskItem（之前
    会嵌套一个冗余的组）
- `components/FileGroup.vue`：新 `initial-collapsed` prop，默认 false；
  被 SourceGroup 用于大 source 优化
- `views/SettingsView.vue`：新「Quick actions」section —— 每种 kind
  一行 checkbox；切换就改写 `enabled_quick_actions`
- `i18n/locales/{en,zh}.ts`：`source.openClaudeCode`、`scanning`、
  `scanningHint`、`settings.sections.quickActions`、
  `settings.quickActions.hint`

## 2026-05-14 inline markdown 渲染 + 应用内 confirm + 更聪明的默认 label

报上来的三个小 UX 打磨一起做：

1. **任务文本渲染 inline markdown** —— `**bold**`、`*italic*`、
   `` `code` ``、`~~strike~~`、`[text](url)` 渲染成正常元素，不再
   原样输出。链接走 OS 默认 handler 打开
2. **删除 source 改成应用内模态确认** —— 替换原生 `window.confirm()`
   （跟浮窗风格不搭，且容易被忽略）。单个全局 `<ConfirmDialog>` 在
   App 根挂载，由 `composables/useConfirm.ts` 的 `confirm()` promise
   驱动
3. **`add_source` 自动推断合理的默认 label** —— 用 source 的
   effective `project_root` 文件夹名（所以
   `D:\Projects\WishTalk\Todo.md` 这种 File source 默认 label 就是
   `WishTalk`，跟「Open in VS Code」/ terminal 跳转目标一致）

顺带修了 QuickAdd source 下拉里 `default (WishTal…` 被截断的老问题。

### 后端
- `commands.rs::add_source`：从 `project_root` 的 `file_name()` 解析
  默认 label（Folder → 文件夹名；File → 父文件夹名）；用户自己传非空
  label 仍然优先
- `shell.rs::open_url(url)`：跨平台默认 handler 启动器（Windows
  `cmd /c start "" <url>`、macOS `open <url>`、Linux `xdg-open`），
  带控制字符防御
- `commands.rs::open_url(url)` + `lib.rs` `invoke_handler` 注册

### 前端
- `src/utils/inline-md.ts`（新）：零依赖 inline 解析器 → segment 数组
  （`text` / `code` / `bold` / `italic` / `strike` / `link`）；不用
  `v-html`，构造上就 XSS-safe
- `src/components/TaskItem.vue`：把 segment 映射成 `<code>` /
  `<strong>` / `<em>` / `<s>` / `<a>`；链接 click 调 `api.openUrl`
- `src/composables/useConfirm.ts`（新）：`confirm({ title, message,
  confirmText, cancelText, danger }) → Promise<boolean>`；单例状态
- `src/components/ConfirmDialog.vue`（新）：Teleport 模态，遮罩 click /
  Esc 取消，confirm 按钮 focus-trap，破坏性动作有 danger 变体；弹出
  动画
- `src/App.vue`：根处挂 `<ConfirmDialog />`
- `src/components/SourceGroup.vue` + `src/views/SettingsView.vue`：
  把 `window.confirm` 换成新的 `confirm()` API
- `src/services/tauri-api.ts`：`openUrl(url)`
- `src/components/TaskList.vue`：`source-select` CSS —— 去掉 95px
  上限；改成行宽 45% + 依赖 select 控件的原生尺寸，`★ WishTalk` 这种
  长 label 不再被截断
- `src/i18n/locales/{en,zh}.ts`：`targetDefault` 短写法（`★ {label}`）、
  更短的 QuickAdd placeholder、`confirm.*` 字符串（title / ok /
  cancel / removeSource{Title,Message,Confirm}）

## 2026-05-14 source 内嵌套按文件分组，文件 label 可重命名

Folder source 现在把任务按 `.md` 文件分成一个个可折叠的子组，所以
vault 里多 note 时不会全部铺成一个长列表。每个文件组可独立展开 /
折叠，并且可以给它起个自定义显示名，避开「五个 `todo.md` 看起来
一模一样」的问题。

### 后端
- `types.rs`：`AppConfig` 加 `file_labels: HashMap<String, String>` ——
  以 canonical / dunce-simplified 绝对路径为 key；`file_label_key()`
  helper 集中处理 key 派生
- `commands.rs`：新 `set_file_label(file_path, label)` 命令 ——
  `None` 或 trim 后为空即清掉覆盖
- `lib.rs`：在 `invoke_handler!` 注册 `set_file_label`
- `config.rs`：已有的 `load_strips_verbatim_prefix_and_remaps_default_id`
  测试更新到新的 `file_labels` 字段
- 38 个单测全过

### 前端
- `src/types/task.ts`：`AppConfig.file_labels: Record<string, string>`
- `src/services/tauri-api.ts`：新 `setFileLabel(filePath, label)`
- `src/stores/settings.ts`：暴露 `fileLabels` / `fileLabel(path)` /
  `setFileLabel(path, label)`
- `src/components/FileGroup.vue`（新）：单文件行，带 caret 切换、
  hover 才出现的 ✎ 重命名按钮、内嵌重命名输入（Enter 保存 / Esc
  取消 / ↺ 重置回默认名），加上任务列表
- `src/components/SourceGroup.vue`：任务现在按 `task.source_file`
  分桶，每个桶渲染成一个 `FileGroup`；File 类 source 仍然渲染成
  单一组；顺序按文件路径稳定
- `src/i18n/locales/{en,zh}.ts`：新增 `file.editLabel` /
  `file.resetLabel` / `file.noTasks`

## 2026-05-14 剥掉 Windows verbatim path 前缀（\\?\），还路径一个友好样子

Rust 的 `std::fs::canonicalize` 在 Windows 上返回 `\\?\D:\...`。这种
路径落到 `Source.path` 再传给 `pwsh -WorkingDirectory` 时，PowerShell
提示符会变成
`PS Microsoft.PowerShell.Core\FileSystem::\\?\D:\Projects\WishTalk>`，
而不是 `PS D:\Projects\WishTalk>`。同样的前缀也会渗进 VS Code 标题栏
和 SettingsView 的路径显示。

- `Cargo.toml`：加 `dunce = "1"`
- `types.rs`：`Source::id_for` 现在先过 `dunce::simplified()` 再 hash，
  保证 verbatim 和友好形态的同一路径算出同一个 id
- `registry.rs`：`best_effort_canonical` 换成 `dunce::canonicalize`
  （文件消失时最后 fallback 到 `dunce::simplified`）
- `parser.rs`：`parse_file` 给 `Task.source_file` 用 `dunce::canonicalize`
- `commands.rs`：`add_source` 在 hash / 持久化前用 `dunce` canonicalize
- `config.rs`：`load_from` 跑 `normalize_paths` —— 幂等迁移：剥掉
  每个 `source.path` / `project_root` 的 `\\?\`、用清洗后的路径
  重算 id、底层 source id 变了就重映射 `default_source_id`；新单测
  `load_strips_verbatim_prefix_and_remaps_default_id` 覆盖迁移
- `lib.rs`：setup 在 load 后 `save_to` 把清洗过的 config 写回，第一次
  升级启动时把磁盘 JSON 也规范化
- 38 个单测通过（之前 35；加上迁移测试 + 早期的两个 shell 测试）

## 2026-05-14 独立 Settings 页（主题 / 语言 / sources）+ i18n（en/zh）

集中式 Settings 页面替换原来的浮动主题按钮。TaskList 左下角的 ⚙
按钮（EmptyState 的角标按钮也一样）打开一个全屏 Settings 视图，
让用户管理主题、显示语言、source 列表（每行都有醒目的删除）。所有 UI
字符串现在都走 vue-i18n，中英文都打进 binary 一起发布。

- `package.json`：加 `vue-i18n@^11`
- `src/i18n/index.ts`（新）：组合式 API 的 `createI18n`；初始 locale
  按 `localStorage['floaty.locale']` → `navigator.language` → `'en'`
  自动判断；导出的 `setLocale(locale)` 持久化选择并更新
  `document.documentElement.lang`
- `src/i18n/locales/en.ts` 和 `src/i18n/locales/zh.ts`：所有 UI
  字符串（empty / tasks / source / settings / errors）
- `src/main.ts`：`app.use(i18n)`
- `src/views/SettingsView.vue`（新）：四个 section —— 外观（主题分段
  控件）、语言（locale 下拉）、Sources（toolbar + 卡片列表，每行 ⎘ /
  ▷ / 📝 / 🗑，内嵌编辑器编辑 label / project_root / set-default）、
  关于；back 按钮回到任务视图
- `src/App.vue`：引入 `view: 'tasks' | 'settings'` 状态；在根处挂
  `useTheme()` 让系统配色监听整个 app 都活；删掉浮动主题按钮；托盘
  「Manage sources…」事件改成打开 Settings 视图
- `src/components/TaskList.vue`：左下角 ⚙ 设置按钮（替换原本的
  📁+/📄+ 行内 chip —— 加 source 移到 Settings）；emit `openSettings`；
  所有字符串走 `t()`
- `src/components/EmptyState.vue`：左下角 ⚙ 角标按钮，无 source
  的用户也能进语言 / 主题设置；字符串走 `t()`
- `src/components/SourceGroup.vue`：字符串走 `t()`（卡内编辑面板
  保留方便浏览任务时快速操作）

## 2026-05-14 source 分组 UI + 单 source 动作 + 内嵌编辑器

任务现在按 source 分组渲染，不再是单个扁平列表，每个 source header
带新的 shell 动作按钮和编辑 label / project_root / set-default /
remove 的内嵌面板。

- `src/components/SourceGroup.vue`：新 —— 可折叠 header（caret +
  类型图标 + label + 默认徽章 + 单 source 待办/已完成计数），三个
  图标按钮（⎘ Open VS Code · ▷ Open Terminal · ⋯ 编辑），内嵌编辑器
  含 Label / Project root（+ 文件夹选择）/ Set-default / Remove
  （带 confirm）/ Save · Cancel
- `src/components/TaskList.vue`：改写成按用户配置顺序为每个 source
  渲染 `SourceGroup`；QuickAdd 输入框加 target source 下拉
  （`default (foo)` + 每个 source 一项），placeholder 反映新任务
  落点；footer 收紧为「总计 + 📁+ / 📄+ / ↻」chip
- 0 任务的 source 也显示「该 source 下没有任务」，让它继续作为
  VS Code / terminal 动作的入口可见

每个 `Source` 现在暴露两个副作用命令，在它的 `effective_project_root()`
（配置的 `project_root`，或 Folder source 退到 `path`、File source 退到
`path.parent()`）启动外部工具。

- `src-tauri/src/shell.rs`：新模块 —— `open_vscode(path)` 和
  `open_terminal(path)`；跨平台终端级联：Windows 试
  Windows Terminal → pwsh.exe → powershell.exe；macOS `open -a Terminal`；
  Linux 试 `x-terminal-emulator` / `gnome-terminal` / `konsole` / `xterm`；
  第一个能 spawn 的赢，全失败返回 `AppError::CommandFailed` 带尝试过的
  binary 列表（UI 可以引导用户装 `code` / 配 `wt`）
- `src-tauri/src/lib.rs`：`mod shell;`；在 `invoke_handler!` 注册两个
  命令
- `src-tauri/src/commands.rs`：新 `open_in_vscode(source_id)` 和
  `open_in_terminal(source_id)` —— 都通过 `find_source_by_id` 拿到
  source，然后调 `shell`
- `src-tauri/src/types.rs`：删掉没用的 `effective_label`（label 回退
  交给前端）
- `src/services/tauri-api.ts`：加 `openInVscode(sourceId)` 和
  `openInTerminal(sourceId)`
- 2 个新单测覆盖平台级联

## 2026-05-14 多 source 聚合（Folder + 单文件 source）

把单 vault 模型换成用户可配置的任务源列表。每个 source 是一个递归
文件夹扫描或单个 `.md` 文件，可选 `project_root` 留给以后的「Open in
VS Code / terminal」动作。

### 后端（Rust）
- `types.rs`：加 `Source`（`id`/`path`/`kind`/`label`/`project_root`）
  和 `SourceKind`（`Folder`/`File`）；`Task` 现在带 `source_id`；
  `AppConfig` 改成 `sources: Vec<Source>` + `default_source_id:
  Option<String>`（vault_path 删除，v0.1 没发布所以不做迁移）
- `error.rs`：`NoVault` → `NoSources`；加 `SourceNotFound` /
  `DuplicateSource` / `InvalidSourcePath` / `CommandFailed`
- `parser.rs`：`parse_file(path)` → `parse_file(path, source_id)`；
  每个 `Task` 透传 `source_id`
- `registry.rs`：重写 —— `rebuild_from_sources(&[Source])`、
  `rebuild_source(&Source)`、`refresh_file(&Source, &Path)`；以
  `(source_id, canonical_path)` 为 key 保证两个 source 覆盖同一文件
  也互相独立；folder source 保留 walkdir 行为，file source 限定到
  单一目标
- `watcher.rs`：`start_watching` → `start_watching_source(&Source, …)`；
  folder = 递归；file = 父目录非递归 + 文件名过滤（canonical 对比）
- `commands.rs`：新 —— `list_sources` / `add_source` / `remove_source` /
  `update_source` / `set_default_source`；`add_task(text, source_id?)`
  （省略 ⇒ 用 `default_source_id`）；`set_vault` 删除；`toggle_task`
  通过 `Task.source_id` 解析 source 并按该 source 范围刷新
- `lib.rs`：单个 `WatcherSlot` → `WatcherSlots = Arc<Mutex<HashMap<source_id, WatcherHandle>>>`；
  setup 为每个 source 各 spawn 一个 scan+watcher；托盘菜单
  「Switch vault folder…」→「Manage sources…」（emit
  `request-manage-sources`）
- 35 个单测通过；新加：`task_carries_source_id`、
  `file_source_collects_only_target_file`、`multi_source_aggregates`、
  `file_source_ignores_sibling_changes`、`file_source_only_fires_for_target_file`

### 前端
- `src/types/task.ts`：对齐 Rust —— `Source` / `SourceKind` / 新
  `AppConfig` 形状；`Task.source_id` 加入
- `src/services/tauri-api.ts`：删掉 `setVault` / `pickVaultFolder`；
  加 `listSources` / `addSource` / `removeSource` / `updateSource` /
  `setDefaultSource`、`pickFolder` / `pickMarkdownFile`，以及
  `sources-changed` / `request-manage-sources` 监听器
- `src/stores/settings.ts`：`pickAndSetVault` 换成 `pickAndAddFolder` /
  `pickAndAddFile`；暴露 `sources` / `hasSources` / `defaultSourceId`
  computed + source CRUD helper
- `src/stores/tasks.ts`：`add(text)` → `add(text, sourceId?)`
- `src/App.vue`：`hasVault` → `hasSources`；订阅 `sources-changed` +
  `request-manage-sources`
- `src/components/EmptyState.vue`：双按钮 onboarding（📁 Folder… /
  📄 File…），调 `pickAndAddFolder` / `pickAndAddFile`
- `src/components/TaskList.vue`：footer chip 变成「📁+」/
  「📄+ N sources」快加器；QuickAdd 输入框加一个行内 source 下拉
  允许按任务选目标（默认到 `default_source_id`）
- v0.2 的 source 分组渲染 + 单 source 快捷按钮（VS Code / terminal）
  在后续 commit 落地 —— 当前 TaskList 仍渲染扁平的 sorted 列表

## 2026-05-14 静默刷新 + 任务排序（未完成在前）

- `src/stores/tasks.ts`：新 `silentRefresh()`（不闪 loading），用于
  toggle / add / fs-event 之后；`refresh()` 在首次加载和手动 ↻ 仍然
  翻 `loading`
- `src/stores/tasks.ts`：新 `sortedTasks` computed —— 未完成在前再
  完成，组内按 `source_file` + `line_number` 稳定排
- `src/components/TaskList.vue`：渲染和计数都改用 `sortedTasks`
  （原来用 `tasks`）
- `src/App.vue`：`tasks-updated` 监听器改成调 `silentRefresh`，不再
  `refresh`

## 2026-05-14 加 Vue UI（EmptyState、TaskItem、TaskList、暗色 CSS）

- `src/components/EmptyState.vue`：vault 选择落地页；先调
  `settings.pickAndSetVault()` 再 `tasks.refresh()`
- `src/components/TaskItem.vue`：单任务行，checkbox + 缩进感知 padding
  + 完成态删除线
- `src/components/TaskList.vue`：完整列表视图 —— 加任务表单、loading /
  error / empty 状态、footer 计数、刷新按钮
- `src/App.vue`：`onMounted` 重接为加载 settings、若 vault 已设则刷
  任务、订阅 `tasks-updated` 事件；`onUnmounted` 清理监听器；按
  `hasVault` computed 在 `EmptyState` 和 `TaskList` 之间路由
- `src/styles/main.css`：CSS 变量（`--bg`、`--bg-hover`、`--fg`、
  `--fg-muted`、`--border`）+ `prefers-color-scheme: dark` 自动暗
  色覆盖
- 脚手架清理：删除 `src/assets/vue.svg`（不再引用）

## 2026-05-14 加前端服务层（types、tauri-api、Pinia store）

- `src/types/task.ts`：`Task` 和 `AppConfig` TypeScript 接口对齐
  Rust 结构体
- `src/services/tauri-api.ts`：`api` 对象封装 6 个 Tauri 命令
  （`get_tasks`、`get_config`、`update_config`、`toggle_task`、`add_task`、
  `set_vault`）、vault 文件夹选择的 `open` 对话框、`tasks-updated`
  事件监听器
- `src/stores/tasks.ts`：`useTaskStore` Pinia store 含 `tasks` /
  `loading` / `error` 状态；`refresh` / `toggle` / `add` action
- `src/stores/settings.ts`：`useSettingsStore` 含 `config` 状态；
  `load` 和 `pickAndSetVault` action
- `src/main.ts`：在挂载前装 Pinia（`createPinia()`）
- `@tauri-apps/plugin-dialog` 加入 npm 依赖

## 2026-05-14 连通整个 app：commands invoke_handler、托盘、watcher 桥

- `lib.rs` 重写：在 `invoke_handler!` 注册 8 个命令（`get_tasks`、
  `get_config`、`update_config`、`toggle_task`、`add_task`、`set_vault`、
  `show_window`、`hide_window`）
- 托盘菜单含 Show window / Hide window / Quit；左键点托盘图标切换窗口
  可见性
- Watcher 在 `setup` hook 里 spawn：后台线程做初始
  `rebuild_from_vault`，然后 `start_watching` 在文件变化时 emit
  `tasks-updated`
- `tauri.conf.json` 窗口改成 380×600、`alwaysOnTop: true`，标签
  `"main"`（原本未标签 800×600）
- Capabilities 更新：`core:default + dialog:default`（删掉没用的
  `opener:default`）
- Cargo.toml 加 `tauri = { version = "2", features = ["tray-icon"] }`
- `lib.rs` 不再初始化 `tauri-plugin-opener`（依赖留在 Cargo.toml
  仅作 warning-only）；`tauri-plugin-dialog` 启用
- `AppConfig` 没用的 import 删掉；`cargo build` 零 warning

## 2026-05-14 加 Tauri IPC 命令（commands.rs + AppState）

- `AppState` 持有 `Arc<RwLock<TaskRegistry>>`、`Arc<RwLock<AppConfig>>`、
  `IgnoreHashes`、`config_path: PathBuf`
- 暴露的命令：`get_tasks`、`get_config`、`update_config`、`toggle_task`、
  `add_task`、`set_vault`、`show_window`、`hide_window`
- `set_vault` 持久化 config，从新 vault 根重建 registry，emit
  `vault-changed` + `tasks-updated` 给前端
- `toggle_task` / `add_task` 写入前先把新内容 hash 注册到
  `IgnoreHashes`，防止 watcher 二次触发循环
- `lib.rs` 加 `mod commands;`；Task 9 把命令接进 `invoke_handler!`
- 上游修复（commit `623b0e8`）：`tempfile` 从 `[dev-dependencies]` 提
  到 `[dependencies]` —— `storage.rs` 的 `atomic_write` 在运行时也用，
  不仅是测试

## 2026-05-14 加 fs watcher（防抖 + 防回写循环）

- `start_watching(vault, ignore, on_event)` 包 `notify-debouncer-full`，
  200ms 防抖；只 emit markdown 路径的 `WatchEvent::Changed` 或
  `WatchEvent::Deleted`
- `IgnoreHashes`（Arc+Mutex HashSet）做一次性回写防御：写入方写入
  前注册内容 hash，watcher 命中后丢事件并清除条目
- 修 `ev.paths` 借用：通过 `ev.event.paths`（owned）访问绕开 Deref
  move-out 错误；`use notify::Watcher` 让 `watch()` 在作用域内
- `WatcherHandle` 包 `Debouncer` 拥有生命周期；drop 停后台线程
- 4 个单测通过（hash register+consume、外部变化检测、hash 抑制、
  非 markdown 忽略），用 `--test-threads=1` 串行跑
- `lib.rs` 加 `mod watcher;`

## 2026-05-14 加 registry（任务索引 + ignore 列表）

- `TaskRegistry` 持有 `HashMap<id, Task>` + `HashMap<PathBuf, Vec<id>>`
  做按文件失效
- `rebuild_from_vault` 用 `walkdir` 遍历 vault，跳过非 markdown 和
  忽略路径
- `refresh_file` 先删过期再重解析;删除文件通过
  `best_effort_canonical`（父目录 canonicalize + 文件名 fallback）
  处理，修 Windows `\\?\` 引发的 key 不匹配
- `is_markdown_target` / `is_not_ignored` 标 `pub` 供 Task 7 的
  watcher 复用
- 忽略列表：`.obsidian`、`.git`、`.trash`、`node_modules`、`~` 前后缀、
  `.swp`、`.tmp`
- 4 个单测通过：vault 扫描、目录过滤、文件刷新、删除处理
- `lib.rs` 加 `pub mod registry;`

## 2026-05-14 加持久化 config（config.rs）

- `load_from` 文件不存在或 JSON 损坏时返回 `AppConfig::default()`
  （防砖）
- `save_to` 创建父目录，用 `std::fs::write` 原子写漂亮 JSON
- `config_file` helper 从 Tauri 的 `app_config_dir` 拼路径
- 3 个单测通过：缺文件、round-trip、损坏 fallback
- `lib.rs` 加 `mod config;`

## 2026-05-14 加原子行级 storage（storage.rs）

- 实现 `toggle_task`（1-indexed、CRLF 安全、`split_inclusive` 保行
  分隔符）和 `append_task`（缺文件就建 + `# Inbox` 标题）
- `atomic_write` 用 `tempfile::NamedTempFile` + `persist`（rename）
  做崩溃安全写；返回 `ContentHash`（SHA-256）供 watcher 防循环
- `replace_first_bracket` 是字节安全的 ASCII 扫描 —— 不用正则，行长
  O(n)
- 9 个单测通过（toggle 双向、CRLF、hash round-trip、非任务行错误、
  各种 append）
- `lib.rs` 加 `mod storage;`

## 2026-05-14 加 markdown 任务解析器（parser.rs）

- 实现 `parse_line`（基于正则，支持 `- * +` bullet、`[ ] [x] [X]`、
  缩进计数、尾部空白 trim）和 `parse_file`（剥 BOM，每个 file+line
  稳定 8 字节 SHA-256 ID）
- 10 个单测通过（各种解析、不同 bullet、缩进、BOM、稳定 ID、非任务行）

## 2026-05-14 加 Rust 基础（error/types/hashing）

- 加 `AppError`（thiserror）+ `Result<T>` 别名 + Tauri 兼容的
  `Serialize` 实现；加 `Task`、`AppConfig`、`ContentHash` 结构体
  和 `hash_content`（sha2 的 SHA-256）
- 加依赖：regex、notify、notify-debouncer-full、sha2、walkdir、
  once_cell、thiserror、tokio、tauri-plugin-dialog、hex、tempfile（dev）

## 2026-05-14 脚手架：Tauri 2 + Vue 3 + TS 项目

- create-tauri-app v4.6.2，模板 vue-ts，identifier
  `com.serendipity.floaty-todo`
- 修模板 bug：`--name` 被当字面量目录名；所有出现都重命名为
  `floaty-todo`
- 装 pinia ^3.0.4
- 冒烟测试通过：360 个 crate 在 2 分 53 秒编译完，Vite 起在
  localhost:1420，`floaty-todo.exe` 启动成功
