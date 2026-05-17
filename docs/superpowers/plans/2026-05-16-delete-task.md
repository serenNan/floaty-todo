# 任务删除功能 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 给每个任务行加一个 hover 显示的垃圾桶按钮，点击经确认对话框后删除任务；删除是可撤销的 history 事件。

**Architecture:** 后端新增 `HistoryAction::Delete`（结构上是 `Add` 的镜像）和 `delete_task` 命令（与 `toggle_task` 同构，复用已存在的 `storage::remove_task_line`）。前端在 `TaskItem.vue` 加垃圾桶按钮，经 `useConfirm` 危险确认后调 `tasks.remove()`。删除自动接入现有 Ctrl+Z / 历史窗口（`peek_undo` 与 `apply_reverse` 无需为新变体改判定逻辑，只需补 match 分支）。

**Tech Stack:** Rust + Tauri 2、Vue 3 + TypeScript + Pinia、vue-i18n。

设计文档：`docs/superpowers/specs/2026-05-15-delete-task-design.md`

---

## 注意：commit 约定

本项目用户偏好「不主动 commit」。下面每个 Task 末尾的 commit 步骤为标准计划格式，**执行时如未获明确授权，跳过 commit 步骤、保留改动即可**。

---

### Task 1: `storage::remove_task_line` 单元测试

`remove_task_line` 函数已存在于 `storage.rs`（带 `#[allow(dead_code)]`），但没有测试。本任务补齐测试，锁定其行为。

**Files:**
- Modify: `src-tauri/src/storage.rs`（在 `#[cfg(test)] mod tests` 内追加测试）

- [ ] **Step 1: 写测试**

在 `src-tauri/src/storage.rs` 的 `mod tests` 内，`move_to_existing_quadrant_returns_before_after_snapshots` 测试之后、`}`（关闭 mod tests）之前追加：

```rust
    #[test]
    fn remove_task_line_deletes_target_and_keeps_others() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "# h\n- [ ] one\n- [ ] two\n- [ ] three\n");
        remove_task_line(&p, 3).unwrap();
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            "# h\n- [ ] one\n- [ ] three\n"
        );
    }

    #[test]
    fn remove_task_line_preserves_crlf() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "- [ ] one\r\n- [ ] two\r\n");
        remove_task_line(&p, 1).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "- [ ] two\r\n");
    }

    #[test]
    fn remove_task_line_rejects_non_task_line() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "# heading\n- [ ] task\n");
        assert!(remove_task_line(&p, 1).is_err());
        // 文件不动
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            "# heading\n- [ ] task\n"
        );
    }

    #[test]
    fn remove_task_line_returns_removed_snapshot() {
        let d = TempDir::new().unwrap();
        let p = write(&d, "a.md", "- [x] done task\n");
        let result = remove_task_line(&p, 1).unwrap();
        assert_eq!(result.removed.line, 1);
        assert_eq!(result.removed.raw, "- [x] done task\n");
        assert_eq!(result.removed.state.as_ref().unwrap().done, true);
        assert_eq!(result.removed.state.as_ref().unwrap().text, "done task");
    }
```

- [ ] **Step 2: 运行测试，确认通过**

Run: `cd src-tauri && cargo test --lib storage::tests::remove_task_line`
Expected: PASS — 4 个测试全过（函数已存在且行为正确，这些测试锁定现有行为）。

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/storage.rs
git commit -m "test: cover storage::remove_task_line"
```

---

### Task 2: `HistoryAction::Delete` 变体 + 撤销/重做逻辑

**Files:**
- Modify: `src-tauri/src/history.rs`

- [ ] **Step 1: 写往返测试**

在 `src-tauri/src/history.rs` 的 `mod tests` 内，`apply_reverse_add_removes_added_line_and_forward_reinserts_it` 测试之后追加：

```rust
    #[test]
    fn apply_forward_delete_removes_line_and_reverse_reinserts_it() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("todo.md");
        std::fs::write(&file, "- [ ] keep\n- [ ] gone\n").unwrap();
        let mut event = event(
            "01",
            HistoryAction::Delete {
                task_id: "task-a".to_string(),
                before: line(2, "gone", false),
            },
        );
        event.file = file.clone();

        apply_forward(&event).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "- [ ] keep\n");

        apply_reverse(&event).unwrap();
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "- [ ] keep\n- [ ] gone\n"
        );
    }

    #[test]
    fn apply_forward_delete_is_hash_guarded() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("todo.md");
        std::fs::write(&file, "- [ ] changed externally\n").unwrap();
        let mut event = event(
            "01",
            HistoryAction::Delete {
                task_id: "task-a".to_string(),
                before: line(1, "original text", false),
            },
        );
        event.file = file.clone();
        let err = apply_forward(&event).unwrap_err();
        assert!(matches!(err, crate::error::AppError::HistoryHashMismatch { .. }));
    }
```

- [ ] **Step 2: 运行测试，确认编译失败**

Run: `cd src-tauri && cargo test --lib history`
Expected: FAIL — 编译错误 `no variant named \`Delete\` found for enum \`HistoryAction\``。

- [ ] **Step 3: 新增 `Delete` 变体**

在 `src-tauri/src/history.rs` 的 `HistoryAction` 枚举里，`Move { ... }` 变体之后、`ExternalEdit { ... }` 之前插入：

```rust
    Delete {
        task_id: String,
        before: LineSnapshot,
    },
```

`is_external()` 无需改动 —— 它的 `matches!` 只匹配 `ExternalEdit`，`Delete` 自动返回 `false`（可撤销事件，会被 `peek_undo` / `peek_redo` 正常拾取）。

- [ ] **Step 4: 在 `apply_reverse` 加 `Delete` 分支**

在 `src-tauri/src/history.rs` 的 `apply_reverse` 函数 `match &event.action` 里，`HistoryAction::Move { .. }` 分支之后、`HistoryAction::ExternalEdit { .. } => None,` 之前插入：

```rust
        HistoryAction::Delete { before, .. } => {
            Some(storage::insert_line_at(&event.file, before.line, &before.raw)?)
        }
```

- [ ] **Step 5: 在 `apply_forward` 加 `Delete` 分支**

在 `src-tauri/src/history.rs` 的 `apply_forward` 函数 `match &event.action` 里，`HistoryAction::Move { .. }` 分支之后、`HistoryAction::ExternalEdit { .. } => None,` 之前插入：

```rust
        HistoryAction::Delete { before, .. } => Some(storage::remove_line_if_hash(
            &event.file,
            before.line,
            &before.hash,
        )?),
```

- [ ] **Step 6: 运行测试，确认通过**

Run: `cd src-tauri && cargo test --lib history`
Expected: PASS — 含新增 2 个测试在内的 history 测试全过。

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/history.rs
git commit -m "feat: add HistoryAction::Delete with reverse/forward apply"
```

---

### Task 3: `delete_task` 命令 + 命令注册

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/storage.rs:46,216`（去掉两处 `#[allow(dead_code)]`）
- Modify: `src-tauri/src/lib.rs:269`（invoke_handler 注册）

- [ ] **Step 1: 新增 `delete_task` 命令**

在 `src-tauri/src/commands.rs` 的 `add_task` 命令（以 `Ok(())\n}` 结束）之后，`get_history` 命令之前插入：

```rust
/// Delete a task line and record it as an undoable `Delete` history event.
/// Structurally mirrors `toggle_task`: look the task up in the registry,
/// remove its line via `storage::remove_task_line`, register the new hash
/// to suppress the watcher loop, refresh the registry, then push history.
#[tauri::command]
pub fn delete_task(state: State<'_, AppState>, app: AppHandle, task_id: String) -> Result<()> {
    let task = {
        let reg = state.registry.read().unwrap();
        reg.get(&task_id).cloned().ok_or_else(|| AppError::TaskNotFound(task_id.clone()))?
    };
    let source = find_source_by_id(&state, &task.source_id)?;
    let result = storage::remove_task_line(&task.source_file, task.line_number)?;
    let before = snapshot_with_quadrant(result.removed, task.quadrant);
    state.ignore_hashes.register(result.new_hash);
    record_snapshot(&state, &task.source_file);
    state.registry.write().unwrap().refresh_file(&source, &task.source_file)?;
    push_history(
        &state,
        &app,
        task.source_id.clone(),
        task.source_file.clone(),
        HistoryAction::Delete { task_id, before },
    )?;
    Ok(())
}
```

- [ ] **Step 2: 去掉 `storage.rs` 两处 `#[allow(dead_code)]`**

`delete_task` 现在是 `RemoveResult` 与 `remove_task_line` 的真实调用方，`dead_code` 抑制不再需要。

在 `src-tauri/src/storage.rs` 删除 `RemoveResult` 结构体上方的 `#[allow(dead_code)]`：

```rust
#[derive(Debug, Clone)]
pub struct RemoveResult {
    pub new_hash: ContentHash,
    pub removed: LineSnapshot,
}
```

并删除 `remove_task_line` 函数上方的 `#[allow(dead_code)]`，使其变为：

```rust
/// Delete the task on `line_number` (1-indexed) from `path`. Verifies the
/// target line still parses as a task — otherwise the registry is stale and
/// we'd be deleting random content. Returns the new content hash.
pub fn remove_task_line(path: &Path, line_number: usize) -> Result<RemoveResult> {
```

- [ ] **Step 3: 在 `lib.rs` 注册命令**

在 `src-tauri/src/lib.rs` 的 `invoke_handler` 列表里，`commands::add_task,` 这一行之后插入：

```rust
            commands::delete_task,
```

- [ ] **Step 4: 编译 + 全量测试**

Run: `cd src-tauri && cargo test --lib`
Expected: PASS — 编译无 warning（两处 `dead_code` 已消除），所有测试通过。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/storage.rs src-tauri/src/lib.rs
git commit -m "feat: add delete_task command"
```

---

### Task 4: 前端 API + store action

**Files:**
- Modify: `src/services/tauri-api.ts`
- Modify: `src/stores/tasks.ts`

- [ ] **Step 1: 在 `tauri-api.ts` 加 `deleteTask`**

在 `src/services/tauri-api.ts` 的 `api` 对象里，`addTask` 方法块（以 `},` 结束）之后、空行与 `getHistory` 之前插入：

```ts
  deleteTask: (taskId: string) => invoke<void>('delete_task', { taskId }),
```

- [ ] **Step 2: 在 `tasks.ts` 加 `remove` action**

在 `src/stores/tasks.ts` 的 `add` 函数之后、`return { ... }` 之前插入：

```ts
  async function remove(id: string) {
    const task = tasks.value.find(tk => tk.id === id);
    const text = task ? trunc(task.text) : '';
    try {
      await api.deleteTask(id);
      await silentRefresh();
      toast.success(t('toast.taskDeleted', { text }));
    } catch (e: any) {
      error.value = errorMessage(e);
      toast.error(t('toast.operationFailed', { reason: error.value }));
    }
  }
```

- [ ] **Step 3: 把 `remove` 加入 store 返回对象**

把 `src/stores/tasks.ts` 末尾的 return 语句改为：

```ts
  return { tasks, sortedTasks, loading, error, refresh, silentRefresh, toggle, update, add, remove };
```

- [ ] **Step 4: 类型检查**

Run: `npx vue-tsc --noEmit -p tsconfig.json`
Expected: 无新增报错（`toast.taskDeleted` 键将在 Task 5 补上，此处类型检查不依赖 i18n 键存在；若 vue-tsc 不可用，跳过此步，留到 Task 8 的 `npm run tauri dev` 一并验证）。

- [ ] **Step 5: Commit**

```bash
git add src/services/tauri-api.ts src/stores/tasks.ts
git commit -m "feat: add deleteTask api and tasks.remove action"
```

---

### Task 5: i18n 文案

**Files:**
- Modify: `src/i18n/locales/zh.ts`
- Modify: `src/i18n/locales/en.ts`

- [ ] **Step 1: 中文文案**

在 `src/i18n/locales/zh.ts` 的 `confirm:` 对象里，`removeSourceConfirm` 这一行之后插入：

```ts
    deleteTaskTitle: '删除任务',
    deleteTaskMessage: '确定删除任务「{text}」？此操作可通过 Ctrl+Z 撤销。',
    deleteTaskConfirm: '删除',
```

在同文件 `toast:` 对象里，`taskEdited` 这一行之后插入：

```ts
    taskDeleted: '已删除「{text}」',
```

- [ ] **Step 2: 英文文案**

在 `src/i18n/locales/en.ts` 的 `confirm:` 对象里，`removeSourceConfirm` 这一行之后插入：

```ts
    deleteTaskTitle: 'Delete task',
    deleteTaskMessage: 'Delete task "{text}"? This can be undone with Ctrl+Z.',
    deleteTaskConfirm: 'Delete',
```

在同文件 `toast:` 对象里，`taskEdited` 这一行之后插入：

```ts
    taskDeleted: 'Deleted "{text}"',
```

- [ ] **Step 3: Commit**

```bash
git add src/i18n/locales/zh.ts src/i18n/locales/en.ts
git commit -m "feat: add i18n strings for task deletion"
```

---

### Task 6: `TaskItem.vue` 垃圾桶按钮

**Files:**
- Modify: `src/components/TaskItem.vue`

- [ ] **Step 1: 扩充 `<script setup>` 的 import 与逻辑**

在 `src/components/TaskItem.vue` 的 `<script setup lang="ts">` 区，现有 import 之后追加：

```ts
import { useI18n } from 'vue-i18n';
import Icon from './icons/Icon.vue';
import { confirm } from '../composables/useConfirm';
```

在 `const tasks = useTaskStore();` 这一行之后追加：

```ts
const { t } = useI18n();
```

在 `openLink` 函数之后追加删除处理函数：

```ts
async function onDelete() {
  const ok = await confirm({
    title: t('confirm.deleteTaskTitle'),
    message: t('confirm.deleteTaskMessage', { text: props.task.text }),
    confirmText: t('confirm.deleteTaskConfirm'),
    danger: true,
  });
  if (ok) await tasks.remove(props.task.id);
}
```

- [ ] **Step 2: 在模板加垃圾桶按钮**

在 `src/components/TaskItem.vue` 模板里，`.text` 的 `</span>` 之后、`</label>` 之前插入：

```html
    <button
      type="button"
      class="del-btn"
      :title="t('confirm.deleteTaskConfirm')"
      @click.prevent.stop="onDelete"
    >
      <Icon name="trash" :size="14" />
    </button>
```

`@click.prevent.stop` 必须保留 —— 阻止点击冒泡到外层 `<label>` 触发隐式的 checkbox 勾选。

- [ ] **Step 3: 加样式**

在 `src/components/TaskItem.vue` 的 `<style scoped>` 末尾、`input[type="checkbox"] { ... }` 规则之后追加：

```css
.del-btn {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  padding: 0;
  margin-top: 1px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  border-radius: 4px;
  cursor: pointer;
  opacity: 0;
  transition: opacity 120ms ease-out, background 120ms ease-out, color 120ms ease-out;
}

/* Mirrors FileGroup's hover-reveal ✎ button: only visible while the row is
   hovered, so the row stays visually quiet at rest. */
.row:hover .del-btn {
  opacity: 1;
}

.del-btn:hover {
  background: color-mix(in srgb, #ef4444 18%, transparent);
  color: #ef4444;
}
```

- [ ] **Step 4: 类型检查**

Run: `npx vue-tsc --noEmit -p tsconfig.json`
Expected: 无新增报错（若 vue-tsc 不可用，跳过，留到 Task 8 验证）。

- [ ] **Step 5: Commit**

```bash
git add src/components/TaskItem.vue
git commit -m "feat: add hover trash button to TaskItem"
```

---

### Task 7: 历史窗口渲染 `delete` 事件

**Files:**
- Modify: `src/types/history.ts`
- Modify: `src/views/HistoryView.vue`

- [ ] **Step 1: 在 `history.ts` 加 `delete` 联合成员**

在 `src/types/history.ts` 的 `HistoryEvent` 联合类型里，`move` 成员之后、`external_edit` 成员之前插入：

```ts
  | (HistoryEventBase & {
      kind: 'delete';
      task_id: string;
      before: LineSnapshot;
    })
```

- [ ] **Step 2: 在 `HistoryView.vue` 的 `eventText` 加分支**

在 `src/views/HistoryView.vue` 的 `eventText` 函数里，`if (event.kind === 'move') { ... }` 块之后、`return \`外部修改 ...\`` 之前插入：

```ts
  if (event.kind === 'delete') return `删除「${event.before.state?.text ?? ''}」`;
```

- [ ] **Step 3: 在 `eventIcon` 加分支**

在 `src/views/HistoryView.vue` 的 `eventIcon` 函数里，`if (event.kind === 'move') return '↕';` 这一行之后插入：

```ts
  if (event.kind === 'delete') return '🗑';
```

diff 面板无需改动：`delete` 事件有 `before` 无 `after`，模板里 `'after' in selected ? selected.after : undefined` 会让 After 列自动显示 `∅`。

- [ ] **Step 4: 类型检查**

Run: `npx vue-tsc --noEmit -p tsconfig.json`
Expected: 无报错。`eventText` / `eventIcon` 因 union 收窄能正确识别 `delete` 分支（若 vue-tsc 不可用，跳过，留到 Task 8 验证）。

- [ ] **Step 5: Commit**

```bash
git add src/types/history.ts src/views/HistoryView.vue
git commit -m "feat: render delete events in history window"
```

---

### Task 8: 手动验证

**Files:** 无（手动测试）

- [ ] **Step 1: 启动 dev**

Run: `npm run tauri dev`
Expected: 编译无错，主窗口正常出现。

- [ ] **Step 2: 走查删除流程**

逐项确认：
1. 鼠标悬停一个任务行 → 行右端出现垃圾桶图标；移开 → 图标消失。
2. 点垃圾桶 → 弹出确认对话框，标题「删除任务」，正文含任务文本且提到 Ctrl+Z，确认按钮为红色。
3. 点「删除」→ 任务从列表消失，所属 source / 象限的计数 -1，底部出现「已删除「…」」success toast。
4. 按 `Ctrl+Z` → 任务恢复到原来的位置（文件、行号、勾选状态一致）。
5. 点底部 🕒 打开历史窗口 → 时间线里有一条「删除「…」」事件（🗑 图标），右侧 detail 的 Before 显示被删的行、After 显示 `∅`，「跳到此处」可用。
6. 点垃圾桶后在确认框点「取消」→ 任务不动，无 toast。
7. 切换语言到 English → 确认框与 toast 文案为英文。

- [ ] **Step 3: 确认无回归**

确认勾选 checkbox、点任务文本编辑、SourceGroup 折叠等原有交互不受垃圾桶按钮影响（尤其点垃圾桶不会误触发勾选）。

- [ ] **Step 4: Commit（如有遗留改动）**

```bash
git add -A
git commit -m "chore: delete-task manual verification pass"
```

---

## Self-Review

- **Spec coverage:** 设计文档 9 个编号小节 + 测试 + 不做 —— §1/§2 → Task 2/Task 3；§3 → Task 3 Step 3；§4/§5 → Task 4；§6 → Task 6；§7/§8 → Task 7；§9 → Task 5；Rust 测试 → Task 1 + Task 2；手动验证 → Task 8。无遗漏。
- **Placeholder scan:** 无 TBD / “类似 Task N” / 无代码的描述步骤。每个改代码的步骤都给了完整代码。
- **Type consistency:** Rust `HistoryAction::Delete { task_id, before }` 字段在 enum 定义、`apply_reverse`/`apply_forward`、`delete_task`、测试 helper `event(...)` 中一致；TS `kind: 'delete'` + `before: LineSnapshot` 在 `history.ts` 与 `HistoryView.vue` 一致；`api.deleteTask` / `tasks.remove` 命名在 api、store、组件间一致；i18n 键 `confirm.deleteTask*` / `toast.taskDeleted` 在定义（Task 5）与使用（Task 6 / store）间一致。
