# 变更日志

## 2026-05-15 quadrant view design spec

- `docs/superpowers/specs/2026-05-15-quadrant-view-design.md` (new):
  brainstorming output for source-internal Eisenhower quadrant
  grouping. Quadrant is inferred from markdown header emoji
  (🔴 / 🟡 / 🟠 / 🟢) on any header level; child headers without
  emoji inherit parent quadrant. No sidecar, no task-row mutation —
  preserves line_number stability and the "markdown is source of
  truth" principle from PLAN.md. Awaiting writing-plans phase before
  implementation

## 2026-05-15 drag-reorder whole source via the `⋯` button (pointer events)

The right-most `⋯` button on each source header is now both the
**settings shortcut** (single click) and the **source drag handle**
(press-and-drag onto another source header to reorder). Inline source
editor inside `SourceGroup` is removed — editing now lives in the
existing Settings page.

HTML5 native drag-and-drop is unusable inside Tauri 2's WebView2:
`dragstart` fires but `dragover` is never delivered to the page, so
the cursor is stuck on "forbidden" for the entire drag. Switched to
pointer events instead, which work in any environment.

- `src-tauri/src/commands.rs` (+ `lib.rs` registration): new
  `reorder_sources(ordered_ids)` Tauri command. Rejects partial lists
  so a stale UI can't accidentally drop a source on reorder
- `src/composables/useSourceDrag.ts` (new): `startSourceDrag({ e,
  sourceId, onClick, onDrop })` — installs document-level
  pointermove/up listeners, 5 px threshold separates click from drag,
  target resolved via `elementFromPoint` walking up to a
  `[data-source-id]` ancestor
- `src/components/SourceGroup.vue`: dots button uses `@pointerdown` →
  `startSourceDrag(...)`; header carries `data-source-id`. Inline
  source editor + related state (`editing`/`labelDraft`/`rootDraft`
  + handlers + dead CSS) deleted; source editing flows through the
  Settings page only
- `src/components/TaskList.vue`: forwards `SourceGroup`'s new
  `open-settings` event up to the App-level view switcher
- `src/services/tauri-api.ts` + `src/stores/settings.ts`: wrappers
  for the new `reorder_sources` command (`api.reorderSources` /
  `settings.reorderSources`)

## 2026-05-15 reveal-action icon: chunky filled folder (was outline + magnifier)

The outlined folder + magnifier overlay looked busy at 14 px and read
more like "search" than "open in file manager". Replace with a single
filled folder with a soft white top-edge highlight — cleaner silhouette,
keeps the existing yellow brand colour, and the action is now visually
honest ("just open the folder").

- `src/components/icons/QuickActionIcon.vue`: rewrite the `reveal` SVG —
  one fill path for the folder body + a 35%-opacity white stroke line
  just under the rim for the fold-edge cue; dropped the magnifier
  `<circle>` and `<line>` elements

## 2026-05-15 default window width 380 → 760, project gets its own TODO.md

- `src-tauri/tauri.conf.json`: window initial width 380 → 760 (height
  unchanged at 600); `minWidth` stays 320 so the user can still shrink
- `TODO.md` (new): project's own backlog in the same `- [ ]` format
  Floaty Todo itself parses, so adding the project as a File source
  immediately makes the app eat its own dog food. Groups items by
  priority (now / v0.3 / v1.0 / v2.0), known small papercuts, and the
  things we've decided not to do (PLAN.md non-goals echoed here)

## 2026-05-15 collapse-all, full-header click, drag-to-reorder actions, reveal-in-explorer

Four UX rough-edges polished in one pass:

1. **Collapse / expand all** — new footer button (next to Settings) flips
   every source + every file group at once via a global trigger.
2. **Whole header clicks toggle** — used to require hitting the tiny
   chevron; now anywhere on a source or file header works. Action
   chips stop propagation so clicking VS Code doesn't also collapse.
3. **Drag-to-reorder quick-action buttons** — grabbing any chip on a
   source header and dropping it on another reorders
   `enabled_quick_actions` globally (same list drives every source).
4. **"Reveal in file manager"** — new `QuickActionKind::Reveal` opens
   the source's path in Explorer / Finder / xdg-open; a matching
   yellow folder button shows up on every source header and in the
   footer's hub cluster. Default `enabled_quick_actions` now ships
   `reveal + vscode + terminal` so the file-manager affordance is
   discoverable out of the box.

### Backend
- `types.rs`: `QuickActionKind::Reveal` variant; `default_quick_actions()`
  now `[Reveal, Vscode, Terminal]`
- `shell.rs::reveal_in_explorer(path)` — Windows `explorer.exe` (with
  `/select,<file>` to highlight a file in its parent dir), macOS
  `open -R` for files / `open` for folders, Linux `xdg-open` on the
  containing directory
- `commands.rs`: new `reveal_source(source_id)`; `run_quick_action` and
  `open_hub` dispatch `Reveal` to `reveal_in_explorer`
- `lib.rs`: registers `reveal_source`

### Frontend
- `src/composables/useCollapse.ts` (new): `collapseAll()` /
  `expandAll()` increment counter refs; `bindCollapse(setter)` wires a
  component's local `collapsed` ref to both tokens via `watch`
- `src/components/icons/Icon.vue`: new `collapse-all` and `expand-all`
  glyphs (chevrons + centre line)
- `src/components/icons/QuickActionIcon.vue`: new `reveal` SVG —
  open-folder with a magnifier inside; yellow `#f59e0b` brand colour
- `src/components/TaskList.vue`: footer `collapse/expand-all` toggle
  next to Settings; hub-reveal button (when hub configured) in the
  brand-coloured right-hand cluster ahead of hub-vscode + hub-claude
- `src/components/SourceGroup.vue`:
  - header click toggles `collapsed` (caret is now a decorative span);
    actions cluster + edit panel use `@click.stop` to avoid
    bubbling into the toggle
  - subscribes to `bindCollapse`
  - each `.icon-btn.brand` is `draggable="true"`; dragstart/over/leave/
    drop/end track `dragKind` + `dropTargetKind`; on drop we splice the
    enabled list and call `setEnabledQuickActions`; CSS gives the
    grabbed chip a 0.4 opacity and the drop target an accent outline
- `src/components/FileGroup.vue`: same header-click + bindCollapse
  treatment; editor uses `@click.stop`
- `src/views/SettingsView.vue`: `ALL_QUICK_ACTIONS` lists `reveal` at
  the top; per-card actions get a reveal button; `revealSource(s)`
  calls `runQuickAction('reveal')`
- `src/i18n/locales/{en,zh}.ts`: `source.reveal`,
  `settings.sources.reveal`, `hub.reveal`, `tasks.collapseAll` /
  `expandAll` strings

## 2026-05-14 hub shortcuts in TaskList footer (VS Code / Claude Code)

When a hub folder is configured, the footer now shows two brand-coloured
buttons that open the hub folder directly — VS Code and a fresh Claude
Code session. They sit between the counts and the pin button, on the
same right-hand cluster as the existing per-source quick-action chips.

- `src-tauri/src/commands.rs`: new `open_hub(kind: QuickActionKind)` —
  reads `hub_folder` from state, dispatches to the matching
  `shell::open_*`; errors out if the hub isn't configured
- `src-tauri/src/lib.rs`: registers the command
- `src/services/tauri-api.ts`: `openHub(kind)`
- `src/components/TaskList.vue`: two new footer buttons gated on
  `settings.hubFolder`, rendered via `<QuickActionIcon>` for visual
  parity with per-source action chips; `.footer-btn.brand` modifier
  reuses the `color-mix(currentColor …)` hover ring from `SourceGroup`
- `src/i18n/locales/{en,zh}.ts`: `hub.openVscode` / `hub.openClaudeCode`
  tooltip strings

## 2026-05-14 source kind icons → real emoji that flip on expand/collapse

Replace the outlined folder / file SVGs on each source header with real
colour emoji, and make folder + file glyphs reflect the disclosure state
so the icon doubles as a visual cue.

- `src/components/SourceGroup.vue`: new `kindEmoji` computed —
  folder: 📁 collapsed / 📂 expanded, file: 📄 collapsed / 📝 expanded;
  template renders a `<span class="kind-icon">` and CSS pins Segoe UI
  Emoji / Apple Color Emoji / Noto Color Emoji so the glyph renders in
  colour
- `src/views/SettingsView.vue`: source-card `src-icon` matches the new
  style (no expand/collapse state here, so just 📁 / 📄)

## 2026-05-14 add-row "+" now adds a source (was: add task)

Visually the "+" sits right next to the source dropdown, so users
expect it to add a *source*. Reading the textbox + Enter is enough to
add a task. Repurpose the button accordingly.

- `src/components/TaskList.vue`: replace the form's submit button with a
  detached `type="button"` "+" wrapped in `.add-source-wrap`; clicking
  pops a small drop-down menu (Folder / File) anchored under the button
- click-outside + Esc close the menu; pop-in animation matches the
  ConfirmDialog
- Adding tasks: just press Enter in the input (placeholder now says
  "…(Enter)" / "…(回车)" so the affordance is discoverable)
- i18n: `tasks.addSourceTitle` (title for the new button); updated
  `tasks.addPlaceholder` with the Enter hint

## 2026-05-14 pin button now uses the U+1F4CC pushpin emoji

The cartoon-SVG pin didn't read as a thumbtack at 14px on Windows
(stroke + ellipse looked more like a balloon). Replace it with the real
📌 emoji — Segoe UI Emoji renders it in proper colour, and the off state
desaturates via CSS `filter: grayscale(0.85) opacity(0.55)`. Active state
adds a small `rotate(-12deg)` so the difference is obvious at a glance.

- `src/components/TaskList.vue`: pin button renders a `<span class="pin-emoji">📌</span>` instead of `<Icon name="pin" />`; new `.pin-emoji` rules force the colour-emoji font and animate filter/transform on toggle
- `src/components/icons/Icon.vue`: dropped the now-unused `pin` and `pin-off` cases from the union and template — keeps the central icon library tight

## 2026-05-14 hub folder — mirror every source via hard links / junctions

Adds an opt-in "hub folder" that mirrors every configured source via
OS-level filesystem links. AI tools and shell scripts can drive every
project's TODO from one place instead of crawling each repo. Two-way
sync is instant because there's no copy — both ends are literally the
same inode (file source → hard link) or the same directory (folder
source → NTFS junction / POSIX symlink).

### Backend
- `src-tauri/src/hub.rs` (new): pure filesystem module — `mirror_path_for`
  derives the hub-side name from the source's sanitised label,
  `create_mirror` / `remove_mirror` / `sync_all` manage individual
  entries and prune orphans on a full resync; cross-platform via
  `std::fs::hard_link` for files and `cmd mklink /J` on Windows /
  POSIX symlink elsewhere for folders
- `types.rs`: `AppConfig` gains `hub_folder: Option<PathBuf>` with
  `#[serde(default)]` so existing configs migrate transparently
- `commands.rs`: new `set_hub_folder(path?)` (full resync on change)
  and `resync_hub()` (manual repair); `add_source` / `remove_source` /
  `update_source` now call into `hub` after their main effect via a
  swallow-errors helper, so a junction-failure can't block source CRUD
- `lib.rs`: registers both commands; declares `mod hub`
- `config.rs`: test updated for the new field
- 44 unit tests pass (added 5 hub tests: name sanitisation, mirror path
  derivation, hard-link create + edit-through, idempotent re-create,
  remove, orphan-pruning sync_all)

### Frontend
- `types/task.ts`: `AppConfig.hub_folder: string | null`
- `services/tauri-api.ts`: `setHubFolder(path?)`, `resyncHub()`
- `stores/settings.ts`: `hubFolder` computed; `setHubFolder` /
  `resyncHub` / `pickAndSetHubFolder` helpers
- `views/SettingsView.vue`: new "Hub folder" section between Quick
  actions and Sources — shows the configured path with `Resync` /
  `Change` / `Disable` buttons, or a single "Choose folder…" CTA when
  unset; surface errors inline
- `i18n/locales/{en,zh}.ts`: `settings.sections.hub` + `settings.hub.*`
  strings

### Trade-offs
- Same-volume only (hard links + NTFS junctions can't cross volumes).
  Cross-volume sources fail their mirror with an actionable message; the
  source itself is added regardless.
- Hub-side label collisions are an open edge case — for now whichever
  source mirrors first wins the name; future work can disambiguate
  with `(source_id)` suffix.

## 2026-05-14 pin icon now classic drawing-pin red, not muted

The pin icon used to inherit the footer's muted text colour even when
pinned, so the active state was barely visible. Now uses #ef4444 across
both themes — saturated thumb-tack red when pinned, half-red half-muted
when floating so the toggle reads clearly without losing the affordance.

- `src/components/TaskList.vue`: `.pin-btn` colour rules — `.active`
  uses #ef4444 for icon / border / soft background, hover deepens to
  ~22% mix; `:not(.active)` uses a 55%-mixed red-ish muted, hover snaps
  to full red

## 2026-05-14 unified cartoon SVG icon library — replaces every ASCII / emoji glyph

Every icon-style button across the app now renders through a single
`<Icon name="…" />` component instead of scattering ⚙ / ↻ / ▾ / 📁 /
📄 / 🗑 / 📝 / ✎ / ↺ / ← / ⟳ / ☀ / 🌙 / 🖥 literals through templates.
Style is intentionally chunky and friendly (Lucide-inspired outlines,
1.9 px stroke, rounded caps) so it reads as a coherent set even at 12 px.

- `src/components/icons/Icon.vue` (new): central component;
  `name: IconName` (literal string union) + `size`; 20 icons —
  `pin` / `pin-off` / `settings` / `refresh` / `plus` / `chevron-down` /
  `chevron-right` / `more-horizontal` / `pencil` / `rotate-ccw` /
  `folder` / `file` / `trash` / `sun` / `moon` / `monitor` /
  `arrow-left` / `loader` / `check` / `x`; chunkier pin variants
  (filled tilted thumbtack with highlight when pinned, outlined and
  more tilted when floating)
- `src/components/TaskList.vue`: replaces `⚙ ↻ +` and the old inline
  pin SVG; new shared `.icon-only` footer-button modifier flexbox-centres
  the icon at 28×26 px
- `src/components/SourceGroup.vue`: caret (`▾/▸`), kind icon (`📁/📄`),
  scanning spinner (`⟳`), more-horizontal `⋯`, and the folder-picker in
  the inline editor all switch to `<Icon>`; brand quick-action buttons
  keep their `<QuickActionIcon>`
- `src/components/FileGroup.vue`: caret + pencil-rename + rotate-ccw
  reset all use `<Icon>`
- `src/views/SettingsView.vue`: back arrow, theme segmented (sun / moon /
  monitor), source toolbar (folder / file with text labels), per-card
  source actions (`QuickActionIcon` for vscode/terminal, `Icon` for
  pencil / trash), inline editor folder-picker
- `src/components/EmptyState.vue`: folder / file picker buttons + corner
  settings cog all use `<Icon>`
- `src/i18n/locales/{en,zh}.ts`: stripped emoji prefixes from `empty.add*`
  and `settings.sources.add*` strings — the icon component supplies the
  visual now, the text is just the label

## 2026-05-14 pin / unpin always-on-top toggle in TaskList footer

The app shipped with always-on-top permanently on. Users who want to
focus on another window without dragging Floaty out of the way can now
toggle it from the footer.

- `src-tauri/src/commands.rs`: new `set_always_on_top(on: bool)`
  command — writes the flag back to config + calls
  `window.set_always_on_top(on)` so the change takes effect immediately
- `src-tauri/src/lib.rs`: setup now `set_always_on_top(cfg.always_on_top)`
  on the main window after load, so the persisted flag wins over the
  initial `tauri.conf.json` value on every launch
- `src/services/tauri-api.ts`: `setAlwaysOnTop(on)`
- `src/stores/settings.ts`: `alwaysOnTop` computed + `setAlwaysOnTop` /
  `toggleAlwaysOnTop` helpers
- `src/components/TaskList.vue`: footer drops in a 28px pin button
  between the counts and the ↻ refresh; inline SVG drawing-pin with two
  states — filled accent colour when pinned, outlined-and-tilted when
  floating
- `src/i18n/locales/{en,zh}.ts`: `window.pin` / `window.unpin` titles

## 2026-05-14 brand-coloured SVG icons for quick actions

Replaces the placeholder Unicode glyphs (⎘ / ▷ / ◆) on quick-action
buttons with real brand-coloured inline SVGs so the user can recognise
each action at a glance.

- `src/components/icons/QuickActionIcon.vue` (new): single Vue
  component, `kind` prop, three inline SVGs sourced from simple-icons
  (CC0) — VS Code folded-V mark (#0098FF), generic terminal window with
  `>_` prompt (#4DAA7F), Anthropic eight-point sparkle (#D97757);
  dark-mode CSS overrides nudge each colour slightly brighter for the
  glass surface
- `src/components/SourceGroup.vue`: renders `<QuickActionIcon>` in each
  source-header button (was: hard-coded glyph strings); buttons gain
  `.brand` modifier — hover ring uses the current icon's colour via
  `color-mix(currentColor 10% / 30%)` so the icon stays legible
- `src/views/SettingsView.vue`: same swap in the Quick actions section
  so the checkbox list shows the real brand icons next to each label
- No new runtime dependency — all SVGs are inline, zero network calls

## 2026-05-14 configurable quick actions (+ Claude Code) + responsive scan UX

Two related polish items:

1. **Quick-action buttons on each source header are now user-configurable.**
   The built-in set is VS Code / Terminal / Claude Code; the user toggles
   which appear in Settings → Quick actions. Order in the saved list is
   the display order on every source.
2. **Adding a big folder source no longer freezes the UI.** Scan runs in
   a background thread (already true) and now emits `source-scan-started` /
   `source-scan-finished` events; the matching source shows a spinning ⟳
   badge in its header and a "scanning…" line in its body, and FileGroups
   inside any source with > 50 tasks render collapsed by default so the
   DOM stays tiny until the user expands a file. Single-file sources also
   stopped nesting a redundant FileGroup wrapper — the source header *is*
   the file header.

### Backend
- `types.rs`: new `QuickActionKind` enum (`Vscode` / `Terminal` /
  `ClaudeCode`); `AppConfig` gains `enabled_quick_actions: Vec<…>` with
  `default_quick_actions()` (VS Code + Terminal) used by `#[serde(default …)]`
- `shell.rs`: new `open_claude_code(path)` — Windows cascade
  `wt.exe -d <p> -- cmd /k claude.cmd` → bare `wt.exe -d <p>` → `cmd /c start cmd /k claude.cmd`;
  macOS via `osascript` → Terminal.app; Linux via the same terminal
  emulator cascade as `open_terminal` with `-e claude`
- `commands.rs`: `open_in_claude_code(source_id)`,
  `run_quick_action(source_id, kind)` (dynamic dispatch on
  `QuickActionKind`), `set_enabled_quick_actions(actions)`; all three
  wired into `invoke_handler!`
- `lib.rs::spawn_source_scan_and_watcher`: emits `source-scan-started`
  before the `rebuild_source` call and `source-scan-finished` after,
  payload = source id
- `config.rs`: legacy `load_strips_verbatim_prefix_and_remaps_default_id`
  test updated for the new `enabled_quick_actions` field
- 38 unit tests pass

### Frontend
- `types/task.ts`: `QuickActionKind = 'vscode' | 'terminal' | 'claude_code'`;
  `AppConfig.enabled_quick_actions: QuickActionKind[]`
- `services/tauri-api.ts`: `openInClaudeCode`, `runQuickAction`,
  `setEnabledQuickActions`, `onSourceScanStarted` / `onSourceScanFinished`
- `stores/settings.ts`: `enabledQuickActions` computed,
  `scanningSourceIds: ref<Set<string>>`, `isScanning` computed,
  `markScanning(id, on)` helper, `setEnabledQuickActions`; `addSource`
  marks scanning immediately to defeat event race
- `App.vue`: subscribes to scan-started / scan-finished and forwards to
  `settings.markScanning`
- `components/SourceGroup.vue`:
  - renders one icon button per entry in `settings.enabledQuickActions`,
    dispatched through `api.runQuickAction`
  - shows spinning ⟳ + "Scanning files…" while scan is in flight
  - `BIG_SOURCE_TASK_THRESHOLD = 50` — sets `initial-collapsed` on every
    FileGroup so big sources don't render thousands of TaskItem DOM
    nodes on first paint
  - File-kind sources skip the FileGroup wrapper and render TaskItems
    directly (was nesting a single redundant group)
- `components/FileGroup.vue`: new `initial-collapsed` prop, defaults
  false; consumed by SourceGroup for the big-source optimisation
- `views/SettingsView.vue`: new "Quick actions" section — one checkbox
  row per kind; toggling rewrites `enabled_quick_actions`
- `i18n/locales/{en,zh}.ts`: `source.openClaudeCode`, `scanning`,
  `scanningHint`, `settings.sections.quickActions`,
  `settings.quickActions.hint`

## 2026-05-14 inline markdown rendering + in-app confirm + smarter default labels

Three small UX polish items reported together:

1. **Task text renders inline markdown** — `**bold**`, `*italic*`,
   `` `code` ``, `~~strike~~`, and `[text](url)` are rendered as proper
   elements instead of dumped as plain text. Links open via the OS default
   handler.
2. **Source delete now shows an in-app modal confirm** — replaces native
   `window.confirm()` (which clashed with the floaty-window aesthetic and
   was easy to miss). Single global `<ConfirmDialog>` mounted at the App
   root, driven by a `confirm()` promise from `composables/useConfirm.ts`.
3. **`add_source` infers a sensible default label** — uses the folder name
   of the source's effective `project_root` (so a File source at
   `D:\Projects\WishTalk\Todo.md` lands with label `WishTalk`, matching
   where "Open in VS Code" / terminal will jump to).

Also fixed the long-standing `default (WishTal…` truncation in the
QuickAdd source dropdown.

### Backend
- `commands.rs::add_source`: resolves the default label from
  `project_root`'s `file_name()` (Folder → folder name; File → parent
  folder name); user-supplied non-empty label still wins
- `shell.rs::open_url(url)`: cross-platform default-handler launcher
  (Windows `cmd /c start "" <url>`, macOS `open <url>`, Linux `xdg-open`)
  with control-character defence
- `commands.rs::open_url(url)` + `lib.rs` `invoke_handler` registration

### Frontend
- `src/utils/inline-md.ts` (new): zero-dep inline parser → segment array
  (`text` / `code` / `bold` / `italic` / `strike` / `link`); no `v-html`,
  XSS-safe by construction
- `src/components/TaskItem.vue`: maps segments to `<code>` / `<strong>` /
  `<em>` / `<s>` / `<a>`; link click invokes `api.openUrl`
- `src/composables/useConfirm.ts` (new): `confirm({ title, message,
  confirmText, cancelText, danger }) → Promise<boolean>`; singleton state
- `src/components/ConfirmDialog.vue` (new): Teleport-mounted modal with
  backdrop click / Esc to cancel, focus-trap on confirm button, danger
  variant for destructive actions; pop animation
- `src/App.vue`: mounts `<ConfirmDialog />` at the root
- `src/components/SourceGroup.vue` + `src/views/SettingsView.vue`:
  replace `window.confirm` with the new `confirm()` API
- `src/services/tauri-api.ts`: `openUrl(url)`
- `src/components/TaskList.vue`: `source-select` CSS — drop the 95px cap;
  cap at 45% of the row width and rely on the select control's native
  sizing so long labels like `★ WishTalk` are no longer clipped
- `src/i18n/locales/{en,zh}.ts`: `targetDefault` short-form (`★ {label}`),
  shorter QuickAdd placeholder, `confirm.*` strings (title / ok / cancel /
  removeSource{Title,Message,Confirm})

## 2026-05-14 per-file nested groups inside each source, with renameable labels

Folder sources now split their tasks into one collapsible group per `.md`
file, so a vault with many notes no longer dumps everything into one giant
list. Each file group can be expanded / collapsed independently, and the
user can give it a custom display name to avoid the "five `todo.md` files
all look the same" problem.

### Backend
- `types.rs`: `AppConfig` gains `file_labels: HashMap<String, String>` —
  keyed by canonical / dunce-simplified absolute path; `file_label_key()`
  helper centralises the key derivation
- `commands.rs`: new `set_file_label(file_path, label)` command —
  `None` or empty-after-trim clears the override
- `lib.rs`: registers `set_file_label` in `invoke_handler!`
- `config.rs`: existing `load_strips_verbatim_prefix_and_remaps_default_id`
  test updated for the new `file_labels` field
- All 38 unit tests pass

### Frontend
- `src/types/task.ts`: `AppConfig.file_labels: Record<string, string>`
- `src/services/tauri-api.ts`: new `setFileLabel(filePath, label)`
- `src/stores/settings.ts`: exposes `fileLabels` / `fileLabel(path)` /
  `setFileLabel(path, label)`
- `src/components/FileGroup.vue` (new): per-file row with caret toggle,
  hover-revealed ✎ rename button, inline rename input (Enter to save,
  Esc to cancel, ↺ to reset to default name), and the task list
- `src/components/SourceGroup.vue`: tasks are now bucketed by
  `task.source_file` and each bucket renders as a `FileGroup`; File-kind
  sources still render as a single group; ordering is stable by file path
- `src/i18n/locales/{en,zh}.ts`: added `file.editLabel` / `file.resetLabel` /
  `file.noTasks`

## 2026-05-14 strip Windows verbatim path prefixes (\\?\) — friendly prompts

Rust's `std::fs::canonicalize` returns `\\?\D:\...` on Windows. When that
path landed in `Source.path` and we passed it to `pwsh -WorkingDirectory`,
the PowerShell prompt rendered as `PS Microsoft.PowerShell.Core\FileSystem::\\?\D:\Projects\WishTalk>`
instead of `PS D:\Projects\WishTalk>`. Same prefix would creep into
VS Code title bars and the SettingsView source-path display.

- `Cargo.toml`: + `dunce = "1"`
- `types.rs`: `Source::id_for` now feeds `dunce::simplified()` into the
  hash so verbatim and friendly forms of the same path agree on an id
- `registry.rs`: `best_effort_canonical` switches to `dunce::canonicalize`
  (with `dunce::simplified` as the final fallback when the file is gone)
- `parser.rs`: `parse_file` uses `dunce::canonicalize` for `Task.source_file`
- `commands.rs`: `add_source` canonicalises with `dunce` before hashing /
  persisting
- `config.rs`: `load_from` runs `normalize_paths` — idempotent migration
  that strips `\\?\` from every `source.path` / `project_root`, recomputes
  ids on the cleaned paths, and remaps `default_source_id` if the
  underlying source id changed; new unit test
  `load_strips_verbatim_prefix_and_remaps_default_id` covers the migration
- `lib.rs`: setup `save_to`'s the cleaned config back after load so the
  on-disk JSON also gets normalised on first launch after this upgrade
- 38 unit tests pass (was 35; added the migration test + two shell tests
  from earlier)

## 2026-05-14 dedicated Settings page (theme / language / sources) + i18n (en/zh)

Centralised settings page replaces the floating theme button. The bottom-left
⚙ button in TaskList (and the corner button in EmptyState) opens a full-screen
Settings view where the user manages theme, display language, and the source
list (with prominent per-row delete). All UI strings now route through vue-i18n
so 中文 and English both ship in the binary.

- `package.json`: + `vue-i18n@^11`
- `src/i18n/index.ts` (new): `createI18n` with composition API; auto-detects
  initial locale from `localStorage['floaty.locale']` → `navigator.language`
  → `'en'`; exported `setLocale(locale)` persists choice and updates
  `document.documentElement.lang`
- `src/i18n/locales/en.ts` and `src/i18n/locales/zh.ts`: every UI string
  (empty / tasks / source / settings / errors)
- `src/main.ts`: `app.use(i18n)`
- `src/views/SettingsView.vue` (new): four sections — Appearance (theme
  segmented control), Language (locale select), Sources (toolbar + card list
  with ⎘ / ▷ / 📝 / 🗑 per row, inline editor for label / project_root /
  set-default), About; back button returns to task view
- `src/App.vue`: introduces `view: 'tasks' | 'settings'` state; mounts
  `useTheme()` at the root so the system-pref watcher lives for the whole
  app; removes the floating theme button; tray "Manage sources…" event now
  opens the Settings view
- `src/components/TaskList.vue`: bottom-left ⚙ settings button (replaces the
  inline 📁+/📄+ chips — adding new sources now lives in Settings); emits
  `openSettings`; all strings via `t()`
- `src/components/EmptyState.vue`: bottom-left corner ⚙ button so a user
  with no sources can still reach Language / Theme; strings via `t()`
- `src/components/SourceGroup.vue`: strings via `t()` (in-card edit panel
  kept for fast access while browsing tasks)

## 2026-05-14 source-grouped UI with per-source actions and inline editor

Tasks now render grouped by source instead of one flat list, with each
source header carrying the new shell-action buttons and an inline edit
panel for label / project_root / set-default / remove.

- `src/components/SourceGroup.vue`: new — collapsible header (caret +
  kind icon + label + default badge + per-source todo/done counts), three
  icon buttons (⎘ open-in-VS-Code · ▷ open-in-terminal · ⋯ edit), and
  inline editor with Label / Project root (+ folder picker) / Set-default
  / Remove (with confirm) / Save · Cancel
- `src/components/TaskList.vue`: rewritten to render `SourceGroup` for
  each source in user-defined order; QuickAdd input gains a target-source
  dropdown (`default (foo)` plus an entry per source) and the placeholder
  reflects where the new task will land; footer collapses to totals +
  `📁+ / 📄+ / ↻` chips
- Sources with zero tasks now show "No tasks in this source." so they
  stay visible as the launchpad for VS Code / terminal actions


Each `Source` now exposes two side-effect commands that launch external tools
at its `effective_project_root()` (the configured `project_root`, or default
to `path` for Folder sources / `path.parent()` for File sources).

- `src-tauri/src/shell.rs`: new module — `open_vscode(path)` and
  `open_terminal(path)`; cross-platform terminal cascade tries
  Windows Terminal → pwsh.exe → powershell.exe on Windows, `open -a Terminal`
  on macOS, and `x-terminal-emulator` / `gnome-terminal` / `konsole` / `xterm`
  on Linux; first successful spawn wins, all-fail surfaces as
  `AppError::CommandFailed` with the attempted-binary list (so the UI can
  prompt the user to install `code` / set up `wt`)
- `src-tauri/src/lib.rs`: `mod shell;`; registered the two commands in
  `invoke_handler!`
- `src-tauri/src/commands.rs`: new `open_in_vscode(source_id)` and
  `open_in_terminal(source_id)` — both resolve the source via
  `find_source_by_id`, then call into `shell`
- `src-tauri/src/types.rs`: dropped unused `effective_label` (frontend does
  the label-fallback)
- `src/services/tauri-api.ts`: added `openInVscode(sourceId)` and
  `openInTerminal(sourceId)`
- 2 new unit tests on the platform-attempt cascade

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
