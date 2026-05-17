# 任务删除功能 — 设计

> 状态：已批准 · 日期：2026-05-15

## 目标

每个任务行右侧增加一个垃圾桶按钮，让用户可以删除单条任务。删除是可撤销的写操作 ——
与现有的 toggle / edit / add / move 一样写入 history，可通过 `Ctrl+Z` 或历史窗口恢复。

## 交互

- 垃圾桶按钮位于任务行右端，**仅在 hover 该任务行时显示**（与 `FileGroup.vue` 的 ✎
  重命名按钮一致），平时 `opacity: 0`。
- 点击垃圾桶 → 弹出危险确认对话框（复用 `useConfirm`）：
  - 标题：`删除任务`
  - 正文：`确定删除任务「<任务文本>」？此操作可通过 Ctrl+Z 撤销。`
  - 确认按钮为红色危险变体。
- 确认后删除该任务行；删除成功 emit 一条 success toast。
- 删除写入 history，是可撤销事件。

## 后端（Rust）

### 1. `HistoryAction::Delete` 新变体

在 `history.rs` 的 `HistoryAction` 枚举新增（结构上是 `Add` 的镜像）：

```rust
Delete {
    task_id: String,
    before: LineSnapshot,
}
```

- `apply_reverse`（撤销删除）→ `storage::insert_line_at(file, before.line, &before.raw)`
- `apply_forward`（重做删除）→ `storage::remove_line_if_hash(file, before.line, &before.hash)`

`is_external()` 对 `Delete` 返回 `false`（可撤销事件，不是外部修改）。

### 2. `delete_task` 命令

在 `commands.rs` 新增，与 `toggle_task` 同构：

1. 按 `task_id` 查 registry，取不到报 `TaskNotFound`。
2. `find_source_by_id` 拿到 source。
3. 调 `storage::remove_task_line(&task.source_file, task.line_number)` ——
   该函数**已存在**，去掉它的 `#[allow(dead_code)]`。返回 `RemoveResult { new_hash, removed }`。
   `RemoveResult` 同样去掉 `#[allow(dead_code)]`。
4. `state.ignore_hashes.register(new_hash)` 防回写循环。
5. `record_snapshot(&state, &task.source_file)`。
6. `state.registry.write().refresh_file(&source, &task.source_file)`。
7. `push_history(... HistoryAction::Delete { task_id, before: snapshot_with_quadrant(removed, task.quadrant) })`。

### 3. 命令注册

`lib.rs` 的 `invoke_handler` 中注册 `commands::delete_task`。

## 前端

### 4. `services/tauri-api.ts`

新增 `deleteTask: (taskId: string) => invoke<void>('delete_task', { taskId })`。

### 5. `stores/tasks.ts`

新增 `remove(id)` action —— 与 `toggle` / `add` 同构：invoke `api.deleteTask` →
`silentRefresh()` → 成功 `toast.success(t('toast.taskDeleted', { text }))`，失败
`error.value = errorMessage(e)` + `toast.error`。文本用 `trunc()` 截断。
store 保持纯粹，不含确认对话框逻辑。`remove` 加入 store 返回对象。

### 6. `components/TaskItem.vue`

- 在 `.text` span 之后、`</label>` 之前新增垃圾桶按钮：
  `<button type="button" class="del-btn">` 内含 `<Icon name="trash" />`
  （`trash` 图标在 `Icon.vue` 已存在）。
- 按钮 `@click.prevent.stop` —— 阻止事件冒泡触发外层 `<label>` 的隐式 checkbox 勾选。
- 点击处理：调 `useConfirm` 弹危险确认，确认返回 `true` 时调 `tasks.remove(props.task.id)`。
- 样式：`.del-btn` 默认 `opacity: 0`；`.row:hover .del-btn` → `opacity: 1`。
  尺寸 / 配色参考 `FileGroup.vue` 的悬停按钮，hover 自身时颜色偏红以提示危险。

### 7. `types/history.ts`

`HistoryEvent` 联合类型新增成员：

```ts
| (HistoryEventBase & {
    kind: 'delete';
    task_id: string;
    before: LineSnapshot;
  })
```

### 8. `views/HistoryView.vue`

- `eventText`：新增 `if (event.kind === 'delete') return \`删除「${event.before.state?.text ?? ''}」\`;`
- `eventIcon`：新增 `if (event.kind === 'delete') return '🗑';`
- diff 面板无需改动 —— `delete` 事件有 `before` 无 `after`，现有
  `'after' in selected ? selected.after : undefined` 逻辑会让 After 列自动显示 `∅`。

### 9. i18n（`i18n/locales/en.ts` + `zh.ts`）

- `toast.taskDeleted` —— 形如 `已删除「{text}」` / `Deleted "{text}"`
- `confirm.deleteTaskTitle` / `confirm.deleteTaskMessage` / `confirm.deleteTaskConfirm`
  —— 正文带占位符 `{text}`。

## 测试（TDD）

### Rust 单元测试

`storage.rs` —— `remove_task_line`（当前无测试）：
- 删除指定行后，其余行字节级不变（含 CRLF 保留）。
- 目标行不是任务行 → 返回错误，文件不动。
- 行号越界 → 返回 `NotATaskLine`。
- 返回的 `RemoveResult.removed` 快照内容正确（line / raw / hash）。

`history.rs` —— `Delete` 的往返：
- `apply_forward(Delete)` 删除目标行。
- `apply_reverse(Delete)` 把行插回原位，文件恢复原状。
- hash 不匹配（行被外部改动）→ `apply_forward` 返回 `HistoryHashMismatch`。

### 手动验证

`npm run tauri dev` 后：
1. hover 任务行 → 垃圾桶出现，移开 → 消失。
2. 点垃圾桶 → 确认框弹出，文案正确。
3. 确认 → 任务从列表消失，source header 计数 -1，出现 success toast。
4. `Ctrl+Z` → 任务恢复到原位置。
5. 打开历史窗口 → 能看到「删除「…」」事件，可「跳到此处」。
6. 取消确认框 → 任务不动。

## 明确不做

- 批量删除 / 多选删除。
- 「清空已完成任务」之类的批量清理动作。
- 把删除塞进 source header 的快捷动作区。
- 删除确认的「不再提示」选项 —— 删除可撤销，确认已足够轻量。
