<script setup lang="ts">
import { computed, watch, nextTick, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useConfirmState } from '../composables/useConfirm';

const { t } = useI18n();
const { visible, pending, answer } = useConfirmState();

const confirmBtn = ref<HTMLButtonElement | null>(null);

const title = computed(() => pending.value?.title ?? t('confirm.defaultTitle'));
const message = computed(() => pending.value?.message ?? '');
const confirmText = computed(() => pending.value?.confirmText ?? t('confirm.ok'));
const cancelText = computed(() => pending.value?.cancelText ?? t('confirm.cancel'));
const danger = computed(() => pending.value?.danger ?? false);

// Move focus to the confirm button when the dialog opens (Enter then commits).
watch(visible, async (v) => {
  if (v) {
    await nextTick();
    confirmBtn.value?.focus();
  }
});

function onKey(e: KeyboardEvent) {
  if (!visible.value) return;
  if (e.key === 'Escape') { e.preventDefault(); answer(false); }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="confirm-fade">
      <div
        v-if="visible"
        class="confirm-overlay"
        @click.self="answer(false)"
        @keydown="onKey"
        tabindex="-1"
      >
        <div class="confirm-card" role="dialog" :aria-label="title">
          <h3 class="title">{{ title }}</h3>
          <p class="message">{{ message }}</p>
          <div class="actions">
            <button class="ghost" @click="answer(false)">{{ cancelText }}</button>
            <button
              ref="confirmBtn"
              :class="['primary', { danger }]"
              @click="answer(true)"
            >{{ confirmText }}</button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.confirm-overlay {
  position: fixed;
  inset: 0;
  background: color-mix(in srgb, #000 35%, transparent);
  backdrop-filter: blur(2px);
  -webkit-backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 1.5rem;
}

.confirm-card {
  width: 100%;
  max-width: 320px;
  background: var(--surface);
  border: 1px solid var(--border-strong);
  border-radius: 12px;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.35);
  padding: 1rem 1.1rem 0.9rem;
  animation: pop 160ms cubic-bezier(0.2, 0.9, 0.3, 1.2);
}

@keyframes pop {
  from { transform: scale(0.94); opacity: 0; }
  to   { transform: scale(1);    opacity: 1; }
}

.title {
  margin: 0 0 0.4rem;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text);
}

.message {
  margin: 0 0 1rem;
  font-size: 0.84rem;
  line-height: 1.5;
  color: var(--text-muted);
  white-space: pre-wrap;
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
.actions button.primary:hover { opacity: 0.9; background: var(--accent); }

.actions button.danger {
  background: #ef4444;
  border-color: #ef4444;
  color: #fff;
}
.actions button.danger:hover { background: #dc2626; border-color: #dc2626; }

.actions button.ghost { background: transparent; }
.actions button.ghost:hover { background: var(--surface-strong); }

/* fade-in for the whole overlay */
.confirm-fade-enter-active,
.confirm-fade-leave-active { transition: opacity 140ms ease-out; }
.confirm-fade-enter-from,
.confirm-fade-leave-to { opacity: 0; }
</style>
