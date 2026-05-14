<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue';
import { useSettingsStore } from './stores/settings';
import { useTaskStore } from './stores/tasks';
import { api } from './services/tauri-api';
import EmptyState from './components/EmptyState.vue';
import TaskList from './components/TaskList.vue';

const settings = useSettingsStore();
const tasks = useTaskStore();
const hasVault = computed(() => !!settings.config?.vault_path);

let unlisten: (() => void) | null = null;

onMounted(async () => {
  await settings.load();
  if (hasVault.value) await tasks.refresh();
  unlisten = await api.onTasksUpdated(() => { tasks.refresh(); });
});

onUnmounted(() => { unlisten?.(); });
</script>

<template>
  <main>
    <EmptyState v-if="!hasVault" />
    <TaskList v-else />
  </main>
</template>

<style>
@import './styles/main.css';
</style>
