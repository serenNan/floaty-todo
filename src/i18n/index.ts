import { createI18n } from 'vue-i18n';
import en from './locales/en';
import zh from './locales/zh';

export const SUPPORTED_LOCALES = ['en', 'zh'] as const;
export type Locale = (typeof SUPPORTED_LOCALES)[number];

const STORAGE_KEY = 'floaty.locale';

function detectInitialLocale(): Locale {
  // Honor explicit user choice first.
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored && (SUPPORTED_LOCALES as readonly string[]).includes(stored)) {
    return stored as Locale;
  }
  // Fall back to the navigator language — anything starting with "zh" → zh.
  const nav = navigator.language?.toLowerCase() ?? '';
  if (nav.startsWith('zh')) return 'zh';
  return 'en';
}

export const i18n = createI18n({
  legacy: false,
  locale: detectInitialLocale(),
  fallbackLocale: 'en',
  messages: { en, zh },
});

export function setLocale(locale: Locale) {
  i18n.global.locale.value = locale;
  localStorage.setItem(STORAGE_KEY, locale);
  document.documentElement.lang = locale === 'zh' ? 'zh-CN' : 'en';
}

// Sync the initial document lang attribute.
document.documentElement.lang =
  (i18n.global.locale.value as Locale) === 'zh' ? 'zh-CN' : 'en';
