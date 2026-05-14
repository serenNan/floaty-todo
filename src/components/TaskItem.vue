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
.row { display: flex; align-items: center; gap: 0.5rem; padding: 0.35rem 0.5rem; cursor: pointer; }
.row:hover { background: var(--bg-hover); }
.row.done .text { text-decoration: line-through; color: var(--fg-muted); }
.text { flex: 1; user-select: text; }
</style>
