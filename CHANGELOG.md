# 变更日志

## 2026-05-14 add atomic line-level storage (storage.rs)

- implemented `toggle_task` (1-indexed, CRLF-safe, `split_inclusive` line preservation) and `append_task` (creates file + `# Inbox` header if missing)
- `atomic_write` uses `tempfile::NamedTempFile` + `persist` (rename) for crash-safe writes; returns `ContentHash` (SHA-256) for watcher loop prevention
- `replace_first_bracket` is byte-safe ASCII scan — no regex, O(n) on line length
- 9 unit tests pass (toggle both directions, CRLF, hash round-trip, non-task error, append variants)
- `mod storage;` added to `lib.rs`

## 2026-05-14 add markdown task parser (parser.rs)

- implemented `parse_line` (regex-based, supports `- * +` bullets, `[ ] [x] [X]`, indent counting, trailing-whitespace trim) and `parse_file` (BOM stripping, stable 8-byte SHA-256 ID per file+line)
- 10 unit tests pass (parse variants, alt bullets, indent, BOM, stable ID, non-task lines)

## 2026-05-14 add Rust foundations (error/types/hashing)

- added `AppError` (thiserror) + `Result<T>` alias with Tauri-compatible `Serialize` impl; added `Task`, `AppConfig`, `ContentHash` structs and `hash_content` (SHA-256 via sha2)
- added deps: regex, notify, notify-debouncer-full, sha2, walkdir, once_cell, thiserror, tokio, tauri-plugin-dialog, hex, tempfile (dev)

## 2026-05-14 scaffold Tauri 2 + Vue 3 + TS project

- create-tauri-app v4.6.2, template vue-ts, identifier `com.serendipity.floaty-todo`
- fixed template bug: `--name` parsed as literal directory name; renamed all occurrences to `floaty-todo`
- installed pinia ^3.0.4
- smoke test passed: 360 crates compiled in 2m 53s, Vite up on localhost:1420, `floaty-todo.exe` launched
