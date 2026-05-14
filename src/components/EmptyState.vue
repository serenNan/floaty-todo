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
  <div class="empty-wrap">
    <div class="empty-card">
      <h2>Floaty Todo</h2>
      <p>Pick an Obsidian vault folder.<br>The app will scan all <code>.md</code> tasks inside.</p>
      <button @click="pick">Choose folder…</button>
    </div>
  </div>
</template>

<style scoped>
.empty-wrap {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
}

.empty-card {
  text-align: center;
  padding: 2rem 1.5rem;
  background: var(--surface);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
  border-radius: 16px;
  border: 1px solid var(--border);
  box-shadow: var(--card-shadow);
  width: 100%;
}

.empty-card h2 {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 0.6rem;
}

.empty-card p {
  font-size: 0.85rem;
  color: var(--text-muted);
  line-height: 1.5;
  margin-bottom: 1.2rem;
}

.empty-card code {
  font-size: 0.8em;
  background: var(--accent-soft);
  padding: 1px 4px;
  border-radius: 3px;
  color: var(--accent);
}

.empty-card button {
  padding: 0.5rem 1.2rem;
  background: var(--surface-strong);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  cursor: pointer;
  font-size: 0.875rem;
  color: var(--text);
  transition: background 140ms ease-out, box-shadow 140ms ease-out;
}

.empty-card button:hover {
  background: var(--accent-soft);
  box-shadow: var(--card-shadow);
}
</style>
