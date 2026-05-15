import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { AppConfig, ApplyResult, HotkeyConfig, QuickActionKind, Source, SourceKind } from '../types/task';
import { api } from '../services/tauri-api';
import { errorMessage } from '../utils/errors';
import { toast } from '../composables/useToast';
import { i18n } from '../i18n';

const t = i18n.global.t;

function failed(e: unknown) {
  toast.error(t('toast.operationFailed', { reason: errorMessage(e) }));
}

export const useSettingsStore = defineStore('settings', () => {
  const config = ref<AppConfig | null>(null);

  const sources = computed<Source[]>(() => config.value?.sources ?? []);
  const hasSources = computed(() => sources.value.length > 0);
  const defaultSourceId = computed(() => config.value?.default_source_id ?? null);
  const fileLabels = computed<Record<string, string>>(() => config.value?.file_labels ?? {});
  const enabledQuickActions = computed<QuickActionKind[]>(
    () => config.value?.enabled_quick_actions ?? [],
  );
  const alwaysOnTop = computed<boolean>(() => config.value?.always_on_top ?? true);
  const hubFolder = computed<string | null>(() => config.value?.hub_folder ?? null);
  const hotkeys = computed<HotkeyConfig>(
    () => config.value?.hotkeys ?? { toggle: null, quick_add: null },
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
    try {
      const src = await api.addSource(args);
      // Spinner is driven entirely by the backend's `source-scan-started` /
      // `source-scan-finished` events. Manually marking here used to race
      // the events for tiny files: backend emits both events before our
      // post-await code runs, then we'd flip scanning back to `true` with
      // no future `finished` event to clear it — spinner stuck forever.
      await load();
      toast.success(t('toast.sourceAdded', { label: src.label || src.path }));
      return src;
    } catch (e) {
      failed(e);
      throw e;
    }
  }

  async function removeSource(sourceId: string) {
    try {
      await api.removeSource(sourceId);
      await load();
      toast.success(t('toast.sourceRemoved'));
    } catch (e) {
      failed(e);
      throw e;
    }
  }

  async function updateSource(args: {
    sourceId: string;
    label?: string | null;
    projectRoot?: string | null;
    color?: string | null;
  }): Promise<Source> {
    try {
      const src = await api.updateSource(args);
      await load();
      toast.success(t('toast.sourceUpdated'));
      return src;
    } catch (e) {
      failed(e);
      throw e;
    }
  }

  async function setDefaultSource(sourceId: string | null) {
    try {
      await api.setDefaultSource(sourceId);
      await load();
      if (sourceId) toast.success(t('toast.sourceDefaultSet'));
    } catch (e) {
      failed(e);
      throw e;
    }
  }

  async function reorderSources(orderedIds: string[]) {
    try {
      await api.reorderSources(orderedIds);
      await load();
      toast.success(t('toast.sourceReordered'));
    } catch (e) {
      failed(e);
      throw e;
    }
  }

  async function setFileLabel(filePath: string, label: string | null) {
    try {
      await api.setFileLabel(filePath, label);
      await load();
      toast.success(t('toast.fileLabelSet'));
    } catch (e) {
      failed(e);
      throw e;
    }
  }

  async function setEnabledQuickActions(actions: QuickActionKind[]) {
    try {
      await api.setEnabledQuickActions(actions);
      await load();
      toast.success(t('toast.quickActionsUpdated'));
    } catch (e) {
      failed(e);
      throw e;
    }
  }

  async function setAlwaysOnTop(on: boolean) {
    await api.setAlwaysOnTop(on);
    await load();
  }
  async function toggleAlwaysOnTop() {
    await setAlwaysOnTop(!alwaysOnTop.value);
  }

  async function setHubFolder(path: string | null) {
    try {
      await api.setHubFolder(path);
      await load();
      toast.success(path === null ? t('toast.hubCleared') : t('toast.hubSet'));
    } catch (e) {
      failed(e);
      throw e;
    }
  }

  const autoCreateQuadrantHeaders = computed(
    () => config.value?.auto_create_quadrant_headers ?? true,
  );

  async function setAutoCreateQuadrantHeaders(on: boolean): Promise<void> {
    if (!config.value) return;
    const next = { ...config.value, auto_create_quadrant_headers: on };
    await api.updateConfig(next);
    config.value = next;
  }

  async function resyncHub() {
    try {
      await api.resyncHub();
      await load();
      toast.success(t('toast.hubResynced'));
    } catch (e) {
      failed(e);
      throw e;
    }
  }
  /// Re-register hotkeys, reload config so the `hotkeys` computed reflects
  /// what actually persisted, and hand the per-key result back to the caller
  /// (SettingsView decides the toast since it knows which key changed).
  async function setHotkeys(
    toggle: string | null,
    quickAdd: string | null,
  ): Promise<ApplyResult> {
    const result = await api.setHotkeys(toggle, quickAdd);
    await load();
    return result;
  }

  /// Open the OS folder picker and use the result as the new hub folder.
  /// Returns the chosen path, or null if the picker was cancelled.
  async function pickAndSetHubFolder(): Promise<string | null> {
    const path = await api.pickFolder();
    if (!path) return null;
    await setHubFolder(path);
    return path;
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
    alwaysOnTop,
    hubFolder,
    hotkeys,
    autoCreateQuadrantHeaders,
    scanningSourceIds,
    isScanning,
    markScanning,
    load,
    addSource,
    removeSource,
    updateSource,
    setDefaultSource,
    reorderSources,
    setFileLabel,
    setEnabledQuickActions,
    setAlwaysOnTop,
    toggleAlwaysOnTop,
    setHubFolder,
    setAutoCreateQuadrantHeaders,
    resyncHub,
    setHotkeys,
    pickAndSetHubFolder,
    pickAndAddFolder,
    pickAndAddFile,
  };
});
