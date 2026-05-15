import { defineConfig } from 'vitest/config';

// 独立配置，不复用 vite.config.ts —— 那份是 Tauri dev/build 专用的。
export default defineConfig({
  test: {
    include: ['src/**/*.test.ts'],
    environment: 'node',
  },
});
