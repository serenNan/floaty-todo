import { ref, watch, type WatchStopHandle } from 'vue';

/// Counter-style trigger. Incrementing it nudges every consumer; the value
/// itself is meaningless, only the change matters. Cheap to broadcast
/// (no Pinia store, no event emitter) and avoids the chore of hoisting
/// every `collapsed: ref<bool>` out of its component.
const collapseToken = ref(0);
const expandToken = ref(0);

export function collapseAll() { collapseToken.value++; }
export function expandAll() { expandToken.value++; }

/// Subscribe a component's local `collapsed` ref to the global tokens.
/// Returns a stop handle the caller can use in `onUnmounted` — but Vue
/// auto-disposes watch effects bound to the current component, so most
/// callers can ignore it.
export function bindCollapse(
  setCollapsed: (next: boolean) => void,
): WatchStopHandle {
  const a = watch(collapseToken, () => setCollapsed(true));
  const b = watch(expandToken, () => setCollapsed(false));
  return () => { a(); b(); };
}
