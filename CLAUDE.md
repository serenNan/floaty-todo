# Floaty Todo — Project Notes

## Tech Stack

- **Frontend:** Vue 3 + TypeScript, Vite 6, Pinia
- **Backend:** Tauri 2 (Rust), `tauri-plugin-opener`
- **Package manager:** npm
- **Identifier:** `com.serendipity.floaty-todo`

## Structure

```
src/              # Vue frontend
src-tauri/        # Rust backend
  src/lib.rs      # Module declarations + Tauri Builder setup
  src/main.rs     # Entry point (calls floaty_todo_lib::run())
  src/commands.rs # Tauri IPC commands + AppState (registry/config/watcher glue)
  src/types.rs    # Task, AppConfig, ContentHash
  src/error.rs    # AppError (thiserror) + Serialize impl for Tauri
  src/parser.rs   # Markdown task parser (parse_line / parse_file)
  src/storage.rs  # Atomic file writes: toggle_task / append_task
  src/config.rs   # AppConfig load/save (JSON, tolerant)
  src/registry.rs # In-memory TaskRegistry (vault scan + per-file refresh)
  src/watcher.rs  # Debounced fs watcher + IgnoreHashes loop prevention
  tauri.conf.json # App config (productName, identifier, devUrl)
  Cargo.toml      # Rust deps (crate name: floaty-todo, lib: floaty_todo_lib)
```

## Rust Modules

| Module | Role |
|---|---|
| `commands` | `AppState` struct + all `#[tauri::command]` fns; wired into `invoke_handler!` in Task 9 |
| `registry` | `TaskRegistry` — HashMap-backed index; `rebuild_from_vault` + `refresh_file` |
| `watcher` | `start_watching` + `IgnoreHashes` (hash-based write loop prevention) |
| `storage` | `toggle_task` / `append_task` — atomic writes via `tempfile::NamedTempFile` |
| `config` | `load_from` / `save_to` / `config_file` — JSON, corrupt-tolerant |
| `parser` | `parse_line` / `parse_file` — regex, stable SHA-256 task IDs |
| `types` | `Task`, `AppConfig`, `ContentHash`, `hash_content` |
| `error` | `AppError` (Io / Json / Watcher / NoVault / TaskNotFound / NotATaskLine) |

## Build Commands

```powershell
npm run tauri dev    # dev mode (Vite + cargo run)
npm run tauri build  # production bundle
```

## Key Notes

- `src-tauri/src/main.rs` calls `floaty_todo_lib::run()` — lib crate name is `floaty_todo_lib` (underscores, not hyphens)
- Dev URL is `http://localhost:1420` (configured in `tauri.conf.json`)
- `node_modules/` and `src-tauri/target/` are gitignored
