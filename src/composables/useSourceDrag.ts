import { ref } from 'vue';

// Pointer-events based source-drag (Tauri WebView2 swallows HTML5 native
// dragover events, so the standard `draggable="true"` flow can't work).
// We track pointerdown on the drag handle, then attach document-level
// pointermove / pointerup listeners that walk the DOM via
// `elementFromPoint` to find the header under the cursor.

export const draggedSourceId = ref<string | null>(null);
export const dropTargetSourceId = ref<string | null>(null);

const DRAG_THRESHOLD = 5; // px — movement below this counts as a click

interface DragState {
  sourceId: string;
  startX: number;
  startY: number;
  isDragging: boolean;
  onClick: () => void;
  onDrop: (targetId: string) => void;
}

let active: DragState | null = null;

function findSourceIdUnderPoint(x: number, y: number): string | null {
  let el = document.elementFromPoint(x, y) as HTMLElement | null;
  while (el && el !== document.body) {
    const id = el.getAttribute('data-source-id');
    if (id) return id;
    el = el.parentElement;
  }
  return null;
}

export interface StartDragOpts {
  e: PointerEvent;
  sourceId: string;
  onClick: () => void;
  onDrop: (targetId: string) => void;
}

export function startSourceDrag(opts: StartDragOpts) {
  active = {
    sourceId: opts.sourceId,
    startX: opts.e.clientX,
    startY: opts.e.clientY,
    isDragging: false,
    onClick: opts.onClick,
    onDrop: opts.onDrop,
  };
  document.addEventListener('pointermove', onMove);
  document.addEventListener('pointerup', onUp);
  document.addEventListener('pointercancel', onCancel);
}

function onMove(e: PointerEvent) {
  if (!active) return;
  const dx = e.clientX - active.startX;
  const dy = e.clientY - active.startY;
  if (!active.isDragging) {
    if (Math.hypot(dx, dy) < DRAG_THRESHOLD) return;
    active.isDragging = true;
    draggedSourceId.value = active.sourceId;
  }
  const tgt = findSourceIdUnderPoint(e.clientX, e.clientY);
  const next = tgt && tgt !== active.sourceId ? tgt : null;
  if (next !== dropTargetSourceId.value) {
    dropTargetSourceId.value = next;
  }
}

function onUp(e: PointerEvent) {
  if (!active) return;
  const a = active;
  cleanupListeners();
  draggedSourceId.value = null;
  dropTargetSourceId.value = null;
  active = null;
  if (!a.isDragging) {
    a.onClick();
    return;
  }
  const tgt = findSourceIdUnderPoint(e.clientX, e.clientY);
  if (tgt && tgt !== a.sourceId) a.onDrop(tgt);
}

function onCancel() {
  if (!active) return;
  cleanupListeners();
  draggedSourceId.value = null;
  dropTargetSourceId.value = null;
  active = null;
}

function cleanupListeners() {
  document.removeEventListener('pointermove', onMove);
  document.removeEventListener('pointerup', onUp);
  document.removeEventListener('pointercancel', onCancel);
}

export function clearSourceDrag() {
  onCancel();
}
