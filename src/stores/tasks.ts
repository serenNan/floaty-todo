import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { Task } from '../types/task';
import { api } from '../services/tauri-api';

export const useTaskStore = defineStore('tasks', () => {
  const tasks = ref<Task[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function refresh() {
    loading.value = true;
    error.value = null;
    try {
      tasks.value = await api.getTasks();
    } catch (e: any) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function toggle(id: string) {
    try { await api.toggleTask(id); await refresh(); }
    catch (e: any) { error.value = String(e); }
  }

  async function add(text: string) {
    if (!text.trim()) return;
    try { await api.addTask(text.trim()); await refresh(); }
    catch (e: any) { error.value = String(e); }
  }

  return { tasks, loading, error, refresh, toggle, add };
});
