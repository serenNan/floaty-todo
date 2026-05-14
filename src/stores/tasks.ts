import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { Task, Quadrant } from '../types/task';
import { api } from '../services/tauri-api';

export const useTaskStore = defineStore('tasks', () => {
  const tasks = ref<Task[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // Undone first, then done; within each group, stable order by file + line.
  const sortedTasks = computed(() => {
    return [...tasks.value].sort((a, b) => {
      if (a.completed !== b.completed) return a.completed ? 1 : -1;
      if (a.source_file !== b.source_file) return a.source_file < b.source_file ? -1 : 1;
      return a.line_number - b.line_number;
    });
  });

  async function _fetch() {
    try {
      tasks.value = await api.getTasks();
      error.value = null;
    } catch (e: any) {
      error.value = String(e);
    }
  }

  // Foreground refresh (shows Loading…). Use only on first load / manual ↻.
  async function refresh() {
    loading.value = true;
    try { await _fetch(); } finally { loading.value = false; }
  }

  // Background refresh — no loading flicker. Use after toggle / add / fs event.
  async function silentRefresh() { await _fetch(); }

  async function toggle(id: string) {
    try { await api.toggleTask(id); await silentRefresh(); }
    catch (e: any) { error.value = String(e); }
  }

  async function update(id: string, text: string) {
    if (!text.trim()) return;
    try { await api.updateTask(id, text.trim()); await silentRefresh(); }
    catch (e: any) { error.value = String(e); }
  }

  async function add(text: string, sourceId?: string, quadrant?: Quadrant | null) {
    if (!text.trim()) return;
    try { await api.addTask(text.trim(), sourceId, quadrant); await silentRefresh(); }
    catch (e: any) { error.value = String(e); }
  }

  return { tasks, sortedTasks, loading, error, refresh, silentRefresh, toggle, update, add };
});
