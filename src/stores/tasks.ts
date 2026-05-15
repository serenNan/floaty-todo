import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { Task, Quadrant } from '../types/task';
import { api } from '../services/tauri-api';
import { errorMessage } from '../utils/errors';
import { toast } from '../composables/useToast';
import { i18n } from '../i18n';

const t = i18n.global.t;

const QUADRANT_LABEL_KEY: Record<Quadrant, string> = {
  urgent_important: 'quadrant.urgent_important',
  not_urgent_important: 'quadrant.not_urgent_important',
  urgent_not_important: 'quadrant.urgent_not_important',
  not_urgent_not_important: 'quadrant.not_urgent_not_important',
};

function trunc(s: string, max = 30): string {
  return s.length <= max ? s : s.slice(0, max - 1) + '…';
}

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
      error.value = errorMessage(e);
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
    const task = tasks.value.find(tk => tk.id === id);
    const text = task ? trunc(task.text) : '';
    const wasCompleted = task?.completed ?? false;
    try {
      await api.toggleTask(id);
      await silentRefresh();
      if (task) {
        toast.success(
          wasCompleted
            ? t('toast.taskUncompleted', { text })
            : t('toast.taskCompleted', { text }),
        );
      }
    } catch (e: any) {
      error.value = errorMessage(e);
      toast.error(t('toast.operationFailed', { reason: error.value }));
    }
  }

  async function update(id: string, text: string, quadrant?: Quadrant | null) {
    if (!text.trim()) return;
    const before = tasks.value.find(tk => tk.id === id);
    try {
      await api.updateTask(id, text.trim(), quadrant);
      await silentRefresh();
      const movedToDifferent =
        quadrant !== undefined && before && before.quadrant !== quadrant;
      if (movedToDifferent) {
        const key = quadrant ? QUADRANT_LABEL_KEY[quadrant] : 'quadrant.unsorted';
        toast.success(t('toast.taskMoved', { quadrant: t(key) }));
      } else {
        toast.success(t('toast.taskEdited'));
      }
    } catch (e: any) {
      error.value = errorMessage(e);
      toast.error(t('toast.operationFailed', { reason: error.value }));
    }
  }

  async function add(text: string, sourceId?: string, quadrant?: Quadrant | null) {
    if (!text.trim()) return;
    try {
      await api.addTask(text.trim(), sourceId, quadrant);
      await silentRefresh();
      toast.success(t('toast.taskAdded'));
    } catch (e: any) {
      error.value = errorMessage(e);
      toast.error(t('toast.operationFailed', { reason: error.value }));
    }
  }

  async function remove(id: string) {
    const task = tasks.value.find(tk => tk.id === id);
    const text = task ? trunc(task.text) : '';
    try {
      await api.deleteTask(id);
      await silentRefresh();
      toast.success(t('toast.taskDeleted', { text }));
    } catch (e: any) {
      error.value = errorMessage(e);
      toast.error(t('toast.operationFailed', { reason: error.value }));
    }
  }

  return { tasks, sortedTasks, loading, error, refresh, silentRefresh, toggle, update, add, remove };
});
