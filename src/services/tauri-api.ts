import { invoke } from '@tauri-apps/api/core';
import { listen, emit, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type { Task, AppConfig, Source, SourceKind, QuickActionKind, Quadrant, ApplyResult } from '../types/task';
import type { HistoryEvent } from '../types/history';

export const api = {
  getTasks: () => invoke<Task[]>('get_tasks'),
  getConfig: () => invoke<AppConfig>('get_config'),
  updateConfig: (cfg: AppConfig) => invoke<void>('update_config', { newConfig: cfg }),
  toggleTask: (taskId: string) => invoke<void>('toggle_task', { taskId }),
  /// `newQuadrant`:
  ///   - `undefined` → keep current quadrant, text-only edit
  ///   - `null`      → move to the unsorted bucket
  ///   - a Quadrant  → move to that quadrant section
  updateTask: (taskId: string, newText: string, newQuadrant?: Quadrant | null) =>
    invoke<void>('update_task', {
      taskId,
      newText,
      changeQuadrant: newQuadrant !== undefined,
      newQuadrant: newQuadrant ?? null,
    }),
  async addTask(text: string, sourceId?: string, quadrant?: Quadrant | null): Promise<void> {
    await invoke('add_task', {
      text,
      sourceId: sourceId ?? null,
      quadrant: quadrant ?? null,
    });
  },

  getHistory: (limit = 500, beforeId?: string | null) =>
    invoke<HistoryEvent[]>('get_history', { limit, beforeId: beforeId ?? null }),
  getHistoryCursor: () => invoke<string | null>('get_history_cursor'),
  undo: () => invoke<HistoryEvent | null>('undo'),
  redo: () => invoke<HistoryEvent | null>('redo'),
  jumpTo: (eventId: string, confirmExternal = false) =>
    invoke<{ undone_count: number; redone_count: number; skipped_external: number }>('jump_to', {
      eventId,
      confirmExternal,
    }),
  openHistoryWindow: () => invoke<void>('open_history_window'),

  listSources: () => invoke<Source[]>('list_sources'),
  addSource: (args: {
    path: string;
    kind: SourceKind;
    label?: string | null;
    projectRoot?: string | null;
  }) =>
    invoke<Source>('add_source', {
      path: args.path,
      kind: args.kind,
      label: args.label ?? null,
      projectRoot: args.projectRoot ?? null,
    }),
  removeSource: (sourceId: string) => invoke<void>('remove_source', { sourceId }),
  updateSource: (args: {
    sourceId: string;
    label?: string | null;
    projectRoot?: string | null;
    color?: string | null;
  }) =>
    invoke<Source>('update_source', {
      sourceId: args.sourceId,
      label: args.label ?? null,
      projectRoot: args.projectRoot ?? null,
      color: args.color ?? null,
    }),
  reorderSources: (orderedIds: string[]) =>
    invoke<void>('reorder_sources', { orderedIds }),
  setDefaultSource: (sourceId: string | null) =>
    invoke<void>('set_default_source', { sourceId }),
  setFileLabel: (filePath: string, label: string | null) =>
    invoke<void>('set_file_label', { filePath, label }),

  openInVscode: (sourceId: string) => invoke<void>('open_in_vscode', { sourceId }),
  openInTerminal: (sourceId: string) => invoke<void>('open_in_terminal', { sourceId }),
  openInClaudeCode: (sourceId: string) => invoke<void>('open_in_claude_code', { sourceId }),
  runQuickAction: (sourceId: string, kind: QuickActionKind) =>
    invoke<void>('run_quick_action', { sourceId, kind }),
  setEnabledQuickActions: (actions: QuickActionKind[]) =>
    invoke<void>('set_enabled_quick_actions', { actions }),
  openUrl: (url: string) => invoke<void>('open_url', { url }),
  setAlwaysOnTop: (on: boolean) => invoke<void>('set_always_on_top', { on }),
  hideWindow: () => invoke<void>('hide_window'),
  /// Re-register both global hotkeys. Pass the full pair every time —
  /// the backend persists the whole HotkeyConfig. `null` = unbind that key.
  setHotkeys: (toggle: string | null, quickAdd: string | null) =>
    invoke<ApplyResult>('set_hotkeys', { toggle, quickAdd }),

  setHubFolder: (path: string | null) => invoke<void>('set_hub_folder', { path }),
  resyncHub: () => invoke<void>('resync_hub'),
  openHub: (kind: QuickActionKind) => invoke<void>('open_hub', { kind }),

  pickFolder: async (): Promise<string | null> => {
    const sel = await open({ directory: true, multiple: false });
    return typeof sel === 'string' ? sel : null;
  },
  pickMarkdownFile: async (): Promise<string | null> => {
    const sel = await open({
      directory: false,
      multiple: false,
      filters: [{ name: 'Markdown', extensions: ['md', 'markdown'] }],
    });
    return typeof sel === 'string' ? sel : null;
  },

  onTasksUpdated: (cb: () => void): Promise<UnlistenFn> => listen('tasks-updated', cb),
  onSourcesChanged: (cb: () => void): Promise<UnlistenFn> => listen('sources-changed', cb),
  onHistoryUpdated: (cb: () => void): Promise<UnlistenFn> => listen('history-updated', cb),
  onManageSourcesRequested: (cb: () => void): Promise<UnlistenFn> =>
    listen('request-manage-sources', cb),
  onSourceScanStarted: (cb: (sourceId: string) => void): Promise<UnlistenFn> =>
    listen<string>('source-scan-started', e => cb(e.payload)),
  onSourceScanFinished: (cb: (sourceId: string) => void): Promise<UnlistenFn> =>
    listen<string>('source-scan-finished', e => cb(e.payload)),
  emitHistorySeen: (): Promise<void> => emit('history-seen-changed'),
  onHistorySeenChanged: (cb: () => void): Promise<UnlistenFn> =>
    listen('history-seen-changed', cb),
  onTriggerQuickAdd: (cb: (wasHidden: boolean) => void): Promise<UnlistenFn> =>
    listen<{ wasHidden: boolean }>('trigger-quick-add', e => cb(e.payload.wasHidden)),
  onHotkeyRegisterFailed: (cb: (accelerator: string) => void): Promise<UnlistenFn> =>
    listen<string>('hotkey-register-failed', e => cb(e.payload)),
};
