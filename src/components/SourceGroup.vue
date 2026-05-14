<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Source, Task } from '../types/task';
import { useSettingsStore } from '../stores/settings';
import { api } from '../services/tauri-api';
import TaskItem from './TaskItem.vue';

const props = defineProps<{ source: Source; tasks: Task[] }>();
const { t } = useI18n();
const settings = useSettingsStore();

const collapsed = ref(false);
const editing = ref(false);
const labelDraft = ref('');
const rootDraft = ref('');
const actionError = ref<string | null>(null);

const isDefault = computed(() => settings.defaultSourceId === props.source.id);

const displayLabel = computed(() => {
  const s = props.source;
  if (s.label && s.label.trim()) return s.label;
  return s.path.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? s.path;
});

const counts = computed(() => {
  let todo = 0, done = 0;
  for (const t of props.tasks) (t.completed ? done++ : todo++);
  return { todo, done };
});

function startEdit() {
  labelDraft.value = props.source.label ?? '';
  rootDraft.value = props.source.project_root ?? '';
  actionError.value = null;
  editing.value = true;
}

function cancelEdit() {
  editing.value = false;
  actionError.value = null;
}

async function saveEdit() {
  try {
    await settings.updateSource({
      sourceId: props.source.id,
      label: labelDraft.value.trim() || null,
      projectRoot: rootDraft.value.trim() || null,
    });
    editing.value = false;
  } catch (e: any) {
    actionError.value = String(e);
  }
}

async function pickRoot() {
  const p = await api.pickFolder();
  if (p) rootDraft.value = p;
}

async function setDefault() {
  try { await settings.setDefaultSource(props.source.id); }
  catch (e: any) { actionError.value = String(e); }
}

async function removeSource() {
  if (!confirm(t('source.removeConfirm', { label: displayLabel.value }))) return;
  try { await settings.removeSource(props.source.id); }
  catch (e: any) { actionError.value = String(e); }
}

async function openVscode() {
  try { await api.openInVscode(props.source.id); }
  catch (e: any) { actionError.value = String(e); }
}
async function openTerminal() {
  try { await api.openInTerminal(props.source.id); }
  catch (e: any) { actionError.value = String(e); }
}
</script>

<template>
  <section class="group" :class="{ collapsed }">
    <header class="group-head">
      <button class="caret" @click="collapsed = !collapsed" :title="collapsed ? t('source.expand') : t('source.collapse')">
        {{ collapsed ? '▸' : '▾' }}
      </button>
      <span class="kind-icon">{{ source.kind === 'folder' ? '📁' : '📄' }}</span>
      <span class="label" :title="source.path">{{ displayLabel }}</span>
      <span v-if="isDefault" class="badge" :title="t('source.defaultBadge')">{{ t('source.defaultBadge') }}</span>
      <span class="counts">
        {{ counts.todo }}<span v-if="counts.done"> · {{ counts.done }}✓</span>
      </span>
      <div class="actions">
        <button class="icon-btn" @click="openVscode" :title="t('source.openVscode')">⎘</button>
        <button class="icon-btn" @click="openTerminal" :title="t('source.openTerminal')">▷</button>
        <button class="icon-btn" :class="{ active: editing }" @click="editing ? cancelEdit() : startEdit()" :title="t('source.edit')">⋯</button>
      </div>
    </header>

    <div v-if="editing" class="editor">
      <label>
        {{ t('source.fields.label') }}
        <input v-model="labelDraft" :placeholder="displayLabel" />
      </label>
      <label>
        {{ t('source.fields.projectRoot') }} <span class="hint">{{ t('source.fields.projectRootHint') }}</span>
        <span class="root-row">
          <input v-model="rootDraft" :placeholder="source.path" />
          <button type="button" @click="pickRoot" :title="t('source.actions.pickFolder')">📁</button>
        </span>
      </label>
      <div class="editor-actions">
        <button type="button" class="ghost" :disabled="isDefault" @click="setDefault">
          {{ isDefault ? t('source.actions.isDefault') : t('source.actions.setDefault') }}
        </button>
        <button type="button" class="danger" @click="removeSource">{{ t('source.actions.remove') }}</button>
        <span class="spacer"></span>
        <button type="button" class="ghost" @click="cancelEdit">{{ t('source.actions.cancel') }}</button>
        <button type="button" class="primary" @click="saveEdit">{{ t('source.actions.save') }}</button>
      </div>
      <p v-if="actionError" class="error">{{ actionError }}</p>
    </div>

    <div v-if="!collapsed" class="rows">
      <TaskItem v-for="tk in tasks" :key="tk.id" :task="tk" />
      <div v-if="tasks.length === 0 && !editing" class="empty-source">{{ t('source.noTasks') }}</div>
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
}

.caret {
  width: 18px;
  padding: 0;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 0.75rem;
}
.caret:hover { color: var(--text); }

.kind-icon { font-size: 0.85rem; opacity: 0.85; }

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

.counts {
  font-size: 0.72rem;
  color: var(--text-muted);
  flex-shrink: 0;
}

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

.editor {
  padding: 0.5rem 0.6rem 0.6rem;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  animation: slide-down 140ms ease-out;
}

@keyframes slide-down {
  from { opacity: 0; transform: translateY(-4px); }
  to { opacity: 1; transform: translateY(0); }
}

.editor label {
  display: flex;
  flex-direction: column;
  gap: 3px;
  font-size: 0.72rem;
  color: var(--text-muted);
}
.editor label .hint { color: var(--text-muted); font-weight: normal; opacity: 0.7; }

.editor input {
  padding: 0.3rem 0.5rem;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--text);
  font-size: 0.82rem;
}
.editor input:focus { outline: none; border-color: var(--border-strong); }

.root-row { display: flex; gap: 4px; }
.root-row input { flex: 1; }
.root-row button {
  width: 30px;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 5px;
  cursor: pointer;
}

.editor-actions {
  display: flex;
  gap: 6px;
  margin-top: 2px;
  align-items: center;
}
.spacer { flex: 1; }

.editor-actions button {
  padding: 0.3rem 0.7rem;
  font-size: 0.78rem;
  border-radius: 5px;
  cursor: pointer;
  border: 1px solid var(--border);
  background: var(--surface-strong);
  color: var(--text);
}
.editor-actions button:hover { background: var(--accent-soft); }
.editor-actions button.primary { background: var(--accent); color: var(--surface); border-color: var(--accent); }
.editor-actions button.primary:hover { opacity: 0.9; }
.editor-actions button.danger { color: #ef4444; border-color: rgba(239,68,68,0.3); }
.editor-actions button.danger:hover { background: rgba(239,68,68,0.1); }
.editor-actions button.ghost { background: transparent; }
.editor-actions button:disabled { opacity: 0.5; cursor: default; }

.error {
  color: #ef4444;
  font-size: 0.75rem;
  margin: 0;
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
