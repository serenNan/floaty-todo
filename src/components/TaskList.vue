<script setup lang="ts">
import { ref } from 'vue';
import { useTaskStore } from '../stores/tasks';
import TaskItem from './TaskItem.vue';

const tasks = useTaskStore();
const newText = ref('');

async function submit() {
  if (!newText.value.trim()) return;
  await tasks.add(newText.value);
  newText.value = '';
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
      <span>{{ tasks.tasks.filter(t => !t.completed).length }} todo · {{ tasks.tasks.filter(t => t.completed).length }} done</span>
      <button @click="tasks.refresh">↻</button>
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
.footer { display: flex; justify-content: space-between; align-items: center; padding: 0.4rem 0.6rem; border-top: 1px solid var(--border); font-size: 0.85em; color: var(--fg-muted); }
</style>
