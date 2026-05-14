<script setup lang="ts">
import { useSettingsStore } from '../stores/settings';
import { useTaskStore } from '../stores/tasks';

const settings = useSettingsStore();
const tasks = useTaskStore();

async function pick() {
  const ok = await settings.pickAndSetVault();
  if (ok) await tasks.refresh();
}
</script>

<template>
  <div class="empty">
    <h2>👋 Welcome to Floaty Todo</h2>
    <p>Pick an Obsidian vault folder. The app will scan all <code>.md</code> tasks inside.</p>
    <button @click="pick">Choose folder…</button>
  </div>
</template>

<style scoped>
.empty { padding: 2rem; text-align: center; color: var(--fg-muted); }
.empty button { margin-top: 1rem; padding: 0.6rem 1.2rem; cursor: pointer; }
</style>
