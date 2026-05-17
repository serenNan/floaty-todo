<script setup lang="ts">
import { computed, ref, watch, inject, type Ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { bindCollapse } from '../composables/useCollapse';
import type { Quadrant, Task } from '../types/task';
import TaskItem from './TaskItem.vue';
import Icon from './icons/Icon.vue';
import { openQuickAdd } from '../composables/useQuickAdd';
import { useTaskStore } from '../stores/tasks';

const props = withDefaults(defineProps<{
  quadrant: Quadrant | null;
  tasks: Task[];
  // Source that owns these tasks. Used by the per-quadrant "+" button so
  // QuickAdd opens locked to this source AND this quadrant.
  sourceId: string;
  // Per-source toggle tokens — parent SourceGroup increments one of these
  // to drive every quadrant in that source open/closed in one click.
  // Default 0 → watches never fire until the parent acts.
  collapseToken?: number;
  expandToken?: number;
  // Stable identity for persisting the collapse state across remounts
  // (source collapse/expand) and across app restarts. Empty = no persist.
  persistenceKey?: string;
}>(), { collapseToken: 0, expandToken: 0, persistenceKey: '' });

const { t } = useI18n();
const taskStore = useTaskStore();

const STORAGE_PREFIX = 'floaty.qcollapse.';

function loadCollapsed(): boolean {
  if (!props.persistenceKey) return true;
  try {
    const v = localStorage.getItem(STORAGE_PREFIX + props.persistenceKey);
    return v === null ? true : v === '1';
  } catch { return true; }
}

const collapsed = ref(loadCollapsed());
watch(collapsed, (v) => {
  if (!props.persistenceKey) return;
  try { localStorage.setItem(STORAGE_PREFIX + props.persistenceKey, v ? '1' : '0'); } catch {}
});

bindCollapse((v) => { collapsed.value = v; });
watch(() => props.collapseToken, () => { collapsed.value = true; });
watch(() => props.expandToken, () => { collapsed.value = false; });

const searchQueryRef = inject<Ref<string>>('searchQuery', ref(''));
const searchActive = computed(() => searchQueryRef.value.trim().length > 0);
const effectiveCollapsed = computed(() => collapsed.value && !searchActive.value);

function emoji(q: Quadrant | null): string {
  switch (q) {
    case 'urgent_important': return '🔴';
    case 'not_urgent_important': return '🟡';
    case 'urgent_not_important': return '🟠';
    case 'not_urgent_not_important': return '🟢';
    default: return '⚪';
  }
}

function nameKey(q: Quadrant | null): string {
  switch (q) {
    case 'urgent_important': return 'quadrant.urgent_important';
    case 'not_urgent_important': return 'quadrant.not_urgent_important';
    case 'urgent_not_important': return 'quadrant.urgent_not_important';
    case 'not_urgent_not_important': return 'quadrant.not_urgent_not_important';
    default: return 'quadrant.unsorted';
  }
}

async function addHere() {
  const result = await openQuickAdd({
    sourceId: props.sourceId,
    quadrant: props.quadrant,
  });
  if (!result) return;
  await taskStore.add(result.text, result.sourceId, result.quadrant);
}
</script>

<template>
  <div v-if="tasks.length > 0" class="quadrant-group" :class="{ collapsed: effectiveCollapsed }">
    <button class="quadrant-header" @click="collapsed = !collapsed">
      <span class="caret">{{ effectiveCollapsed ? '▶' : '▼' }}</span>
      <span class="emoji">{{ emoji(quadrant) }}</span>
      <span class="name">{{ t(nameKey(quadrant)) }}</span>
      <span class="count">{{ tasks.length }}</span>
      <span
        class="add-btn"
        role="button"
        :title="t('source.addTask')"
        @click.stop="addHere"
      >
        <Icon name="plus" :size="12" />
      </span>
    </button>
    <div v-show="!effectiveCollapsed" class="quadrant-tasks">
      <TaskItem v-for="task in tasks" :key="task.id" :task="task" />
    </div>
  </div>
</template>

<style scoped>
.quadrant-group {
  margin: 0.25rem 0 0.5rem;
}
.quadrant-header {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  width: 100%;
  /* Left padding pulls the whole quadrant header ~5px to the left of the
     source header (whose left padding is 0.6rem) so the disclosure caret
     pokes slightly ahead of the source caret. */
  padding: 0.15rem 0.4rem 0.15rem 0.3rem;
  background: none;
  border: 0;
  font: inherit;
  color: var(--text-muted, #888);
  cursor: pointer;
  text-align: left;
}
.quadrant-header:hover { color: var(--text, #ddd); }
/* Match the source caret's 20px box so the two carets line up on the
   same grid — only the header's left padding sets the slight offset. */
.quadrant-header .caret {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  font-size: 0.7em;
}
.quadrant-header .emoji { font-size: 0.95em; }
.quadrant-header .name { flex: 1; font-size: 0.82em; }
.quadrant-header .count {
  font-variant-numeric: tabular-nums;
  font-size: 0.78em;
  opacity: 0.7;
}
/* Always-visible "+" — rendered as a span+role="button" because <button>
   can't nest inside the header's <button>. */
.quadrant-header .add-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  padding: 0;
  margin-left: 2px;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 4px;
  color: var(--text-muted);
  cursor: pointer;
  transition: background 100ms, border-color 100ms, color 100ms;
}
.quadrant-header .add-btn:hover {
  background: color-mix(in srgb, var(--accent) 15%, transparent);
  border-color: color-mix(in srgb, var(--accent) 35%, transparent);
  color: var(--accent);
}
.quadrant-tasks { padding-left: 0.6rem; }
</style>
