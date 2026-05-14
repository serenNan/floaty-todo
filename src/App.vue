<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue';
import { useSettingsStore } from './stores/settings';
import { useTaskStore } from './stores/tasks';
import { useTheme } from './composables/useTheme';
import { api } from './services/tauri-api';
import EmptyState from './components/EmptyState.vue';
import TaskList from './components/TaskList.vue';

const settings = useSettingsStore();
const tasks = useTaskStore();
const { currentTheme, setTheme } = useTheme();
const hasSources = computed(() => settings.hasSources);

const themeIcon = computed(() =>
  currentTheme.value === 'light' ? '☀' :
  currentTheme.value === 'dark' ? '🌙' : '🖥'
);
const themeButtonTitle = computed(() => `Theme: ${currentTheme.value} (click to cycle)`);

function cycleTheme() {
  const next = currentTheme.value === 'system' ? 'light'
    : currentTheme.value === 'light' ? 'dark' : 'system';
  setTheme(next);
}

const unlisteners: Array<() => void> = [];

onMounted(async () => {
  await settings.load();
  if (hasSources.value) await tasks.refresh();
  unlisteners.push(await api.onTasksUpdated(() => { tasks.silentRefresh(); }));
  unlisteners.push(await api.onSourcesChanged(async () => {
    await settings.load();
    await tasks.silentRefresh();
  }));
  unlisteners.push(await api.onManageSourcesRequested(async () => {
    // TODO(v0.2 UI): open source manager panel. For now, just refresh.
    await settings.load();
  }));
});

onUnmounted(() => { unlisteners.forEach(u => u()); });
</script>

<template>
  <main>
    <div class="content">
      <Transition name="fade" mode="out-in">
        <EmptyState v-if="!hasSources" key="empty" />
        <TaskList v-else key="list" />
      </Transition>
    </div>
    <button
      class="theme-toggle"
      :title="themeButtonTitle"
      @click="cycleTheme"
    >{{ themeIcon }}</button>
  </main>
</template>

<style>
@import './styles/main.css';

main {
  position: relative;
}

.content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
}

.theme-toggle {
  position: absolute;
  bottom: 0.4rem;
  right: 0.5rem;
  width: 26px;
  height: 26px;
  padding: 0;
  font-size: 0.95rem;
  line-height: 1;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  opacity: 0.7;
  z-index: 10;
}
.theme-toggle:hover { opacity: 1; background: var(--surface-strong); }
</style>
