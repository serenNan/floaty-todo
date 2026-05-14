<script setup lang="ts">
import { computed } from 'vue';
import type { Task } from '../types/task';
import { useTaskStore } from '../stores/tasks';
import { api } from '../services/tauri-api';
import { parseInline } from '../utils/inline-md';
import { editTask } from '../composables/useTaskEditor';

const props = defineProps<{ task: Task }>();
const tasks = useTaskStore();

const segments = computed(() => parseInline(props.task.text));

async function onTextClick() {
  const next = await editTask(props.task);
  if (next !== null) await tasks.update(props.task.id, next);
}

async function openLink(href: string) {
  try { await api.openUrl(href); }
  catch (e) { console.warn('openUrl failed:', e); }
}
</script>

<template>
  <label class="row" :class="{ done: task.completed }" :style="{ paddingLeft: 8 + task.indent * 12 + 'px' }">
    <input type="checkbox" :checked="task.completed" @change="tasks.toggle(task.id)" />
    <!-- @click.prevent.stop blocks the <label>'s default "toggle the
         wrapped checkbox" behaviour, so clicking task text opens the
         editor modal instead of flipping the checkbox. Inline link
         clicks already use .prevent.stop so they keep priority. -->
    <span class="text" @click.prevent.stop="onTextClick">
      <template v-for="(seg, i) in segments" :key="i">
        <code v-if="seg.type === 'code'" class="md-code">{{ seg.text }}</code>
        <strong v-else-if="seg.type === 'bold'">{{ seg.text }}</strong>
        <em v-else-if="seg.type === 'italic'">{{ seg.text }}</em>
        <s v-else-if="seg.type === 'strike'">{{ seg.text }}</s>
        <a
          v-else-if="seg.type === 'link'"
          class="md-link"
          :href="seg.href"
          :title="seg.href"
          @click.prevent.stop="openLink(seg.href)"
        >{{ seg.text }}</a>
        <template v-else>{{ seg.text }}</template>
      </template>
    </span>
  </label>
</template>

<style scoped>
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(4px); }
  to   { opacity: 1; transform: translateY(0); }
}

.row {
  display: flex;
  align-items: flex-start;
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
  word-break: break-word;
  cursor: text;
}

.text strong { font-weight: 600; color: var(--text); }
.text em { font-style: italic; }
.text s { color: var(--text-muted); }
.text .md-code {
  font-family: 'Cascadia Code', 'Consolas', 'JetBrains Mono', ui-monospace, monospace;
  font-size: 0.82em;
  padding: 1px 5px;
  background: var(--accent-soft);
  color: var(--accent);
  border-radius: 4px;
  border: 1px solid var(--border);
}
.text .md-link {
  color: var(--accent);
  text-decoration: underline;
  text-decoration-color: color-mix(in srgb, var(--accent) 40%, transparent);
  text-underline-offset: 2px;
  cursor: pointer;
}
.text .md-link:hover {
  text-decoration-color: var(--accent);
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
  margin-top: 3px;
}
</style>
