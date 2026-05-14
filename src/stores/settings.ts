import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { AppConfig } from '../types/task';
import { api } from '../services/tauri-api';

export const useSettingsStore = defineStore('settings', () => {
  const config = ref<AppConfig | null>(null);

  async function load() { config.value = await api.getConfig(); }

  async function pickAndSetVault(): Promise<boolean> {
    const path = await api.pickVaultFolder();
    if (!path) return false;
    await api.setVault(path);
    await load();
    return true;
  }

  return { config, load, pickAndSetVault };
});
