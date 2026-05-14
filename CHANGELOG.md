# 变更日志

## 2026-05-14 scaffold Tauri 2 + Vue 3 + TS project

- create-tauri-app v4.6.2, template vue-ts, identifier `com.serendipity.floaty-todo`
- fixed template bug: `--name` parsed as literal directory name; renamed all occurrences to `floaty-todo`
- installed pinia ^3.0.4
- smoke test passed: 360 crates compiled in 2m 53s, Vite up on localhost:1420, `floaty-todo.exe` launched
