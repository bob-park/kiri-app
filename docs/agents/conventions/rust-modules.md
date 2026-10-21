---
title: Rust Modules & Layering
scope: crates/**, src-tauri/src/**
applies_to: adding or moving Rust code, deciding which crate or module owns logic
related:
  - ../architecture.md
  - ../structure.md
  - ./tauri-commands.md
  - ./error-handling.md
---

# Rust Modules & Layering

> Engine logic the CLI also needs lives in `crates/kiri-core` (no `tauri`, no `rusqlite`). In the app, `lib.rs` only assembles: shell code goes in `app/`, business logic in `domain/<name>/`, infrastructure in `shared/`. Domain core and `shared/` never import `tauri`, so they are unit-testable with an in-memory DB.

## Crate direction

```
kiri-cli ──→ kiri-core ←── kiri-app (src-tauri)
```

- `kiri-core` depends on neither other crate, nor on Tauri or SQLite. Storage reaches it through a trait (`queue::QueueStore`), implemented in the app by `domain/queue/repository.rs::SqliteQueueStore`.
- `kiri-cli` talks to the app only over the IPC socket (`kiri_core::ipc`); it never links the app.
- Add to `kiri-core` what both the app and the CLI need (job model, engine, pipeline, tool installers, the IPC protocol). Keep UI-only and Tauri-only concerns in `src-tauri`.

## Module direction (`src-tauri/src`)

```
lib.rs → app/*, domain/*/commands.rs → domain/*/{mod,model,repository}.rs → shared/*
                         (any layer may use kiri_core)
```

- `domain/<name>/{mod,model,repository}.rs` and `shared/**` **must not** `use tauri`.
  - Check with `grep -rn "tauri::\|use tauri" src-tauri/src/domain src-tauri/src/shared | grep -v commands.rs` and `grep -rn "tauri" crates/kiri-core/src`. Both outputs must be empty.
- A domain never calls another domain's `repository` directly. Use the other domain's `mod.rs` functions; they are its public API.
- `app/` may call a domain's public API (example: `app/updater.rs` reads `AppState::settings().update.auto_check`). Domains never call `app/`. The exception is `commands.rs`, which may trigger shell side effects such as relabelling the tray (`queue::commands::on_change`, `settings::commands::patch_and_notify`).

## Files in a domain

| File | Holds | Visibility |
| ---- | ----- | ---------- |
| `mod.rs` | public API: validation + rules, returns `AppResult<T>` | `pub fn` |
| `model.rs` | types, `serde` attributes, normalization helpers | re-exported from `mod.rs` |
| `repository.rs` | SQL for this domain's tables, takes `&Connection` (and the `QueueStore` impl for `queue`) | `pub` (used by its own `mod.rs` and tests) |
| `commands.rs` | `#[tauri::command]` adapters, the domain's event consts and background loops | `pub` (registered in `lib.rs`) |

- A domain without tables (`tools`) has only `mod.rs` + `commands.rs`. Don't create empty files.
- Add `service.rs` only when logic spans several repositories or domains. Don't scaffold it in advance.
- Split a file once it holds two unrelated responsibilities. Otherwise, prefer fewer files.

## Style

- Log with the `log` macros (`log::info!`, `log::warn!`), never `println!`/`eprintln!` in the app or `kiri-core`. `tauri-plugin-log` routes them to stdout and the log file. (The CLI prints to stdout/stderr — that is its output.)
- Never use `unwrap()` on I/O or DB results outside tests. Return `AppResult` and let the command surface the error. `Mutex::lock().unwrap()` is fine, since poisoning means a prior panic.
- A module opens with a `//!` doc line that says what it owns.
- Mark known shortcuts with `// ponytail:` plus their ceiling and the upgrade path. Example: `lib.rs` aborts setup on a DB open failure.
- Tests live in a `#[cfg(test)] mod tests` at the bottom of the file they test ([testing](../testing.md)); CLI end-to-end tests in `crates/kiri-cli/tests/`.
