import { ref, shallowRef } from 'vue';
import type { Quadrant, Task } from '../types/task';

export interface TaskEditResult {
  text: string;
  quadrant: Quadrant | null;
}

interface PendingEdit {
  task: Task;
  resolve: (result: TaskEditResult | null) => void;
}

// Singleton pattern (same shape as useConfirm): any component can call
// `editTask(task)`, the single `<TaskEditorDialog>` at the App root renders UI.
// The resolved value bundles the new text and (possibly changed) quadrant;
// `null` means the user cancelled or made no effective change.
const pending = shallowRef<PendingEdit | null>(null);
const visible = ref(false);

export function editTask(task: Task): Promise<TaskEditResult | null> {
  if (pending.value) pending.value.resolve(null);
  return new Promise<TaskEditResult | null>(resolve => {
    pending.value = { task, resolve };
    visible.value = true;
  });
}

export function useTaskEditorState() {
  function close(result: TaskEditResult | null) {
    const p = pending.value;
    visible.value = false;
    pending.value = null;
    p?.resolve(result);
  }
  return { visible, pending, close };
}
