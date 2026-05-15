# Toast 提示系统 + 历史按钮未读外部修改徽章

**日期：** 2026-05-15
**作者：** brainstorming（Claude + Serendipity）
**状态：** 已批准设计，待写实现计划

## 1. 背景

当前所有 in-app 操作（toggle / add / update / source CRUD / hub 配置 / undo / redo / jumpTo）出错后只是把 `error.value` 写到 Pinia store，UI 上没有任何反馈。用户做完一个动作不知道是否成功；watcher 检测到外部修改进 history 也只是默默写一条事件，主窗口看不到。

目标：

1. 提供轻量、统一的操作反馈机制（success / warning / error / info），位置在程序底部中间偏上（不挡 footer）
2. 外部修改不刷屏 toast，改为在主窗口历史按钮上挂未读红点徽章，用户打开历史窗口就视为「都看过」

## 2. 设计范围

### 包含
- 全局 toast 系统（composable + 组件 + 4 个变体）
- 三个 Pinia store（tasks / settings / history）里所有写操作的成功/失败 toast 接入
- 历史按钮的「未看过外部修改」红点徽章 + 跨窗口同步

### 不包含（YAGNI）
- toast 里的「撤销」按钮（一句 toast 上面挂操作按钮，复杂度先不上）
- 长按 / 滑动消除等手势
- toast 历史日志页面
- 通过系统通知（OS notification）补发 —— 仍是窗口内 UI

## 3. 架构

沿用项目已有的「单例 composable + App 根挂全局组件」模式（参考 `useConfirm` + `<ConfirmDialog />`、`useTaskEditor` + `<TaskEditorDialog />`）。

```
src/composables/useToast.ts          ← 模块级单例状态 + API
src/components/ToastContainer.vue    ← 渲染队列 + 动画 + 关闭
src/App.vue                          ← 挂一次（主窗口）
src/views/HistoryView.vue            ← 挂一次（历史窗口）
```

**两个独立 Tauri 窗口各自一个 in-memory 队列**，触发动作的窗口出自己的反馈。跨窗口不共享 toast 队列。

### 3.1 `useToast.ts` 设计

```ts
export type ToastVariant = 'success' | 'warning' | 'error' | 'info';

export interface ToastItem {
  id: number;                // 自增 id
  variant: ToastVariant;
  message: string;
  duration: number;          // ms，0 表示不自动消失
  createdAt: number;         // 用于内部 timer / hover 暂停
}

export interface ToastOptions {
  duration?: number;         // 覆盖默认 duration
}

// 默认 duration（ms）
const DEFAULTS: Record<ToastVariant, number> = {
  success: 2000,
  info:    3000,
  warning: 4000,
  error:   6000,
};

// 公开 API（模块级单例）
export const toast: {
  success: (msg: string, opts?: ToastOptions) => void;
  warning: (msg: string, opts?: ToastOptions) => void;
  error:   (msg: string, opts?: ToastOptions) => void;
  info:    (msg: string, opts?: ToastOptions) => void;
};

// 供组件使用
export function useToastState(): {
  items: Ref<ToastItem[]>;     // 当前显示队列（含 pending）
  dismiss: (id: number) => void;
  pause:   (id: number) => void;  // hover 时暂停
  resume:  (id: number) => void;
};
```

**内部行为**：
- `MAX_VISIBLE = 3`，超出排队。一条消失后队列前移
- 每条 toast 启动一个 `setTimeout`，到点 dismiss
- `pause(id)` 清掉 timer 并记录剩余时间；`resume(id)` 用剩余时间重启
- 模块级 state，所以同一窗口内的任何组件 import `toast` 都共用同一个队列

### 3.2 `ToastContainer.vue` 设计

```vue
<template>
  <Teleport to="body">
    <div class="toast-stack">
      <TransitionGroup name="toast">
        <div
          v-for="item in visibleItems"
          :key="item.id"
          class="toast"
          :class="['toast-' + item.variant]"
          @mouseenter="pause(item.id)"
          @mouseleave="resume(item.id)"
        >
          <span class="stripe"></span>
          <Icon :name="iconFor(item.variant)" :size="14" />
          <span class="text">{{ item.message }}</span>
          <button class="close" @click="dismiss(item.id)" tabindex="-1">×</button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>
```

**关键样式（精简版）**

```css
.toast-stack {
  position: fixed;
  bottom: 52px;                /* 压在 footer 上方，footer ~48px */
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  flex-direction: column-reverse;  /* 新的从下方插入 */
  gap: 6px;
  z-index: 1000;
  pointer-events: none;        /* 让队列容器不挡点击 */
}
.toast {
  pointer-events: auto;        /* 单条 toast 可交互 */
  display: grid;
  grid-template-columns: 3px 14px minmax(0, 1fr) 18px;
  gap: 8px;
  align-items: center;
  min-width: 180px;
  max-width: 280px;
  padding: 7px 10px;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
  font-size: 0.82rem;
}
.toast .stripe { width: 3px; height: 20px; border-radius: 2px; }
.toast-success .stripe { background: #16a34a; }
.toast-warning .stripe { background: #f59e0b; }
.toast-error   .stripe { background: #ef4444; }
.toast-info    .stripe { background: #64748b; }

.toast .close {
  opacity: 0;
  transition: opacity 120ms;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
}
.toast:hover .close { opacity: 1; }

/* 进入 / 离开动画 */
.toast-enter-from { opacity: 0; transform: translateY(8px); }
.toast-leave-to   { opacity: 0; transform: translateY(8px); }
.toast-enter-active, .toast-leave-active { transition: all 200ms ease-out; }
```

**Pointer-events 处理**：`.toast-stack` 不挡点击，单条 `.toast` 自己可交互（hover 暂停 + 点 × 关闭）。

### 3.3 Pinia store 接入

修改三个 store 的写操作：

**`stores/tasks.ts`**
```ts
async function toggle(id: string) {
  const task = tasks.value.find(t => t.id === id);
  try {
    await api.toggleTask(id);
    await silentRefresh();
    if (task) {
      toast.success(task.completed
        ? t('toast.taskUncompleted', { text: trunc(task.text) })
        : t('toast.taskCompleted',   { text: trunc(task.text) }));
    }
  } catch (e: any) {
    error.value = errorMessage(e);
    toast.error(t('toast.operationFailed', { reason: error.value }));
  }
}
// update / add 同款 try/catch + toast
```

**`stores/settings.ts`**：所有 source CRUD / hub 配置 / quick action 设置都走同样模式。**只在用户主动操作的入口加 toast**（比如 `pickAndAddFolder` / `pickAndAddFile` / `pickAndSetHubFolder`），不在底层 `addSource(...)` 加，避免 watcher 自动 reload 时误触发。

**`stores/history.ts`**：`undo` / `redo` / `jumpTo` 成功后 toast。`jumpTo` 的「区间内有外部编辑」warning 走 `confirm` + 用户确认后继续仍走 toast。

### 3.4 文案 i18n

新增 `src/i18n/locales/en.ts` 和 `zh.ts` 里的 `toast.*` namespace：

| key | zh | en |
|---|---|---|
| `toast.taskAdded` | `已添加任务` | `Task added` |
| `toast.taskEdited` | `已编辑任务` | `Task edited` |
| `toast.taskCompleted` | `已完成「{text}」` | `Completed "{text}"` |
| `toast.taskUncompleted` | `已取消完成「{text}」` | `Marked "{text}" undone` |
| `toast.taskMoved` | `已移动到 {quadrant}` | `Moved to {quadrant}` |
| `toast.sourceAdded` | `已添加源「{label}」` | `Source "{label}" added` |
| `toast.sourceRemoved` | `已删除源「{label}」` | `Source "{label}" removed` |
| `toast.sourceUpdated` | `已更新源` | `Source updated` |
| `toast.hubSet` | `已设置 Hub 目录` | `Hub folder set` |
| `toast.hubResynced` | `已重新同步 Hub` | `Hub resynced` |
| `toast.historyUndone` | `已撤销：{desc}` | `Undone: {desc}` |
| `toast.historyRedone` | `已重做：{desc}` | `Redone: {desc}` |
| `toast.historyJumped` | `已跳转到 {time}` | `Jumped to {time}` |
| `toast.operationFailed` | `操作失败：{reason}` | `Operation failed: {reason}` |

`trunc()` 工具：超过 30 个字符截断 + 加 `…`，避免 toast 撑爆。

## 4. 历史按钮未读外部修改徽章

### 4.1 `useHistoryStore` 扩展

```ts
const lastSeenAt = ref<string | null>(loadLastSeen());

function loadLastSeen(): string | null {
  return localStorage.getItem('floaty.history.lastSeenAt');
}

const unseenExternal = computed(() => {
  if (!lastSeenAt.value) return events.value.filter(e => e.kind === 'external_edit').length;
  return events.value.filter(e => e.kind === 'external_edit' && e.ts > lastSeenAt.value!).length;
});

function markSeen() {
  const now = new Date().toISOString();
  lastSeenAt.value = now;
  localStorage.setItem('floaty.history.lastSeenAt', now);
}

function syncLastSeenFromStorage() {
  lastSeenAt.value = loadLastSeen();
}
```

### 4.2 触发时机

- `HistoryView.vue` `onMounted` 里 `await history.refresh()` 之后调用 `history.markSeen()`，然后通过 Tauri 发事件 `history-seen-changed` 给所有窗口
- 主窗口 `App.vue` 监听 `history-seen-changed` → 调 `history.syncLastSeenFromStorage()` 强制重新计算 `unseenExternal`

### 4.3 UI 改动 — `TaskList.vue` `.history-btn`

```vue
<button
  class="footer-btn icon-only history-btn"
  :class="{ redo: history.hasRedo, unseen: history.unseenExternal > 0 }"
  @click="openHistory"
  :title="historyTitle"
>
  <span aria-hidden="true">🕒</span>
</button>
```

```ts
const historyTitle = computed(() => {
  const parts = ['History (Ctrl+H)'];
  if (history.unseenExternal > 0) parts.push(`${history.unseenExternal} 条未看过的外部修改`);
  return parts.join(' · ');
});
```

```css
/* 既有绿点（hasRedo）位置：右上角 */
.history-btn.redo::after   { /* 现有，不变 */ }

/* 新增红点（unseen）位置：左上角，避免和绿点重叠 */
.history-btn.unseen::before {
  content: '';
  width: 5px;
  height: 5px;
  border-radius: 999px;
  background: #ef4444;
  position: absolute;
  transform: translate(-3px, -8px);
}
```

### 4.4 跨窗口同步流程

Tauri 2 的 `@tauri-apps/api/event.emit` 默认就是**跨窗口广播**，不需要 Rust 后端转发。

```
[历史窗口] 用户打开 HistoryView
   ↓ history.refresh()
   ↓ history.markSeen()
   ↓ localStorage.setItem('floaty.history.lastSeenAt', ISO)
   ↓ emit('history-seen-changed')           ← 前端直接广播
       ↓
[主窗口] listen('history-seen-changed')
   ↓ history.syncLastSeenFromStorage()
   ↓ unseenExternal 重新计算 → 0 → .unseen class 去掉 → 红点消失
```

localStorage 在两个窗口之间是共享的（同一个 origin），后端完全不掺和。

## 5. 文件改动清单

### 新增
- `src/composables/useToast.ts`
- `src/components/ToastContainer.vue`
- `docs/superpowers/specs/2026-05-15-toast-notifications-design.md`（本文件）

### 修改
- `src/i18n/locales/en.ts`、`src/i18n/locales/zh.ts` — 加 `toast.*` namespace
- `src/stores/tasks.ts` — toggle / update / add 加 toast
- `src/stores/settings.ts` — source / hub / quick action 操作加 toast
- `src/stores/history.ts` — undo / redo / jumpTo 加 toast；扩展 lastSeenAt / unseenExternal / markSeen / syncLastSeenFromStorage
- `src/App.vue` — 挂 `<ToastContainer />`，监听 `history-seen-changed`
- `src/views/HistoryView.vue` — 挂 `<ToastContainer />`，`onMounted` 里 markSeen + emit
- `src/components/TaskList.vue` — `.history-btn` 加 `.unseen` class + 红点样式 + tooltip
- `src/services/tauri-api.ts` — 加 `onHistorySeenChanged` 监听器 + `emitHistorySeen` 包装（直接用 `@tauri-apps/api/event` 的 `emit` / `listen`）

### Rust 端：不需要改

`history-seen-changed` 是纯前端广播事件（Tauri 2 `@tauri-apps/api/event.emit` 默认跨窗口），后端无感。

## 6. 测试计划（手动验证步骤）

启动 dev (`npm run tauri dev`)，依次验证：

**Toast 主路径**
- [ ] 勾一个任务 → 底部出绿色 toast「已完成「xxx」」2s 后消失
- [ ] 添加任务 → 绿色 toast「已添加任务」
- [ ] 编辑任务文本 → 绿色 toast「已编辑任务」
- [ ] 编辑时改象限 → 绿色 toast「已移动到 🟡 重要不紧急」
- [ ] 在 settings 加 folder source → 绿色 toast「已添加源「xxx」」
- [ ] 删 source → 绿色 toast「已删除源」
- [ ] 设 hub → 绿色 toast「已设置 Hub 目录」

**Toast 错误路径**
- [ ] 把一个任务文件删掉，再点勾选 → 红色 toast「操作失败：xxx」6s 不自动消失（其实 6s 后还是消失，但够看清）
- [ ] hover 上去 → 计时暂停；移开 → 继续计时

**Toast 堆叠**
- [ ] 快速点 4-5 个 toggle → 同时最多 3 条可见，超出排队，前面消失后下一条接上

**历史窗口**
- [ ] 在历史窗口点 undo → 历史窗口里出绿色 toast「已撤销：xxx」
- [ ] 历史窗口的 toast 在历史窗口出，不在主窗口出

**未读红点**
- [ ] 启动后用 VS Code 改一个 .md 文件保存 → 主窗口 footer 历史按钮左上角出现**红点**
- [ ] 再改一次 → 红点还在（计数累加）
- [ ] 点击历史按钮 → 历史窗口打开，红点立刻消失
- [ ] 已存在的「有 redo」**绿点**和新的**红点**可以同时出现（左上红 + 右上绿）

**localStorage 持久化**
- [ ] 关掉历史窗口 → 重启 App → 之前看过的 external_edit 不再算 unseen

## 7. 风险与边缘情况

| 风险 | 处理 |
|---|---|
| 操作太快 toast 堆积 | `MAX_VISIBLE = 3` + 队列 |
| 错误 toast 把窗口塞满 | error 6s 自动消失（不 sticky）+ × 可手动关 |
| 多条相同 toast | 暂不去重 —— 用户多次操作就该多次反馈；后续如果嫌吵再加 dedupe |
| watcher 自动 reload 误触发 toast | 只在用户主动调用入口（store action）加 toast，不在底层 mutation / fs 事件处加 |
| 切换主题时 toast 颜色 | 用 CSS 变量 + 固定 hex，绿/红/琥珀在深浅模式下都成立 |
| localStorage 损坏 | `loadLastSeen` 失败时返回 null，所有 external_edit 都算未读 —— 用户打开历史一次即恢复 |
| Tauri emit 失败 | 前端调 `invoke('notify_history_seen')` 失败时 console.warn 但不阻塞；下次 history-updated 事件会顺带触发主窗口 refresh，从 localStorage 拿到新值 |

## 8. 后续可能扩展（不在本次范围）

- toast 上加「撤销」按钮（add / edit 之后给个 3s 撤销机会）
- toast 历史日志面板
- 系统通知（OS notification）补发
- 按 source 着色 toast 左侧色条（让用户一眼看到是哪个源的动作）
