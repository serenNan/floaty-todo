<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useHistoryStore } from '../stores/history';
import { useSettingsStore } from '../stores/settings';
import { useTheme } from '../composables/useTheme';
import { api } from '../services/tauri-api';
import { confirm } from '../composables/useConfirm';
import ConfirmDialog from '../components/ConfirmDialog.vue';
import ToastContainer from '../components/ToastContainer.vue';
import { errorCode, errorField } from '../utils/errors';
import type { HistoryEvent, LineSnapshot } from '../types/history';
import type { Quadrant } from '../types/task';

const history = useHistoryStore();
const settings = useSettingsStore();
useTheme();

const selectedId = ref<string | null>(null);
const selected = computed(() =>
  history.events.find(e => e.id === selectedId.value) ?? history.events[0] ?? null
);
const selectedIsCurrent = computed(() => !!selected.value && history.cursorId === selected.value.id);
const selectedJumpLabel = computed(() =>
  selected.value && isUndone(selected.value) ? '重做到此处' : '跳到此处'
);

const chronological = computed(() => [...history.events].reverse());
const cursorIndex = computed(() => {
  if (history.cursorId === null) return -1;
  return chronological.value.findIndex(e => e.id === history.cursorId);
});

function isUndone(event: HistoryEvent) {
  const idx = chronological.value.findIndex(e => e.id === event.id);
  return cursorIndex.value >= 0 ? idx > cursorIndex.value : idx >= 0;
}

function sourceLabel(event: HistoryEvent) {
  const source = settings.sources.find(s => s.id === event.source_id);
  if (!source) return 'Unknown';
  return source.label?.trim() || lastPathPart(source.path) || 'Source';
}

function sourceColor(event: HistoryEvent) {
  return settings.sources.find(s => s.id === event.source_id)?.color || '#64748b';
}

const QUADRANT_LABEL: Record<Quadrant, string> = {
  urgent_important: '🔴 紧急重要',
  not_urgent_important: '🟡 重要不紧急',
  urgent_not_important: '🟠 紧急不重要',
  not_urgent_not_important: '🟢 不紧急不重要',
};
function quadrantLabel(q?: Quadrant | null) {
  return q ? QUADRANT_LABEL[q] : '未分类';
}

function eventText(event: HistoryEvent) {
  if (event.kind === 'toggle') {
    const done = event.after.state?.done;
    return `${done ? '完成' : '取消完成'}「${event.after.state?.text ?? ''}」`;
  }
  if (event.kind === 'edit') return `编辑「${event.after.state?.text ?? ''}」`;
  if (event.kind === 'add') return `新增「${event.after.state?.text ?? ''}」`;
  if (event.kind === 'move') {
    const from = quadrantLabel(event.before.state?.quadrant);
    const to = quadrantLabel(event.after.state?.quadrant);
    return `「${event.after.state?.text ?? ''}」 ${from} → ${to}`;
  }
  if (event.kind === 'delete') return `删除「${event.before.state?.text ?? ''}」`;
  return `外部修改 ${fileName(event.file)} (+${event.diff_summary.added}/-${event.diff_summary.removed})`;
}

function eventIcon(event: HistoryEvent) {
  if (event.kind === 'toggle') return event.after.state?.done ? '✓' : '↶';
  if (event.kind === 'edit') return '✎';
  if (event.kind === 'add') return '+';
  if (event.kind === 'move') return '↕';
  if (event.kind === 'delete') return '🗑';
  return '•';
}

function fileName(path: string) {
  return lastPathPart(path) || path;
}

function lastPathPart(path: string) {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts.length ? parts[parts.length - 1] : '';
}

function formatTime(ts: string) {
  const date = new Date(ts);
  if (Number.isNaN(date.getTime())) return ts;
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function raw(snapshot?: LineSnapshot) {
  return snapshot?.raw.trimEnd() || '∅';
}

function pick(event: HistoryEvent) {
  selectedId.value = event.id;
}

async function jumpToSelected() {
  if (!selected.value || selectedIsCurrent.value) return;
  try {
    await history.jumpTo(selected.value.id, false);
  } catch (e) {
    if (errorCode(e) !== 'EXTERNAL_IN_UNDO_RANGE') return;
    const count = errorField<number>(e, 'count') ?? 0;
    const ok = await confirm({
      title: '区间内包含外部编辑',
      message: `检测到 ${count} 条外部编辑记录（VS Code / Obsidian 等改的）。Floaty Todo 会跳过这些事件，只回滚 App 内动作。继续？`,
      confirmText: '继续',
      danger: true,
    });
    if (ok) await history.jumpTo(selected.value.id, true);
  }
}

const unlisteners: Array<() => void> = [];

onMounted(async () => {
  // No show()/setFocus() here on purpose — the window is pre-created hidden
  // at app startup (see lib.rs setup) and Rust's open_history_window does
  // the show. If we called show() here, the window would appear at app
  // startup, before the user ever clicked the history button.
  await settings.load();
  await history.refresh();
  selectedId.value = history.events[0]?.id ?? null;
  history.markSeen();
  unlisteners.push(await api.onHistoryUpdated(() => history.refresh()));
});

onUnmounted(() => {
  unlisteners.forEach(fn => fn());
});
</script>

<template>
  <main class="history-shell">
    <aside class="timeline">
      <header class="timeline-head">
        <div>
          <h1>History</h1>
          <p>{{ history.events.length }} events</p>
        </div>
        <button type="button" @click="history.refresh()" title="Refresh">↻</button>
      </header>

      <div v-if="history.loading" class="timeline-hint">Loading…</div>
      <div v-else-if="history.error" class="timeline-error">{{ history.error }}</div>
      <div v-else class="event-list">
        <button
          v-for="event in history.events"
          :key="event.id"
          type="button"
          class="event-row"
          :class="{ active: selected?.id === event.id, undone: isUndone(event), external: event.kind === 'external_edit' }"
          @click="pick(event)"
        >
          <span class="source-stripe" :style="{ background: sourceColor(event) }"></span>
          <span class="event-icon">{{ eventIcon(event) }}</span>
          <span class="event-main">
            <span class="event-title">{{ eventText(event) }}</span>
            <span class="event-meta">
              {{ formatTime(event.ts) }} · {{ sourceLabel(event) }}
              <span v-if="isUndone(event)"> · 已撤销</span>
            </span>
          </span>
        </button>
      </div>
    </aside>

    <section class="detail">
      <template v-if="selected">
        <header class="detail-head">
          <div>
            <span class="detail-kind">{{ selected.kind.replace(/_/g, ' ') }}</span>
            <h2>{{ eventText(selected) }}</h2>
            <p>{{ selected.file }}</p>
          </div>
          <div class="detail-actions">
            <span class="detail-time">{{ formatTime(selected.ts) }}</span>
            <button
              type="button"
              class="jump-btn"
              :disabled="selectedIsCurrent"
              @click="jumpToSelected"
            >
              {{ selectedJumpLabel }}
            </button>
          </div>
        </header>

        <div v-if="selected.kind === 'external_edit'" class="external-box">
          <div>Added {{ selected.diff_summary.added }}</div>
          <div>Removed {{ selected.diff_summary.removed }}</div>
          <div>Modified {{ selected.diff_summary.modified }}</div>
          <p>{{ selected.note }}</p>
        </div>

        <!-- `add` events carry no `before` and `delete` events carry no
             `after` — `raw()` renders ∅ for the absent side, which is the
             correct depiction (nothing there before / after the action). -->
        <div v-else class="diff">
          <div class="diff-col before">
            <div class="diff-label">Before</div>
            <pre>{{ raw('before' in selected ? selected.before : undefined) }}</pre>
          </div>
          <div class="diff-col after">
            <div class="diff-label">After</div>
            <pre>{{ raw('after' in selected ? selected.after : undefined) }}</pre>
          </div>
        </div>
      </template>
      <div v-else class="empty-detail">No history yet.</div>
    </section>
    <ConfirmDialog />
    <ToastContainer />
  </main>
</template>

<style scoped>
.history-shell {
  display: grid;
  grid-template-columns: minmax(240px, 0.95fr) minmax(320px, 1.35fr);
  height: 100vh;
  background: var(--bg);
  color: var(--text);
}

.timeline {
  min-width: 0;
  border-right: 1px solid var(--border);
  background: var(--surface);
  display: flex;
  flex-direction: column;
}

.timeline-head,
.detail-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 14px;
  border-bottom: 1px solid var(--border);
}

.timeline-head h1,
.detail-head h2 {
  font-size: 1rem;
  line-height: 1.25;
  letter-spacing: 0;
}

.timeline-head p,
.detail-head p,
.event-meta,
.detail-kind,
.detail-time {
  color: var(--text-muted);
  font-size: 0.76rem;
}

.timeline-head button {
  width: 28px;
  height: 26px;
  padding: 0;
}

.event-list {
  flex: 1;
  overflow-y: auto;
  padding: 6px;
}

.event-row {
  width: 100%;
  min-height: 48px;
  display: grid;
  grid-template-columns: 3px 26px minmax(0, 1fr);
  align-items: center;
  gap: 8px;
  padding: 7px 8px;
  margin-bottom: 3px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--text);
  text-align: left;
}

.event-row:hover,
.event-row.active {
  background: var(--surface-strong);
  border-color: var(--border);
}

.event-row.undone {
  opacity: 0.48;
}

.event-row.external {
  color: var(--text-muted);
}

.source-stripe {
  width: 3px;
  height: 30px;
  border-radius: 2px;
}

.event-icon {
  width: 26px;
  height: 26px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  background: var(--accent-soft);
  font-size: 0.82rem;
  font-weight: 700;
}

.event-main {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.event-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.86rem;
}

.detail {
  min-width: 0;
  overflow: auto;
  background: var(--bg);
}

.detail-head {
  background: var(--surface);
}

.detail-kind {
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.detail-actions {
  display: flex;
  align-items: flex-end;
  flex-direction: column;
  gap: 8px;
  flex-shrink: 0;
}

.jump-btn {
  min-width: 92px;
  height: 28px;
  padding: 0 10px;
  border-color: var(--border-strong);
  background: var(--surface-strong);
  font-size: 0.78rem;
}

.jump-btn:disabled {
  opacity: 0.45;
  cursor: default;
}

.diff {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  padding: 12px;
}

.diff-col {
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
  background: var(--surface);
}

.diff-label {
  padding: 8px 10px;
  border-bottom: 1px solid var(--border);
  font-size: 0.75rem;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.diff-col.before .diff-label {
  color: #dc2626;
}

.diff-col.after .diff-label {
  color: #16a34a;
}

pre {
  min-height: 88px;
  padding: 10px;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 0.82rem;
  line-height: 1.45;
}

.external-box,
.empty-detail,
.timeline-hint,
.timeline-error {
  margin: 12px;
  padding: 12px;
  color: var(--text-muted);
}

.external-box {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface);
}

.external-box div {
  color: var(--text);
  font-weight: 600;
}

.external-box p {
  grid-column: 1 / -1;
  font-size: 0.82rem;
}

.timeline-error {
  color: #ef4444;
}
</style>
