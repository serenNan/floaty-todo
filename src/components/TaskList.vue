<script setup lang="ts">
import { computed, ref } from 'vue';
import { useTaskStore } from '../stores/tasks';
import { useSettingsStore } from '../stores/settings';
import TaskItem from './TaskItem.vue';

const tasks = useTaskStore();
const settings = useSettingsStore();
const newText = ref('');
const selectedSourceId = ref<string | null>(null);

const sourcesSummary = computed(() => {
  const n = settings.sources.length;
  return n === 1 ? '1 source' : `${n} sources`;
});

const addTargetId = computed(() => selectedSourceId.value ?? settings.defaultSourceId);

const addTargetLabel = computed(() => {
  const id = addTargetId.value;
  if (!id) return '(none)';
  const s = settings.sources.find(x => x.id === id);
  if (!s) return '(missing)';
  return s.label ?? s.path.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? s.path;
});

async function submit() {
  if (!newText.value.trim()) return;
  await tasks.add(newText.value, addTargetId.value ?? undefined);
  newText.value = '';
}

async function addFolder() {
  const src = await settings.pickAndAddFolder();
  if (src) await tasks.refresh();
}
async function addFile() {
  const src = await settings.pickAndAddFile();
  if (src) await tasks.refresh();
}
</script>

<template>
  <div class="list">
    <form class="add-row" @submit.prevent="submit">
      <input v-model="newText" :placeholder="`Add task → ${addTargetLabel}`" />
      <select v-model="selectedSourceId" class="source-select" title="Target source for new task">
        <option :value="null">{{ addTargetLabel }} (default)</option>
        <option v-for="s in settings.sources" :key="s.id" :value="s.id">
          {{ s.label ?? s.path }}
        </option>
      </select>
      <button type="submit" title="Add task">+</button>
    </form>

    <div class="rows-wrap">
      <div v-if="tasks.loading" class="hint">Loading…</div>
      <div v-else-if="tasks.error" class="error">{{ tasks.error }}</div>
      <div v-else-if="tasks.tasks.length === 0" class="hint">No tasks yet.</div>
      <div v-else class="rows">
        <TaskItem v-for="t in tasks.sortedTasks" :key="t.id" :task="t" />
      </div>
    </div>

    <div class="footer">
      <span class="counts">
        {{ tasks.sortedTasks.filter(t => !t.completed).length }} todo
        · {{ tasks.sortedTasks.filter(t => t.completed).length }} done
      </span>
      <button
        class="vault-switch"
        title="Add folder source"
        @click="addFolder"
      >📁+</button>
      <button
        class="vault-switch"
        title="Add file source"
        @click="addFile"
      >📄+ {{ sourcesSummary }}</button>
      <button class="refresh" @click="tasks.refresh" title="Refresh (re-read all .md files)">↻</button>
    </div>
  </div>
</template>

<style scoped>
.list {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.add-row {
  display: flex;
  padding: 0.5rem;
  gap: 0.4rem;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
  flex-shrink: 0;
}

.add-row input {
  flex: 1;
  padding: 0.4rem 0.6rem;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  transition: border-color 120ms ease-out;
}

.add-row input:focus {
  outline: none;
  border-color: var(--border-strong);
}

.add-row input::placeholder {
  color: var(--text-muted);
}

.source-select {
  max-width: 80px;
  padding: 0.4rem 0.3rem;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  font-size: 0.8rem;
  cursor: pointer;
}

.add-row button {
  width: 32px;
  padding: 0;
  font-size: 18px;
  font-weight: 300;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-muted);
}

.add-row button:hover {
  background: var(--accent-soft);
  color: var(--text);
}

.rows-wrap {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  background: var(--surface);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
}

.rows {
  padding: 0.3rem 0;
}

.hint {
  padding: 2rem 1rem;
  text-align: center;
  color: var(--text-muted);
  font-size: 0.875rem;
}

.error {
  padding: 1rem;
  text-align: center;
  color: #ef4444;
  font-size: 0.875rem;
}

.footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.4rem;
  padding: 0.4rem 0.6rem;
  border-top: 1px solid var(--border);
  background: var(--surface);
  font-size: 0.8rem;
  color: var(--text-muted);
  flex-shrink: 0;
}

.footer .vault-switch {
  flex: 0 1 auto;
  max-width: 55%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding: 0.2rem 0.5rem;
  font-size: 0.8rem;
  color: var(--text-muted);
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 5px;
  cursor: pointer;
  transition: all 120ms ease-out;
}

.footer .vault-switch:hover {
  color: var(--text);
  background: var(--accent-soft);
  border-color: var(--border-strong);
}

.footer .refresh {
  padding: 0.2rem 0.45rem;
  font-size: 0.85rem;
  flex-shrink: 0;
}
</style>
