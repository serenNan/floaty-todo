<script setup lang="ts">
import { computed, ref } from 'vue';
import { useTaskStore } from '../stores/tasks';
import { useSettingsStore } from '../stores/settings';
import TaskItem from './TaskItem.vue';

const tasks = useTaskStore();
const settings = useSettingsStore();
const newText = ref('');

const vaultName = computed(() => {
  const p = settings.config?.vault_path;
  if (!p) return '(none)';
  return p.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? p;
});

async function submit() {
  if (!newText.value.trim()) return;
  await tasks.add(newText.value);
  newText.value = '';
}

async function switchVault() {
  const ok = await settings.pickAndSetVault();
  if (ok) await tasks.refresh();
}
</script>

<template>
  <div class="list">
    <form class="add-row" @submit.prevent="submit">
      <input v-model="newText" placeholder="Add task, Enter to confirm…" />
      <button type="submit">+</button>
    </form>

    <div v-if="tasks.loading" class="hint">Loading…</div>
    <div v-else-if="tasks.error" class="error">{{ tasks.error }}</div>
    <div v-else-if="tasks.tasks.length === 0" class="hint">No tasks yet.</div>
    <div v-else class="rows">
      <TaskItem v-for="t in tasks.tasks" :key="t.id" :task="t" />
    </div>

    <div class="footer">
      <span class="counts">{{ tasks.tasks.filter(t => !t.completed).length }} todo · {{ tasks.tasks.filter(t => t.completed).length }} done</span>
      <span class="vault" :title="settings.config?.vault_path ?? ''">📁 {{ vaultName }}</span>
      <span class="actions">
        <button @click="switchVault" title="Switch vault folder">📂</button>
        <button @click="tasks.refresh" title="Refresh">↻</button>
      </span>
    </div>
  </div>
</template>

<style scoped>
.list { display: flex; flex-direction: column; height: 100vh; }
.add-row { display: flex; padding: 0.5rem; gap: 0.4rem; border-bottom: 1px solid var(--border); }
.add-row input { flex: 1; padding: 0.4rem; }
.rows { flex: 1; overflow-y: auto; }
.hint, .error { padding: 1rem; text-align: center; color: var(--fg-muted); }
.error { color: #c33; }
.footer { display: flex; justify-content: space-between; align-items: center; gap: 0.4rem; padding: 0.4rem 0.6rem; border-top: 1px solid var(--border); font-size: 0.85em; color: var(--fg-muted); }
.footer .vault { flex: 0 1 auto; max-width: 40%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; opacity: 0.8; }
.footer .actions { display: flex; gap: 0.3rem; }
.footer button { padding: 0.2rem 0.45rem; }
</style>
