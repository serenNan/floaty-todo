# 变更日志

## 2026-05-14 add Tauri IPC commands (commands.rs + AppState)

- `AppState` holds `Arc<RwLock<TaskRegistry>>`, `Arc<RwLock<AppConfig>>`, `IgnoreHashes`, and `config_path: PathBuf`
- Commands exposed: `get_tasks`, `get_config`, `update_config`, `toggle_task`, `add_task`, `set_vault`, `show_window`, `hide_window`
- `set_vault` persists config, rebuilds registry from new vault root, and emits `vault-changed` + `tasks-updated` events to the frontend
- `toggle_task` / `add_task` register the new content hash into `IgnoreHashes` before writing to prevent watcher re-fire loop
- `mod commands;` added to `lib.rs`; commands wired into `invoke_handler!` in Task 9
- Prior fix (commit `623b0e8`): `tempfile` promoted from `[dev-dependencies]` to `[dependencies]` — `atomic_write` in `storage.rs` uses it at runtime, not only in tests

## 2026-05-14 add fs watcher (debounced + loop prevention)

- `start_watching(vault, ignore, on_event)` wraps `notify-debouncer-full` with 200ms debounce; emits `WatchEvent::Changed` or `WatchEvent::Deleted` for markdown paths only
- `IgnoreHashes` (Arc+Mutex HashSet) provides single-shot loop prevention: writer registers content hash before write, watcher discards matching events and removes the entry
- Fixed `ev.paths` borrow: accessed via `ev.event.paths` (owned) to avoid Deref move-out error; added `use notify::Watcher` for `watch()` method in scope
- `WatcherHandle` wraps `Debouncer` to own its lifetime; drop stops the background thread
- 4 unit tests pass (hash register+consume, external change detection, hash-based suppression, non-markdown ignore) — run serialized with `--test-threads=1`
- `mod watcher;` added to `lib.rs`

## 2026-05-14 add registry (task index + ignore list)

- `TaskRegistry` holds `HashMap<id, Task>` + `HashMap<PathBuf, Vec<id>>` for per-file invalidation
- `rebuild_from_vault` walks vault via `walkdir`, skips non-markdown and ignored paths
- `refresh_file` removes stale entries then re-parses; handles deleted files via `best_effort_canonical` (parent-dir canonicalize + filename fallback, fixes Windows `\\?\` key mismatch)
- `is_markdown_target` / `is_not_ignored` are `pub` for reuse by Task 7 watcher
- Ignore list: `.obsidian`, `.git`, `.trash`, `node_modules`, `~`-prefix/suffix, `.swp`, `.tmp`
- 4 unit tests pass: vault scan, dir filtering, file refresh, deletion handling
- `pub mod registry;` added to `lib.rs`

## 2026-05-14 add persistent config (config.rs)

- `load_from` returns `AppConfig::default()` for missing or corrupt JSON (bricking prevention)
- `save_to` creates parent dirs, writes pretty JSON atomically via `std::fs::write`
- `config_file` helper composes path from Tauri's `app_config_dir`
- 3 unit tests pass: missing file, round-trip, corrupt fallback
- `mod config;` added to `lib.rs`

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
