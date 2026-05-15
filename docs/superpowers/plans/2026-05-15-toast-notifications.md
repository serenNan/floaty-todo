# Toast 提示系统 + 历史未读徽章 — 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 给 Floaty Todo 加全局 toast 反馈系统，覆盖所有 in-app 写操作；外部修改不刷屏 toast，改在主窗口历史按钮上挂未读红点。

**Architecture:** 沿用项目「单例 composable + App 根挂全局组件」模式。新增 `useToast` + `ToastContainer`，挂在主窗口（App.vue）和历史窗口（HistoryView.vue）两个 root。三个 Pinia store 的写操作 try/catch 里加 toast 调用。历史按钮基于 localStorage `floaty.history.lastSeenAt` 计算 `unseenExternal`，跨窗口同步用 Tauri `@tauri-apps/api/event` 前端 emit/listen（自带跨窗口广播）。

**Tech Stack:** Vue 3 (Composition API) + Pinia + vue-i18n + Tauri 2 (`@tauri-apps/api/event`) + Teleport / TransitionGroup（Vue 内置）。

**Spec:** [docs/superpowers/specs/2026-05-15-toast-notifications-design.md](../specs/2026-05-15-toast-notifications-design.md)

**用户偏好：** 本项目 CLAUDE.md 明确「不要帮我 commit，除非我明确要求了」，所以本计划**不包含 commit 步骤**。每个 Task 结束时由用户决定是否提交。本项目没有自动化测试框架，所有验证都用 `npm run tauri dev` 手动跑（CLAUDE.md「加个功能 → 先定义"完成"长什么样（具体输入输出 / 手动验证步骤）」）。

---

## 文件结构

**新建：**
- `src/composables/useToast.ts` — 模块级单例 state + 公开 API（`toast.success/warning/error/info`）
- `src/components/ToastContainer.vue` — 渲染队列 + Teleport 到 body + 进入/离开动画

**修改：**
- `src/i18n/locales/en.ts` — 加 `toast.*` namespace
- `src/i18n/locales/zh.ts` — 加 `toast.*` namespace
- `src/stores/tasks.ts` — toggle / update / add 接 toast
- `src/stores/settings.ts` — source CRUD / hub / quick action 接 toast
- `src/stores/history.ts` — undo / redo / jumpTo 接 toast；扩展 lastSeenAt / unseenExternal / markSeen / syncLastSeenFromStorage
- `src/services/tauri-api.ts` — 加 `emitHistorySeen` + `onHistorySeenChanged`
- `src/App.vue` — 挂 `<ToastContainer />`、监听 `history-seen-changed`
- `src/views/HistoryView.vue` — 挂 `<ToastContainer />`、`onMounted` 里 markSeen + emit
- `src/components/TaskList.vue` — `.history-btn` 加 `.unseen` class + 红点样式 + tooltip 拼接

**Rust 端无改动。**

---

### Task 1: Toast 核心 — composable + 组件 + 主窗口挂载

**Files:**
- Create: `src/composables/useToast.ts`
- Create: `src/components/ToastContainer.vue`
- Modify: `src/App.vue`

- [ ] **Step 1: 创建 `src/composables/useToast.ts`**

```ts
import { ref, type Ref } from 'vue';

export type ToastVariant = 'success' | 'warning' | 'error' | 'info';

export interface ToastItem {
  id: number;
  variant: ToastVariant;
  message: string;
  duration: number;
  remaining: number;        // 剩余 ms（hover 暂停用）
  startedAt: number;        // 当前计时段的起点
  timerId: number | null;
}

export interface ToastOptions {
  duration?: number;
}

const DEFAULTS: Record<ToastVariant, number> = {
  success: 2000,
  info:    3000,
  warning: 4000,
  error:   6000,
};

const MAX_VISIBLE = 3;

const items = ref<ToastItem[]>([]);
let nextId = 1;

function push(variant: ToastVariant, message: string, opts?: ToastOptions) {
  const duration = opts?.duration ?? DEFAULTS[variant];
  const item: ToastItem = {
    id: nextId++,
    variant,
    message,
    duration,
    remaining: duration,
    startedAt: Date.now(),
    timerId: null,
  };
  items.value = [...items.value, item];
  startTimer(item);
  // 维持 MAX_VISIBLE 上限 —— 超出的最旧 toast 立即消失
  while (items.value.length > MAX_VISIBLE) {
    const oldest = items.value[0];
    if (oldest) dismiss(oldest.id);
    else break;
  }
}

function startTimer(item: ToastItem) {
  if (item.duration <= 0) return;
  item.startedAt = Date.now();
  item.timerId = window.setTimeout(() => dismiss(item.id), item.remaining);
}

function dismiss(id: number) {
  const idx = items.value.findIndex(i => i.id === id);
  if (idx < 0) return;
  const item = items.value[idx];
  if (item.timerId !== null) window.clearTimeout(item.timerId);
  items.value = items.value.filter(i => i.id !== id);
}

function pause(id: number) {
  const item = items.value.find(i => i.id === id);
  if (!item || item.timerId === null) return;
  window.clearTimeout(item.timerId);
  item.timerId = null;
  item.remaining = Math.max(0, item.remaining - (Date.now() - item.startedAt));
}

function resume(id: number) {
  const item = items.value.find(i => i.id === id);
  if (!item || item.timerId !== null || item.remaining <= 0) return;
  startTimer(item);
}

export const toast = {
  success: (msg: string, opts?: ToastOptions) => push('success', msg, opts),
  warning: (msg: string, opts?: ToastOptions) => push('warning', msg, opts),
  error:   (msg: string, opts?: ToastOptions) => push('error',   msg, opts),
  info:    (msg: string, opts?: ToastOptions) => push('info',    msg, opts),
};

export function useToastState(): {
  items: Ref<ToastItem[]>;
  dismiss: (id: number) => void;
  pause:   (id: number) => void;
  resume:  (id: number) => void;
} {
  return { items, dismiss, pause, resume };
}
```

- [ ] **Step 2: 创建 `src/components/ToastContainer.vue`**

```vue
<script setup lang="ts">
import { useToastState } from '../composables/useToast';

const { items, dismiss, pause, resume } = useToastState();

function iconChar(variant: string): string {
  switch (variant) {
    case 'success': return '✓';
    case 'warning': return '⚠';
    case 'error':   return '✕';
    default:        return 'ⓘ';
  }
}
</script>

<template>
  <Teleport to="body">
    <div class="toast-stack">
      <TransitionGroup name="toast" tag="div" class="toast-stack-inner">
        <div
          v-for="item in items"
          :key="item.id"
          class="toast"
          :class="['toast-' + item.variant]"
          role="status"
          @mouseenter="pause(item.id)"
          @mouseleave="resume(item.id)"
        >
          <span class="stripe"></span>
          <span class="icon" aria-hidden="true">{{ iconChar(item.variant) }}</span>
          <span class="text">{{ item.message }}</span>
          <button
            type="button"
            class="close"
            tabindex="-1"
            aria-label="Dismiss"
            @click="dismiss(item.id)"
          >×</button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-stack {
  position: fixed;
  bottom: 52px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 1000;
  pointer-events: none;
  width: max-content;
  max-width: calc(100% - 24px);
}
.toast-stack-inner {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.toast {
  pointer-events: auto;
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
  color: var(--text);
}
.toast .stripe {
  width: 3px;
  height: 20px;
  border-radius: 2px;
}
.toast-success .stripe { background: #16a34a; }
.toast-warning .stripe { background: #f59e0b; }
.toast-error   .stripe { background: #ef4444; }
.toast-info    .stripe { background: #64748b; }
.toast-success .icon { color: #16a34a; }
.toast-warning .icon { color: #f59e0b; }
.toast-error   .icon { color: #ef4444; }
.toast-info    .icon { color: var(--text-muted); }
.toast .text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.toast .close {
  opacity: 0;
  transition: opacity 120ms ease-out;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
  padding: 0;
  width: 18px;
  height: 18px;
}
.toast:hover .close { opacity: 1; }
.toast .close:hover { color: var(--text); }

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
.toast-enter-active,
.toast-leave-active {
  transition: opacity 200ms ease-out, transform 200ms ease-out;
}
.toast-leave-active {
  position: absolute;
}
</style>
```

- [ ] **Step 3: 在 `src/App.vue` 挂载 `<ToastContainer />`**

修改 `src/App.vue`：

在 import 区加：
```ts
import ToastContainer from './components/ToastContainer.vue';
```

在 `<template>` 的 `<main>` 内、`<QuickAddDialog />` 之后加一行：
```vue
    <QuickAddDialog />
    <ToastContainer />
  </main>
```

- [ ] **Step 4: 烟雾测试 toast 渲染**

临时在 `src/App.vue` 的 `<script setup>` 末尾、`function backToTasks` 之前加一行测试：

```ts
import { toast } from './composables/useToast';
onMounted(() => { setTimeout(() => toast.success('Toast 已就绪'), 600); });
```

注意：这里**新加**一个 `onMounted`，原有的 `onMounted` 保持不变（Vue 支持多个 `onMounted` 注册）。

跑 `npm run tauri dev`，应用启动 ~0.6s 后底部应出现一条绿色 toast「Toast 已就绪」，2s 后消失。

- [ ] **Step 5: 验证 hover 暂停**

再次手动触发（可临时把 setTimeout 改 0 或加按钮调用 toast.error 较长的提示）：
- toast 出现后立刻把鼠标移上去 → 计时停止
- 移开 → 继续计时直到自动消失
- 点 × → 立即消失

- [ ] **Step 6: 移除烟雾测试代码**

把 Step 4 加的临时 `onMounted` 和 import 删掉。`ToastContainer` 在 template 里保留。

---

### Task 2: 加 i18n 文案 (`toast.*` namespace)

**Files:**
- Modify: `src/i18n/locales/zh.ts`
- Modify: `src/i18n/locales/en.ts`

- [ ] **Step 1: 改 `src/i18n/locales/zh.ts`**

在文件末尾（`confirm` 块之后、`}` 之前）插入：

```ts
  toast: {
    taskAdded: '已添加任务',
    taskEdited: '已编辑任务',
    taskCompleted: '已完成「{text}」',
    taskUncompleted: '已取消完成「{text}」',
    taskMoved: '已移动到 {quadrant}',
    sourceAdded: '已添加源「{label}」',
    sourceRemoved: '已删除源',
    sourceUpdated: '已更新源',
    sourceReordered: '已调整源顺序',
    sourceDefaultSet: '已设为默认源',
    hubSet: '已设置 Hub 目录',
    hubResynced: '已重新同步 Hub',
    hubCleared: '已关闭 Hub',
    fileLabelSet: '已重命名文件',
    quickActionsUpdated: '已更新快捷动作',
    historyUndone: '已撤销',
    historyRedone: '已重做',
    historyJumped: '已跳转',
    historyNothing: '没有可撤销/重做的操作',
    externalEditSkipped: '已跳过 {n} 条外部编辑',
    operationFailed: '操作失败：{reason}',
  },
```

注意末尾的逗号 —— `zh.ts` 是 `export default {...}` 对象字面量。

- [ ] **Step 2: 改 `src/i18n/locales/en.ts`**

同样位置插入对应英文：

```ts
  toast: {
    taskAdded: 'Task added',
    taskEdited: 'Task edited',
    taskCompleted: 'Completed "{text}"',
    taskUncompleted: 'Marked "{text}" undone',
    taskMoved: 'Moved to {quadrant}',
    sourceAdded: 'Source "{label}" added',
    sourceRemoved: 'Source removed',
    sourceUpdated: 'Source updated',
    sourceReordered: 'Source order updated',
    sourceDefaultSet: 'Default source set',
    hubSet: 'Hub folder set',
    hubResynced: 'Hub resynced',
    hubCleared: 'Hub disabled',
    fileLabelSet: 'File renamed',
    quickActionsUpdated: 'Quick actions updated',
    historyUndone: 'Undone',
    historyRedone: 'Redone',
    historyJumped: 'Jumped',
    historyNothing: 'Nothing to undo/redo',
    externalEditSkipped: 'Skipped {n} external edit(s)',
    operationFailed: 'Operation failed: {reason}',
  },
```

- [ ] **Step 3: 跑 `npm run tauri dev` 确认没有 i18n missing-key 报错**

不需要触发 toast，只确认 vite 热重载完成、console 没有 `[intlify] Not found 'toast.xxx'` 警告。

---

### Task 3: 接入 tasks store（toggle / update / add）

**Files:**
- Modify: `src/stores/tasks.ts`

- [ ] **Step 1: 引入 toast + i18n + 工具**

把 `src/stores/tasks.ts` 顶部 import 改成：

```ts
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Task, Quadrant } from '../types/task';
import { api } from '../services/tauri-api';
import { errorMessage } from '../utils/errors';
import { toast } from '../composables/useToast';

const QUADRANT_LABEL_KEY: Record<Quadrant, string> = {
  urgent_important: 'quadrant.urgent_important',
  not_urgent_important: 'quadrant.not_urgent_important',
  urgent_not_important: 'quadrant.urgent_not_important',
  not_urgent_not_important: 'quadrant.not_urgent_not_important',
};

function trunc(s: string, max = 30): string {
  return s.length <= max ? s : s.slice(0, max - 1) + '…';
}
```

- [ ] **Step 2: 在 store setup 里拿 i18n `t`**

在 `defineStore('tasks', () => { ... })` 内部的 `const tasks = ref<Task[]>([]);` 上方加：

```ts
  const { t } = useI18n();
```

注意 Pinia composable-style stores 内可以用 `useI18n()`，因为 store 在 setup 阶段被实例化。

- [ ] **Step 3: 改 `toggle` 函数**

把现有的 `async function toggle(id: string)` 整段替换为：

```ts
  async function toggle(id: string) {
    const task = tasks.value.find(t => t.id === id);
    const text = task ? trunc(task.text) : '';
    const wasCompleted = task?.completed ?? false;
    try {
      await api.toggleTask(id);
      await silentRefresh();
      // 注意：API 调用前 task.completed 是旧值；调用后变成新值。
      // 这里 toast 描述的是「现在变成了什么状态」。
      if (task) {
        toast.success(
          wasCompleted
            ? t('toast.taskUncompleted', { text })
            : t('toast.taskCompleted', { text }),
        );
      }
    } catch (e: any) {
      error.value = errorMessage(e);
      toast.error(t('toast.operationFailed', { reason: error.value }));
    }
  }
```

- [ ] **Step 4: 改 `update` 函数**

替换为：

```ts
  async function update(id: string, text: string, quadrant?: Quadrant | null) {
    if (!text.trim()) return;
    const before = tasks.value.find(tk => tk.id === id);
    try {
      await api.updateTask(id, text.trim(), quadrant);
      await silentRefresh();
      const movedToDifferent =
        quadrant !== undefined && before && before.quadrant !== quadrant;
      if (movedToDifferent) {
        const key = quadrant ? QUADRANT_LABEL_KEY[quadrant] : 'quadrant.unsorted';
        toast.success(t('toast.taskMoved', { quadrant: t(key) }));
      } else {
        toast.success(t('toast.taskEdited'));
      }
    } catch (e: any) {
      error.value = errorMessage(e);
      toast.error(t('toast.operationFailed', { reason: error.value }));
    }
  }
```

- [ ] **Step 5: 改 `add` 函数**

替换为：

```ts
  async function add(text: string, sourceId?: string, quadrant?: Quadrant | null) {
    if (!text.trim()) return;
    try {
      await api.addTask(text.trim(), sourceId, quadrant);
      await silentRefresh();
      toast.success(t('toast.taskAdded'));
    } catch (e: any) {
      error.value = errorMessage(e);
      toast.error(t('toast.operationFailed', { reason: error.value }));
    }
  }
```

- [ ] **Step 6: 手动验证**

跑 `npm run tauri dev`。在主窗口里：
- 勾一个任务 → 出现绿色 toast「已完成「xxx」」
- 再点取消勾选 → 出现绿色 toast「已取消完成「xxx」」
- 通过任一 SourceGroup header 的「+」按钮添加任务 → 出现「已添加任务」
- 双击/编辑一个任务改文本（不改象限）→ 出现「已编辑任务」
- 编辑时把象限改了（比如改成 🟡 重要不紧急）→ 出现「已移动到 重要不紧急」

如果某个失败 → toast 应该是红色「操作失败：xxx」（可暂时手动删一个被监听的 .md 文件再触发 toggle 验证错误路径）。

---

### Task 4: 接入 settings store（source CRUD + hub + quick actions）

**Files:**
- Modify: `src/stores/settings.ts`

- [ ] **Step 1: 引入 toast + i18n + errors 工具**

把 `src/stores/settings.ts` 顶部 import 改成：

```ts
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { AppConfig, QuickActionKind, Source, SourceKind } from '../types/task';
import { api } from '../services/tauri-api';
import { errorMessage } from '../utils/errors';
import { toast } from '../composables/useToast';
```

在 `defineStore('settings', () => {` 内部的最顶端（`const config = ref<AppConfig | null>(null);` 上方）加：

```ts
  const { t } = useI18n();

  function failed(e: unknown) {
    toast.error(t('toast.operationFailed', { reason: errorMessage(e) }));
  }
```

- [ ] **Step 2: 改 `addSource`**

替换原函数为：

```ts
  async function addSource(args: {
    path: string;
    kind: SourceKind;
    label?: string | null;
    projectRoot?: string | null;
  }): Promise<Source> {
    try {
      const src = await api.addSource(args);
      await load();
      toast.success(t('toast.sourceAdded', { label: src.label || src.path }));
      return src;
    } catch (e) {
      failed(e);
      throw e;
    }
  }
```

- [ ] **Step 3: 改 `removeSource`**

```ts
  async function removeSource(sourceId: string) {
    try {
      await api.removeSource(sourceId);
      await load();
      toast.success(t('toast.sourceRemoved'));
    } catch (e) {
      failed(e);
      throw e;
    }
  }
```

- [ ] **Step 4: 改 `updateSource`**

```ts
  async function updateSource(args: {
    sourceId: string;
    label?: string | null;
    projectRoot?: string | null;
    color?: string | null;
  }): Promise<Source> {
    try {
      const src = await api.updateSource(args);
      await load();
      toast.success(t('toast.sourceUpdated'));
      return src;
    } catch (e) {
      failed(e);
      throw e;
    }
  }
```

- [ ] **Step 5: 改 `setDefaultSource` / `reorderSources` / `setFileLabel` / `setEnabledQuickActions` / `setHubFolder` / `resyncHub`**

每个都套上 try/catch + 对应 toast。完整替换段：

```ts
  async function setDefaultSource(sourceId: string | null) {
    try {
      await api.setDefaultSource(sourceId);
      await load();
      if (sourceId) toast.success(t('toast.sourceDefaultSet'));
    } catch (e) {
      failed(e);
      throw e;
    }
  }

  async function reorderSources(orderedIds: string[]) {
    try {
      await api.reorderSources(orderedIds);
      await load();
      toast.success(t('toast.sourceReordered'));
    } catch (e) {
      failed(e);
      throw e;
    }
  }

  async function setFileLabel(filePath: string, label: string | null) {
    try {
      await api.setFileLabel(filePath, label);
      await load();
      toast.success(t('toast.fileLabelSet'));
    } catch (e) {
      failed(e);
      throw e;
    }
  }

  async function setEnabledQuickActions(actions: QuickActionKind[]) {
    try {
      await api.setEnabledQuickActions(actions);
      await load();
      toast.success(t('toast.quickActionsUpdated'));
    } catch (e) {
      failed(e);
      throw e;
    }
  }

  async function setHubFolder(path: string | null) {
    try {
      await api.setHubFolder(path);
      await load();
      toast.success(path === null ? t('toast.hubCleared') : t('toast.hubSet'));
    } catch (e) {
      failed(e);
      throw e;
    }
  }

  async function resyncHub() {
    try {
      await api.resyncHub();
      await load();
      toast.success(t('toast.hubResynced'));
    } catch (e) {
      failed(e);
      throw e;
    }
  }
```

**保留不改：** `setAlwaysOnTop` / `toggleAlwaysOnTop` / `setAutoCreateQuadrantHeaders` / `load` / `markScanning` / `fileLabel` / `pickAndSetHubFolder` / `pickAndAddFolder` / `pickAndAddFile`（pickAnd* 已经间接调用 addSource / setHubFolder，会得到 toast；alwaysOnTop 是高频静默切换，不出 toast）。

- [ ] **Step 6: 手动验证**

跑 `npm run tauri dev`，到 Settings：
- 加一个新 folder source → 「已添加源「xxx」」
- 编辑某个 source 的 label/color → 「已更新源」
- 把一个 source 设为默认 → 「已设为默认源」
- 删一个 source → 「已删除源」
- 设置 Hub 目录 → 「已设置 Hub 目录」
- 点 Hub 重同步 → 「已重新同步 Hub」
- 关闭 Hub（disable）→ 「已关闭 Hub」
- 在 quickActions 里关闭某个动作 → 「已更新快捷动作」

---

### Task 5: 接入 history store（undo / redo / jumpTo）+ 历史窗口挂 ToastContainer

**Files:**
- Modify: `src/stores/history.ts`
- Modify: `src/views/HistoryView.vue`

- [ ] **Step 1: 引入 toast + i18n**

`src/stores/history.ts` 顶部加：

```ts
import { useI18n } from 'vue-i18n';
import { toast } from '../composables/useToast';
```

在 `defineStore('history', () => {` 内部最顶端加：

```ts
  const { t } = useI18n();
```

- [ ] **Step 2: 改 `undo` / `redo`**

替换为：

```ts
  async function undo() {
    try {
      const ev = await api.undo();
      await refresh();
      if (ev) toast.success(t('toast.historyUndone'));
      else toast.info(t('toast.historyNothing'));
    } catch (e: any) {
      error.value = errorMessage(e);
      toast.error(t('toast.operationFailed', { reason: error.value }));
    }
  }

  async function redo() {
    try {
      const ev = await api.redo();
      await refresh();
      if (ev) toast.success(t('toast.historyRedone'));
      else toast.info(t('toast.historyNothing'));
    } catch (e: any) {
      error.value = errorMessage(e);
      toast.error(t('toast.operationFailed', { reason: error.value }));
    }
  }
```

- [ ] **Step 3: 改 `jumpTo`**

```ts
  async function jumpTo(eventId: string, confirmExternal = false) {
    try {
      const result = await api.jumpTo(eventId, confirmExternal);
      await refresh();
      error.value = null;
      toast.success(t('toast.historyJumped'));
      if (result.skipped_external > 0) {
        toast.warning(t('toast.externalEditSkipped', { n: result.skipped_external }));
      }
    } catch (e: any) {
      error.value = errorMessage(e);
      // EXTERNAL_IN_UNDO_RANGE 由调用方（HistoryView）转 confirm 流程，
      // 这种"业务确认"路径不要 toast 报错 —— 调用方会再次调进来。
      if (errorCode(e) !== 'EXTERNAL_IN_UNDO_RANGE') {
        toast.error(t('toast.operationFailed', { reason: error.value }));
      }
      throw e;
    }
  }
```

注意 `errorCode` 需要 import：把 `import { errorMessage } from '../utils/errors';` 改为：

```ts
import { errorMessage, errorCode } from '../utils/errors';
```

- [ ] **Step 4: 在 `src/views/HistoryView.vue` 挂 `<ToastContainer />`**

在 import 区加：

```ts
import ToastContainer from '../components/ToastContainer.vue';
```

在 `<template>` 内 `<ConfirmDialog />` 之后加：

```vue
    <ConfirmDialog />
    <ToastContainer />
  </main>
```

- [ ] **Step 5: 手动验证**

跑 `npm run tauri dev`。在主窗口先做几个操作（添加任务、勾选、改象限）攒一些历史。然后点 footer 的 🕒 打开历史窗口：
- 选一个早一点的事件 → 点「跳到此处」 → **在历史窗口里** 出现绿色 toast「已跳转」
- 如果撤销区间正好覆盖一条外部编辑 → 会先弹 `confirm` 对话框，确认后跳转完出现一条绿色 toast「已跳转」 + 一条黄色 toast「已跳过 1 条外部编辑」
- 直接对一个没事件的状态点 undo（先连续 undo 到底）→ 蓝色 info toast「没有可撤销/重做的操作」

**确认 toast 出在历史窗口里，不是主窗口。**

---

### Task 6: 历史 store 扩展 `lastSeenAt` / `unseenExternal` / `markSeen`

**Files:**
- Modify: `src/stores/history.ts`

- [ ] **Step 1: 加 lastSeenAt + 持久化 + computed**

在 `src/stores/history.ts` 里，`const events = ref<HistoryEvent[]>([]);` 上方加：

```ts
  const LAST_SEEN_KEY = 'floaty.history.lastSeenAt';

  function loadLastSeen(): string | null {
    try {
      return localStorage.getItem(LAST_SEEN_KEY);
    } catch {
      return null;
    }
  }

  const lastSeenAt = ref<string | null>(loadLastSeen());
```

在已有的 `const hasRedo = computed(...)` 之后加：

```ts
  const unseenExternal = computed<number>(() => {
    const cutoff = lastSeenAt.value;
    return events.value.filter(e => {
      if (e.kind !== 'external_edit') return false;
      if (!cutoff) return true;
      return e.ts > cutoff;
    }).length;
  });

  function markSeen() {
    const now = new Date().toISOString();
    lastSeenAt.value = now;
    try {
      localStorage.setItem(LAST_SEEN_KEY, now);
    } catch {
      /* 写失败就算了，刷新后会重新计算 */
    }
  }

  function syncLastSeenFromStorage() {
    lastSeenAt.value = loadLastSeen();
  }
```

- [ ] **Step 2: 在 store 返回值里导出新成员**

修改文件末尾的 return：

```ts
  return {
    events,
    cursorId,
    loading,
    error,
    hasRedo,
    refresh,
    undo,
    redo,
    jumpTo,
    lastSeenAt,
    unseenExternal,
    markSeen,
    syncLastSeenFromStorage,
  };
```

- [ ] **Step 3: 在 `HistoryView.vue` `onMounted` 里调 `markSeen`**

修改 `src/views/HistoryView.vue` 的 `onMounted`：

```ts
onMounted(async () => {
  await settings.load();
  await history.refresh();
  selectedId.value = history.events[0]?.id ?? null;
  history.markSeen();
  unlisteners.push(await api.onHistoryUpdated(() => history.refresh()));
});
```

注意位置：`history.refresh()` 必须先跑，确保 events 加载完；`markSeen()` 在 selectedId 设置之后。

- [ ] **Step 4: 验证 localStorage 写入**

跑 `npm run tauri dev`，开历史窗口。然后 DevTools（F12）→ Application → Local Storage → 看到 `floaty.history.lastSeenAt` = 当前 ISO 时间戳。

---

### Task 7: 跨窗口同步 — Tauri 前端 emit/listen

**Files:**
- Modify: `src/services/tauri-api.ts`
- Modify: `src/stores/history.ts`
- Modify: `src/App.vue`

- [ ] **Step 1: 在 `tauri-api.ts` 加 emit/listen 包装**

修改 `src/services/tauri-api.ts`：

把顶部 `import { listen, type UnlistenFn } from '@tauri-apps/api/event';` 改成：

```ts
import { listen, emit, type UnlistenFn } from '@tauri-apps/api/event';
```

在 `api` 对象内 `onSourceScanFinished` 之后加：

```ts
  emitHistorySeen: (): Promise<void> => emit('history-seen-changed'),
  onHistorySeenChanged: (cb: () => void): Promise<UnlistenFn> =>
    listen('history-seen-changed', cb),
```

- [ ] **Step 2: 在 `history` store 的 `markSeen` 里广播事件**

修改 `src/stores/history.ts` 的 `markSeen`：

```ts
  function markSeen() {
    const now = new Date().toISOString();
    lastSeenAt.value = now;
    try {
      localStorage.setItem(LAST_SEEN_KEY, now);
    } catch {
      /* 写失败就算了 */
    }
    // 通知其它窗口（主窗口）重新读 localStorage —— Tauri 2 的前端 emit
    // 默认跨窗口广播，主窗口 listen 后会调 syncLastSeenFromStorage。
    api.emitHistorySeen().catch(err => console.warn('emit history-seen-changed failed:', err));
  }
```

- [ ] **Step 3: 在主窗口 `App.vue` 监听**

修改 `src/App.vue` 的 `onMounted` 里 `unlisteners.push` 部分，在 `onHistoryUpdated` 监听之后加一条：

```ts
  unlisteners.push(await api.onHistoryUpdated(() => { history.refresh(); }));
  unlisteners.push(await api.onHistorySeenChanged(() => { history.syncLastSeenFromStorage(); }));
```

- [ ] **Step 4: 手动验证事件转发**

跑 `npm run tauri dev`。
- 主窗口和历史窗口都打开
- 主窗口 DevTools console 里执行：`window.addEventListener('storage', e => console.log('storage', e.key))` — 用来观察 localStorage 变化（同源 storage 事件理论上不跨 webview，所以这里看不到也正常）
- 实际验证：在主窗口编辑一个外部 .md（用 VS Code 改一行保存）→ 主窗口 footer history-btn 上**还没出**红点（红点 UI 还没做），但 store 里 `unseenExternal` 应该 > 0
- 关掉历史窗口，在主窗口 DevTools console 里：
  ```js
  const { useHistoryStore } = await import('/src/stores/history.ts');
  // 实际上从 Pinia 拿 — 用全局 hint：window.__PINIA__ 或直接 DevTools 看 store
  ```
  跳过，留到 Task 9 整体测试一并验证。

---

### Task 8: 历史按钮红点徽章 UI

**Files:**
- Modify: `src/components/TaskList.vue`

- [ ] **Step 1: 改 `<button class="footer-btn history-btn">` 模板**

在 `src/components/TaskList.vue` 里找到 `<button class="footer-btn icon-only history-btn">` 那段（约 line 188-195），替换为：

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

- [ ] **Step 2: 在 `<script setup>` 加 `historyTitle` computed**

在文件顶部 import 之后、`defineEmits` 之前已经有 `const { t } = useI18n()`、`const history = useHistoryStore()`。在 `async function openHistory()` 之前加：

```ts
const historyTitle = computed(() => {
  const parts = ['History (Ctrl+H)'];
  if (history.unseenExternal > 0) {
    parts.push(`${history.unseenExternal} 条未看过的外部修改`);
  }
  return parts.join(' · ');
});
```

（`computed` 已经在 import 列表里，无需追加。）

- [ ] **Step 3: 加 `.history-btn.unseen` 红点 CSS**

在 `<style scoped>` 里现有 `.history-btn.redo::after { ... }` 块之后加：

```css
.history-btn.unseen::before {
  content: '';
  width: 5px;
  height: 5px;
  border-radius: 999px;
  background: #ef4444;
  position: absolute;
  transform: translate(-9px, -8px);
}
```

`transform: translate(-9px, -8px)` 把红点放到左上角（绿点是 `+9px, -8px` 在右上角，左右对称）。

- [ ] **Step 4: 手动验证红点**

跑 `npm run tauri dev`。
- 启动时若 localStorage 里有 `floaty.history.lastSeenAt`，且历史里有该时间戳之后的 `external_edit` → 红点出现
- 若 localStorage 没记录、历史里有任何 external_edit → 红点出现
- 用 VS Code 改一个被监听的 .md 保存 → 历史 store refresh 之后红点出现（因为 `tasks-updated` / `history-updated` 事件会触发 history.refresh()，refresh 会重算 unseenExternal）
- 鼠标 hover 历史按钮 → tooltip 显示「History (Ctrl+H) · 3 条未看过的外部修改」
- 点开历史窗口 → 主窗口红点立刻消失（通过 `history-seen-changed` 事件同步）

---

### Task 9: 综合测试 — 走一遍 spec 第 6 节完整 checklist

**Files:** 无修改，纯手动测试。

- [ ] **Step 1: 启动 dev**

```powershell
npm run tauri dev
```

- [ ] **Step 2: Toast 主路径**

- 勾一个任务 → 底部出绿色 toast「已完成「xxx」」2s 后消失
- 添加任务 → 绿色 toast「已添加任务」
- 编辑任务文本（不改象限）→ 绿色 toast「已编辑任务」
- 编辑时改象限 → 绿色 toast「已移动到 🟡 重要不紧急」
- 加 folder source → 绿色 toast「已添加源「xxx」」
- 删 source → 「已删除源」
- 设 hub → 「已设置 Hub 目录」

- [ ] **Step 3: Toast 错误路径**

把一个被监听的 .md 文件删掉再点对应任务的勾选 → 红色 toast「操作失败：xxx」自动 6s 后消失，hover 可暂停，× 可立即关闭。

- [ ] **Step 4: Toast 堆叠**

快速连续勾选 4-5 个任务 → 最多同时显示 3 条，超出的最旧一条立即消失为下一条腾位。

- [ ] **Step 5: 历史窗口 toast**

打开历史 → 在历史窗口里 undo / redo / 跳到此处 → toast **出现在历史窗口**，不在主窗口。

- [ ] **Step 6: 未读红点**

- 关闭 App，用 VS Code 改一个被监听的 .md 保存（确保监听器在 App 关掉的情况下不工作 —— 这一条要重启 App 才能验证）

实际操作：保持 App 开着用 VS Code 改 .md 保存：
- 主窗口历史按钮**左上角**出现红点
- 同时如果之前 undo 过没 redo，**右上角**还有绿点 —— 两个点都在
- hover 历史按钮 → tooltip 含「N 条未看过的外部修改」
- 点开历史窗口 → 主窗口红点**立刻消失**（验证 history-seen-changed 跨窗口事件）
- 关闭历史窗口 → 再 VS Code 改一次保存 → 红点又出现

- [ ] **Step 7: localStorage 持久化**

- DevTools → Application → Local Storage → 看到 `floaty.history.lastSeenAt` = 最后一次开历史窗口的 ISO 时间
- 重启 App → 之前看过的 external_edit 不再算 unseen（红点应不出现，除非有更新的 external_edit）

- [ ] **Step 8: 确认无遗留 console 错误**

跑完上述步骤，浏览主窗口 DevTools console + 历史窗口 console，不应有红色 error（黄色 warning 如有 source map 之类的不算）。

- [ ] **Step 9: 走一遍 spec 第 7 节的风险表确认**

- ✅ 操作快速堆积 → MAX_VISIBLE = 3 生效
- ✅ 错误 toast 不 sticky
- ✅ 多条相同 toast → 不去重，每个动作一条
- ✅ watcher 自动 reload 不误触 toast（只在 store action 入口加）
- ✅ 深浅模式下颜色都成立（在 settings 切主题确认）
- ✅ localStorage 损坏 / 缺失 → fallback null，看一次后恢复
- ✅ Tauri emit 失败 console.warn 但不阻塞

---

## 完成定义

- 全部 9 个 Task 的所有 Step 完成、勾选
- spec 第 6 节完整 checklist 全部 ✅
- `npm run tauri dev` 启动后无 console error
- 用户主观感受：日常勾选 / 添加 / 编辑都能看到不打扰的反馈；外部编辑不刷屏，历史按钮上红点能直观提示

## 不在本计划范围（YAGNI 已确认）

- toast 上的「撤销」按钮
- 长按 / 滑动消除手势
- toast 历史日志页面
- 系统级 OS notification
- 按 source 着色 toast 左侧色条
