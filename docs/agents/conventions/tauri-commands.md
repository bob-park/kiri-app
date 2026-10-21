---
title: Tauri Commands & Events
scope: src-tauri/src/**/commands.rs, src-tauri/src/app/**, src/domain/*/apis/**
applies_to: adding a command, calling Rust from the frontend, emitting or listening to an event
related:
  - ./rust-modules.md
  - ./error-handling.md
  - ../libs/react-query.md
---

# Tauri Commands & Events

> A command is a thin adapter: take state → call the domain API or the `Engine` → return `AppResult<T>`. Register it in `lib.rs`, mirror it with one function in `src/domain/<name>/apis/<name>.ts` that goes through `call()`. Events are named `<domain>://<event>`.

## Rust side

```rust
// src-tauri/src/domain/settings/commands.rs
#[tauri::command]
pub fn patch_settings(app: AppHandle, patch: Value) -> AppResult<Settings> {
    patch_and_notify(&app, &patch)
}

pub fn patch_and_notify(app: &AppHandle, patch: &Value) -> AppResult<Settings> {
    // The guard is a temporary: the DB lock is released before the side effects.
    let next = super::patch(&app.state::<AppState>().db.lock().unwrap(), patch)?; // validation in the domain API
    let _ = app.emit(SETTINGS_CHANGED, &next);                                     // tell every window
    // … engine config, tray/menu labels
    Ok(next)
}
```

- **Naming:** snake_case `verb_noun` (`get_settings`, `add_job`, `check_update`), unique across the whole app.
- **Logic:** none in the command. Validate and apply rules in the domain's `mod.rs` (unit-tested) or in `kiri_core::engine`.
- **Inputs:** take raw input (`Value`, `String`, primitives, serde DTOs like `NewJob`) and let the domain parse it. A bad value becomes `AppError::InvalidInput`. Never trust the webview.
- **Lock scope:** keep the `MutexGuard` inside one expression. Don't hold the DB lock across `await` or while calling code that locks again (`tray::relabel` reads settings).
- **Engine callbacks:** `queue::commands::on_change` runs inside the engine's change listener. Never call an `Engine` mutator from it (deadlock); only emit and relabel.
- **Registration:** add every command to `tauri::generate_handler![…]` in `lib.rs`. An unregistered command fails at runtime, not at compile time.
- **Async:** use an `async` command only for real async work (`probe`, `add_url`, `check_update`, `install_cli`).

## Frontend side

```ts
// src/domain/settings/apis/settings.ts
/** Rust merges the patch, saves it and emits `settings://changed` to every window. */
export async function patchSettings(patch: SettingsPatch) {
  return call<Settings>('patch_settings', { patch });
}
```

- `call()` (`src/shared/api/call.ts`) is the **only** place that calls `invoke`. It rejects with `AppError`.
- Argument keys are camelCase in TS; Tauri maps them to snake_case Rust params (`installUpdate(afterQueue)` → `after_queue`).
- DTO types in `apis/<name>.dto.ts` mirror the Rust `serde` output exactly (`rename_all`, tagged enums). The job model's serde form (`kiri_core::model`) is shared with the CLI's `--json` output.

## Events

| Event | Const | Payload | Emitted by | Consumed by |
| ----- | ----- | ------- | ---------- | ----------- |
| `settings://changed` | `SETTINGS_CHANGED` | `Settings` | `settings::commands::patch_and_notify` | `ThemeProvider` (writes the `['settings']` cache; `I18nProvider` and `useSettings()` follow) |
| `settings://opened` | `SETTINGS_OPENED` | — | `windows::show_settings` (re-showing an existing window) | nothing yet |
| `queue://changed` | `QUEUE_CHANGED` | `Job[]` (full list) | `queue::commands::on_change` (every engine change) | `useJobs` |
| `tools://changed` | `TOOLS_CHANGED` | `ToolsStatus` | `tools::commands` (check/install loop) | `useToolsStatus` |
| `update://status` | `UPDATE_STATUS` | `UpdateStatus` (`#[serde(tag = "kind")]`) | `app/updater.rs` | `useUpdateStatus` |

- Name events `<domain>://<event>` in kebab-case and export the name as a `const` in Rust, next to the code that emits it.
- Emit an event whenever a change made in one window (or by the CLI, or a background loop) must reach the UI. Each window has its own cache ([react-query](../libs/react-query.md#multiple-windows)).
- Listen with `listen()` in a `useEffect`, and unlisten in its cleanup (`void unlisten.then((fn) => fn())`). Keep listeners alive for the window's lifetime (e.g. `SettingsApp` reads queue/tools/update hooks at the top so tab switches don't drop events).

## Permissions

- Plugins used only from Rust (updater, log, single-instance) need no capability.
- A plugin called **from JS** needs its permission in `src-tauri/capabilities/default.json` (today: opener `reveal-item-in-dir`, dialog `open`, clipboard-manager `read-text`).
- A new window label must be added to that file's `windows` list.
