import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type { Task, AppConfig } from '../types/task';

export const api = {
  getTasks: () => invoke<Task[]>('get_tasks'),
  getConfig: () => invoke<AppConfig>('get_config'),
  updateConfig: (cfg: AppConfig) => invoke<void>('update_config', { newConfig: cfg }),
  toggleTask: (taskId: string) => invoke<void>('toggle_task', { taskId }),
  addTask: (text: string) => invoke<void>('add_task', { text }),
  setVault: (path: string) => invoke<void>('set_vault', { path }),

  pickVaultFolder: async (): Promise<string | null> => {
    const sel = await open({ directory: true, multiple: false });
    return typeof sel === 'string' ? sel : null;
  },

  onTasksUpdated: (cb: () => void): Promise<UnlistenFn> =>
    listen('tasks-updated', cb),

  onSwitchVaultRequested: (cb: () => void): Promise<UnlistenFn> =>
    listen('request-switch-vault', cb),
};
