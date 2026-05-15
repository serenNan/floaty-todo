<script setup lang="ts">
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { bindCollapse } from '../composables/useCollapse';
import type { Quadrant, Task } from '../types/task';
import TaskItem from './TaskItem.vue';

const props = withDefaults(defineProps<{
  quadrant: Quadrant | null;
  tasks: Task[];
  // Per-source toggle tokens — parent SourceGroup increments one of these
  // to drive every quadrant in that source open/closed in one click.
  // Default 0 → watches never fire until the parent acts.
  collapseToken?: number;
  expandToken?: number;
}>(), { collapseToken: 0, expandToken: 0 });

const { t } = useI18n();

const collapsed = ref(true);
bindCollapse((v) => { collapsed.value = v; });
watch(() => props.collapseToken, () => { collapsed.value = true; });
watch(() => props.expandToken, () => { collapsed.value = false; });

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
</script>

<template>
  <div v-if="tasks.length > 0" class="quadrant-group" :class="{ collapsed }">
    <button class="quadrant-header" @click="collapsed = !collapsed">
      <span class="caret">{{ collapsed ? '▶' : '▼' }}</span>
      <span class="emoji">{{ emoji(quadrant) }}</span>
      <span class="name">{{ t(nameKey(quadrant)) }}</span>
      <span class="count">{{ tasks.length }}</span>
    </button>
    <div v-show="!collapsed" class="quadrant-tasks">
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
.quadrant-tasks { padding-left: 0.6rem; }
</style>
