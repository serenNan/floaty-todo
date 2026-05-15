import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { HistoryEvent } from '../types/history';
import { api } from '../services/tauri-api';
import { errorCode, errorMessage } from '../utils/errors';
import { toast } from '../composables/useToast';
import { i18n } from '../i18n';

const t = i18n.global.t;

export const useHistoryStore = defineStore('history', () => {
  const LAST_SEEN_KEY = 'floaty.history.lastSeenAt';

  function loadLastSeen(): string | null {
    try {
      return localStorage.getItem(LAST_SEEN_KEY);
    } catch {
      return null;
    }
  }

  const lastSeenAt = ref<string | null>(loadLastSeen());

  const events = ref<HistoryEvent[]>([]);
  const cursorId = ref<string | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const hasRedo = computed(() => {
    if (!events.value.length) return false;
    if (cursorId.value === null) return true;
    return events.value[0]?.id !== cursorId.value;
  });

  const unseenExternal = computed<number>(() => {
    const cutoff = lastSeenAt.value;
    return events.value.filter(e => {
      if (e.kind !== 'external_edit') return false;
      if (!cutoff) return true;
      return e.ts > cutoff;
    }).length;
  });

  function markSeen() {
    const now = new Date().toISOString();
    lastSeenAt.value = now;
    try {
      localStorage.setItem(LAST_SEEN_KEY, now);
    } catch {
      /* localStorage may fail (private mode, quota); next refresh recovers */
    }
    // Tauri 2 frontend emit broadcasts cross-window — main window listens
    // and syncs its own lastSeenAt from localStorage.
    api.emitHistorySeen().catch(err =>
      console.warn('emit history-seen-changed failed:', err),
    );
  }

  function syncLastSeenFromStorage() {
    lastSeenAt.value = loadLastSeen();
  }

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
      const ev = await api.undo();
      await refresh();
      if (ev) toast.success(t('toast.historyUndone'));
      else toast.info(t('toast.historyNothing'));
    } catch (e: any) {
      error.value = errorMessage(e);
      toast.error(t('toast.operationFailed', { reason: error.value }));
    }
  }

  async function redo() {
    try {
      const ev = await api.redo();
      await refresh();
      if (ev) toast.success(t('toast.historyRedone'));
      else toast.info(t('toast.historyNothing'));
    } catch (e: any) {
      error.value = errorMessage(e);
      toast.error(t('toast.operationFailed', { reason: error.value }));
    }
  }

  async function jumpTo(eventId: string, confirmExternal = false) {
    try {
      const result = await api.jumpTo(eventId, confirmExternal);
      await refresh();
      error.value = null;
      toast.success(t('toast.historyJumped'));
      if (result.skipped_external > 0) {
        toast.warning(t('toast.externalEditSkipped', { n: result.skipped_external }));
      }
    } catch (e: any) {
      error.value = errorMessage(e);
      // EXTERNAL_IN_UNDO_RANGE is a "business confirm" path — the caller
      // (HistoryView) catches this and re-invokes with confirmExternal=true.
      // We don't toast that as an error.
      if (errorCode(e) !== 'EXTERNAL_IN_UNDO_RANGE') {
        toast.error(t('toast.operationFailed', { reason: error.value }));
      }
      throw e;
    }
  }

  return {
    events,
    cursorId,
    loading,
    error,
    hasRedo,
    refresh,
    undo,
    redo,
    jumpTo,
    lastSeenAt,
    unseenExternal,
    markSeen,
    syncLastSeenFromStorage,
  };
});
