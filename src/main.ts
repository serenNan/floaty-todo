import { createApp } from 'vue';
import { createPinia } from 'pinia';
import { getCurrentWindow } from '@tauri-apps/api/window';
import App from './App.vue';
import HistoryView from './views/HistoryView.vue';
import { i18n } from './i18n';
import { useHistoryStore } from './stores/history';
import { useTaskStore } from './stores/tasks';
import { api } from './services/tauri-api';
// Theme tokens (CSS vars + dark-mode rules). Must be imported globally for
// BOTH windows — the history window's root is HistoryView, which means
// App.vue's `@import` won't fire there. Without this the history window
// renders with all `var(--bg)` etc. undefined → pure white screen until
// the user clicks again.
import './styles/main.css';

const pinia = createPinia();
const root = getCurrentWindow().label === 'history' ? HistoryView : App;

createApp(root).use(pinia).use(i18n).mount('#app');

const history = useHistoryStore(pinia);
const tasks = useTaskStore(pinia);

function isEditingText(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName.toLowerCase();
  return tag === 'input' || tag === 'textarea' || target.isContentEditable;
}

window.addEventListener('keydown', async event => {
  if (!event.ctrlKey || event.altKey || event.metaKey || isEditingText(event.target)) return;

  const key = event.key.toLowerCase();
  if (key === 'z' && !event.shiftKey) {
    event.preventDefault();
    await history.undo();
    await tasks.silentRefresh();
    return;
  }
  if (key === 'y' || (key === 'z' && event.shiftKey)) {
    event.preventDefault();
    await history.redo();
    await tasks.silentRefresh();
    return;
  }
  if (key === 'h') {
    event.preventDefault();
    await api.openHistoryWindow();
  }
});
