<script setup lang="ts">
import { computed, ref } from 'vue';
import type { Task } from '../types/task';
import { useTaskStore } from '../stores/tasks';
import { useSettingsStore } from '../stores/settings';
import SourceGroup from './SourceGroup.vue';

const tasks = useTaskStore();
const settings = useSettingsStore();
const newText = ref('');
const pickedSourceId = ref<string | null>(null);

const addTargetId = computed(() => pickedSourceId.value ?? settings.defaultSourceId);

const addTargetLabel = computed(() => {
  const id = addTargetId.value;
  if (!id) return '(no source)';
  const s = settings.sources.find(x => x.id === id);
  if (!s) return '(missing)';
  return s.label ?? s.path.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? s.path;
});

// Group tasks by source, in user-defined source order.
const groupedTasks = computed(() => {
  const buckets = new Map<string, Task[]>();
  for (const s of settings.sources) buckets.set(s.id, []);
  for (const t of tasks.sortedTasks) {
    const arr = buckets.get(t.source_id);
    if (arr) arr.push(t);
  }
  return settings.sources.map(s => ({ source: s, tasks: buckets.get(s.id) ?? [] }));
});

const totals = computed(() => {
  let todo = 0, done = 0;
  for (const t of tasks.tasks) (t.completed ? done++ : todo++);
  return { todo, done };
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
      <input v-model="newText" :placeholder="`Add to ${addTargetLabel}…`" />
      <select v-model="pickedSourceId" class="source-select" :title="`Target: ${addTargetLabel}`">
        <option :value="null">default ({{ addTargetLabel }})</option>
        <option v-for="s in settings.sources" :key="s.id" :value="s.id">
          {{ s.label ?? s.path.split(/[\\/]/).filter(Boolean).pop() ?? s.path }}
        </option>
      </select>
      <button type="submit" title="Add task">+</button>
    </form>

    <div class="rows-wrap">
      <div v-if="tasks.loading" class="hint">Loading…</div>
      <div v-else-if="tasks.error" class="error">{{ tasks.error }}</div>
      <template v-else>
        <SourceGroup
          v-for="g in groupedTasks"
          :key="g.source.id"
          :source="g.source"
          :tasks="g.tasks"
        />
      </template>
    </div>

    <div class="footer">
      <span class="counts">
        {{ totals.todo }} todo · {{ totals.done }} done
      </span>
      <span class="spacer"></span>
      <button class="footer-btn" @click="addFolder" title="Add folder source">📁+</button>
      <button class="footer-btn" @click="addFile" title="Add file source">📄+</button>
      <button class="footer-btn" @click="tasks.refresh" title="Refresh (re-read all .md files)">↻</button>
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
  min-width: 0;
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

.add-row input::placeholder { color: var(--text-muted); }

.source-select {
  max-width: 90px;
  padding: 0.4rem 0.3rem;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  font-size: 0.78rem;
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

.hint, .error {
  padding: 2rem 1rem;
  text-align: center;
  font-size: 0.875rem;
}
.hint { color: var(--text-muted); }
.error { color: #ef4444; }

.footer {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.4rem 0.6rem;
  border-top: 1px solid var(--border);
  background: var(--surface);
  font-size: 0.8rem;
  color: var(--text-muted);
  flex-shrink: 0;
}

.spacer { flex: 1; }

.footer-btn {
  padding: 0.2rem 0.5rem;
  font-size: 0.8rem;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 120ms ease-out;
}
.footer-btn:hover {
  color: var(--text);
  background: var(--accent-soft);
  border-color: var(--border-strong);
}
</style>
