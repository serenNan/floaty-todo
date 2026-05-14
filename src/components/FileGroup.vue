<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Source, Task } from '../types/task';
import { useSettingsStore } from '../stores/settings';
import TaskItem from './TaskItem.vue';

const props = defineProps<{
  /// The source this file belongs to. Used to compute a relative path for the
  /// fallback display name (so two `todo.md` files in different folders show
  /// `sub-a/todo.md` and `sub-b/todo.md` instead of just `todo.md`).
  source: Source;
  /// Canonical absolute path of the file (matches the key used by
  /// `AppConfig.file_labels` and `Task.source_file`).
  filePath: string;
  tasks: Task[];
}>();

const { t } = useI18n();
const settings = useSettingsStore();

const collapsed = ref(false);
const editing = ref(false);
const labelDraft = ref('');

// User-provided label, if any.
const customLabel = computed(() => settings.fileLabel(props.filePath));

// Fallback: filename for File sources / single-file folders; relative path
// inside the source for nested files (so `sub/todo.md` is shown when there
// are siblings).
const fallbackLabel = computed(() => {
  const norm = (p: string) => p.replace(/\\/g, '/');
  const file = norm(props.filePath);
  const root = norm(props.source.path);
  if (props.source.kind === 'folder' && file.toLowerCase().startsWith(root.toLowerCase() + '/')) {
    return file.slice(root.length + 1);
  }
  return file.split('/').filter(Boolean).pop() ?? props.filePath;
});

const displayLabel = computed(() => customLabel.value ?? fallbackLabel.value);

const counts = computed(() => {
  let todo = 0, done = 0;
  for (const tk of props.tasks) (tk.completed ? done++ : todo++);
  return { todo, done };
});

function startEdit() {
  labelDraft.value = customLabel.value ?? '';
  editing.value = true;
}

function cancelEdit() { editing.value = false; }

async function saveLabel() {
  const next = labelDraft.value.trim();
  await settings.setFileLabel(props.filePath, next === '' ? null : next);
  editing.value = false;
}

async function clearLabel() {
  await settings.setFileLabel(props.filePath, null);
  editing.value = false;
}
</script>

<template>
  <div class="file-group" :class="{ collapsed }">
    <header class="head">
      <button class="caret" @click="collapsed = !collapsed" :title="collapsed ? t('source.expand') : t('source.collapse')">
        {{ collapsed ? '▸' : '▾' }}
      </button>
      <span class="name" :title="filePath">{{ displayLabel }}</span>
      <span class="count">{{ counts.todo }}<span v-if="counts.done" class="done">·{{ counts.done }}✓</span></span>
      <button
        class="edit-btn"
        :class="{ active: editing }"
        @click="editing ? cancelEdit() : startEdit()"
        :title="t('file.editLabel')"
      >✎</button>
    </header>

    <div v-if="editing" class="editor">
      <input
        v-model="labelDraft"
        :placeholder="fallbackLabel"
        @keydown.enter.prevent="saveLabel"
        @keydown.escape.prevent="cancelEdit"
        autofocus
      />
      <button class="ghost" @click="clearLabel" :title="t('file.resetLabel')">↺</button>
      <button class="primary" @click="saveLabel">{{ t('source.actions.save') }}</button>
    </div>

    <div v-if="!collapsed" class="rows">
      <TaskItem v-for="tk in tasks" :key="tk.id" :task="tk" />
      <div v-if="tasks.length === 0" class="empty">{{ t('file.noTasks') }}</div>
    </div>
  </div>
</template>

<style scoped>
.file-group {
  border-top: 1px solid var(--border);
}
.file-group:first-child { border-top: none; }

.head {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.32rem 0.55rem 0.32rem 1.6rem;
  background: var(--surface);
  font-size: 0.78rem;
  color: var(--text);
  user-select: none;
}

.caret {
  width: 16px;
  padding: 0;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 0.7rem;
  line-height: 1;
}
.caret:hover { color: var(--text); }

.name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-muted);
}

.count {
  font-size: 0.7rem;
  color: var(--text-muted);
  flex-shrink: 0;
}
.count .done { margin-left: 4px; opacity: 0.8; }

.edit-btn {
  width: 22px;
  height: 20px;
  padding: 0;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 4px;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 0.75rem;
  line-height: 1;
  opacity: 0;
  transition: opacity 120ms;
}
.head:hover .edit-btn { opacity: 1; }
.edit-btn:hover {
  background: var(--surface-strong);
  border-color: var(--border);
  color: var(--text);
}
.edit-btn.active {
  opacity: 1;
  background: var(--accent-soft);
  color: var(--accent);
  border-color: var(--border);
}

.editor {
  display: flex;
  gap: 4px;
  padding: 0.4rem 0.55rem 0.4rem 1.6rem;
  background: var(--surface-strong);
  animation: slide-down 140ms ease-out;
}
@keyframes slide-down {
  from { opacity: 0; transform: translateY(-3px); }
  to { opacity: 1; transform: translateY(0); }
}
.editor input {
  flex: 1;
  padding: 0.28rem 0.5rem;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--text);
  font-size: 0.8rem;
}
.editor input:focus { outline: none; border-color: var(--border-strong); }
.editor button {
  padding: 0.28rem 0.55rem;
  font-size: 0.75rem;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--text);
  cursor: pointer;
}
.editor button.primary {
  background: var(--accent);
  color: var(--surface);
  border-color: var(--accent);
}
.editor button.primary:hover { opacity: 0.9; }
.editor button.ghost:hover { background: var(--accent-soft); }

.rows { padding: 0.15rem 0; }

.empty {
  padding: 0.4rem 0.55rem 0.4rem 1.6rem;
  font-size: 0.72rem;
  color: var(--text-muted);
  font-style: italic;
}
</style>
