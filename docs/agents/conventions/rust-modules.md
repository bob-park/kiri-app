---
title: Rust Modules & Layering
scope: src-tauri/src/**
applies_to: adding or moving Rust code, deciding which module owns logic
related:
  - ../architecture.md
  - ../structure.md
  - ./tauri-commands.md
  - ./error-handling.md
---

# Rust Modules & Layering

> `lib.rs` only assembles. Shell code goes in `app/`, business logic in `domain/<name>/`, infrastructure in `shared/`. Domain core and `shared/` never import `tauri`, so they are unit-testable with an in-memory DB.

## Dependency direction

```
lib.rs → app/*, domain/*/commands.rs → domain/*/{mod,model,repository}.rs → shared/*
```

- `domain/<name>/{mod,model,repository}.rs` and `shared/**` **must not** `use tauri`.
  - Check with `grep -rn "tauri::\|use tauri" src-tauri/src/domain src-tauri/src/shared | grep -v commands.rs`. The output must be empty.
- A domain never calls another domain's `repository` directly. Use the other domain's `mod.rs` functions; they are its public API.
- `app/` may call a domain's public API (example: `app/updater.rs` reads `settings::auto_update`). Domains never call `app/`. The exception is `commands.rs`, which may trigger shell side effects such as refreshing the tray.

## Files in a domain

| File | Holds | Visibility |
| ---- | ----- | ---------- |
| `mod.rs` | public API: validation + rules, returns `AppResult<T>` | `pub fn` |
| `model.rs` | types, `parse`/`as_str` helpers, `serde` attributes | re-exported from `mod.rs` |
| `repository.rs` | SQL for this domain's tables, takes `&Connection` | `pub` (used by its own `mod.rs` and tests) |
| `commands.rs` | `#[tauri::command]` adapters | `pub` (registered in `lib.rs`) |

- Add `service.rs` only when logic spans several repositories or domains. Don't scaffold it in advance.
- Split a file once it holds two unrelated responsibilities. Otherwise, prefer fewer files.

## Style

- Log with the `log` macros (`log::info!`, `log::warn!`), never `println!`. `tauri-plugin-log` routes them to stdout and the log file.
- Never use `unwrap()` on I/O or DB results outside tests. Return `AppResult` and let the command surface the error. `Mutex::lock().unwrap()` is fine, since poisoning means a prior panic.
- A module opens with a `//!` doc line that says what it owns.
- Mark known shortcuts with `// ponytail:` plus their ceiling and the upgrade path. Example: `lib.rs` aborts setup on a DB open failure.
- Tests live in a `#[cfg(test)] mod tests` at the bottom of the file they test ([testing](../testing.md)).
