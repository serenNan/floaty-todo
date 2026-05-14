<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Task } from '../types/task';
import { useTaskStore } from '../stores/tasks';
import { useSettingsStore } from '../stores/settings';
import { api } from '../services/tauri-api';
import SourceGroup from './SourceGroup.vue';
import Icon from './icons/Icon.vue';
import QuickActionIcon from './icons/QuickActionIcon.vue';

defineEmits<{ openSettings: [] }>();

const { t } = useI18n();
const tasks = useTaskStore();
const settings = useSettingsStore();
const newText = ref('');
const pickedSourceId = ref<string | null>(null);

const addTargetId = computed(() => pickedSourceId.value ?? settings.defaultSourceId);

const addTargetLabel = computed(() => {
  const id = addTargetId.value;
  if (!id) return '—';
  const s = settings.sources.find(x => x.id === id);
  if (!s) return '—';
  return s.label ?? s.path.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? s.path;
});

const groupedTasks = computed(() => {
  const buckets = new Map<string, Task[]>();
  for (const s of settings.sources) buckets.set(s.id, []);
  for (const tk of tasks.sortedTasks) {
    const arr = buckets.get(tk.source_id);
    if (arr) arr.push(tk);
  }
  return settings.sources.map(s => ({ source: s, tasks: buckets.get(s.id) ?? [] }));
});

const totals = computed(() => {
  let todo = 0, done = 0;
  for (const tk of tasks.tasks) (tk.completed ? done++ : todo++);
  return { todo, done };
});

async function submit() {
  if (!newText.value.trim()) return;
  await tasks.add(newText.value, addTargetId.value ?? undefined);
  newText.value = '';
}

// "+" button now adds a *source* (folder or file), not a task. Tasks are
// still added by pressing Enter in the input. We pop a tiny menu so the
// user picks the kind without us having to choose for them.
const showAddSourceMenu = ref(false);
const addSourceWrap = ref<HTMLElement | null>(null);

function toggleAddSource() { showAddSourceMenu.value = !showAddSourceMenu.value; }
function closeAddSource() { showAddSourceMenu.value = false; }

function onDocClick(e: MouseEvent) {
  if (!showAddSourceMenu.value) return;
  const wrap = addSourceWrap.value;
  if (wrap && !wrap.contains(e.target as Node)) closeAddSource();
}
function onEsc(e: KeyboardEvent) {
  if (e.key === 'Escape' && showAddSourceMenu.value) closeAddSource();
}
onMounted(() => {
  document.addEventListener('click', onDocClick);
  document.addEventListener('keydown', onEsc);
});
onUnmounted(() => {
  document.removeEventListener('click', onDocClick);
  document.removeEventListener('keydown', onEsc);
});

async function addFolderSource() {
  closeAddSource();
  const src = await settings.pickAndAddFolder();
  if (src) await tasks.refresh();
}
async function addFileSource() {
  closeAddSource();
  const src = await settings.pickAndAddFile();
  if (src) await tasks.refresh();
}

async function openHubInVscode() {
  try { await api.openHub('vscode'); }
  catch (e) { console.warn('openHub vscode failed:', e); }
}
async function openHubInClaudeCode() {
  try { await api.openHub('claude_code'); }
  catch (e) { console.warn('openHub claude_code failed:', e); }
}
</script>

<template>
  <div class="list">
    <form class="add-row" @submit.prevent="submit">
      <input v-model="newText" :placeholder="t('tasks.addPlaceholder', { target: addTargetLabel })" />
      <select v-model="pickedSourceId" class="source-select" :title="addTargetLabel">
        <option :value="null">{{ t('tasks.targetDefault', { label: addTargetLabel }) }}</option>
        <option v-for="s in settings.sources" :key="s.id" :value="s.id">
          {{ s.label ?? s.path.split(/[\\/]/).filter(Boolean).pop() ?? s.path }}
        </option>
      </select>
      <div class="add-source-wrap" ref="addSourceWrap">
        <button
          type="button"
          class="add-source-btn"
          :class="{ open: showAddSourceMenu }"
          @click.stop="toggleAddSource"
          :title="t('tasks.addSourceTitle')"
        >
          <Icon name="plus" :size="16" />
        </button>
        <div v-if="showAddSourceMenu" class="add-source-menu" @click.stop>
          <button type="button" @click="addFolderSource">
            <Icon name="folder" :size="14" />
            <span>{{ t('empty.addFolder') }}</span>
          </button>
          <button type="button" @click="addFileSource">
            <Icon name="file" :size="14" />
            <span>{{ t('empty.addFile') }}</span>
          </button>
        </div>
      </div>
    </form>

    <div class="rows-wrap">
      <div v-if="tasks.loading" class="hint">{{ t('tasks.loading') }}</div>
      <div v-else-if="tasks.error" class="error">{{ tasks.error }}</div>
      <template v-else>
        <SourceGroup
          v-for="g in groupedTasks"
          :key="g.source.id"
          :source="g.source"
          :tasks="g.tasks"
        />
      </template>
    </div>

    <div class="footer">
      <button class="footer-btn icon-only" @click="$emit('openSettings')" :title="t('settings.title')">
        <Icon name="settings" :size="15" />
      </button>
      <span class="counts">
        {{ t('tasks.todoCount', { n: totals.todo }) }} · {{ t('tasks.doneCount', { n: totals.done }) }}
      </span>
      <span class="spacer"></span>
      <!-- Hub shortcuts — only show when the user has configured a hub,
           otherwise they have nowhere to point. Brand-coloured so they
           feel like the per-source action buttons above. -->
      <template v-if="settings.hubFolder">
        <button
          class="footer-btn icon-only brand"
          @click="openHubInVscode"
          :title="t('hub.openVscode')"
        >
          <QuickActionIcon kind="vscode" />
        </button>
        <button
          class="footer-btn icon-only brand"
          @click="openHubInClaudeCode"
          :title="t('hub.openClaudeCode')"
        >
          <QuickActionIcon kind="claude_code" />
        </button>
      </template>
      <button
        class="footer-btn icon-only pin-btn"
        :class="{ active: settings.alwaysOnTop }"
        @click="settings.toggleAlwaysOnTop"
        :title="settings.alwaysOnTop ? t('window.unpin') : t('window.pin')"
      >
        <!-- Use the actual U+1F4CC pushpin emoji — looks like a real
             cartoon thumbtack from the OS font (Segoe UI Emoji on
             Windows). Unpinned state uses grayscale + reduced opacity
             so the affordance reads as "off" without losing the shape. -->
        <span class="pin-emoji" aria-hidden="true">📌</span>
      </button>
      <button class="footer-btn icon-only" @click="tasks.refresh" :title="t('tasks.refreshTitle')">
        <Icon name="refresh" :size="15" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.list {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.add-row {
  display: flex;
  padding: 0.5rem;
  gap: 0.4rem;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
  flex-shrink: 0;
}

.add-row input {
  flex: 1;
  min-width: 0;
  padding: 0.4rem 0.6rem;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  transition: border-color 120ms ease-out;
}

.add-row input:focus {
  outline: none;
  border-color: var(--border-strong);
}

.add-row input::placeholder { color: var(--text-muted); }

.source-select {
  /* No fixed max-width — let the control size to its longest option
     so labels like "★ WishTalk" don't get clipped. We cap at 45% of
     the row width to keep some room for the input. */
  max-width: 45%;
  padding: 0.4rem 0.4rem;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  font-size: 0.78rem;
  cursor: pointer;
  text-overflow: ellipsis;
}

.add-source-wrap {
  position: relative;
  display: inline-block;
}
.add-source-btn {
  width: 32px;
  height: 100%;
  padding: 0;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-muted);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.add-source-btn:hover,
.add-source-btn.open {
  background: var(--accent-soft);
  color: var(--text);
  border-color: var(--border-strong);
}

.add-source-menu {
  position: absolute;
  top: calc(100% + 4px);
  right: 0;
  min-width: 140px;
  background: var(--surface);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
  padding: 4px;
  z-index: 50;
  display: flex;
  flex-direction: column;
  gap: 2px;
  animation: pop-down 140ms cubic-bezier(0.2, 0.9, 0.3, 1.2);
}
@keyframes pop-down {
  from { opacity: 0; transform: translateY(-4px); }
  to { opacity: 1; transform: translateY(0); }
}
.add-source-menu button {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0.4rem 0.6rem;
  background: transparent;
  border: none;
  border-radius: 5px;
  font-size: 0.82rem;
  color: var(--text);
  cursor: pointer;
  text-align: left;
  width: 100%;
}
.add-source-menu button:hover {
  background: var(--accent-soft);
}

.rows-wrap {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  background: var(--surface);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
}

.hint, .error {
  padding: 2rem 1rem;
  text-align: center;
  font-size: 0.875rem;
}
.hint { color: var(--text-muted); }
.error { color: #ef4444; }

.footer {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.4rem 0.6rem;
  border-top: 1px solid var(--border);
  background: var(--surface);
  font-size: 0.8rem;
  color: var(--text-muted);
  flex-shrink: 0;
}

.spacer { flex: 1; }

.footer-btn {
  padding: 0.2rem 0.5rem;
  font-size: 0.85rem;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 120ms ease-out;
  line-height: 1;
}
.footer-btn:hover {
  color: var(--text);
  background: var(--accent-soft);
  border-color: var(--border-strong);
}

.footer-btn.icon-only {
  width: 28px;
  padding: 0;
  height: 26px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

/* Brand hover ring picks up the icon's own colour, mirroring the
   .brand modifier used on per-source quick-action buttons. */
.footer-btn.brand:hover {
  background: color-mix(in srgb, currentColor 10%, transparent);
  border-color: color-mix(in srgb, currentColor 30%, transparent);
}

.pin-emoji {
  font-size: 15px;
  line-height: 1;
  display: inline-block;
  /* Force the system's colour-emoji glyph rather than the monochrome
     fallback some browsers pick when an emoji sits next to plain text. */
  font-family: 'Segoe UI Emoji', 'Apple Color Emoji', 'Noto Color Emoji', sans-serif;
  transition: filter 140ms ease-out, transform 140ms ease-out;
}

.pin-btn.active {
  border-color: color-mix(in srgb, #ef4444 35%, var(--border));
  background: color-mix(in srgb, #ef4444 12%, transparent);
}
.pin-btn.active:hover {
  background: color-mix(in srgb, #ef4444 20%, transparent);
  border-color: color-mix(in srgb, #ef4444 55%, var(--border));
}
.pin-btn.active .pin-emoji {
  /* Tilt slightly so it visibly differs from the off state at a glance. */
  transform: rotate(-12deg);
}

/* Off: desaturate the colour emoji + fade so the toggle reads as "loose". */
.pin-btn:not(.active) .pin-emoji {
  filter: grayscale(0.85) opacity(0.55);
}
.pin-btn:not(.active):hover .pin-emoji {
  filter: grayscale(0) opacity(0.85);
}
</style>
