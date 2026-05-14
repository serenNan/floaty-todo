import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { AppConfig, QuickActionKind, Source, SourceKind } from '../types/task';
import { api } from '../services/tauri-api';

export const useSettingsStore = defineStore('settings', () => {
  const config = ref<AppConfig | null>(null);

  const sources = computed<Source[]>(() => config.value?.sources ?? []);
  const hasSources = computed(() => sources.value.length > 0);
  const defaultSourceId = computed(() => config.value?.default_source_id ?? null);
  const fileLabels = computed<Record<string, string>>(() => config.value?.file_labels ?? {});
  const enabledQuickActions = computed<QuickActionKind[]>(
    () => config.value?.enabled_quick_actions ?? [],
  );
  /// Source ids currently being scanned by the backend. UI shows a spinner /
  /// disables actions for those sources. Track-by-id to avoid blocking
  /// unrelated sources when one is scanning.
  const scanningSourceIds = ref<Set<string>>(new Set());
  const isScanning = computed(() => scanningSourceIds.value.size > 0);

  function markScanning(sourceId: string, on: boolean) {
    const next = new Set(scanningSourceIds.value);
    if (on) next.add(sourceId);
    else next.delete(sourceId);
    scanningSourceIds.value = next;
  }

  function fileLabel(filePath: string): string | null {
    return fileLabels.value[filePath] ?? null;
  }

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
    // Mark the source as scanning *before* `load()` so the SourceGroup
    // renders the spinner immediately. Backend's `source-scan-finished`
    // event will clear it; if that event races ahead of us, the worst
    // case is a brief stale spinner that the next emit clears.
    markScanning(src.id, true);
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

  async function setFileLabel(filePath: string, label: string | null) {
    await api.setFileLabel(filePath, label);
    await load();
  }

  async function setEnabledQuickActions(actions: QuickActionKind[]) {
    await api.setEnabledQuickActions(actions);
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
    fileLabels,
    fileLabel,
    enabledQuickActions,
    scanningSourceIds,
    isScanning,
    markScanning,
    load,
    addSource,
    removeSource,
    updateSource,
    setDefaultSource,
    setFileLabel,
    setEnabledQuickActions,
    pickAndAddFolder,
    pickAndAddFile,
  };
});
