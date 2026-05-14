# Quadrant View — Source 内艾森豪威尔四象限分组

**Status:** Draft (awaiting user review)
**Date:** 2026-05-15
**Author:** brainstorming session

## 背景

Floaty Todo 当前的 source 内任务渲染层级:

- File source: `SourceGroup → TaskItem`
- Folder source: `SourceGroup → FileGroup → TaskItem`

任务排序固定 `undone-first`, 没有维度区分优先级。当 source 里 30+ 任务时, 缺少"哪些必须先做"的视觉信号。

用户希望把每个 markdown 文件**内部**按艾森豪威尔矩阵(紧急 × 重要)的四象限分组渲染, 与项目内 `D:\Projects\Floaty-todo\TODO.md` 已采纳的四象限结构、以及 `~/.skills-manager/skills/todo/SKILL.md` 中 `/todo` skill 的写入模板保持一致。

## 目标与非目标

### 目标

- markdown 文件内出现 `## 🔴 紧急+重要` / `## 🟡 重要不紧急` / `## 🟠 紧急不重要` / `## 🟢 不紧急不重要` 这类含 quadrant emoji 的标题时, 标题下的任务在 UI 上按象限分组渲染
- 不引入 sidecar、不修改任务行文本、不破坏 line_number 稳定性
- File source 与 Folder source 行为对齐 (folder source 内每个文件各自有自己的四象限)
- `add_task` 写入时支持指定象限, 自动在对应 header 之后插入

### 非目标

- 不引入"按象限跨 source 聚合"视图 (source 仍然完全隔离)
- 不引入手动拖拽改变 quadrant 的 UI (本期; 后续可叠加)
- 不解析 Obsidian Tasks 优先级 emoji (⏫🔼🔽⏬) 映射到 quadrant (PLAN.md Q4 是独立的 v1.0 议题)
- 不强制旧 vault 迁移 — 没有 quadrant header 的文件自动降级到"未分类"

## 数据模型

### Rust (`src-tauri/src/types.rs`)

新增 `Quadrant` 枚举:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Quadrant {
    UrgentImportant,        // 🔴
    NotUrgentImportant,     // 🟡
    UrgentNotImportant,     // 🟠
    NotUrgentNotImportant,  // 🟢
}
```

`Task` 增加字段:

```rust
pub struct Task {
    // ... 现有字段
    pub quadrant: Option<Quadrant>,   // None = "未分类"
}
```

`AppConfig` 增加字段:

```rust
pub struct AppConfig {
    // ... 现有字段
    #[serde(default = "default_true")]
    pub auto_create_quadrant_headers: bool,
}
fn default_true() -> bool { true }
```

### TypeScript (`src/types/task.ts`)

镜像上述 `Quadrant` 枚举和 `Task.quadrant`, `AppConfig.auto_create_quadrant_headers`。

## 解析逻辑 (`src-tauri/src/parser.rs`)

`parse_file` 改为 stateful 扫描:

```text
current_quadrant: Option<Quadrant> = None
for each line:
    if line matches header regex (^#{1,6}\s+.+$):
        if header text contains 🔴 → current_quadrant = UrgentImportant
        elif 🟡 → NotUrgentImportant
        elif 🟠 → UrgentNotImportant
        elif 🟢 → NotUrgentNotImportant
        else → current_quadrant 保持不变 (子标题继承父标题)
    elif line matches task regex:
        task.quadrant = current_quadrant
        push task
```

### 匹配规则

- 任意 header 级别 (`#` 到 `######`) 都参与判定
- emoji 出现在标题文字任意位置即识别 (不限定行首紧邻 `##`)
- 同一文件多次出现同一象限 emoji 标题, 任务全部归到同一象限 (自然合并)
- header 不含 quadrant emoji → 不重置 `current_quadrant` (子标题继承)
- 任务上方完全没出现过 quadrant emoji 标题 → `quadrant = None`

### Edge case

| 情况 | 处理 |
|---|---|
| `## 🔴 紧急+重要` 后又出现 `## 备注` | "备注"小节里的任务仍然算 🔴 (子标题继承) |
| 同文件出现两次 `## 🔴` | 两次下面的任务合并到同一 🔴 象限 |
| 文件首行就是 `- [ ] 啥` (没 header) | quadrant = None |
| `## 🔴🟡 混合` 标题 | 取**第一个**匹配到的 emoji (按 🔴 → 🟡 → 🟠 → 🟢 顺序), 简单可预测 |

## 写入逻辑 (`src-tauri/src/storage.rs`)

`toggle_task` / `update_task_text` 不变。

新增 `append_task_to_quadrant`:

```rust
pub fn append_task_to_quadrant(
    file: &Path,
    text: &str,
    quadrant: Option<Quadrant>,
    auto_create_header: bool,
) -> Result<()>
```

行为:

1. **`quadrant = None`**: 沿用现有 `append_task` 行为, 追加到文件末尾
2. **`quadrant = Some(q)`**: 扫文件找第一个匹配的 quadrant header → 该 section 末尾 (下一个 `#{1,header_level}` 标题前 或 文件末尾) 插入 `- [ ] text`
3. **找不到目标 header 且 `auto_create_header=true`**: 在文件末尾追加 `\n## <emoji> <name>\n\n- [ ] text\n` (emoji + 中文名按 todo skill 模板); 若文件末尾不是换行则先补一个换行, 避免与原最后一行黏连
4. **找不到目标 header 且 `auto_create_header=false`**: 返回 `AppError::QuadrantHeaderMissing(quadrant)`, 前端 toast 提示用户

### 副作用 — line_number 漂移

在文件中间插入新行会让后面任务的 `line_number` 全部 +1, 进而让 `task_id = hash(path+line)` 全部变化。

**对齐 PLAN.md 一节"绝不破坏 line_number 稳定性"**: 该原则的本意是 "不挪现有任务行 / 不跨文件移动 / 不自动归档插段"。**新增一行**是用户主动操作的 INSERT, 不是后台自动 mutation, 不在被禁之列。

处理:

- 写入完成后, `commands::add_task` 触发 `registry.refresh_file()` 完整重扫该文件
- registry 重扫 → emit `tasks-updated` → 前端 store re-fetch → UI 自然刷新, 用户无感
- 跨 watcher 用现有 SHA-256 IgnoreHashes 机制防回环

### `commands::add_task` 接口变化

```rust
#[tauri::command]
async fn add_task(
    text: String,
    source_id: Option<String>,
    quadrant: Option<Quadrant>,    // 新增
) -> Result<()>
```

向后兼容: `quadrant=None` 时行为等同当前实现。

## 前端 UI

### 层级

| Source 类型 | 渲染层级 |
|---|---|
| File source | `SourceGroup → QuadrantGroup × ≤5 → TaskItem` |
| Folder source | `SourceGroup → FileGroup × N → QuadrantGroup × ≤5 → TaskItem` |

### 新组件 `src/components/QuadrantGroup.vue`

- props: `quadrant: Quadrant | null`, `tasks: Task[]`, `collapsed?: boolean`
- header: emoji + 中文名 + count badge
- 折叠/展开复用 `useCollapse` (响应全局 collapse-all)
- **count = 0 时该组件完全不渲染** (不占空间, 不显示"空🔴象限")

### 中文名 + emoji 映射

| Quadrant | Emoji | 中文 | 英文 (i18n) |
|---|---|---|---|
| UrgentImportant | 🔴 | 紧急+重要 | Urgent · Important |
| NotUrgentImportant | 🟡 | 重要不紧急 | Important · Not Urgent |
| UrgentNotImportant | 🟠 | 紧急不重要 | Urgent · Not Important |
| NotUrgentNotImportant | 🟢 | 不紧急不重要 | Neither |
| None | ⚪ | 未分类 | Unsorted |

### 渲染顺序

固定 🔴 → 🟡 → 🟠 → 🟢 → ⚪ (按艾森豪威尔矩阵优先级降序, 未分类置末)。

### 视图模式

**Auto, 不加配置开关**:

- 文件解析后有任一任务 `quadrant != None` → 走 QuadrantGroup 视图
- 整个文件所有任务 `quadrant = None` → 退化为单个 ⚪ 未分类组, 视觉上等同扁平列表

完全无配置, 用户加 quadrant header 后下次刷新自动切换。

### QuickAdd 输入栏

QuickAdd 当前已有 source 选择器。增加紧凑 quadrant 选择器:

```
[ 任务文本............................ ] [src▾] [🔴|🟡|🟠|🟢|⚪]
```

- 5 个 emoji 按钮 (single-select toggle); 默认选中上次使用的 (localStorage `floaty.lastQuadrant`), 首次启动默认 ⚪ 未分类
- 前端永远调 `add_task(text, source_id, quadrant)`, 由后端根据 `quadrant` 是否为 None 分发到 `append_task` (文件末尾) 或 `append_task_to_quadrant`
- header 不存在 + `auto_create_quadrant_headers=false` → 后端返回 `QuadrantHeaderMissing`, 前端显示 toast 错误

## 配置

`AppConfig` 增加一项, Settings 面板 "Behavior" section 暴露:

- **自动创建象限标题** (`auto_create_quadrant_headers`, 默认 ON)
  - ON: QuickAdd 写入不存在的象限时, 自动追加 `## 🔴 紧急+重要` 等 header
  - OFF: 写入失败, 给 toast 让用户手动加 header

## 兼容性与现有约束对齐

| 约束 (来自 PLAN.md) | 本设计 |
|---|---|
| 第一节 #6 line_number 稳定 | ✅ 已有任务行不动; 新增行视为用户主动 INSERT |
| 第十节 #6 任务行不加 emoji 元数据 | ✅ emoji 在 header, 任务行文本不变 |
| 第十三节 Q1 markdown 是真相之源 | ✅ quadrant 100% 从文件结构推断, 零 sidecar 状态 |
| 第十三节 Q4 应用自身添加用 `- [ ]` | ✅ 写入仍是 `- [ ]`, emoji 只出现在 header |
| AI 集成约定 | ✅ `/todo` skill 已按本结构写, 双向兼容 |

旧文件不包含 quadrant header → 解析后所有任务 `quadrant = None` → UI 全部归到 ⚪ 未分类, 行为等同当前扁平视图。**零迁移成本**。

## 测试覆盖

### parser.rs

- `parse_file_quadrant_basic`: `## 🔴` 下的任务 quadrant = UrgentImportant
- `parse_file_quadrant_inheritance`: `## 🔴 X` → `### 子分组` → 任务仍 = UrgentImportant
- `parse_file_quadrant_multiple_same`: 两次出现 `## 🔴`, 下面所有任务都 = UrgentImportant
- `parse_file_quadrant_missing`: 没 quadrant header 的任务 = None
- `parse_file_quadrant_any_header_level`: `# 🔴` 和 `###### 🔴` 都识别
- `parse_file_quadrant_emoji_anywhere`: `## 今天 🔴 紧急` 也识别
- `parse_file_quadrant_mixed_picks_first`: `## 🔴🟡` → 取 🔴

### storage.rs

- `append_to_existing_quadrant`: 现有 `## 🔴` section 末尾插入, 验证 line_number
- `append_to_quadrant_inserts_before_next_header`: 下一个 `## 🟡` 之前插入
- `append_to_quadrant_at_file_end`: 目标 quadrant 是最后一个 section, 插入到文件末尾
- `auto_create_header_when_missing`: 不存在的 quadrant 自动建 header
- `auto_create_disabled_returns_error`: `auto_create=false` 返回 `QuadrantHeaderMissing`
- `quadrant_none_falls_back_to_append`: `None` 等同 `append_task`

### 前端

- QuadrantGroup count=0 不渲染
- 5 个 quadrant 渲染顺序固定 🔴→🟡→🟠→🟢→⚪
- Folder source 内 FileGroup × QuadrantGroup 双层嵌套折叠正常
- QuickAdd 选中 quadrant 后调用 `add_task` 带正确参数
- localStorage `floaty.lastQuadrant` 跨重启保持

## 模块影响清单

| 模块 | 改动量 |
|---|---|
| `src-tauri/src/types.rs` | 新增 `Quadrant` enum, `Task.quadrant`, `AppConfig.auto_create_quadrant_headers` |
| `src-tauri/src/parser.rs` | `parse_file` 改 stateful 扫描, 新增 header 识别 |
| `src-tauri/src/storage.rs` | 新增 `append_task_to_quadrant` |
| `src-tauri/src/commands.rs` | `add_task` 接受 `quadrant` 参数 |
| `src-tauri/src/error.rs` | 新增 `QuadrantHeaderMissing` 变体 |
| `src-tauri/src/config.rs` | `AppConfig` 默认值兼容 |
| `src/types/task.ts` | 镜像类型 |
| `src/services/tauri-api.ts` | `addTask` 签名 |
| `src/stores/tasks.ts` | `add(text, sourceId?, quadrant?)` |
| `src/components/QuadrantGroup.vue` | **新建** |
| `src/components/SourceGroup.vue` | 渲染层级改 (file source 走 QuadrantGroup) |
| `src/components/FileGroup.vue` | 渲染层级改 (内嵌 QuadrantGroup) |
| `src/components/TaskList.vue` | QuickAdd 加 quadrant 选择器 |
| `src/i18n/locales/{en,zh}.ts` | 新增 quadrant 名称 + Settings 文案 |
| `src/views/SettingsView.vue` | 新增 "Behavior" 区域 + auto-create header toggle |

## 开放问题 (本期搁置)

- 拖拽任务跨 quadrant 移动 — 涉及行级 cut+paste, 复杂度独立, 留待后续
- "全部展开/全部折叠"是否要细分到 quadrant 级 — 当前 useCollapse 全局 token 已够用, 不细分
- File source 标签为 `TODO.md` 时, 是否在 SourceGroup header 上加 "象限模式" 小标记 — 视觉冗余, 不加
