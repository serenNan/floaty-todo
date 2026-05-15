<script setup lang="ts">
import { computed, ref, inject, type Ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { QuickActionKind, Source, Task } from '../types/task';
import { useSettingsStore } from '../stores/settings';
import { api } from '../services/tauri-api';
import { confirm } from '../composables/useConfirm';
import { bindCollapse } from '../composables/useCollapse';
import {
  draggedSourceId,
  dropTargetSourceId,
  startSourceDrag,
} from '../composables/useSourceDrag';
import FileGroup from './FileGroup.vue';
import QuickActionIcon from './icons/QuickActionIcon.vue';
import Icon from './icons/Icon.vue';
import { SOURCE_COLORS, safeHexColor } from '../utils/colors';
import QuadrantGroup from './QuadrantGroup.vue';
import type { Quadrant } from '../types/task';
import { openQuickAdd } from '../composables/useQuickAdd';
import { useTaskStore } from '../stores/tasks';

const QUADRANT_ORDER: (Quadrant | null)[] = [
  'urgent_important',
  'not_urgent_important',
  'urgent_not_important',
  'not_urgent_not_important',
  null,
];

function groupByQuadrant(tasks: Task[]): Array<{ quadrant: Quadrant | null; tasks: Task[] }> {
  const buckets = new Map<Quadrant | null, Task[]>();
  for (const q of QUADRANT_ORDER) buckets.set(q, []);
  for (const t of tasks) buckets.get(t.quadrant ?? null)!.push(t);
  return QUADRANT_ORDER
    .map((q) => ({ quadrant: q, tasks: buckets.get(q)! }))
    .filter((g) => g.tasks.length > 0);
}

/// Auto-collapse every FileGroup the first time we render a source whose
/// task count exceeds this threshold. Keeps the DOM tree small enough to
/// stay responsive on huge vaults (thousands of tasks → only a few dozen
/// file headers in the DOM until the user expands one).
const BIG_SOURCE_TASK_THRESHOLD = 50;

const props = defineProps<{ source: Source; tasks: Task[] }>();
defineEmits<{ 'open-settings': [] }>();
const { t } = useI18n();
const settings = useSettingsStore();
const taskStore = useTaskStore();

const collapsed = ref(true);
// Per-source "expand/collapse all quadrants" state. Tokens are bumped on
// each toggle so child QuadrantGroup watches fire even when the boolean
// hasn't actually flipped on their side (e.g. user manually opened one).
const quadrantsExpanded = ref(false);
const collapseQuadrantToken = ref(0);
const expandQuadrantToken = ref(0);
function toggleAllQuadrants() {
  if (quadrantsExpanded.value) {
    quadrantsExpanded.value = false;
    collapseQuadrantToken.value++;
  } else {
    quadrantsExpanded.value = true;
    expandQuadrantToken.value++;
  }
}
const editing = ref(false);
const labelDraft = ref('');
const rootDraft = ref('');
const colorDraft = ref<string | null>(null);
const actionError = ref<string | null>(null);

// Active search query (injected from TaskList). When non-empty, override
// the user's collapse state so matches are visible without manual expansion.
const searchQueryRef = inject<Ref<string>>('searchQuery', ref(''));
const searchActive = computed(() => searchQueryRef.value.trim().length > 0);
const effectiveCollapsed = computed(() => collapsed.value && !searchActive.value);

// React to global "Collapse all" / "Expand all" from the footer button.
bindCollapse(next => { collapsed.value = next; });

const isDefault = computed(() => settings.defaultSourceId === props.source.id);

const displayLabel = computed(() => {
  const s = props.source;
  if (s.label && s.label.trim()) return s.label;
  return s.path.replace(/\\/g, '/').split('/').filter(Boolean).pop() ?? s.path;
});

/// Group tasks by their `source_file`. For File sources this collapses to a
/// single bucket; for Folder sources each `.md` file becomes its own group.
/// Files with zero tasks are included only when this is a File source — for
/// Folder sources we keep the list to "files that have tasks" so the source
/// header stays compact.
const fileGroups = computed(() => {
  const map = new Map<string, Task[]>();
  for (const tk of props.tasks) {
    const arr = map.get(tk.source_file);
    if (arr) arr.push(tk);
    else map.set(tk.source_file, [tk]);
  }
  // For a File source, ensure the source path is always present even when empty.
  if (props.source.kind === 'file' && !map.has(props.source.path)) {
    map.set(props.source.path, []);
  }
  // Stable ordering: by file path string.
  return Array.from(map.entries())
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    .map(([filePath, fileTasks]) => ({ filePath, tasks: fileTasks }));
});

async function runAction(kind: QuickActionKind) {
  try { await api.runQuickAction(props.source.id, kind); }
  catch (e: any) { actionError.value = String(e); }
}

async function addTaskHere() {
  const result = await openQuickAdd({ sourceId: props.source.id });
  if (!result) return;
  try { await taskStore.add(result.text, result.sourceId, result.quadrant); }
  catch (e: any) { actionError.value = String(e); }
}

// In-header quick editor — replaces the "jump to Settings page" detour the
// grip's click used to trigger. Lives next to the source so the user can
// rename / repoint / set-default / remove without losing context.
function startEdit() {
  labelDraft.value = props.source.label ?? '';
  rootDraft.value = props.source.project_root ?? '';
  colorDraft.value = props.source.color ?? null;
  actionError.value = null;
  editing.value = true;
}

function cancelEdit() {
  editing.value = false;
  actionError.value = null;
}

async function saveEdit() {
  try {
    await settings.updateSource({
      sourceId: props.source.id,
      label: labelDraft.value.trim() || null,
      projectRoot: rootDraft.value.trim() || null,
      color: colorDraft.value,
    });
    editing.value = false;
  } catch (e: any) {
    actionError.value = String(e);
  }
}

async function pickRoot() {
  const p = await api.pickFolder();
  if (p) rootDraft.value = p;
}

async function setDefault() {
  try { await settings.setDefaultSource(props.source.id); }
  catch (e: any) { actionError.value = String(e); }
}

async function removeSource() {
  const ok = await confirm({
    title: t('confirm.removeSourceTitle'),
    message: t('confirm.removeSourceMessage', { label: displayLabel.value }),
    confirmText: t('confirm.removeSourceConfirm'),
    danger: true,
  });
  if (!ok) return;
  try { await settings.removeSource(props.source.id); }
  catch (e: any) { actionError.value = String(e); }
}

// Drag-and-drop reordering for quick-action buttons. The new order is
// persisted to settings.enabled_quick_actions, so reordering here also
// reorders the same buttons on every *other* source — desired, since
// the toggle list is global.
const dragKind = ref<QuickActionKind | null>(null);
const dropTargetKind = ref<QuickActionKind | null>(null);

function onActionDragStart(e: DragEvent, kind: QuickActionKind) {
  dragKind.value = kind;
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move';
    // Some browsers require a payload or the drag never starts.
    e.dataTransfer.setData('text/plain', kind);
  }
}
function onActionDragOver(e: DragEvent, kind: QuickActionKind) {
  if (!dragKind.value || dragKind.value === kind) return;
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
  dropTargetKind.value = kind;
}
function onActionDragLeave(kind: QuickActionKind) {
  if (dropTargetKind.value === kind) dropTargetKind.value = null;
}
async function onActionDrop(e: DragEvent, targetKind: QuickActionKind) {
  e.preventDefault();
  const src = dragKind.value;
  dragKind.value = null;
  dropTargetKind.value = null;
  if (!src || src === targetKind) return;
  const order = [...settings.enabledQuickActions];
  const srcIdx = order.indexOf(src);
  if (srcIdx < 0) return;
  order.splice(srcIdx, 1);
  const newTgtIdx = order.indexOf(targetKind);
  if (newTgtIdx < 0) {
    order.push(src);
  } else {
    order.splice(newTgtIdx, 0, src);
  }
  try { await settings.setEnabledQuickActions(order); }
  catch (e: any) { actionError.value = String(e); }
}
function onActionDragEnd() {
  dragKind.value = null;
  dropTargetKind.value = null;
}

// Pointer-events based source-drag. Tauri's WebView2 doesn't deliver
// HTML5 `dragover` events to in-page elements, so we can't use the
// `draggable="true"` flow. Pointer events work reliably everywhere.
const isSourceDragging = computed(() => draggedSourceId.value === props.source.id);
const isSourceDropTarget = computed(() => dropTargetSourceId.value === props.source.id);

function onDotsPointerDown(e: PointerEvent) {
  if (e.button !== 0) return; // left button only
  e.preventDefault();
  startSourceDrag({
    e,
    sourceId: props.source.id,
    // Grip handle is drag-only. The dedicated ⚙ button next to it opens
    // the in-header editor — no more silent jump to Settings on click.
    onClick: () => {},
    onDrop: async (targetId: string) => {
      const order = settings.sources.map(s => s.id);
      const srcIdx = order.indexOf(props.source.id);
      const tgtIdx = order.indexOf(targetId);
      if (srcIdx < 0 || tgtIdx < 0) return;
      // Capture tgtIdx BEFORE splicing srcIdx — otherwise downward drags
      // (srcIdx < tgtIdx) end up off-by-one because removing srcIdx shifts
      // the target's index down by 1 in the modified array.
      order.splice(srcIdx, 1);
      order.splice(tgtIdx, 0, props.source.id);
      try { await settings.reorderSources(order); }
      catch (err: any) { actionError.value = String(err); }
    },
  });
}

interface ActionMeta {
  kind: QuickActionKind;
  i18n: string;
}
const ACTION_META: Record<QuickActionKind, ActionMeta> = {
  reveal:      { kind: 'reveal',      i18n: 'source.reveal' },
  vscode:      { kind: 'vscode',      i18n: 'source.openVscode' },
  terminal:    { kind: 'terminal',    i18n: 'source.openTerminal' },
  claude_code: { kind: 'claude_code', i18n: 'source.openClaudeCode' },
};

const enabledActions = computed(() =>
  settings.enabledQuickActions
    .map(k => ACTION_META[k])
    .filter(Boolean),
);

const isScanning = computed(() => settings.scanningSourceIds.has(props.source.id));
const isBigSource = computed(() => props.tasks.length > BIG_SOURCE_TASK_THRESHOLD);

/// Kind icon as a real emoji that flips on expand/collapse. Folder gets the
/// open-folder glyph when expanded so the visual matches the disclosure
/// state; file gets a pencil-and-page when expanded to imply "editable".
const kindEmoji = computed(() => {
  if (props.source.kind === 'folder') return effectiveCollapsed.value ? '📁' : '📂';
  return effectiveCollapsed.value ? '📄' : '📝';
});

// Source color comes from config; validate as hex so untrusted config can't
// smuggle arbitrary CSS through inline style. While the inline editor is
// open we preview the draft instead of the saved value so swatches feel
// live — picking a color updates the card immediately, Save only persists.
const safeColor = computed(() => {
  const raw = editing.value ? colorDraft.value : props.source.color;
  return safeHexColor(raw);
});
const colorStyle = computed(() =>
  safeColor.value ? { '--src-color': safeColor.value } : undefined,
);
</script>

<template>
  <section
    class="group"
    :class="{ collapsed: effectiveCollapsed, colored: !!safeColor }"
    :style="colorStyle"
  >
    <header
      class="group-head"
      :class="{
        'source-dragging': isSourceDragging,
        'source-drop-target': isSourceDropTarget,
      }"
      :data-source-id="source.id"
      @click="collapsed = !collapsed"
      :title="effectiveCollapsed ? t('source.expand') : t('source.collapse')"
    >
      <!-- Caret is decorative now — the whole header handles the toggle.
           Keeping the visual chevron so the disclosure affordance still
           reads, but click cost goes to anywhere in the bar. -->
      <span class="caret" aria-hidden="true">
        <Icon :name="effectiveCollapsed ? 'chevron-right' : 'chevron-down'" :size="14" />
      </span>
      <span class="kind-icon" aria-hidden="true">{{ kindEmoji }}</span>
      <span class="label" :title="source.path">{{ displayLabel }}</span>
      <span v-if="isScanning" class="scanning" :title="t('source.scanning')">
        <Icon name="loader" :size="14" />
      </span>
      <!-- Actions cluster: clicks here must NOT bubble to the header
           toggle, otherwise tapping an action would collapse the group.
           Drag-and-drop reorders the buttons across every source. -->
      <div class="actions" @click.stop>
        <button
          class="icon-btn add-btn"
          @click="addTaskHere"
          :title="t('source.addTask')"
        >
          <Icon name="plus" :size="14" />
        </button>
        <button
          v-for="a in enabledActions"
          :key="a.kind"
          class="icon-btn brand"
          :class="{
            dragging: dragKind === a.kind,
            'drop-target': dropTargetKind === a.kind,
          }"
          draggable="true"
          @dragstart.stop="onActionDragStart($event, a.kind)"
          @dragover="onActionDragOver($event, a.kind)"
          @dragleave="onActionDragLeave(a.kind)"
          @drop="onActionDrop($event, a.kind)"
          @dragend.stop="onActionDragEnd"
          @click="runAction(a.kind)"
          :title="t(a.i18n)"
        >
          <QuickActionIcon :kind="a.kind" />
        </button>
        <button
          v-if="!effectiveCollapsed"
          class="icon-btn"
          @click="toggleAllQuadrants"
          :title="quadrantsExpanded ? t('source.collapseQuadrants') : t('source.expandQuadrants')"
        >
          <Icon :name="quadrantsExpanded ? 'collapse-all' : 'expand-all'" :size="14" />
        </button>
        <button
          class="icon-btn"
          :class="{ active: editing }"
          @click="editing ? cancelEdit() : startEdit()"
          :title="t('source.edit')"
        >
          <Icon name="settings" :size="14" />
        </button>
        <button
          class="icon-btn drag-handle"
          @pointerdown.stop="onDotsPointerDown"
          :title="t('source.dragHandle')"
        >
          <Icon name="grip-vertical" :size="14" />
        </button>
      </div>
    </header>

    <div v-if="editing" class="editor" @click.stop>
      <label>
        {{ t('source.fields.label') }}
        <input
          v-model="labelDraft"
          :placeholder="displayLabel"
          @keydown.enter.prevent="saveEdit"
          @keydown.esc.prevent="cancelEdit"
        />
      </label>
      <label>
        {{ t('source.fields.projectRoot') }} <span class="hint">{{ t('source.fields.projectRootHint') }}</span>
        <span class="root-row">
          <input
            v-model="rootDraft"
            :placeholder="source.path"
            @keydown.enter.prevent="saveEdit"
            @keydown.esc.prevent="cancelEdit"
          />
          <button type="button" class="pick-btn" @click="pickRoot" :title="t('source.actions.pickFolder')">
            <Icon name="folder" :size="14" />
          </button>
        </span>
      </label>
      <div class="color-row">
        <span class="color-label">{{ t('source.fields.color') }}</span>
        <div class="swatches" role="radiogroup" :aria-label="t('source.fields.color')">
          <button
            type="button"
            class="swatch none"
            :class="{ active: !colorDraft }"
            @click="colorDraft = null"
            :title="t('source.fields.colorNone')"
            role="radio"
            :aria-checked="!colorDraft"
          >
            <Icon name="x" :size="11" />
          </button>
          <button
            v-for="c in SOURCE_COLORS"
            :key="c"
            type="button"
            class="swatch"
            :class="{ active: colorDraft === c }"
            :style="{ background: c }"
            @click="colorDraft = c"
            :title="c"
            role="radio"
            :aria-checked="colorDraft === c"
          ></button>
        </div>
      </div>
      <div class="editor-actions">
        <button type="button" class="ghost" :disabled="isDefault" @click="setDefault">
          {{ isDefault ? t('source.actions.isDefault') : t('source.actions.setDefault') }}
        </button>
        <button type="button" class="danger" @click="removeSource">{{ t('source.actions.remove') }}</button>
        <span class="spacer"></span>
        <button type="button" class="ghost" @click="cancelEdit">{{ t('source.actions.cancel') }}</button>
        <button type="button" class="primary" @click="saveEdit">{{ t('source.actions.save') }}</button>
      </div>
    </div>

    <p v-if="actionError" class="error" @click="actionError = null">{{ actionError }}</p>

    <div v-if="!effectiveCollapsed" class="rows">
      <div v-if="isScanning" class="scanning-row">{{ t('source.scanningHint') }}</div>

      <!-- File source: render tasks grouped by quadrant. -->
      <template v-if="source.kind === 'file'">
        <QuadrantGroup
          v-for="g in groupByQuadrant(tasks)"
          :key="String(g.quadrant)"
          :quadrant="g.quadrant"
          :tasks="g.tasks"
          :collapse-token="collapseQuadrantToken"
          :expand-token="expandQuadrantToken"
        />
        <div v-if="tasks.length === 0 && !isScanning" class="empty-source">{{ t('source.noTasks') }}</div>
      </template>

      <!-- Folder source: one FileGroup per .md file. -->
      <template v-else>
        <FileGroup
          v-for="g in fileGroups"
          :key="g.filePath"
          :source="source"
          :file-path="g.filePath"
          :tasks="g.tasks"
          :initial-collapsed="isBigSource"
          :collapse-token="collapseQuadrantToken"
          :expand-token="expandQuadrantToken"
        />
        <div v-if="fileGroups.length === 0 && !isScanning" class="empty-source">{{ t('source.noTasks') }}</div>
      </template>
    </div>
  </section>
</template>

<style scoped>
.group {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  margin-bottom: 0.5rem;
  overflow: hidden;
  box-shadow: var(--card-shadow);
  transition: border-color 120ms ease-out, box-shadow 120ms ease-out;
}
.group:hover { border-color: var(--border-strong); }
.group:last-child { margin-bottom: 0; }

/* Colored variant: left-edge accent stripe + a body-wide tint so the
   stripe doesn't look brighter in the expanded body section than in the
   header band. Dark theme's --surface is rgba(.., .72) — without the
   tint, any src-color band gets visually dimmed by the surrounding
   half-transparent dark surface. Header carries a slightly stronger
   tint so it still reads as the header. --src-color is bound by inline
   style on .group. */
.group.colored {
  background: color-mix(in srgb, var(--src-color) 7%, var(--surface));
  box-shadow: inset 4px 0 0 var(--src-color), var(--card-shadow);
  border-color: color-mix(in srgb, var(--src-color) 30%, var(--border));
}
.group.colored:hover {
  border-color: color-mix(in srgb, var(--src-color) 55%, var(--border-strong));
}
.group.colored .group-head {
  background: color-mix(in srgb, var(--src-color) 16%, var(--surface));
}
.group.colored .group-head:hover {
  background: color-mix(in srgb, var(--src-color) 26%, var(--surface));
}

.group-head {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.45rem 0.6rem;
  background: var(--surface-strong);
  font-size: 0.82rem;
  color: var(--text);
  user-select: none;
  cursor: pointer;
  transition: background 120ms ease-out, opacity 120ms ease-out;
}
.group:not(.collapsed) .group-head {
  border-bottom: 1px solid var(--border);
}
.group-head:hover { background: var(--accent-soft); }

/* Drag feedback: dim the source being dragged, accent-outline the drop
   target so the user can see where the source will land. Outline (not
   border) keeps the header height stable during the dance. */
.group-head.source-dragging { opacity: 0.4; }
.group-head.source-drop-target {
  background: var(--accent-soft);
  outline: 2px dashed var(--accent);
  outline-offset: -2px;
}

.caret {
  width: 20px;
  height: 20px;
  color: var(--text-muted);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.group-head:hover .caret { color: var(--text); }

.kind-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  font-size: 14px;
  line-height: 1;
  /* Pin OS colour-emoji fonts so the glyphs render as real cartoon
     emoji rather than the monochrome fallback some webviews pick. */
  font-family: 'Segoe UI Emoji', 'Apple Color Emoji', 'Noto Color Emoji', sans-serif;
  flex-shrink: 0;
}

.label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 500;
}

.badge {
  font-size: 0.65rem;
  padding: 1px 5px;
  background: var(--accent-soft);
  color: var(--accent);
  border-radius: 3px;
  flex-shrink: 0;
}

.scanning {
  display: inline-flex;
  color: var(--accent);
  flex-shrink: 0;
  animation: spin 1.1s linear infinite;
}
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.scanning-row {
  padding: 0.5rem 0.75rem;
  font-size: 0.78rem;
  color: var(--text-muted);
  font-style: italic;
}

.actions {
  display: flex;
  gap: 2px;
  flex-shrink: 0;
}

.icon-btn {
  width: 24px;
  height: 22px;
  padding: 0;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 4px;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 0.8rem;
  line-height: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.icon-btn:hover {
  background: var(--surface);
  border-color: var(--border);
  color: var(--text);
}
.icon-btn.active {
  background: var(--accent-soft);
  color: var(--accent);
  border-color: var(--border);
}
.icon-btn.brand:hover {
  /* Use the brand colour as the hover background so the icon stays
     readable against a soft tint of itself. */
  background: color-mix(in srgb, currentColor 10%, transparent);
  border-color: color-mix(in srgb, currentColor 30%, transparent);
}

/* Drag feedback: dim the grabbed button, accent-outline the drop target. */
.icon-btn.dragging {
  opacity: 0.4;
  cursor: grabbing;
}
.icon-btn.drop-target {
  background: var(--accent-soft);
  border-color: var(--accent);
  transform: scale(1.05);
}
.icon-btn.brand[draggable="true"] {
  cursor: grab;
}
.icon-btn.brand[draggable="true"]:active {
  cursor: grabbing;
}

/* Drag handle — six-dot grip is drag-only; the ⚙ button right before it
   opens the in-header editor. Grab cursor advertises the affordance. */
.icon-btn.drag-handle { cursor: grab; }
.icon-btn.drag-handle:active { cursor: grabbing; }

/* Add-task button — accent-tinted so it stands out from the muted
   quick-action and edit buttons; this is the primary CTA on the row. */
.icon-btn.add-btn {
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 10%, transparent);
  border-color: color-mix(in srgb, var(--accent) 25%, transparent);
}
.icon-btn.add-btn:hover {
  background: color-mix(in srgb, var(--accent) 20%, transparent);
  border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  color: var(--accent);
}

.editor {
  padding: 0.5rem 0.6rem 0.6rem;
  background: var(--surface);
  border-top: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  animation: slide-down 140ms ease-out;
}

@keyframes slide-down {
  from { opacity: 0; transform: translateY(-4px); }
  to { opacity: 1; transform: translateY(0); }
}

.editor label {
  display: flex;
  flex-direction: column;
  gap: 3px;
  font-size: 0.72rem;
  color: var(--text-muted);
}
.editor label .hint { color: var(--text-muted); font-weight: normal; opacity: 0.7; }

.editor input {
  padding: 0.3rem 0.5rem;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 5px;
  color: var(--text);
  font-size: 0.82rem;
}
.editor input:focus { outline: none; border-color: var(--border-strong); }

.root-row { display: flex; gap: 4px; }
.root-row input { flex: 1; }
.root-row .pick-btn {
  width: 30px;
  background: var(--surface-strong);
  border: 1px solid var(--border);
  border-radius: 5px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
}
.root-row .pick-btn:hover { background: var(--accent-soft); color: var(--text); }

.color-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 0.72rem;
  color: var(--text-muted);
}
.color-label { line-height: 1; }
.swatches {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.swatch {
  width: 22px;
  height: 22px;
  padding: 0;
  border-radius: 999px;
  border: 2px solid transparent;
  background: transparent;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: transform 100ms, border-color 100ms;
  box-shadow: 0 0 0 1px var(--border) inset;
}
.swatch:hover { transform: scale(1.1); }
.swatch.active {
  border-color: var(--text);
  box-shadow: 0 0 0 1px var(--bg) inset, 0 0 0 3px var(--text);
}
.swatch.none {
  background: var(--surface);
  color: var(--text-muted);
}
.swatch.none.active {
  background: var(--surface);
  border-color: var(--text);
  color: var(--text);
}

.editor-actions {
  display: flex;
  gap: 6px;
  margin-top: 2px;
  align-items: center;
  flex-wrap: wrap;
}
.editor-actions .spacer { flex: 1; }

.editor-actions button {
  padding: 0.3rem 0.7rem;
  font-size: 0.78rem;
  border-radius: 5px;
  cursor: pointer;
  border: 1px solid var(--border);
  background: var(--surface-strong);
  color: var(--text);
}
.editor-actions button:hover { background: var(--accent-soft); }
.editor-actions button.primary { background: var(--accent); color: var(--surface); border-color: var(--accent); }
.editor-actions button.primary:hover { opacity: 0.9; }
.editor-actions button.danger { color: #ef4444; border-color: rgba(239,68,68,0.3); }
.editor-actions button.danger:hover { background: rgba(239,68,68,0.1); }
.editor-actions button.ghost { background: transparent; }
.editor-actions button:disabled { opacity: 0.5; cursor: default; }

.error {
  color: #ef4444;
  font-size: 0.75rem;
  margin: 0;
  padding: 0.35rem 0.75rem;
  background: rgba(239, 68, 68, 0.08);
  border-bottom: 1px solid rgba(239, 68, 68, 0.2);
  cursor: pointer;
}

.rows {
  padding: 0.2rem 0;
}

.empty-source {
  padding: 0.6rem 0.75rem;
  font-size: 0.75rem;
  color: var(--text-muted);
  font-style: italic;
}
</style>
