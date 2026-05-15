<script setup lang="ts">
import { useToastState, type ToastVariant } from '../composables/useToast';

const { items, dismiss, pause, resume } = useToastState();

function iconChar(variant: ToastVariant): string {
  switch (variant) {
    case 'success': return '✓';
    case 'warning': return '⚠';
    case 'error':   return '✕';
    default:        return 'ⓘ';
  }
}
</script>

<template>
  <Teleport to="body">
    <div class="toast-stack">
      <TransitionGroup name="toast" tag="div" class="toast-stack-inner">
        <div
          v-for="item in items"
          :key="item.id"
          class="toast"
          :class="['toast-' + item.variant]"
          role="status"
          @mouseenter="pause(item.id)"
          @mouseleave="resume(item.id)"
        >
          <span class="icon" aria-hidden="true">{{ iconChar(item.variant) }}</span>
          <span class="text">{{ item.message }}</span>
          <button
            type="button"
            class="close"
            tabindex="-1"
            aria-label="Dismiss"
            @click="dismiss(item.id)"
          >×</button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-stack {
  position: fixed;
  bottom: 52px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 1000;
  pointer-events: none;
  width: max-content;
  max-width: calc(100% - 24px);
}
.toast-stack-inner {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}
.toast {
  pointer-events: auto;
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  max-width: 100%;
  padding: 8px 26px 8px 14px;
  border: 1px solid transparent;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
  font-size: 0.82rem;
  font-weight: 500;
  color: #fff;
  text-align: center;
}
.toast-success { background: #16a34a; border-color: #15803d; }
.toast-warning { background: #f59e0b; border-color: #d97706; color: #422006; }
.toast-error   { background: #ef4444; border-color: #dc2626; }
.toast-info    { background: #475569; border-color: #334155; }
.toast .icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  font-weight: 700;
}
.toast .text {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.toast .close {
  position: absolute;
  top: 50%;
  right: 6px;
  transform: translateY(-50%);
  opacity: 0;
  transition: opacity 120ms ease-out;
  background: transparent;
  border: none;
  color: inherit;
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
  padding: 0;
  width: 16px;
  height: 16px;
}
.toast:hover .close { opacity: 0.85; }
.toast .close:hover { opacity: 1; }

.toast-enter-from {
  opacity: 0;
  transform: translateY(10px);
}
.toast-enter-active {
  transition: opacity 220ms ease-out, transform 220ms cubic-bezier(0.2, 0.9, 0.3, 1.2);
}
.toast-leave-to {
  opacity: 0;
  transform: scale(0.92);
}
.toast-leave-active {
  transition: opacity 180ms ease-in, transform 180ms ease-in;
  position: absolute;
}
.toast-move {
  transition: transform 220ms ease-out;
}
</style>
