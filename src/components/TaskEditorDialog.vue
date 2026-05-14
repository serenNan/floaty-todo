<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useTaskEditorState } from '../composables/useTaskEditor';
import { parseInline } from '../utils/inline-md';
import { api } from '../services/tauri-api';

const { t } = useI18n();
const { visible, pending, close } = useTaskEditorState();

const draft = ref('');
const editInput = ref<HTMLTextAreaElement | null>(null);

const task = computed(() => pending.value?.task ?? null);

const fileLabel = computed(() => {
  const tk = task.value;
  if (!tk) return '';
  const name = tk.source_file.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? tk.source_file;
  return `${name} · L${tk.line_number}`;
});

// Live inline-markdown preview — same parser TaskItem uses, so the preview
// matches the row once the user saves. Strips multi-line input (collapsed
// to a single space) because storage only accepts single-line tasks.
const previewSegments = computed(() => {
  const flat = draft.value.replace(/\s*\n+\s*/g, ' ').trim();
  return flat ? parseInline(flat) : [];
});

async function openPreviewLink(href: string) {
  try { await api.openUrl(href); }
  catch (e) { console.warn('openUrl failed:', e); }
}

// Sync draft from the task when the dialog opens; focus, caret to end.
// We intentionally do NOT select-all — that turned a single accidental
// keystroke into a full content wipe.
watch(visible, async (v) => {
  if (!v) return;
  draft.value = task.value?.text ?? '';
  await nextTick();
  const el = editInput.value;
  if (!el) return;
  el.focus();
  const end = el.value.length;
  el.setSelectionRange(end, end);
});

function save() {
  // Task lines are single-line by definition (storage rejects \n) — squash any
  // newlines the user typed/pasted into a single space so save still succeeds.
  const next = draft.value.replace(/\s*\n+\s*/g, ' ').trim();
  // Empty / unchanged → treat as cancel; caller writes nothing.
  if (!next || next === task.value?.text) {
    close(null);
    return;
  }
  close(next);
}

function cancel() { close(null); }

function onKey(e: KeyboardEvent) {
  if (!visible.value) return;
  if (e.key === 'Escape') { e.preventDefault(); cancel(); }
}

// Wrap or unwrap the textarea selection with markdown markers.
// Toggle rules:
//   - If the selection itself is `prefix...suffix` → unwrap.
//   - Else if the chars *around* the selection are markers → unwrap.
//   - Else wrap; when there's no selection, drop the cursor between markers.
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
    <Transition name="editor-fade">
      <div
        v-if="visible"
        class="editor-overlay"
        @click.self="cancel"
        @keydown="onKey"
        tabindex="-1"
      >
        <div class="editor-card" role="dialog" :aria-label="t('task.edit')">
          <h3 class="title">{{ t('task.edit') }}</h3>
          <p class="subtitle" :title="task?.source_file">{{ fileLabel }}</p>
          <textarea
            ref="editInput"
            v-model="draft"
            class="editor-input"
            rows="5"
            :placeholder="t('task.editPlaceholder')"
            @keydown="onMdShortcut"
            @keydown.enter.exact.prevent="save"
          ></textarea>
          <!-- Live preview — only visible once the user has typed something
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
            <button class="primary" @click="save">{{ t('source.actions.save') }}</button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.editor-overlay {
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

.editor-card {
  /* The Floaty window is narrow (680px×520) — let the modal use almost the
     full viewport. The 1.2rem horizontal padding plus the overlay's 0.6rem
     gutter give a comfortable but not luxurious frame. */
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
}

@keyframes pop {
  from { transform: scale(0.94); opacity: 0; }
  to   { transform: scale(1);    opacity: 1; }
}

.title {
  margin: 0 0 0.15rem;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text);
}

.subtitle {
  margin: 0 0 0.8rem;
  font-size: 0.72rem;
  color: var(--text-muted);
  font-family: 'Cascadia Code', 'Consolas', ui-monospace, monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

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
  margin-top: 0.5rem;
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
.preview .preview-body {
  display: block;
}
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
  margin: 0.4rem 0 0.9rem;
  font-size: 0.7rem;
  color: var(--text-muted);
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
}
.actions button {
  padding: 0.4rem 0.95rem;
  font-size: 0.82rem;
  border-radius: 6px;
  cursor: pointer;
  border: 1px solid var(--border);
  background: var(--surface-strong);
  color: var(--text);
  transition: background 100ms, border-color 100ms;
}
.actions button:hover { background: var(--accent-soft); }
.actions button:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
.actions button.primary {
  background: var(--accent);
  color: var(--surface);
  border-color: var(--accent);
}
.actions button.primary:hover { opacity: 0.9; }
.actions button.ghost { background: transparent; }
.actions button.ghost:hover { background: var(--surface-strong); }

.editor-fade-enter-active,
.editor-fade-leave-active { transition: opacity 140ms ease-out; }
.editor-fade-enter-from,
.editor-fade-leave-to { opacity: 0; }
</style>
