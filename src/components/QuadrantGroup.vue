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
}>(), { collapseToken: 0, expandToken: 0 });

const { t } = useI18n();
const taskStore = useTaskStore();

const collapsed = ref(true);
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
  padding: 0.15rem 0.4rem;
  background: none;
  border: 0;
  font: inherit;
  color: var(--text-muted, #888);
  cursor: pointer;
  text-align: left;
}
.quadrant-header:hover { color: var(--text, #ddd); }
.quadrant-header .caret { width: 0.8em; font-size: 0.75em; }
.quadrant-header .emoji { font-size: 0.95em; }
.quadrant-header .name { flex: 1; font-size: 0.82em; }
.quadrant-header .count {
  font-variant-numeric: tabular-nums;
  font-size: 0.78em;
  opacity: 0.7;
}
/* Hover-revealed "+" — sits in the header but only fades in when the row
   is hovered, so the resting state stays as minimal as before. Rendered
   as a span+role="button" because <button> can't nest inside <button>. */
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
  opacity: 0;
  transition: opacity 120ms, background 100ms, border-color 100ms, color 100ms;
}
.quadrant-header:hover .add-btn,
.quadrant-header:focus-within .add-btn { opacity: 1; }
.quadrant-header .add-btn:hover {
  background: color-mix(in srgb, var(--accent) 15%, transparent);
  border-color: color-mix(in srgb, var(--accent) 35%, transparent);
  color: var(--accent);
}
.quadrant-tasks { padding-left: 0.6rem; }
</style>
