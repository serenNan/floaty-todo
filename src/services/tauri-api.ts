import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type { Task, AppConfig, Source, SourceKind } from '../types/task';

export const api = {
  getTasks: () => invoke<Task[]>('get_tasks'),
  getConfig: () => invoke<AppConfig>('get_config'),
  updateConfig: (cfg: AppConfig) => invoke<void>('update_config', { newConfig: cfg }),
  toggleTask: (taskId: string) => invoke<void>('toggle_task', { taskId }),
  addTask: (text: string, sourceId?: string) =>
    invoke<void>('add_task', { text, sourceId: sourceId ?? null }),

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
  }) =>
    invoke<Source>('update_source', {
      sourceId: args.sourceId,
      label: args.label ?? null,
      projectRoot: args.projectRoot ?? null,
    }),
  setDefaultSource: (sourceId: string | null) =>
    invoke<void>('set_default_source', { sourceId }),

  openInVscode: (sourceId: string) => invoke<void>('open_in_vscode', { sourceId }),
  openInTerminal: (sourceId: string) => invoke<void>('open_in_terminal', { sourceId }),

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
  onManageSourcesRequested: (cb: () => void): Promise<UnlistenFn> =>
    listen('request-manage-sources', cb),
};
