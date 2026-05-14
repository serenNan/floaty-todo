<script setup lang="ts">
import type { Task } from '../types/task';
import { useTaskStore } from '../stores/tasks';

defineProps<{ task: Task }>();
const tasks = useTaskStore();
</script>

<template>
  <label class="row" :class="{ done: task.completed }" :style="{ paddingLeft: 8 + task.indent * 12 + 'px' }">
    <input type="checkbox" :checked="task.completed" @change="tasks.toggle(task.id)" />
    <span class="text">{{ task.text }}</span>
  </label>
</template>

<style scoped>
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(4px); }
  to   { opacity: 1; transform: translateY(0); }
}

.row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.38rem 0.75rem;
  cursor: pointer;
  border-radius: 0;
  transition: background 120ms ease-out;
  animation: fadeIn 140ms ease-out;
  border-bottom: 1px solid transparent;
}

.row:hover {
  background: var(--surface-strong);
  border-bottom-color: var(--border);
}

.row.done .text {
  text-decoration: line-through;
  color: var(--text-muted);
  opacity: 0.7;
}

.text {
  flex: 1;
  user-select: text;
  color: var(--text);
  font-size: 0.875rem;
  line-height: 1.4;
}

input[type="checkbox"] {
  width: 15px;
  height: 15px;
  flex-shrink: 0;
  accent-color: var(--accent);
  cursor: pointer;
  background: var(--surface);
  border: 1px solid var(--border-strong);
  border-radius: 3px;
}
</style>
