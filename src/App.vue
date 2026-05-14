<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useSettingsStore } from './stores/settings';
import { useTaskStore } from './stores/tasks';
import { useTheme } from './composables/useTheme';
import { api } from './services/tauri-api';
import EmptyState from './components/EmptyState.vue';
import TaskList from './components/TaskList.vue';
import SettingsView from './views/SettingsView.vue';
import ConfirmDialog from './components/ConfirmDialog.vue';

type View = 'tasks' | 'settings';

const settings = useSettingsStore();
const tasks = useTaskStore();
// Touch useTheme so its system-pref listener mounts at the App root.
useTheme();
const hasSources = computed(() => settings.hasSources);
const view = ref<View>('tasks');

const unlisteners: Array<() => void> = [];

onMounted(async () => {
  await settings.load();
  if (hasSources.value) await tasks.refresh();
  unlisteners.push(await api.onTasksUpdated(() => { tasks.silentRefresh(); }));
  unlisteners.push(await api.onSourcesChanged(async () => {
    await settings.load();
    await tasks.silentRefresh();
  }));
  unlisteners.push(await api.onManageSourcesRequested(() => {
    view.value = 'settings';
  }));
  unlisteners.push(await api.onSourceScanStarted(id => {
    settings.markScanning(id, true);
  }));
  unlisteners.push(await api.onSourceScanFinished(id => {
    settings.markScanning(id, false);
  }));
});

onUnmounted(() => { unlisteners.forEach(u => u()); });

function openSettings() { view.value = 'settings'; }
function backToTasks() { view.value = 'tasks'; }
</script>

<template>
  <main>
    <div class="content">
      <Transition name="fade" mode="out-in">
        <SettingsView
          v-if="view === 'settings'"
          key="settings"
          @back="backToTasks"
        />
        <EmptyState
          v-else-if="!hasSources"
          key="empty"
          @open-settings="openSettings"
        />
        <TaskList
          v-else
          key="list"
          @open-settings="openSettings"
        />
      </Transition>
    </div>
    <ConfirmDialog />
  </main>
</template>

<style>
@import './styles/main.css';

main {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100vh;
}

.content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
}

/* Fade transition for view switches */
.fade-enter-active, .fade-leave-active {
  transition: opacity 140ms ease-out;
}
.fade-enter-from, .fade-leave-to {
  opacity: 0;
}
</style>
