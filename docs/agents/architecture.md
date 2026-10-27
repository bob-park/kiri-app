---
title: Architecture
scope: "**"
applies_to: deciding where code belongs, tracing a request, touching app lifecycle, the engine or the CLI
related:
  - ./structure.md
  - ./conventions/rust-modules.md
  - ./conventions/tauri-commands.md
  - ./libs/react-query.md
---

# Architecture

> A Cargo workspace of three crates: `kiri-core` (Tauri-free engine), `kiri-cli` (the `kiri` binary) and `kiri-app` (`src-tauri`, the Tauri app). The React SPA (one bundle, one screen per window label) reaches Rust only through `call()` → `#[tauri::command]`; the CLI reaches the same `Engine` through a Unix socket. Front and back are split into **domains with the same name** on both sides.

## Workspace

| Crate | Path | Depends on | Holds |
| ----- | ---- | ---------- | ----- |
| `kiri-core` | `crates/kiri-core` | — (no `tauri`, no `rusqlite`) | `model` (Job, Preset, Tools), `engine` (queue + concurrent runs + cancellation), `pipeline` (yt-dlp → ffmpeg → move), `ytdlp`/`ffmpeg` (args and output parsers), `runner` (child processes, process-group kill), `files`, `tools` (yt-dlp/FFmpeg/Deno releases and installer), `ipc` (CLI socket protocol), `queue` (`QueueState`, the `QueueStore` trait) |
| `kiri-cli` | `crates/kiri-cli` | `kiri-core` | the `kiri` CLI: `status`, `list`, `add`, `stop`, `remove`, `transcode` over the socket |
| `kiri-app` (lib `kiri_lib`) | `src-tauri` | `kiri-core`, Tauri | app shell, domains (`settings`, `queue`, `tools`), SQLite |

The CLI and the app cannot merge: the CLI is a separate process that talks to the running app. `kiri-core` stays Tauri-free so both can use it.

### Sidecars

`tauri.conf.json` `bundle.externalBin` ships one binary next to the app executable (`app::sidecar(name)` resolves it):

- `kiri-cli` — `scripts/build-cli.sh` builds `kiri-cli` in release mode. Settings → CLI links it to `/usr/local/bin/kiri` (`app/cli_install.rs`).

`yarn sidecars` runs it into `src-tauri/binaries/` (git-ignored). Tauri's build checks that it exists, so run it before the first build.

yt-dlp, FFmpeg (ffmpeg + ffprobe) and Deno are **not** bundled. On first launch `install_tools` downloads whichever is missing (SHA256-verified) into `<app data>/bin/`. Startup never checks for updates; Settings → Updates checks (`check_tool_updates`) and updates (`update_tools`) on request. Sources: GitHub Releases (yt-dlp, Deno) and ffmpeg.martin-riedl.de (FFmpeg).

## Request paths

```
UI
  component
  → queries/<domain>.tsx        React Query hook (cache, loading, events)
  → apis/<domain>.ts            typed function per command
  → shared/api  call<T>()       the only `invoke` caller; rejects with AppError
  ⇢ Tauri IPC
  → domain/<name>/commands.rs   #[tauri::command], thin
  → domain/<name>/mod.rs        public domain API: validation + rules   (settings)
  → domain/<name>/repository.rs SQL for this domain's tables
  → shared/db.rs                Connection (opened + migrated at startup)
      or
  → kiri_core::engine::Engine   queue commands call the engine directly (queue)

CLI
  kiri <cmd>  (crates/kiri-cli)
  ⇢ Unix socket ~/Library/Application Support/org.bobpark.kiri/kiri.sock  (KIRI_SOCKET overrides)
  → app/ipc_server.rs           binds the socket, hands connections to kiri_core::ipc::serve
  → kiri_core::engine::Engine   the same Engine the UI uses
```

- One JSON request line and one JSON response line per connection (`kiri_core::ipc`).
- Every engine change calls `queue::commands::on_change`. It emits `queue://changed`, relabels the tray, and lets the updater run an install that waits for the queue.
- Persistence: the engine saves through `Arc<dyn QueueStore>`. The app passes `domain/queue/repository.rs::SqliteQueueStore` (rows in `jobs` + `meta.next_job_id`); `kiri-core`'s `JsonQueueStore` remains for tests and the one-time import.

Errors travel back as `AppError` → `{ code, message }` ([error-handling](./conventions/error-handling.md)).

## Rust layers (`src-tauri/src`)

| Layer | Path | May import `tauri`? |
| ----- | ---- | ------------------- |
| Assembly | `lib.rs` | yes — plugins, setup, `generate_handler!` only |
| App shell | `app/` (state, windows, tray + app menu, updater, quit, cli_install, ipc_server, i18n) | yes |
| Domain adapters | `domain/<name>/commands.rs` | yes |
| Domain core | `domain/<name>/{mod,model,repository}.rs` | **no** |
| Infrastructure | `shared/` (db, error, migrations) | **no** |
| Engine | `kiri_core::*` (separate crate) | **no** |

Dependencies point one way: `commands → domain core → shared`, and anything may use `kiri_core`. The app shell may call a domain's public API (e.g. `updater` reads `AppState::settings().update.auto_check`).

## State

- `AppState { db: Arc<Mutex<Connection>> }` (`app/state.rs`) — one connection, shared with `SqliteQueueStore`. `AppState::settings()` reads settings for shell code that can't surface an error.
- `Engine` (`kiri_core::engine`) — the queue, managed as Tauri state and shared with the IPC server.
- `ToolsState` (`domain/tools`) — yt-dlp/Deno readiness, in memory.
- `IpcState` (`app/ipc_server.rs`) — the socket bind error, if any, shown in Settings → CLI.
- `UpdateState` (`app/updater.rs`) — the found update and the downloaded bundle, in memory.
- Each **window is a separate JS context**: React Query caches and Zustand stores are per window. Cross-window changes are broadcast as Tauri events ([react-query](./libs/react-query.md#multiple-windows)).

## Storage

- `kiri.db` in `app_local_data_dir` (`~/Library/Application Support/org.bobpark.kiri/`). Schema in `shared/migrations/`, version in `PRAGMA user_version` ([sqlite-migrations](./conventions/sqlite-migrations.md)).
- Tables: `settings` (one row per section `general` / `download` / `update`, value = that section's JSON), `jobs` (one row per job, `data` = serde JSON of `kiri_core::model::Job`), `meta` (`next_job_id`).
- **One-time import:** at startup, `settings::import_json` and `queue::import_json` copy `settings.json` / `queue.json` (pre-SQLite versions) into an empty DB, then rename the file to `*.json.migrated`. A failed import keeps the file and starts from defaults.
- Outside the DB: tools in `<app data>/bin/`, per-job logs in `~/Library/Logs/org.bobpark.kiri/jobs/`, the CLI socket `kiri.sock`.

## Startup (`lib.rs` → `setup`)

1. Plugins: single-instance (must be first; a second launch calls `windows::show_main`), log, opener, dialog, clipboard-manager, updater.
2. Open + migrate `kiri.db`, import legacy JSON, then `manage` `AppState`, `IpcState`, `UpdateState`, `ToolsState`.
3. Build the `Engine` with `SqliteQueueStore`, tool paths (sidecars + `<app data>/bin`) and the settings-derived config; `manage` it.
4. Show the `main` window, build the tray and the macOS app menu.
5. Start the engine, the IPC server, the tools loop (yt-dlp/Deno ensured now and every hour; yt-dlp updated at most daily) and the periodic updater.

## Windows

Windows are built in code (`app/windows.rs`); `tauri.conf.json` lists none.

| Label | Screen | Size | Opened by |
| ----- | ------ | ---- | --------- |
| `main` | `src/app/App.tsx` | 760×680, min 600×560 | launch, tray double-click or "Open kiri", second launch |
| `settings` | `src/app/settings/SettingsApp.tsx` | 560×460, fixed | ⌘, in the main window (`open_settings`), settings links in the UI |

- Closing `main` hides it and drops the Dock icon when `general.close_to_tray` is on (the app lives in the menu bar); otherwise it quits.
- Quitting (tray "Quit", ⌘Q) asks first while downloads are running (`app/quit.rs`).
- The settings window title is localized (`i18n::Labels::settings_title`) and follows language changes. Re-showing an existing settings window emits `settings://opened`.

## Updater

- Checks GitHub Releases `latest.json` 10 s after launch and then every 24 h while `update.auto_check` is on (dev builds check too; there is no debug gate).
- Downloads in the background; installs only when the user chooses (Settings → Updates, the status bar, or the tray item). "Install after the queue" waits for running downloads.
- Status changes emit `update://status` and relabel the tray item.
