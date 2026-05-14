import { ref, shallowRef } from 'vue';

export interface ConfirmOptions {
  title?: string;
  message: string;
  /// Confirm button label (defaults to i18n "OK").
  confirmText?: string;
  /// Cancel button label (defaults to i18n "Cancel").
  cancelText?: string;
  /// Render confirm in a destructive style (red).
  danger?: boolean;
}

interface PendingConfirm extends ConfirmOptions {
  resolve: (ok: boolean) => void;
}

// Module-level singleton state: any component can call `confirm(...)`, the
// single `<ConfirmDialog>` mounted at the App root handles UI.
const pending = shallowRef<PendingConfirm | null>(null);
const visible = ref(false);

export function confirm(options: ConfirmOptions): Promise<boolean> {
  // If a confirm is already pending, reject it (the new request wins).
  if (pending.value) pending.value.resolve(false);
  return new Promise<boolean>(resolve => {
    pending.value = { ...options, resolve };
    visible.value = true;
  });
}

export function useConfirmState() {
  function answer(ok: boolean) {
    const p = pending.value;
    visible.value = false;
    pending.value = null;
    p?.resolve(ok);
  }
  return { visible, pending, answer };
}
