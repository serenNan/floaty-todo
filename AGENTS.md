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
  src/lib.rs      # Tauri commands + app setup
  src/main.rs     # Entry point (calls floaty_todo_lib::run())
  tauri.conf.json # App config (productName, identifier, devUrl)
  Cargo.toml      # Rust deps (crate name: floaty-todo, lib: floaty_todo_lib)
```

## Build Commands

```powershell
npm run tauri dev    # dev mode (Vite + cargo run)
npm run tauri build  # production bundle
```

## Key Notes

- `src-tauri/src/main.rs` calls `floaty_todo_lib::run()` — lib crate name is `floaty_todo_lib` (underscores, not hyphens)
- Dev URL is `http://localhost:1420` (configured in `tauri.conf.json`)
- `node_modules/` and `src-tauri/target/` are gitignored
