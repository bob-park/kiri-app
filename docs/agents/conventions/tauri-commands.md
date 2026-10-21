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

> A command is a thin adapter: lock state → call the domain API → return `AppResult<T>`. Register it in `lib.rs`, mirror it with one function in `src/domain/<name>/apis/<name>.ts` that goes through `call()`. Events are named `<domain>://<event>`.

## Rust side

```rust
#[tauri::command]
pub fn set_theme(app: AppHandle, state: State<AppState>, theme: String) -> AppResult<Theme> {
    let theme = super::set_theme(&state.db.lock().unwrap(), &theme)?; // validation lives in the domain API
    let _ = app.emit(THEME_CHANGED, theme);                            // tell every window
    Ok(theme)
}
```

- **Naming:** snake_case `verb_noun` (`get_theme`, `set_locale`, `check_update`), unique across the whole app.
- **Logic:** none in the command. Validate and apply rules in the domain's `mod.rs`, which is unit-tested.
- **Inputs:** take raw input (`String`, primitives) and let the domain parse it. A bad value becomes `AppError::InvalidInput`. Never trust the webview.
- **Lock scope:** keep the `MutexGuard` inside one expression. Don't hold the DB lock across `await` or while calling code that locks again (`tray::refresh` reads the DB).
- **Registration:** add every command to `tauri::generate_handler![…]` in `lib.rs`. An unregistered command fails at runtime, not at compile time.
- **Async:** use an `async` command only for real async work (e.g. `updater::check_update`).

## Frontend side

```ts
// src/domain/settings/apis/settings.ts
export async function setTheme(theme: ThemePref) {
  return call<ThemePref>('set_theme', { theme });
}
```

- `call()` (`src/shared/api`) is the **only** place that calls `invoke`. It rejects with `AppError`.
- Argument keys are camelCase in TS; Tauri maps them to snake_case Rust params.
- DTO types in `apis/<name>.dto.ts` mirror the Rust `serde` output exactly (`rename_all`, tagged enums).

## Events

| Event | Payload | Emitted by | Consumed by |
| ----- | ------- | ---------- | ----------- |
| `settings://theme-changed` (`THEME_CHANGED`) | `ThemePref` | `set_theme` | `ThemeProvider` (every window) |
| `settings://locale-changed` (`LOCALE_CHANGED`) | `LocalePref` | `set_locale` | `I18nProvider` (every window) |
| `settings://opened` (`SETTINGS_OPENED`) | — | `window::show_settings` | `SettingsApp` (reset to first tab) |
| `update://status` (`UPDATE_STATUS`) | `UpdateStatus` | `updater::set_status` | `useUpdateStatus` |

- Name events `<domain>://<event>` in kebab-case and export the name as a `const` in Rust.
- Emit an event whenever a change made in one window must reach others. Each window has its own cache ([react-query](../libs/react-query.md#multiple-windows)).
- Listen with `listen()` in a `useEffect`, and unlisten in its cleanup (`void unlisten.then((fn) => fn())`).

## Permissions

- Plugins used only from Rust (autostart, updater) need no capability.
- A plugin called **from JS** needs its permission in `src-tauri/capabilities/default.json`.
- A new window label must be added to that file's `windows` list.
