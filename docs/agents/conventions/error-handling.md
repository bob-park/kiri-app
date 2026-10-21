---
title: Error Handling
scope: src-tauri/src/**, crates/kiri-core/src/engine.rs, src/shared/api/**, src/domain/*/queries/**
applies_to: returning an error from Rust, handling a failed command in the UI
related:
  - ./tauri-commands.md
  - ../libs/react-query.md
---

# Error Handling

> Rust returns `AppResult<T>` = `Result<T, AppError>`. `AppError` serializes to `{ code, message }`. On the frontend, every failed **mutation** is caught by the query client's `MutationCache`, stored in the error slice, and shown by `<ErrorAlert />` (inside `Toasts`) with text from `t.errors[code]`.

## Rust

`src-tauri/src/shared/error.rs`:

| Variant | `code` | Use for |
| ------- | ------ | ------- |
| `InvalidInput(String)` | `INVALID_INPUT` | bad values from the webview (validation in the domain API, e.g. a settings patch of the wrong type) |
| `Db(rusqlite::Error)` | `DB` | any SQLite failure (`?` converts automatically) |
| `Io(std::io::Error)` | `IO` | filesystem failures |
| `SchemaTooNew { found, supported }` | `SCHEMA_TOO_NEW` | DB written by a newer app version (`db::migrate` refuses to run against it) |
| `NotFound(String)` | `NOT_FOUND` | reserved; engine misses arrive as `Engine(NotFound)` |
| `Failed(String)` | `FAILED` | shell failures with no better code (window, CLI link) |
| `Engine(EngineError)` | the engine's code, uppercased | queue/probe failures from `kiri_core::engine` |

Engine codes (`EngineError::code()` uppercased in `engine_code`): `NOT_FOUND`, `NOT_STOPPABLE`, `NOT_RESTARTABLE`, `YTDLP_MISSING`, `INVALID_URL`, `PROBE_FAILED`, `BAD_QUALITY`, `BAD_PRESET`, `SOURCE_NOT_FOUND`, `INVALID_MEDIA`. The exhaustive match breaks the build when the engine gains a variant; the test `engine_codes_are_uppercased` checks the spelling.

- Add a variant only when the frontend must **branch** on it or show a specific message. Give it a stable SCREAMING_SNAKE `code`.
- The `message` is for logs and developers. User-facing text comes from `t.errors[code]` in `src/shared/i18n/{en,ko}.ts` — **add the code there too** (unknown codes fall back to `t.errors.UNKNOWN`). `t.errors` also holds keys that arrive as plain messages: a job failure or updater error starting with `error.` (`error.no_output` → `NO_OUTPUT`, `error.download_dir_unwritable`, `error.no_update`) is looked up uppercased by `failureText` (`src/utils/format.ts`); other messages are raw tool output.
- The updater commands (`check_update`, `install_update`) wrap plugin errors and return `Result<T, String>`. `call()` turns a string into `{ code: 'UNKNOWN', message }`.

## Frontend

- `src/shared/api/common.dto.ts` defines `interface AppError { code: string; message: string }`.
- `call()` always rejects with an `AppError`, never a raw string or `Error`.
- **Default display:**
  1. `queryClient`'s `MutationCache.onError` (`RQProvider.tsx`) calls `useStore.getState().showError(err)` for every failed mutation, unless the mutation sets `meta: { silent: true }` and shows its own error.
  2. `<ErrorAlert />` (`src/shared/components/error/`) renders `errorMessage(err, t.errors)` (`src/utils/errorMessage.ts`) with a dismiss button. `Toasts` renders it at the end of the toast stack, so every window has it.
  3. Direct calls outside React Query (e.g. `openSettings()`) catch and pass the error to `showError` themselves.
- **Queries (reads)** are not routed to the banner. Use the query's `error` where a failed read matters.
