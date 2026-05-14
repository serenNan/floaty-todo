import { ref, computed, onMounted, onUnmounted } from 'vue';

type ThemePref = 'system' | 'light' | 'dark';
type ThemeValue = 'light' | 'dark';

const STORAGE_KEY = 'floaty.theme';

function resolveSystem(): ThemeValue {
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function applyToDOM(value: ThemeValue) {
  document.documentElement.setAttribute('data-theme', value);
}

// Module-level singleton state so all consumers share one instance
const currentTheme = ref<ThemePref>(
  (localStorage.getItem(STORAGE_KEY) as ThemePref | null) ?? 'system'
);

const effectiveTheme = computed<ThemeValue>(() =>
  currentTheme.value === 'system' ? resolveSystem() : currentTheme.value
);

function setTheme(t: ThemePref) {
  currentTheme.value = t;
  localStorage.setItem(STORAGE_KEY, t);
  applyToDOM(effectiveTheme.value);
}

export function useTheme() {
  // Watch system preference changes when in "system" mode
  let mql: MediaQueryList | null = null;

  function onSystemChange() {
    if (currentTheme.value === 'system') {
      applyToDOM(resolveSystem());
    }
  }

  onMounted(() => {
    mql = window.matchMedia('(prefers-color-scheme: dark)');
    mql.addEventListener('change', onSystemChange);
  });

  onUnmounted(() => {
    mql?.removeEventListener('change', onSystemChange);
    mql = null;
  });

  return { currentTheme, effectiveTheme, setTheme };
}
