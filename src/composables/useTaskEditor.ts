import { ref, shallowRef } from 'vue';
import type { Task } from '../types/task';

interface PendingEdit {
  task: Task;
  resolve: (newText: string | null) => void;
}

// Singleton pattern (same shape as useConfirm): any component can call
// `editTask(task)`, the single `<TaskEditorDialog>` at the App root renders UI.
const pending = shallowRef<PendingEdit | null>(null);
const visible = ref(false);

export function editTask(task: Task): Promise<string | null> {
  if (pending.value) pending.value.resolve(null);
  return new Promise<string | null>(resolve => {
    pending.value = { task, resolve };
    visible.value = true;
  });
}

export function useTaskEditorState() {
  function close(newText: string | null) {
    const p = pending.value;
    visible.value = false;
    pending.value = null;
    p?.resolve(newText);
  }
  return { visible, pending, close };
}
