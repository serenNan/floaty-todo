# 变更日志

## 2026-05-14 multi-source aggregation (folder + single-file sources)

Replaces the single-vault model with a user-configurable list of task sources. Each source is either a recursive folder scan or a single `.md` file, with an optional `project_root` for future "Open in VS Code / terminal" actions.

### Backend (Rust)
- `types.rs`: added `Source` (`id`/`path`/`kind`/`label`/`project_root`) and `SourceKind` (`Folder`/`File`); `Task` now carries `source_id`; `AppConfig` now holds `sources: Vec<Source>` + `default_source_id: Option<String>` (vault_path removed, no migration since v0.1 was not released)
- `error.rs`: `NoVault` → `NoSources`; added `SourceNotFound` / `DuplicateSource` / `InvalidSourcePath` / `CommandFailed`
- `parser.rs`: `parse_file(path)` → `parse_file(path, source_id)`; each `Task` propagates `source_id`
- `registry.rs`: rewrote — `rebuild_from_sources(&[Source])`, `rebuild_source(&Source)`, `refresh_file(&Source, &Path)`; keyed by `(source_id, canonical_path)` so two sources covering the same file stay independent; folder sources keep walkdir behaviour, file sources scope to their single target
- `watcher.rs`: `start_watching` → `start_watching_source(&Source, …)`; folder = recursive, file = parent-dir non-recursive + filename filter (canonical compare)
- `commands.rs`: new — `list_sources` / `add_source` / `remove_source` / `update_source` / `set_default_source`; `add_task(text, source_id?)` (omitted ⇒ uses `default_source_id`); `set_vault` removed; `toggle_task` resolves the source via `Task.source_id` and refreshes scoped to that source
- `lib.rs`: `WatcherSlot` (one) → `WatcherSlots = Arc<Mutex<HashMap<source_id, WatcherHandle>>>`; setup spawns one scan+watcher per source; tray menu item "Switch vault folder…" → "Manage sources…" (emits `request-manage-sources`)
- 35 unit tests pass; added: `task_carries_source_id`, `file_source_collects_only_target_file`, `multi_source_aggregates`, `file_source_ignores_sibling_changes`, `file_source_only_fires_for_target_file`

### Frontend
- `src/types/task.ts`: mirrors Rust — `Source` / `SourceKind` / new `AppConfig` shape; `Task.source_id` added
- `src/services/tauri-api.ts`: drops `setVault` / `pickVaultFolder`; adds `listSources` / `addSource` / `removeSource` / `updateSource` / `setDefaultSource`, `pickFolder` / `pickMarkdownFile`, and listeners for `sources-changed` / `request-manage-sources`
- `src/stores/settings.ts`: replaced `pickAndSetVault` with `pickAndAddFolder` / `pickAndAddFile`; exposes `sources` / `hasSources` / `defaultSourceId` computeds and source CRUD helpers
- `src/stores/tasks.ts`: `add(text)` → `add(text, sourceId?)`
- `src/App.vue`: `hasVault` → `hasSources`; subscribes to `sources-changed` + `request-manage-sources`
- `src/components/EmptyState.vue`: two-button onboarding (📁 Folder… / 📄 File…) via `pickAndAddFolder` / `pickAndAddFile`
- `src/components/TaskList.vue`: footer chips become "📁+" / "📄+ N sources" quick-adders; QuickAdd input gains an inline source dropdown so the user can pick the destination per task (defaults to `default_source_id`)
- v0.2 source-grouped rendering + per-source quick-action buttons (VS Code / terminal) land in the next commits — current TaskList still renders the flat sorted list

## 2026-05-14 silent refresh + sorted tasks (undone-first)

- `src/stores/tasks.ts`: added `silentRefresh()` (no Loading flicker) for use after toggle / add / fs-event; `refresh()` still flips `loading` for first load and manual ↻
- `src/stores/tasks.ts`: new `sortedTasks` computed — undone before done, then stable by `source_file` + `line_number`
- `src/components/TaskList.vue`: renders and counts via `sortedTasks` (was `tasks`)
- `src/App.vue`: `tasks-updated` event listener now calls `silentRefresh` instead of `refresh`

## 2026-05-14 add Vue UI (EmptyState, TaskItem, TaskList, dark-mode CSS)

- `src/components/EmptyState.vue`: vault picker landing screen; calls `settings.pickAndSetVault()` then `tasks.refresh()`
- `src/components/TaskItem.vue`: single task row with checkbox, indent-aware padding, strikethrough-on-done styling
- `src/components/TaskList.vue`: full list view — add-task form, loading/error/empty states, footer counter, refresh button
- `src/App.vue`: rewired `onMounted` to load settings, conditionally refresh tasks if vault set, and subscribe to `tasks-updated` event; `onUnmounted` cleans up listener; routes between `EmptyState` and `TaskList` via `hasVault` computed
- `src/styles/main.css`: CSS variable tokens (`--bg`, `--bg-hover`, `--fg`, `--fg-muted`, `--border`) with automatic dark-mode override via `prefers-color-scheme: dark`
- Scaffold cleanup: deleted `src/assets/vue.svg` (no longer referenced)

## 2026-05-14 add frontend service layer (types, tauri-api, Pinia stores)

- `src/types/task.ts`: `Task` and `AppConfig` TypeScript interfaces mirroring Rust structs
- `src/services/tauri-api.ts`: `api` object wrapping 6 Tauri commands (`get_tasks`, `get_config`, `update_config`, `toggle_task`, `add_task`, `set_vault`), `open` dialog for vault folder picking, and `tasks-updated` event listener
- `src/stores/tasks.ts`: `useTaskStore` Pinia store with `tasks`, `loading`, `error` state; `refresh` / `toggle` / `add` actions
- `src/stores/settings.ts`: `useSettingsStore` with `config` state; `load` and `pickAndSetVault` actions
- `src/main.ts`: wires Pinia (`createPinia()`) before mounting
- `@tauri-apps/plugin-dialog` added to npm dependencies

## 2026-05-14 wire app: commands invoke_handler, tray icon, watcher bridge

- `lib.rs` rewritten: 8 commands registered in `invoke_handler!` (`get_tasks`, `get_config`, `update_config`, `toggle_task`, `add_task`, `set_vault`, `show_window`, `hide_window`)
- Tray menu with Show window / Hide window / Quit items; left-click tray icon toggles window visibility
- Watcher spawned in `setup` hook: initial `rebuild_from_vault` in background thread, then `start_watching` wired to emit `tasks-updated` events on file changes
- `tauri.conf.json` window now 380×600, `alwaysOnTop: true`, labeled `"main"` (was unlabeled 800×600)
- Capabilities updated: `core:default + dialog:default` (dropped unused `opener:default`)
- `tauri = { version = "2", features = ["tray-icon"] }` added to Cargo.toml
- `tauri-plugin-opener` no longer initialized in `lib.rs` (dep left in Cargo.toml as warning-only); `tauri-plugin-dialog` now active
- `AppConfig` unused import removed; zero warnings on `cargo build`

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
