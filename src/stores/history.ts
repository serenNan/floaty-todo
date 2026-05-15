import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { HistoryEvent } from '../types/history';
import { api } from '../services/tauri-api';
import { errorMessage } from '../utils/errors';

export const useHistoryStore = defineStore('history', () => {
  const events = ref<HistoryEvent[]>([]);
  const cursorId = ref<string | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const hasRedo = computed(() => {
    if (!events.value.length) return false;
    if (cursorId.value === null) return true;
    return events.value[0]?.id !== cursorId.value;
  });

  async function refresh(limit = 500) {
    loading.value = true;
    try {
      const [nextEvents, nextCursor] = await Promise.all([
        api.getHistory(limit),
        api.getHistoryCursor(),
      ]);
      events.value = nextEvents;
      cursorId.value = nextCursor;
      error.value = null;
    } catch (e: any) {
      error.value = errorMessage(e);
    } finally {
      loading.value = false;
    }
  }

  async function undo() {
    try {
      await api.undo();
      await refresh();
    } catch (e: any) {
      error.value = errorMessage(e);
    }
  }

  async function redo() {
    try {
      await api.redo();
      await refresh();
    } catch (e: any) {
      error.value = errorMessage(e);
    }
  }

  async function jumpTo(eventId: string, confirmExternal = false) {
    try {
      await api.jumpTo(eventId, confirmExternal);
      await refresh();
      error.value = null;
    } catch (e: any) {
      error.value = errorMessage(e);
      // re-throw the raw error so callers can inspect `.code` etc.
      throw e;
    }
  }

  return { events, cursorId, loading, error, hasRedo, refresh, undo, redo, jumpTo };
});
