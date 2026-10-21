---
title: Architecture
scope: "**"
applies_to: deciding where code belongs, tracing a request, touching app lifecycle
related:
  - ./structure.md
  - ./conventions/rust-modules.md
  - ./conventions/tauri-commands.md
  - ./libs/react-query.md
---

# Architecture

> One process, two halves: a React SPA (one bundle, one screen per window label) and Rust. The frontend reaches Rust only through `call()` → `#[tauri::command]`. Front and back are split into **domains with the same name** on both sides.

## Request path

```
component
  → queries/<domain>.tsx        React Query hook (cache, loading, invalidation)
  → apis/<domain>.ts            typed function per command
  → shared/api  call<T>()       the only `invoke` caller; rejects with AppError
  ⇢ IPC
  → domain/<name>/commands.rs   #[tauri::command], thin: lock state, call domain API
  → domain/<name>/mod.rs        public domain API: validation + rules
  → domain/<name>/repository.rs SQL for this domain's tables
  → shared/db.rs                Connection (opened + migrated at startup)
```

Errors travel back as `AppError` → `{ code, message }` ([error-handling](./conventions/error-handling.md)).

## Rust layers

| Layer | Path | May import `tauri`? |
| ----- | ---- | ------------------- |
| Assembly | `lib.rs` | yes — plugins, setup, `generate_handler!` only |
| App shell | `app/` (state, window, menu, tray, updater, autostart, i18n) | yes |
| Domain adapters | `domain/<name>/commands.rs` | yes |
| Domain core | `domain/<name>/{mod,model,repository}.rs` | **no** |
| Infrastructure | `shared/` (db, error, migrations) | **no** |

Dependencies point one way: `commands → domain core → shared`. The app shell may call a domain's public API (e.g. `updater` reads `settings::auto_update`).

## State

- `AppState { db: Mutex<Connection> }` (`app/state.rs`) — one connection, locked per command.
- `UpdateState` (`app/updater.rs`) — update status and the downloaded bundle, in memory.
- Each **window is a separate JS context**: React Query caches and Zustand stores are per window. Cross-window changes are broadcast as Tauri events ([react-query](./libs/react-query.md#multiple-windows)).

## Startup (`lib.rs` → `setup`)

1. Plugins: single-instance (must be first; a second launch calls `show_main`), log, autostart, updater.
2. Open + migrate `app.db` in the app data dir → `manage(AppState)`.
3. macOS app menu (needs `AppState` for its localized labels — keep it after `manage`).
4. Build windows. They are `create: false` in `tauri.conf.json` so no webview can call a command before state exists.
5. Tray, then the periodic updater thread.

## Windows

| Label | Screen | Opened by |
| ----- | ------ | --------- |
| `main` | `src/app/App.tsx` | launch, tray "Open", Dock click (`RunEvent::Reopen`), second launch |
| `settings` | `src/app/settings/SettingsApp.tsx` | `Cmd+,` app menu (macOS), `Ctrl+,` keydown (`useSettingsShortcut`), tray "Settings" |

Closing any window **hides** it (`window::hide_on_close`); the app lives in the tray until "Quit". `show_settings` emits `settings://opened` so the page resets to its first tab.

## Updater

- Release builds only: checks GitHub Releases `latest.json` at startup and every 6 h while `settings.auto_update` is on.
- Downloads in the background; installs only when the user picks "Restart to Update" (tray or Settings → Updates).
- Status changes emit `update://status` and rebuild the tray menu.
