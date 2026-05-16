<script setup lang="ts">
import { computed, inject, ref, type Ref } from 'vue';
import type { Task } from '../types/task';
import { useTaskStore } from '../stores/tasks';
import { api } from '../services/tauri-api';
import { parseInline } from '../utils/inline-md';
import { editTask } from '../composables/useTaskEditor';
import { useI18n } from 'vue-i18n';
import Icon from './icons/Icon.vue';
import { confirm } from '../composables/useConfirm';

const props = defineProps<{ task: Task }>();
const tasks = useTaskStore();
const { t } = useI18n();

const segments = computed(() => parseInline(props.task.text));

const searchQueryRef = inject<Ref<string>>('searchQuery', ref(''));
const query = computed(() => searchQueryRef.value.trim().toLowerCase());

interface HighlightPart { text: string; match: boolean; }

function splitByQuery(text: string): HighlightPart[] {
  const q = query.value;
  if (!q) return [{ text, match: false }];
  const lower = text.toLowerCase();
  const out: HighlightPart[] = [];
  let i = 0;
  while (i < text.length) {
    const j = lower.indexOf(q, i);
    if (j < 0) {
      out.push({ text: text.slice(i), match: false });
      break;
    }
    if (j > i) out.push({ text: text.slice(i, j), match: false });
    out.push({ text: text.slice(j, j + q.length), match: true });
    i = j + q.length;
  }
  return out;
}

async function onTextClick() {
  const result = await editTask(props.task);
  if (!result) return;
  // Only pass quadrant through when the user actually changed it — otherwise
  // we'd force the backend's cross-quadrant move path for a plain text edit
  // and lose the in-place rewrite (which preserves indent / checkbox state).
  const quadrantArg = result.quadrant !== props.task.quadrant ? result.quadrant : undefined;
  await tasks.update(props.task.id, result.text, quadrantArg);
}

async function openLink(href: string) {
  try { await api.openUrl(href); }
  catch (e) { console.warn('openUrl failed:', e); }
}

async function onDelete() {
  const ok = await confirm({
    title: t('confirm.deleteTaskTitle'),
    message: t('confirm.deleteTaskMessage', { text: props.task.text }),
    confirmText: t('confirm.deleteTaskConfirm'),
    danger: true,
  });
  if (ok) await tasks.remove(props.task.id);
}
</script>

<template>
  <!-- @click.self.prevent on the label catches clicks that land on the
       label's own padding/gap whitespace (i.e. not on a child element) and
       suppresses the implicit `<label>` → wrapped-input toggle that would
       otherwise flip the checkbox. The `.self` modifier means clicks
       bubbling up from .text or the checkbox aren't touched. Especially
       relevant right after expanding a SourceGroup: the row's fadeIn
       translateY animation means the user's pointer often lands in the
       padding above where .text *will* be, hitting the label instead. -->
  <label class="row" :class="{ done: task.completed }" :style="{ paddingLeft: 8 + task.indent * 12 + 'px' }" @click.self.prevent>
    <input type="checkbox" :checked="task.completed" @change="tasks.toggle(task.id)" />
    <span class="text" @click.prevent.stop="onTextClick">
      <template v-for="(seg, i) in segments" :key="i">
        <code v-if="seg.type === 'code'" class="md-code">
          <template v-for="(p, j) in splitByQuery(seg.text)" :key="j"
            ><mark v-if="p.match" class="match">{{ p.text }}</mark
            ><template v-else>{{ p.text }}</template></template>
        </code>
        <strong v-else-if="seg.type === 'bold'">
          <template v-for="(p, j) in splitByQuery(seg.text)" :key="j"
            ><mark v-if="p.match" class="match">{{ p.text }}</mark
            ><template v-else>{{ p.text }}</template></template>
        </strong>
        <em v-else-if="seg.type === 'italic'">
          <template v-for="(p, j) in splitByQuery(seg.text)" :key="j"
            ><mark v-if="p.match" class="match">{{ p.text }}</mark
            ><template v-else>{{ p.text }}</template></template>
        </em>
        <s v-else-if="seg.type === 'strike'">
          <template v-for="(p, j) in splitByQuery(seg.text)" :key="j"
            ><mark v-if="p.match" class="match">{{ p.text }}</mark
            ><template v-else>{{ p.text }}</template></template>
        </s>
        <a
          v-else-if="seg.type === 'link'"
          class="md-link"
          :href="seg.href"
          :title="seg.href"
          @click.prevent.stop="openLink(seg.href)"
        ><template v-for="(p, j) in splitByQuery(seg.text)" :key="j"
            ><mark v-if="p.match" class="match">{{ p.text }}</mark
            ><template v-else>{{ p.text }}</template></template></a>
        <template v-else>
          <template v-for="(p, j) in splitByQuery(seg.text)" :key="j"
            ><mark v-if="p.match" class="match">{{ p.text }}</mark
            ><template v-else>{{ p.text }}</template></template>
        </template>
      </template>
    </span>
    <button
      type="button"
      class="del-btn"
      :title="t('confirm.deleteTaskTitle')"
      @click.prevent.stop="onDelete"
    >
      <Icon name="trash" :size="14" />
    </button>
  </label>
</template>

<style scoped>
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(4px); }
  to   { opacity: 1; transform: translateY(0); }
}

.row {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  padding: 0.38rem 0.75rem;
  cursor: pointer;
  border-radius: 0;
  transition: background 120ms ease-out;
  animation: fadeIn 140ms ease-out;
}

/* Hover tint inherits the parent SourceGroup's --src-color CSS var
   (set by `:style="--src-color: …"` on `.group.colored`). When no source
   color is configured, fall back to --accent so plain rows still get the
   neutral grey tint. ~14% mix matches the strength of --accent-soft and
   stays subtle on the card's semi-transparent body. */
.row:hover {
  background: color-mix(in srgb, var(--src-color, var(--accent)) 14%, transparent);
}

.row.done .text {
  text-decoration: line-through;
  color: var(--text-muted);
  opacity: 0.7;
}

.text {
  flex: 1;
  user-select: text;
  color: var(--text);
  font-size: 0.875rem;
  line-height: 1.4;
  word-break: break-word;
  cursor: text;
}

.text strong { font-weight: 600; color: var(--text); }
.text em { font-style: italic; }
.text s { color: var(--text-muted); }
.text .md-code {
  font-family: 'Cascadia Code', 'Consolas', 'JetBrains Mono', ui-monospace, monospace;
  font-size: 0.82em;
  padding: 1px 5px;
  background: var(--accent-soft);
  color: var(--accent);
  border-radius: 4px;
  border: 1px solid var(--border);
}
.text .md-link {
  color: var(--accent);
  text-decoration: underline;
  text-decoration-color: color-mix(in srgb, var(--accent) 40%, transparent);
  text-underline-offset: 2px;
  cursor: pointer;
}
.text .md-link:hover {
  text-decoration-color: var(--accent);
}

.text .match {
  background: #fde047;
  color: #1f2937;
  border-radius: 2px;
  padding: 0 2px;
  font-weight: 600;
}

input[type="checkbox"] {
  width: 15px;
  height: 15px;
  flex-shrink: 0;
  accent-color: var(--accent);
  cursor: pointer;
  background: var(--surface);
  border: 1px solid var(--border-strong);
  border-radius: 3px;
  margin-top: 3px;
}

.del-btn {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  padding: 0;
  margin-top: 1px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  border-radius: 4px;
  cursor: pointer;
  opacity: 0;
  transition: opacity 120ms ease-out, background 120ms ease-out, color 120ms ease-out;
}

/* Mirrors FileGroup's hover-reveal pencil button: only visible while the row
   is hovered, so the row stays visually quiet at rest. */
.row:hover .del-btn {
  opacity: 1;
}

.del-btn:hover {
  background: color-mix(in srgb, #ef4444 18%, transparent);
  color: #ef4444;
}
</style>
