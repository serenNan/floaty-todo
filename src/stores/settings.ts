import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { AppConfig, Source, SourceKind } from '../types/task';
import { api } from '../services/tauri-api';

export const useSettingsStore = defineStore('settings', () => {
  const config = ref<AppConfig | null>(null);

  const sources = computed<Source[]>(() => config.value?.sources ?? []);
  const hasSources = computed(() => sources.value.length > 0);
  const defaultSourceId = computed(() => config.value?.default_source_id ?? null);

  async function load() {
    config.value = await api.getConfig();
  }

  async function addSource(args: {
    path: string;
    kind: SourceKind;
    label?: string | null;
    projectRoot?: string | null;
  }): Promise<Source> {
    const src = await api.addSource(args);
    await load();
    return src;
  }

  async function removeSource(sourceId: string) {
    await api.removeSource(sourceId);
    await load();
  }

  async function updateSource(args: {
    sourceId: string;
    label?: string | null;
    projectRoot?: string | null;
  }): Promise<Source> {
    const src = await api.updateSource(args);
    await load();
    return src;
  }

  async function setDefaultSource(sourceId: string | null) {
    await api.setDefaultSource(sourceId);
    await load();
  }

  /// Convenience: open the folder picker, then add the chosen path as a Folder source.
  async function pickAndAddFolder(): Promise<Source | null> {
    const path = await api.pickFolder();
    if (!path) return null;
    return await addSource({ path, kind: 'folder' });
  }

  /// Convenience: open the file picker, then add the chosen path as a File source.
  async function pickAndAddFile(): Promise<Source | null> {
    const path = await api.pickMarkdownFile();
    if (!path) return null;
    return await addSource({ path, kind: 'file' });
  }

  return {
    config,
    sources,
    hasSources,
    defaultSourceId,
    load,
    addSource,
    removeSource,
    updateSource,
    setDefaultSource,
    pickAndAddFolder,
    pickAndAddFile,
  };
});
