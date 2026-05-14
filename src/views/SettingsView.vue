<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useSettingsStore } from '../stores/settings';
import { useTheme } from '../composables/useTheme';
import { setLocale, SUPPORTED_LOCALES, type Locale } from '../i18n';
import { api } from '../services/tauri-api';
import type { Source } from '../types/task';

defineEmits<{ back: [] }>();

const { t, locale } = useI18n();
const settings = useSettingsStore();
const { currentTheme, setTheme } = useTheme();

const editingId = ref<string | null>(null);
const labelDraft = ref('');
const rootDraft = ref('');
const actionError = ref<string | null>(null);

const themes: Array<{ value: 'system' | 'light' | 'dark'; key: 'system' | 'light' | 'dark'; icon: string }> = [
  { value: 'system', key: 'system', icon: '🖥' },
  { value: 'light', key: 'light', icon: '☀' },
  { value: 'dark', key: 'dark', icon: '🌙' },
];

const languages: Array<{ value: Locale; label: string }> = [
  { value: 'en', label: 'English' },
  { value: 'zh', label: '中文' },
];

const defaultId = computed(() => settings.defaultSourceId);

function displayLabel(s: Source): string {
  if (s.label && s.label.trim()) return s.label;
  return s.path.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? s.path;
}

async function pickLocale(e: Event) {
  const v = (e.target as HTMLSelectElement).value;
  if ((SUPPORTED_LOCALES as readonly string[]).includes(v)) {
    setLocale(v as Locale);
  }
}

function startEdit(s: Source) {
  editingId.value = s.id;
  labelDraft.value = s.label ?? '';
  rootDraft.value = s.project_root ?? '';
  actionError.value = null;
}

function cancelEdit() {
  editingId.value = null;
  actionError.value = null;
}

async function saveEdit(s: Source) {
  try {
    await settings.updateSource({
      sourceId: s.id,
      label: labelDraft.value.trim() || null,
      projectRoot: rootDraft.value.trim() || null,
    });
    editingId.value = null;
  } catch (e: any) {
    actionError.value = String(e);
  }
}

async function pickRoot() {
  const p = await api.pickFolder();
  if (p) rootDraft.value = p;
}

async function setDefault(s: Source) {
  try { await settings.setDefaultSource(s.id); }
  catch (e: any) { actionError.value = String(e); }
}

async function removeSource(s: Source) {
  if (!confirm(t('source.removeConfirm', { label: displayLabel(s) }))) return;
  try {
    await settings.removeSource(s.id);
    if (editingId.value === s.id) editingId.value = null;
  } catch (e: any) {
    actionError.value = String(e);
  }
}

async function addFolder() {
  try {
    const src = await settings.pickAndAddFolder();
    if (!src) return;
  } catch (e: any) { actionError.value = String(e); }
}
async function addFile() {
  try {
    const src = await settings.pickAndAddFile();
    if (!src) return;
  } catch (e: any) { actionError.value = String(e); }
}

async function openVscode(s: Source) {
  try { await api.openInVscode(s.id); }
  catch (e: any) { actionError.value = String(e); }
}
async function openTerminal(s: Source) {
  try { await api.openInTerminal(s.id); }
  catch (e: any) { actionError.value = String(e); }
}
</script>

<template>
  <div class="settings">
    <header class="head">
      <button class="back-btn" @click="$emit('back')" :title="t('settings.back')">←</button>
      <h2>{{ t('settings.title') }}</h2>
    </header>

    <div class="body">
      <!-- Appearance -->
      <section class="section">
        <h3>{{ t('settings.sections.appearance') }}</h3>
        <div class="row">
          <span class="row-label">{{ t('settings.theme.label') }}</span>
          <div class="segmented">
            <button
              v-for="th in themes"
              :key="th.value"
              :class="{ active: currentTheme === th.value }"
              @click="setTheme(th.value)"
              :title="t(`settings.theme.${th.key}`)"
            >
              <span class="seg-icon">{{ th.icon }}</span>
              <span class="seg-label">{{ t(`settings.theme.${th.key}`) }}</span>
            </button>
          </div>
        </div>
      </section>

      <!-- Language -->
      <section class="section">
        <h3>{{ t('settings.sections.language') }}</h3>
        <div class="row">
          <span class="row-label">{{ t('settings.language.label') }}</span>
          <select :value="locale" @change="pickLocale">
            <option v-for="l in languages" :key="l.value" :value="l.value">{{ l.label }}</option>
          </select>
        </div>
      </section>

      <!-- Sources -->
      <section class="section">
        <h3>{{ t('settings.sections.sources') }}</h3>
        <div class="source-toolbar">
          <button @click="addFolder">{{ t('settings.sources.addFolder') }}</button>
          <button @click="addFile">{{ t('settings.sources.addFile') }}</button>
        </div>

        <p v-if="settings.sources.length === 0" class="muted">{{ t('settings.sources.empty') }}</p>

        <div v-for="s in settings.sources" :key="s.id" class="source-card">
          <div class="src-row">
            <span class="src-icon">{{ s.kind === 'folder' ? '📁' : '📄' }}</span>
            <div class="src-main">
              <div class="src-label">
                {{ displayLabel(s) }}
                <span v-if="defaultId === s.id" class="default-pill">{{ t('settings.sources.defaultBadge') }}</span>
              </div>
              <div class="src-path" :title="s.path">{{ s.path }}</div>
            </div>
            <div class="src-actions">
              <button class="icon-btn" @click="openVscode(s)" :title="t('settings.sources.openVscode')">⎘</button>
              <button class="icon-btn" @click="openTerminal(s)" :title="t('settings.sources.openTerminal')">▷</button>
              <button class="icon-btn" :class="{ active: editingId === s.id }"
                @click="editingId === s.id ? cancelEdit() : startEdit(s)"
                :title="t('settings.sources.edit')">📝</button>
              <button class="icon-btn danger" @click="removeSource(s)" :title="t('settings.sources.remove')">🗑</button>
            </div>
          </div>

          <div v-if="editingId === s.id" class="src-editor">
            <label>
              {{ t('source.fields.label') }}
              <input v-model="labelDraft" :placeholder="displayLabel(s)" />
            </label>
            <label>
              {{ t('source.fields.projectRoot') }}
              <span class="hint">{{ t('source.fields.projectRootHint') }}</span>
              <span class="root-row">
                <input v-model="rootDraft" :placeholder="s.path" />
                <button type="button" @click="pickRoot" :title="t('source.actions.pickFolder')">📁</button>
              </span>
            </label>
            <div class="edit-actions">
              <button type="button" class="ghost" :disabled="defaultId === s.id" @click="setDefault(s)">
                {{ defaultId === s.id ? t('source.actions.isDefault') : t('settings.sources.setDefault') }}
              </button>
              <span class="spacer"></span>
              <button type="button" class="ghost" @click="cancelEdit">{{ t('source.actions.cancel') }}</button>
              <button type="button" class="primary" @click="saveEdit(s)">{{ t('source.actions.save') }}</button>
            </div>
          </div>
        </div>

        <p v-if="actionError" class="error">{{ actionError }}</p>
      </section>

      <!-- About -->
      <section class="section">
        <h3>{{ t('settings.sections.about') }}</h3>
        <p class="muted">{{ t('settings.about.tagline') }}</p>
      </section>
    </div>
  </div>
</template>

<style scoped>
.settings {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  background: var(--surface);
}

.head {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.6rem;
  border-bottom: 1px solid var(--border);
  background: var(--surface-strong);
  flex-shrink: 0;
}
.head h2 {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text);
}
.back-btn {
  width: 28px;
  height: 26px;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 5px;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 1rem;
  line-height: 1;
}
.back-btn:hover { background: var(--surface); border-color: var(--border); color: var(--text); }

.body {
  flex: 1;
  overflow-y: auto;
  padding: 0.6rem 0.7rem 1rem;
}

.section { margin-bottom: 1.1rem; }
.section h3 {
  margin: 0 0 0.4rem;
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-muted);
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.4rem 0;
}
.row-label { font-size: 0.85rem; color: var(--text); }

.segmented {
  display: inline-flex;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 7px;
  overflow: hidden;
}
.segmented button {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 0.35rem 0.55rem;
  background: transparent;
  border: none;
  cursor: pointer;
  font-size: 0.78rem;
  color: var(--text-muted);
}
.segmented button:hover { background: var(--surface); color: var(--text); }
.segmented button.active {
  background: var(--accent-soft);
  color: var(--accent);
}
.segmented button + button { border-left: 1px solid var(--border); }
.seg-icon { font-size: 0.8rem; }

select {
  padding: 0.35rem 0.5rem;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  font-size: 0.82rem;
  cursor: pointer;
}

.source-toolbar {
  display: flex;
  gap: 0.4rem;
  margin-bottom: 0.5rem;
}
.source-toolbar button {
  padding: 0.3rem 0.7rem;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  font-size: 0.8rem;
  cursor: pointer;
}
.source-toolbar button:hover { background: var(--accent-soft); }

.source-card {
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 8px;
  margin-bottom: 0.45rem;
  overflow: hidden;
}

.src-row {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.5rem 0.6rem;
}
.src-icon { font-size: 0.95rem; flex-shrink: 0; }
.src-main { flex: 1; min-width: 0; }
.src-label {
  font-size: 0.86rem;
  font-weight: 500;
  color: var(--text);
  display: flex;
  align-items: center;
  gap: 5px;
}
.default-pill {
  font-size: 0.62rem;
  padding: 1px 5px;
  background: var(--accent-soft);
  color: var(--accent);
  border-radius: 3px;
  font-weight: normal;
}
.src-path {
  font-size: 0.72rem;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.src-actions { display: flex; gap: 2px; flex-shrink: 0; }
.icon-btn {
  width: 26px;
  height: 24px;
  padding: 0;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 4px;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 0.82rem;
  line-height: 1;
}
.icon-btn:hover {
  background: var(--surface);
  border-color: var(--border);
  color: var(--text);
}
.icon-btn.active {
  background: var(--accent-soft);
  color: var(--accent);
  border-color: var(--border);
}
.icon-btn.danger:hover {
  color: #ef4444;
  border-color: rgba(239,68,68,0.3);
  background: rgba(239,68,68,0.08);
}

.src-editor {
  border-top: 1px solid var(--border);
  padding: 0.5rem 0.6rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  animation: slide-down 140ms ease-out;
}
@keyframes slide-down {
  from { opacity: 0; transform: translateY(-4px); }
  to { opacity: 1; transform: translateY(0); }
}
.src-editor label {
  display: flex;
  flex-direction: column;
  gap: 3px;
  font-size: 0.72rem;
  color: var(--text-muted);
}
.src-editor .hint { color: var(--text-muted); opacity: 0.7; font-weight: normal; }
.src-editor input {
  padding: 0.3rem 0.5rem;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--text);
  font-size: 0.82rem;
}
.src-editor input:focus { outline: none; border-color: var(--border-strong); }
.root-row { display: flex; gap: 4px; }
.root-row input { flex: 1; }
.root-row button {
  width: 30px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 5px;
  cursor: pointer;
}

.edit-actions {
  display: flex;
  gap: 6px;
  align-items: center;
  margin-top: 2px;
}
.spacer { flex: 1; }
.edit-actions button {
  padding: 0.3rem 0.7rem;
  font-size: 0.78rem;
  border-radius: 5px;
  cursor: pointer;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text);
}
.edit-actions button:hover { background: var(--accent-soft); }
.edit-actions button.primary { background: var(--accent); color: var(--surface); border-color: var(--accent); }
.edit-actions button.primary:hover { opacity: 0.9; }
.edit-actions button.ghost { background: transparent; }
.edit-actions button:disabled { opacity: 0.5; cursor: default; }

.muted { color: var(--text-muted); font-size: 0.82rem; margin: 0.2rem 0 0; }
.error { color: #ef4444; font-size: 0.78rem; margin: 0.4rem 0 0; }
</style>
