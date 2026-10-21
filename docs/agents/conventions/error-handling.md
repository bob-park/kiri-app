---
title: Error Handling
scope: src-tauri/src/**, src/shared/api/**, src/domain/*/queries/**
applies_to: returning an error from Rust, handling a failed command in the UI
related:
  - ./tauri-commands.md
  - ../libs/react-query.md
---

# Error Handling

> Rust returns `AppResult<T>` = `Result<T, AppError>`. `AppError` serializes to `{ code, message }`. On the frontend, every failed **mutation** is caught by the query client's `MutationCache`, stored in the error slice, and shown by `<ErrorAlert />` with text from `t.errors[code]`.

## Rust

`src-tauri/src/shared/error.rs`:

| Variant | `code` | Use for |
| ------- | ------ | ------- |
| `InvalidInput(String)` | `INVALID_INPUT` | bad values from the webview (validation in the domain API) |
| `Db(rusqlite::Error)` | `DB` | any SQLite failure (`?` converts automatically) |
| `Io(std::io::Error)` | `IO` | filesystem failures |
| `SchemaTooNew { found, supported }` | `SCHEMA_TOO_NEW` | DB written by a newer app version (`db::migrate` refuses to run against it) |

- Add a variant only when the frontend must **branch** on it. Give it a stable SCREAMING_SNAKE `code` in `AppError::code()`.
- The `message` is for logs and developers. User-facing text comes from `t.errors[code]` in `src/shared/i18n/{en,ko}.ts` — **add the code there too** (unknown codes fall back to `t.errors.UNKNOWN`).
- App-shell commands that wrap plugin errors (`autostart`, `updater`) may return `Result<T, String>`. `call()` turns a string into `{ code: 'UNKNOWN', message }`.

## Frontend

- `src/shared/api/common.dto.ts` defines `interface AppError { code: string; message: string }`.
- `call()` always rejects with an `AppError`, never a raw string or `Error`.
- **Default display:**
  1. `queryClient`'s `MutationCache.onError` (`RQProvider.tsx`) calls `useStore.getState().showError(err)` for every failed mutation.
  2. `<ErrorAlert />` (`src/shared/components/error/`) renders `errorMessage(err, t.errors)` (`src/utils/errorMessage.ts`) with a dismiss button.
  3. Place `<ErrorAlert />` at the top of every screen that runs mutations. `SettingsApp` does.
- **Extra handling:** mutation hooks also accept `QueryMutationHandle<T>` (`onSuccess`, `onError(err: AppError)`) for anything beyond the banner, such as reverting a local draft.
- **Queries (reads)** are not routed to the banner. Use the query's `error` where a failed read matters.
