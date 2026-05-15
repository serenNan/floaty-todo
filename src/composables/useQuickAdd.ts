import { ref, shallowRef } from 'vue';
import type { Quadrant } from '../types/task';

export interface QuickAddOptions {
  /// Source initially selected in the dialog. Required — the entry point is
  /// always a specific source's "+" button, so we always have one.
  sourceId: string;
}

export interface QuickAddResult {
  text: string;
  sourceId: string;
  quadrant: Quadrant | null;
}

interface PendingAdd extends QuickAddOptions {
  resolve: (result: QuickAddResult | null) => void;
}

// Singleton — same shape as useConfirm / useTaskEditor. A single
// <QuickAddDialog/> at App root renders the UI; any component calls
// openQuickAdd(...) and awaits the promise.
const pending = shallowRef<PendingAdd | null>(null);
const visible = ref(false);

export function openQuickAdd(options: QuickAddOptions): Promise<QuickAddResult | null> {
  if (pending.value) pending.value.resolve(null);
  return new Promise<QuickAddResult | null>(resolve => {
    pending.value = { ...options, resolve };
    visible.value = true;
  });
}

export function useQuickAddState() {
  function close(result: QuickAddResult | null) {
    const p = pending.value;
    visible.value = false;
    pending.value = null;
    p?.resolve(result);
  }
  return { visible, pending, close };
}
