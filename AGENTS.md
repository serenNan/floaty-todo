# Floaty Todo — Project Notes

## Tech Stack

- **Frontend:** Vue 3 + TypeScript, Vite 6, Pinia, vue-i18n (en/zh)
- **Backend:** Tauri 2 (Rust), `tauri-plugin-dialog` (file picker)
- **Package manager:** npm
- **Identifier:** `com.serendipity.floaty-todo`

## Structure

```
src/              # Vue frontend
src-tauri/        # Rust backend
  src/lib.rs      # App init, tray, watcher dispatch + invoke_handler registration
  src/main.rs     # Entry point (calls floaty_todo_lib::run())
  src/commands.rs # Tauri IPC commands + AppState (registry/config/watcher glue)
  src/types.rs    # Task, AppConfig, ContentHash
  src/error.rs    # AppError (thiserror) + Serialize impl for Tauri
  src/parser.rs   # Markdown task parser (parse_line / parse_file)
  src/storage.rs  # Atomic file writes: toggle_task / append_task
  src/config.rs   # AppConfig load/save (JSON, tolerant)
  src/registry.rs # In-memory TaskRegistry (per-source scan + per-file refresh)
  src/watcher.rs  # Debounced fs watcher (one per source) + IgnoreHashes loop prevention
  src/shell.rs    # External-process launchers (VS Code / terminal) with platform cascade
  tauri.conf.json # App config (productName, identifier, devUrl, window 340×520 transparent decorations:false skipTaskbar alwaysOnTop)
  Cargo.toml      # Rust deps (crate name: floaty-todo, lib: floaty_todo_lib)
```

## Data Model

Multi-source aggregation (v0.2): user configures **N task sources**, each one of:
- **Folder source** — recursive `.md` scan under `path`
- **File source** — one specific `.md` file (watcher tracks parent dir, filters by filename)

Each `Source` carries `id` (8-byte hex sha256 of canonical path), `path`, `kind`, optional `label`, and optional `project_root` (used by `open_in_vscode` / `open_in_terminal`; defaults: Folder→`path`, File→`path.parent()`).

Tasks reference their source via `Task.source_id`. The registry keys files by `(source_id, canonical_path)` so a file appearing under two sources stays independent.

## Rust Modules

| Module | Role |
|---|---|
| `commands` | `AppState` + Tauri commands: `get_tasks`/`toggle_task`/`add_task`, source CRUD (`list_sources`/`add_source`/`remove_source`/`update_source`/`set_default_source`), per-file label override (`set_file_label`), shell actions (`open_in_vscode`/`open_in_terminal`), window control |
| `shell` | Side-effect launchers: `open_vscode(path)` (Windows `code.cmd`, else `code`), `open_terminal(path)` (Windows: wt → pwsh → powershell; macOS: `open -a Terminal`; Linux: x-terminal-emulator → gnome-terminal → konsole → xterm) |
| `registry` | `TaskRegistry` keyed by `(source_id, canonical_path)`; `rebuild_from_sources` / `rebuild_source` / `refresh_file(source, file)` |
| `watcher` | `start_watching_source` (Folder = recursive, File = parent dir + filename filter) + `IgnoreHashes` for write-loop prevention; one `WatcherHandle` per source in `WatcherSlots: Arc<Mutex<HashMap<source_id, WatcherHandle>>>` |
| `storage` | `toggle_task` / `append_task` — atomic writes via `tempfile::NamedTempFile` |
| `config` | `load_from` / `save_to` / `config_file` — JSON, corrupt-tolerant |
| `parser` | `parse_line` / `parse_file(path, source_id)` — regex, stable SHA-256 task IDs |
| `types` | `Task` (with `source_id`), `Source` / `SourceKind` (Folder/File), `AppConfig` (`sources` Vec + `default_source_id` + `file_labels` HashMap), `ContentHash`, `file_label_key()` helper |
| `error` | `AppError` (Io/Json/Watcher/NoSources/SourceNotFound/DuplicateSource/InvalidSourcePath/TaskNotFound/NotATaskLine/CommandFailed) |

## Frontend Modules

| Module | Role |
|---|---|
| `src/types/task.ts` | `Task` / `Source` / `SourceKind` / `AppConfig` TS interfaces (mirror Rust) |
| `src/services/tauri-api.ts` | `api` object — wraps `invoke` commands + dialog pickers (`pickFolder` / `pickMarkdownFile`) + event listeners (`tasks-updated`, `sources-changed`, `request-manage-sources`) |
| `src/stores/tasks.ts` | `useTaskStore` — `tasks` / `sortedTasks` / `loading` / `error`; `refresh` / `silentRefresh` / `toggle` / `add(text, sourceId?)` |
| `src/stores/settings.ts` | `useSettingsStore` — `config` / `sources` / `hasSources` / `defaultSourceId`; `addSource` / `removeSource` / `updateSource` / `setDefaultSource` / `pickAndAddFolder` / `pickAndAddFile` |
| `src/main.ts` | App entry — wires `createPinia()` + i18n then mounts `App` |
| `src/i18n/` | `vue-i18n` setup + `locales/en.ts` / `locales/zh.ts`; `setLocale()` persists to localStorage `floaty.locale` and syncs `<html lang>` |
| `src/composables/useTheme.ts` | Theme composable — `currentTheme` / `effectiveTheme` / `setTheme`; localStorage `floaty.theme`, system media query listener |
| `src/views/SettingsView.vue` | Full-screen settings page — Appearance (theme segmented), Language (locale select), Sources (cards with ⎘ / ▷ / 📝 / 🗑 + inline editor), About; emits `back` |
| `src/components/SourceGroup.vue` | Collapsible per-source group: header (caret + kind icon + label + default badge + counts) + action chips (⎘ VS Code / ▷ terminal / ⋯ edit) + inline editor (label / project_root / set-default / remove); tasks are bucketed by `source_file` and rendered as nested `FileGroup` children |
| `src/components/FileGroup.vue` | Per-file sub-group inside a `SourceGroup`: independently collapsible, hover-revealed ✎ rename button, inline rename input (Enter / Esc / ↺ reset); falls back to the file's relative path inside the source when no custom label is set |
| `src/components/TaskList.vue` | Grouped task view (renders `SourceGroup` per source in config order); QuickAdd input + per-task source dropdown; footer with bottom-left ⚙ Settings + totals + ↻ refresh |
| `src/components/EmptyState.vue` | First-run landing: 📁 Folder / 📄 File picker buttons + bottom-left ⚙ Settings corner button |

## Build Commands

```powershell
npm run tauri dev    # dev mode (Vite + cargo run)
npm run tauri build  # production bundle
```

## Key Notes

- `src-tauri/src/main.rs` calls `floaty_todo_lib::run()` — lib crate name is `floaty_todo_lib` (underscores, not hyphens)
- Dev URL is `http://localhost:1420` (configured in `tauri.conf.json`)
- `node_modules/` and `src-tauri/target/` are gitignored
