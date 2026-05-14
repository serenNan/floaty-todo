<script setup lang="ts">
import { useSettingsStore } from '../stores/settings';
import { useTaskStore } from '../stores/tasks';

const settings = useSettingsStore();
const tasks = useTaskStore();

async function addFolder() {
  const src = await settings.pickAndAddFolder();
  if (src) await tasks.refresh();
}

async function addFile() {
  const src = await settings.pickAndAddFile();
  if (src) await tasks.refresh();
}
</script>

<template>
  <div class="empty-wrap">
    <div class="empty-card">
      <h2>Floaty Todo</h2>
      <p>Add your first todo source.<br>A folder (recursive) or a single <code>.md</code> file.</p>
      <div class="actions">
        <button @click="addFolder">📁 Folder…</button>
        <button @click="addFile">📄 File…</button>
      </div>
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

.actions {
  display: flex;
  gap: 0.6rem;
  justify-content: center;
}

.empty-card button {
  padding: 0.5rem 1.1rem;
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
