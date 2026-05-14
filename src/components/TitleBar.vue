<script setup lang="ts">
import { computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useTheme } from '../composables/useTheme';

const { currentTheme, setTheme } = useTheme();

const themeIcon = computed(() => {
  if (currentTheme.value === 'light') return '☀';
  if (currentTheme.value === 'dark') return '🌙';
  return '🖥';
});

const themeTitle = computed(() => {
  if (currentTheme.value === 'light') return 'Theme: Light (click for Dark)';
  if (currentTheme.value === 'dark') return 'Theme: Dark (click for System)';
  return 'Theme: System (click for Light)';
});

function cycleTheme() {
  if (currentTheme.value === 'system') setTheme('light');
  else if (currentTheme.value === 'light') setTheme('dark');
  else setTheme('system');
}

async function minimize() {
  await getCurrentWindow().minimize();
}

async function close() {
  await invoke('hide_window');
}
</script>

<template>
  <div class="titlebar" data-tauri-drag-region>
    <span class="title" data-tauri-drag-region>Floaty Todo</span>
    <div class="controls">
      <button
        class="ctrl-btn"
        :title="themeTitle"
        @click="cycleTheme"
        data-tauri-drag-region="false"
      >{{ themeIcon }}</button>
      <button
        class="ctrl-btn"
        title="Minimize"
        @click="minimize"
        data-tauri-drag-region="false"
      >—</button>
      <button
        class="ctrl-btn close-btn"
        title="Hide to tray"
        @click="close"
        data-tauri-drag-region="false"
      >×</button>
    </div>
  </div>
</template>

<style scoped>
.titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 32px;
  padding: 0 8px 0 12px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
  user-select: none;
}

.title {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-muted);
  letter-spacing: 0.02em;
}

.controls {
  display: flex;
  gap: 2px;
}

.ctrl-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 22px;
  padding: 0;
  font-size: 13px;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: var(--text-muted);
  cursor: pointer;
  transition: background 100ms ease-out, color 100ms ease-out;
  line-height: 1;
}

.ctrl-btn:hover {
  background: var(--accent-soft);
  color: var(--text);
}

.close-btn:hover {
  background: rgba(239, 68, 68, 0.12);
  color: #ef4444;
}
</style>
