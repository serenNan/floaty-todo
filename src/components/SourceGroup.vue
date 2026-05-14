<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { QuickActionKind, Source, Task } from '../types/task';
import { useSettingsStore } from '../stores/settings';
import { api } from '../services/tauri-api';
import { bindCollapse } from '../composables/useCollapse';
import {
  draggedSourceId,
  dropTargetSourceId,
  startSourceDrag,
} from '../composables/useSourceDrag';
import FileGroup from './FileGroup.vue';
import TaskItem from './TaskItem.vue';
import QuickActionIcon from './icons/QuickActionIcon.vue';
import Icon from './icons/Icon.vue';

/// Auto-collapse every FileGroup the first time we render a source whose
/// task count exceeds this threshold. Keeps the DOM tree small enough to
/// stay responsive on huge vaults (thousands of tasks → only a few dozen
/// file headers in the DOM until the user expands one).
const BIG_SOURCE_TASK_THRESHOLD = 50;

const props = defineProps<{ source: Source; tasks: Task[] }>();
const emit = defineEmits<{ 'open-settings': [] }>();
const { t } = useI18n();
const settings = useSettingsStore();

const collapsed = ref(true);
const actionError = ref<string | null>(null);

// React to global "Collapse all" / "Expand all" from the footer button.
bindCollapse(next => { collapsed.value = next; });

const isDefault = computed(() => settings.defaultSourceId === props.source.id);

const displayLabel = computed(() => {
  const s = props.source;
  if (s.label && s.label.trim()) return s.label;
  return s.path.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? s.path;
});

const counts = computed(() => {
  let todo = 0, done = 0;
  for (const tk of props.tasks) (tk.completed ? done++ : todo++);
  return { todo, done };
});

/// Group tasks by their `source_file`. For File sources this collapses to a
/// single bucket; for Folder sources each `.md` file becomes its own group.
/// Files with zero tasks are included only when this is a File source — for
/// Folder sources we keep the list to "files that have tasks" so the source
/// header stays compact.
const fileGroups = computed(() => {
  const map = new Map<string, Task[]>();
  for (const tk of props.tasks) {
    const arr = map.get(tk.source_file);
    if (arr) arr.push(tk);
    else map.set(tk.source_file, [tk]);
  }
  // For a File source, ensure the source path is always present even when empty.
  if (props.source.kind === 'file' && !map.has(props.source.path)) {
    map.set(props.source.path, []);
  }
  // Stable ordering: by file path string.
  return Array.from(map.entries())
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    .map(([filePath, fileTasks]) => ({ filePath, tasks: fileTasks }));
});

async function runAction(kind: QuickActionKind) {
  try { await api.runQuickAction(props.source.id, kind); }
  catch (e: any) { actionError.value = String(e); }
}

// Drag-and-drop reordering for quick-action buttons. The new order is
// persisted to settings.enabled_quick_actions, so reordering here also
// reorders the same buttons on every *other* source — desired, since
// the toggle list is global.
const dragKind = ref<QuickActionKind | null>(null);
const dropTargetKind = ref<QuickActionKind | null>(null);

function onActionDragStart(e: DragEvent, kind: QuickActionKind) {
  dragKind.value = kind;
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move';
    // Some browsers require a payload or the drag never starts.
    e.dataTransfer.setData('text/plain', kind);
  }
}
function onActionDragOver(e: DragEvent, kind: QuickActionKind) {
  if (!dragKind.value || dragKind.value === kind) return;
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
  dropTargetKind.value = kind;
}
function onActionDragLeave(kind: QuickActionKind) {
  if (dropTargetKind.value === kind) dropTargetKind.value = null;
}
async function onActionDrop(e: DragEvent, targetKind: QuickActionKind) {
  e.preventDefault();
  const src = dragKind.value;
  dragKind.value = null;
  dropTargetKind.value = null;
  if (!src || src === targetKind) return;
  const order = [...settings.enabledQuickActions];
  const srcIdx = order.indexOf(src);
  if (srcIdx < 0) return;
  order.splice(srcIdx, 1);
  const newTgtIdx = order.indexOf(targetKind);
  if (newTgtIdx < 0) {
    order.push(src);
  } else {
    order.splice(newTgtIdx, 0, src);
  }
  try { await settings.setEnabledQuickActions(order); }
  catch (e: any) { actionError.value = String(e); }
}
function onActionDragEnd() {
  dragKind.value = null;
  dropTargetKind.value = null;
}

// Pointer-events based source-drag. Tauri's WebView2 doesn't deliver
// HTML5 `dragover` events to in-page elements, so we can't use the
// `draggable="true"` flow. Pointer events work reliably everywhere.
const isSourceDragging = computed(() => draggedSourceId.value === props.source.id);
const isSourceDropTarget = computed(() => dropTargetSourceId.value === props.source.id);

function onDotsPointerDown(e: PointerEvent) {
  if (e.button !== 0) return; // left button only
  e.preventDefault();
  startSourceDrag({
    e,
    sourceId: props.source.id,
    onClick: () => emit('open-settings'),
    onDrop: async (targetId: string) => {
      const order = settings.sources.map(s => s.id);
      const srcIdx = order.indexOf(props.source.id);
      if (srcIdx < 0) return;
      order.splice(srcIdx, 1);
      const tgtIdx = order.indexOf(targetId);
      if (tgtIdx < 0) order.push(props.source.id);
      else order.splice(tgtIdx, 0, props.source.id);
      try { await settings.reorderSources(order); }
      catch (err: any) { actionError.value = String(err); }
    },
  });
}

interface ActionMeta {
  kind: QuickActionKind;
  i18n: string;
}
const ACTION_META: Record<QuickActionKind, ActionMeta> = {
  reveal:      { kind: 'reveal',      i18n: 'source.reveal' },
  vscode:      { kind: 'vscode',      i18n: 'source.openVscode' },
  terminal:    { kind: 'terminal',    i18n: 'source.openTerminal' },
  claude_code: { kind: 'claude_code', i18n: 'source.openClaudeCode' },
};

const enabledActions = computed(() =>
  settings.enabledQuickActions
    .map(k => ACTION_META[k])
    .filter(Boolean),
);

const isScanning = computed(() => settings.scanningSourceIds.has(props.source.id));
const isBigSource = computed(() => props.tasks.length > BIG_SOURCE_TASK_THRESHOLD);

/// Kind icon as a real emoji that flips on expand/collapse. Folder gets the
/// open-folder glyph when expanded so the visual matches the disclosure
/// state; file gets a pencil-and-page when expanded to imply "editable".
const kindEmoji = computed(() => {
  if (props.source.kind === 'folder') return collapsed.value ? '📁' : '📂';
  return collapsed.value ? '📄' : '📝';
});
</script>

<template>
  <section class="group" :class="{ collapsed }">
    <header
      class="group-head"
      :class="{
        'source-dragging': isSourceDragging,
        'source-drop-target': isSourceDropTarget,
      }"
      :data-source-id="source.id"
      @click="collapsed = !collapsed"
      :title="collapsed ? t('source.expand') : t('source.collapse')"
    >
      <!-- Caret is decorative now — the whole header handles the toggle.
           Keeping the visual chevron so the disclosure affordance still
           reads, but click cost goes to anywhere in the bar. -->
      <span class="caret" aria-hidden="true">
        <Icon :name="collapsed ? 'chevron-right' : 'chevron-down'" :size="14" />
      </span>
      <span class="kind-icon" aria-hidden="true">{{ kindEmoji }}</span>
      <span class="label" :title="source.path">{{ displayLabel }}</span>
      <span v-if="isDefault" class="badge" :title="t('source.defaultBadge')">{{ t('source.defaultBadge') }}</span>
      <span v-if="isScanning" class="scanning" :title="t('source.scanning')">
        <Icon name="loader" :size="14" />
      </span>
      <span class="counts">
        <span class="count-todo">{{ counts.todo }}</span>
        <template v-if="counts.done">
          <span class="count-sep"> · </span>
          <span class="count-done">{{ counts.done }}</span>
        </template>
      </span>
      <!-- Actions cluster: clicks here must NOT bubble to the header
           toggle, otherwise tapping an action would collapse the group.
           Drag-and-drop reorders the buttons across every source. -->
      <div class="actions" @click.stop>
        <button
          v-for="a in enabledActions"
          :key="a.kind"
          class="icon-btn brand"
          :class="{
            dragging: dragKind === a.kind,
            'drop-target': dropTargetKind === a.kind,
          }"
          draggable="true"
          @dragstart.stop="onActionDragStart($event, a.kind)"
          @dragover="onActionDragOver($event, a.kind)"
          @dragleave="onActionDragLeave(a.kind)"
          @drop="onActionDrop($event, a.kind)"
          @dragend.stop="onActionDragEnd"
          @click="runAction(a.kind)"
          :title="t(a.i18n)"
        >
          <QuickActionIcon :kind="a.kind" />
        </button>
        <button
          class="icon-btn drag-handle"
          @pointerdown.stop="onDotsPointerDown"
          :title="t('source.edit')"
        >
          <Icon name="more-horizontal" :size="14" />
        </button>
      </div>
    </header>

    <p v-if="actionError" class="error" @click="actionError = null">{{ actionError }}</p>

    <div v-if="!collapsed" class="rows">
      <div v-if="isScanning" class="scanning-row">{{ t('source.scanningHint') }}</div>

      <!-- File source: render tasks directly. The source header *is* the
           file header; an extra FileGroup wrapper would just nest the same
           label twice. -->
      <template v-if="source.kind === 'file'">
        <TaskItem v-for="tk in tasks" :key="tk.id" :task="tk" />
        <div v-if="tasks.length === 0 && !isScanning" class="empty-source">{{ t('source.noTasks') }}</div>
      </template>

      <!-- Folder source: one FileGroup per .md file. -->
      <template v-else>
        <FileGroup
          v-for="g in fileGroups"
          :key="g.filePath"
          :source="source"
          :file-path="g.filePath"
          :tasks="g.tasks"
          :initial-collapsed="isBigSource"
        />
        <div v-if="fileGroups.length === 0 && !isScanning" class="empty-source">{{ t('source.noTasks') }}</div>
      </template>
    </div>
  </section>
</template>

<style scoped>
.group {
  border-bottom: 1px solid var(--border);
}
.group:last-child { border-bottom: none; }

.group-head {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.45rem 0.6rem;
  background: var(--surface-strong);
  font-size: 0.82rem;
  color: var(--text);
  user-select: none;
  cursor: pointer;
  transition: background 120ms ease-out, opacity 120ms ease-out;
}
.group-head:hover { background: var(--accent-soft); }

/* Drag feedback: dim the source being dragged, accent-outline the drop
   target so the user can see where the source will land. Outline (not
   border) keeps the header height stable during the dance. */
.group-head.source-dragging { opacity: 0.4; }
.group-head.source-drop-target {
  background: var(--accent-soft);
  outline: 2px dashed var(--accent);
  outline-offset: -2px;
}

.caret {
  width: 20px;
  height: 20px;
  color: var(--text-muted);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.group-head:hover .caret { color: var(--text); }

.kind-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  font-size: 14px;
  line-height: 1;
  /* Pin OS colour-emoji fonts so the glyphs render as real cartoon
     emoji rather than the monochrome fallback some webviews pick. */
  font-family: 'Segoe UI Emoji', 'Apple Color Emoji', 'Noto Color Emoji', sans-serif;
  flex-shrink: 0;
}

.label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 500;
}

.badge {
  font-size: 0.65rem;
  padding: 1px 5px;
  background: var(--accent-soft);
  color: var(--accent);
  border-radius: 3px;
  flex-shrink: 0;
}

.scanning {
  display: inline-flex;
  color: var(--accent);
  flex-shrink: 0;
  animation: spin 1.1s linear infinite;
}
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.scanning-row {
  padding: 0.5rem 0.75rem;
  font-size: 0.78rem;
  color: var(--text-muted);
  font-style: italic;
}

.counts {
  font-size: 0.72rem;
  flex-shrink: 0;
}
.counts .count-todo { color: var(--count-todo); font-weight: 500; }
.counts .count-sep  { color: var(--text-muted); }
.counts .count-done { color: var(--count-done); font-weight: 500; }

.actions {
  display: flex;
  gap: 2px;
  flex-shrink: 0;
}

.icon-btn {
  width: 24px;
  height: 22px;
  padding: 0;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 4px;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 0.8rem;
  line-height: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.icon-btn:hover {
  background: var(--surface);
  border-color: var(--border);
  color: var(--text);
}
.icon-btn.active {
  background: var(--accent-soft);
  color: var(--accent);
  border-color: var(--border);
}
.icon-btn.brand:hover {
  /* Use the brand colour as the hover background so the icon stays
     readable against a soft tint of itself. */
  background: color-mix(in srgb, currentColor 10%, transparent);
  border-color: color-mix(in srgb, currentColor 30%, transparent);
}

/* Drag feedback: dim the grabbed button, accent-outline the drop target. */
.icon-btn.dragging {
  opacity: 0.4;
  cursor: grabbing;
}
.icon-btn.drop-target {
  background: var(--accent-soft);
  border-color: var(--accent);
  transform: scale(1.05);
}
.icon-btn.brand[draggable="true"] {
  cursor: grab;
}
.icon-btn.brand[draggable="true"]:active {
  cursor: grabbing;
}

/* The "..." button is both a settings shortcut and the source drag handle.
   Grab cursor advertises the drag affordance; a plain click still opens
   settings because HTML5 distinguishes click from drag for us. */
.icon-btn.drag-handle { cursor: grab; }
.icon-btn.drag-handle:active { cursor: grabbing; }

.error {
  color: #ef4444;
  font-size: 0.75rem;
  margin: 0;
  padding: 0.35rem 0.75rem;
  background: rgba(239, 68, 68, 0.08);
  border-bottom: 1px solid rgba(239, 68, 68, 0.2);
  cursor: pointer;
}

.rows {
  padding: 0.2rem 0;
}

.empty-source {
  padding: 0.6rem 0.75rem;
  font-size: 0.75rem;
  color: var(--text-muted);
  font-style: italic;
}
</style>
