import { ref, type Ref } from 'vue';

export type ToastVariant = 'success' | 'warning' | 'error' | 'info';

export interface ToastItem {
  id: number;
  variant: ToastVariant;
  message: string;
  duration: number;
  remaining: number;
  startedAt: number;
  timerId: number | null;
}

export interface ToastOptions {
  duration?: number;
}

const DEFAULTS: Record<ToastVariant, number> = {
  success: 2000,
  info:    3000,
  warning: 4000,
  error:   6000,
};

const MAX_VISIBLE = 3;

const items = ref<ToastItem[]>([]);
let nextId = 1;

function push(variant: ToastVariant, message: string, opts?: ToastOptions) {
  const duration = opts?.duration ?? DEFAULTS[variant];
  const item: ToastItem = {
    id: nextId++,
    variant,
    message,
    duration,
    remaining: duration,
    startedAt: Date.now(),
    timerId: null,
  };
  items.value = [...items.value, item];
  startTimer(item);
  while (items.value.length > MAX_VISIBLE) {
    const oldest = items.value[0];
    if (oldest) dismiss(oldest.id);
    else break;
  }
}

function startTimer(item: ToastItem) {
  if (item.duration <= 0) return;
  item.startedAt = Date.now();
  item.timerId = window.setTimeout(() => dismiss(item.id), item.remaining);
}

function dismiss(id: number) {
  const idx = items.value.findIndex(i => i.id === id);
  if (idx < 0) return;
  const item = items.value[idx];
  if (item.timerId !== null) window.clearTimeout(item.timerId);
  items.value = items.value.filter(i => i.id !== id);
}

function pause(id: number) {
  const item = items.value.find(i => i.id === id);
  if (!item || item.timerId === null) return;
  window.clearTimeout(item.timerId);
  item.timerId = null;
  item.remaining = Math.max(0, item.remaining - (Date.now() - item.startedAt));
}

function resume(id: number) {
  const item = items.value.find(i => i.id === id);
  if (!item || item.timerId !== null || item.remaining <= 0) return;
  startTimer(item);
}

export const toast = {
  success: (msg: string, opts?: ToastOptions) => push('success', msg, opts),
  warning: (msg: string, opts?: ToastOptions) => push('warning', msg, opts),
  error:   (msg: string, opts?: ToastOptions) => push('error',   msg, opts),
  info:    (msg: string, opts?: ToastOptions) => push('info',    msg, opts),
};

export function useToastState(): {
  items: Ref<ToastItem[]>;
  dismiss: (id: number) => void;
  pause:   (id: number) => void;
  resume:  (id: number) => void;
} {
  return { items, dismiss, pause, resume };
}
