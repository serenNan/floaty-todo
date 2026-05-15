<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import { useSettingsStore } from './stores/settings';
import { useTaskStore } from './stores/tasks';
import { useHistoryStore } from './stores/history';
import { useTheme } from './composables/useTheme';
import { openQuickAdd } from './composables/useQuickAdd';
import { toast } from './composables/useToast';
import { api } from './services/tauri-api';
import EmptyState from './components/EmptyState.vue';
import TaskList from './components/TaskList.vue';
import SettingsView from './views/SettingsView.vue';
import ConfirmDialog from './components/ConfirmDialog.vue';
import TaskEditorDialog from './components/TaskEditorDialog.vue';
import QuickAddDialog from './components/QuickAddDialog.vue';
import ToastContainer from './components/ToastContainer.vue';

type View = 'tasks' | 'settings';

const settings = useSettingsStore();
const tasks = useTaskStore();
const history = useHistoryStore();
// Touch useTheme so its system-pref listener mounts at the App root.
useTheme();
const { t } = useI18n();
const hasSources = computed(() => settings.hasSources);
const view = ref<View>('tasks');

const unlisteners: Array<() => void> = [];

onMounted(async () => {
  await settings.load();
  if (hasSources.value) await tasks.refresh();
  await history.refresh();
  unlisteners.push(await api.onTasksUpdated(() => { tasks.silentRefresh(); }));
  unlisteners.push(await api.onHistoryUpdated(() => { history.refresh(); }));
  unlisteners.push(await api.onHistorySeenChanged(() => { history.syncLastSeenFromStorage(); }));
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
  unlisteners.push(await api.onTriggerQuickAdd(async (wasHidden) => {
    // 全局 quick-add 快捷键触发：选默认源（没有默认就第一个），弹 QuickAdd。
    const sourceId = settings.defaultSourceId ?? settings.sources[0]?.id ?? null;
    if (!sourceId) {
      toast.info(t('toast.addSourceFirst'));
      return;
    }
    const result = await openQuickAdd({ sourceId });
    if (result) {
      await tasks.add(result.text, result.sourceId, result.quadrant);
      // 窗口本是被快捷键临时呼出的 —— 存完任务退回隐藏。取消(Esc)则不动。
      if (wasHidden) await invoke('hide_window');
    }
  }));
  unlisteners.push(await api.onHotkeyRegisterFailed((accelerator) => {
    toast.warning(t('toast.hotkeyRegisterFailed', { accel: accelerator }));
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
    <TaskEditorDialog />
    <QuickAddDialog />
    <ToastContainer />
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
