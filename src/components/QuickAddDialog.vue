<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Quadrant } from '../types/task';
import { useSettingsStore } from '../stores/settings';
import { useQuickAddState } from '../composables/useQuickAdd';
import { safeHexColor } from '../utils/colors';
import { parseInline } from '../utils/inline-md';
import { api } from '../services/tauri-api';

const { t } = useI18n();
const settings = useSettingsStore();
const { visible, pending, close } = useQuickAddState();

const QUADRANT_BUTTONS: Array<{ q: Quadrant | null; emoji: string; tooltipKey: string }> = [
  { q: 'urgent_important', emoji: '🔴', tooltipKey: 'quadrant.urgent_important' },
  { q: 'not_urgent_important', emoji: '🟡', tooltipKey: 'quadrant.not_urgent_important' },
  { q: 'urgent_not_important', emoji: '🟠', tooltipKey: 'quadrant.urgent_not_important' },
  { q: 'not_urgent_not_important', emoji: '🟢', tooltipKey: 'quadrant.not_urgent_not_important' },
  { q: null, emoji: '⚪', tooltipKey: 'quadrant.unsorted' },
];

// Per UX call (2026-05-15): always reset to "not urgent / not important" so the
// dialog never silently inherits last session's choice. Users almost always want
// to grade *down* rather than up — high-priority tasks tend to be entered with
// more deliberation anyway.
const DEFAULT_QUADRANT: Quadrant = 'not_urgent_not_important';

const draft = ref('');
const sourceId = ref<string | null>(null);
const quadrant = ref<Quadrant | null>(DEFAULT_QUADRANT);
const editInput = ref<HTMLTextAreaElement | null>(null);

const targetSource = computed(() => {
  const id = sourceId.value;
  if (!id) return null;
  return settings.sources.find(s => s.id === id) ?? null;
});

const targetLabel = computed(() => {
  const s = targetSource.value;
  if (!s) return '—';
  if (s.label && s.label.trim()) return s.label;
  return s.path.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? s.path;
});

const targetColor = computed(() => safeHexColor(targetSource.value?.color ?? null));

// Live inline-markdown preview — same parser TaskItem & TaskEditorDialog use,
// so what you see here is what'll render in the task list. Multi-line input is
// flattened (newlines → space) because storage rejects multi-line tasks.
const previewSegments = computed(() => {
  const flat = draft.value.replace(/\s*\n+\s*/g, ' ').trim();
  return flat ? parseInline(flat) : [];
});

async function openPreviewLink(href: string) {
  try { await api.openUrl(href); }
  catch (e) { console.warn('openUrl failed:', e); }
}

watch(visible, async (v) => {
  if (!v) return;
  draft.value = '';
  sourceId.value = pending.value?.sourceId ?? settings.defaultSourceId ?? null;
  quadrant.value = DEFAULT_QUADRANT;
  await nextTick();
  const el = editInput.value;
  if (!el) return;
  el.focus();
  // Empty draft — cursor lands at offset 0 by default. No need to setSelectionRange.
});

function pickQuadrant(q: Quadrant | null) {
  quadrant.value = q;
}

function cancel() { close(null); }

function submit() {
  // Task lines are single-line by definition — squash any newlines into a
  // single space so the storage layer accepts the write.
  const flat = draft.value.replace(/\s*\n+\s*/g, ' ').trim();
  if (!flat || !sourceId.value) {
    close(null);
    return;
  }
  close({ text: flat, sourceId: sourceId.value, quadrant: quadrant.value });
}

function onKey(e: KeyboardEvent) {
  if (!visible.value) return;
  if (e.key === 'Escape') { e.preventDefault(); cancel(); }
}

// Wrap/unwrap the textarea selection with markdown markers — same toggle
// semantics as TaskEditorDialog so the two dialogs feel identical.
async function wrap(prefix: string, suffix: string = prefix) {
  const el = editInput.value;
  if (!el) return;
  const start = el.selectionStart;
  const end = el.selectionEnd;
  const value = el.value;
  const selected = value.slice(start, end);

  const setRange = async (a: number, b: number, next: string) => {
    draft.value = next;
    await nextTick();
    el.focus();
    el.setSelectionRange(a, b);
  };

  if (
    selected.length >= prefix.length + suffix.length &&
    selected.startsWith(prefix) &&
    selected.endsWith(suffix)
  ) {
    const inner = selected.slice(prefix.length, selected.length - suffix.length);
    const next = value.slice(0, start) + inner + value.slice(end);
    await setRange(start, start + inner.length, next);
    return;
  }

  if (
    start >= prefix.length &&
    end + suffix.length <= value.length &&
    value.slice(start - prefix.length, start) === prefix &&
    value.slice(end, end + suffix.length) === suffix
  ) {
    const next =
      value.slice(0, start - prefix.length) + selected + value.slice(end + suffix.length);
    await setRange(start - prefix.length, end - prefix.length, next);
    return;
  }

  const next = value.slice(0, start) + prefix + selected + suffix + value.slice(end);
  if (selected.length === 0) {
    const pos = start + prefix.length;
    await setRange(pos, pos, next);
  } else {
    await setRange(start + prefix.length, end + prefix.length, next);
  }
}

async function insertLink() {
  const el = editInput.value;
  if (!el) return;
  const start = el.selectionStart;
  const end = el.selectionEnd;
  const value = el.value;
  const selected = value.slice(start, end);
  const before = value.slice(0, start);
  const after = value.slice(end);
  if (selected) {
    const urlPlaceholder = 'url';
    draft.value = `${before}[${selected}](${urlPlaceholder})${after}`;
    await nextTick();
    el.focus();
    const urlStart = start + 1 + selected.length + 2;
    el.setSelectionRange(urlStart, urlStart + urlPlaceholder.length);
  } else {
    draft.value = `${before}[](url)${after}`;
    await nextTick();
    el.focus();
    const pos = start + 1;
    el.setSelectionRange(pos, pos);
  }
}

function onMdShortcut(e: KeyboardEvent) {
  if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
  switch (e.key.toLowerCase()) {
    case 'b': e.preventDefault(); wrap('**'); break;
    case 'i': e.preventDefault(); wrap('*'); break;
    case 'e': e.preventDefault(); wrap('`'); break;
    case 'd': e.preventDefault(); wrap('~~'); break;
    case 'k': e.preventDefault(); insertLink(); break;
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="quickadd-fade">
      <div
        v-if="visible"
        class="overlay"
        @click.self="cancel"
        @keydown="onKey"
        tabindex="-1"
      >
        <div class="card" role="dialog" :aria-label="t('quickAdd.title')">
          <header class="head">
            <h3 class="title">{{ t('quickAdd.title') }}</h3>
            <span
              v-if="targetColor"
              class="dot"
              :style="{ background: targetColor }"
              aria-hidden="true"
            ></span>
            <span class="subtitle" :title="targetSource?.path ?? ''">{{ targetLabel }}</span>
          </header>

          <label class="field">
            <span class="field-label">{{ t('quickAdd.targetLabel') }}</span>
            <select v-model="sourceId" class="source-select" :title="targetLabel">
              <option v-for="s in settings.sources" :key="s.id" :value="s.id">
                {{ s.label && s.label.trim()
                  ? s.label
                  : (s.path.split(/[\\/]/).filter(Boolean).pop() ?? s.path) }}
              </option>
            </select>
          </label>

          <div class="field">
            <span class="field-label">{{ t('quickAdd.quadrantLabel') }}</span>
            <div class="quadrant-row">
              <button
                v-for="b in QUADRANT_BUTTONS"
                :key="String(b.q)"
                type="button"
                class="q-btn"
                :class="{ active: quadrant === b.q }"
                :title="t(b.tooltipKey)"
                @click="pickQuadrant(b.q)"
              >
                <span class="q-emoji" aria-hidden="true">{{ b.emoji }}</span>
                <span class="q-text">{{ t(b.tooltipKey) }}</span>
              </button>
            </div>
          </div>

          <textarea
            ref="editInput"
            v-model="draft"
            class="editor-input"
            rows="5"
            :placeholder="t('quickAdd.placeholder')"
            @keydown="onMdShortcut"
            @keydown.enter.exact.prevent="submit"
          ></textarea>

          <!-- Live preview — only renders once the user has typed something
               renderable, otherwise it just adds visual noise to an empty draft. -->
          <div v-if="previewSegments.length" class="preview" :aria-label="t('task.previewLabel')">
            <span class="preview-tag">{{ t('task.previewLabel') }}</span>
            <span class="preview-body">
              <template v-for="(seg, i) in previewSegments" :key="i">
                <code v-if="seg.type === 'code'" class="md-code">{{ seg.text }}</code>
                <strong v-else-if="seg.type === 'bold'">{{ seg.text }}</strong>
                <em v-else-if="seg.type === 'italic'">{{ seg.text }}</em>
                <s v-else-if="seg.type === 'strike'">{{ seg.text }}</s>
                <a
                  v-else-if="seg.type === 'link'"
                  class="md-link"
                  :href="seg.href"
                  :title="seg.href"
                  @click.prevent.stop="openPreviewLink(seg.href)"
                >{{ seg.text }}</a>
                <template v-else>{{ seg.text }}</template>
              </template>
            </span>
          </div>

          <p class="hint">{{ t('task.saveHint') }}</p>

          <div class="actions">
            <button class="ghost" @click="cancel">{{ t('confirm.cancel') }}</button>
            <button class="primary" :disabled="!draft.trim() || !sourceId" @click="submit">
              {{ t('quickAdd.submit') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: color-mix(in srgb, #000 35%, transparent);
  backdrop-filter: blur(2px);
  -webkit-backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 0.6rem;
}

.card {
  width: 100%;
  max-width: none;
  max-height: calc(100vh - 1.2rem);
  background: var(--surface);
  border: 1px solid var(--border-strong);
  border-radius: 12px;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.35);
  padding: 0.9rem 1rem 0.8rem;
  animation: pop 160ms cubic-bezier(0.2, 0.9, 0.3, 1.2);
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}

@keyframes pop {
  from { transform: scale(0.94); opacity: 0; }
  to   { transform: scale(1);    opacity: 1; }
}

.head {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  margin-bottom: 0.05rem;
}
.title {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text);
  flex-shrink: 0;
}
.dot {
  width: 10px;
  height: 10px;
  border-radius: 999px;
  flex-shrink: 0;
}
.subtitle {
  font-size: 0.72rem;
  color: var(--text-muted);
  font-family: 'Cascadia Code', 'Consolas', ui-monospace, monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.field-label {
  font-size: 0.7rem;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  font-weight: 600;
}

.source-select {
  padding: 0.4rem 0.5rem;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  font-size: 0.85rem;
  cursor: pointer;
}
.source-select:focus { outline: none; border-color: var(--accent); }

.quadrant-row {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
.q-btn {
  flex: 1 1 0;
  min-width: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 0.35rem 0.45rem;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  font-size: 0.74rem;
  cursor: pointer;
  opacity: 0.7;
  transition: opacity 100ms, border-color 100ms, background 100ms;
}
.q-btn:hover { opacity: 1; }
.q-btn.active {
  opacity: 1;
  border-color: var(--accent);
  background: var(--accent-soft);
}
.q-emoji { font-size: 0.95em; line-height: 1; }
.q-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Textarea + preview block: matches TaskEditorDialog sizing so the two
   dialogs feel identical when editing vs. creating. */
.editor-input {
  display: block;
  width: 100%;
  font-size: 0.92rem;
  line-height: 1.5;
  padding: 0.55rem 0.7rem;
  min-height: 6.5em;
  max-height: 16em;
  background: var(--surface-strong);
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  color: var(--text);
  font-family: inherit;
  resize: vertical;
  word-break: break-word;
}
.editor-input:focus {
  outline: none;
  border-color: var(--accent);
}

.preview {
  padding: 0.5rem 0.7rem;
  background: var(--surface-strong);
  border: 1px dashed var(--border);
  border-radius: 6px;
  font-size: 0.88rem;
  line-height: 1.5;
  color: var(--text);
  word-break: break-word;
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}
.preview .preview-tag {
  font-size: 0.65rem;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  font-weight: 600;
}
.preview .preview-body { display: block; }
.preview strong { font-weight: 600; }
.preview em { font-style: italic; }
.preview s { color: var(--text-muted); }
.preview .md-code {
  font-family: 'Cascadia Code', 'Consolas', 'JetBrains Mono', ui-monospace, monospace;
  font-size: 0.82em;
  padding: 1px 5px;
  background: var(--accent-soft);
  color: var(--accent);
  border-radius: 4px;
  border: 1px solid var(--border);
}
.preview .md-link {
  color: var(--accent);
  text-decoration: underline;
  text-decoration-color: color-mix(in srgb, var(--accent) 40%, transparent);
  text-underline-offset: 2px;
  cursor: pointer;
}
.preview .md-link:hover { text-decoration-color: var(--accent); }

.hint {
  margin: 0;
  font-size: 0.7rem;
  color: var(--text-muted);
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  margin-top: 0.1rem;
}
.actions button {
  padding: 0.4rem 0.95rem;
  font-size: 0.82rem;
  border-radius: 6px;
  cursor: pointer;
  border: 1px solid var(--border);
  background: var(--surface-strong);
  color: var(--text);
  transition: background 100ms, border-color 100ms, opacity 100ms;
}
.actions button:hover { background: var(--accent-soft); }
.actions button:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
.actions button.primary {
  background: var(--accent);
  color: var(--surface);
  border-color: var(--accent);
}
.actions button.primary:hover { opacity: 0.9; }
.actions button.primary:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.actions button.ghost { background: transparent; }
.actions button.ghost:hover { background: var(--surface-strong); }

.quickadd-fade-enter-active,
.quickadd-fade-leave-active { transition: opacity 140ms ease-out; }
.quickadd-fade-enter-from,
.quickadd-fade-leave-to { opacity: 0; }
</style>
