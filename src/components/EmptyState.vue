<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { useSettingsStore } from '../stores/settings';
import { useTaskStore } from '../stores/tasks';
import Icon from './icons/Icon.vue';

defineEmits<{ openSettings: [] }>();

const { t } = useI18n();
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
      <h2>{{ t('empty.title') }}</h2>
      <p class="blurb">
        <template v-for="(line, idx) in t('empty.blurb', { ext: '.md' }).split('\n')" :key="idx">
          <span>{{ line }}</span><br v-if="idx === 0" />
        </template>
      </p>
      <div class="actions">
        <button @click="addFolder">
          <Icon name="folder" :size="15" />
          <span>{{ t('empty.addFolder') }}</span>
        </button>
        <button @click="addFile">
          <Icon name="file" :size="15" />
          <span>{{ t('empty.addFile') }}</span>
        </button>
      </div>
    </div>
    <button class="settings-corner" @click="$emit('openSettings')" :title="t('settings.title')">
      <span aria-hidden="true">⚙️</span>
    </button>
  </div>
</template>

<style scoped>
.empty-wrap {
  position: relative;
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

.empty-card .blurb {
  font-size: 0.85rem;
  color: var(--text-muted);
  line-height: 1.5;
  margin-bottom: 1.2rem;
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
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.empty-card button:hover {
  background: var(--accent-soft);
  box-shadow: var(--card-shadow);
}

.settings-corner {
  position: absolute;
  bottom: 0.6rem;
  left: 0.6rem;
  width: 28px;
  height: 28px;
  padding: 0;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-muted);
  opacity: 0.7;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.settings-corner:hover { opacity: 1; background: var(--surface-strong); color: var(--text); }
</style>
