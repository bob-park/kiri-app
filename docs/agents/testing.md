---
title: Testing
scope: "**"
applies_to: writing tests, verifying a change before calling it done
related:
  - ./build-run.md
  - ./conventions/sqlite-migrations.md
  - ./conventions/module-aliases.md
---

# Testing

> Rust: `cargo test --workspace` (engine, CLI, app domains, DB, errors). Frontend and scripts: `yarn test` (Node built-in `node:test`, no extra libraries). Plus `yarn typecheck` and `yarn lint`. All four must pass before a change is done.

| Command | Covers | Lives in |
| ------- | ------ | -------- |
| `cargo test --workspace` | `kiri-core` (engine, pipeline with fake tools, parsers, IPC, files, tools), `kiri-cli` (against a fake socket), `kiri-app` (settings/queue domains and import, migrations, `AppError` serialization, updater/tray/quit/cli_install helpers, native i18n, the pubkey guard) | `#[cfg(test)] mod tests` at the bottom of each Rust file; `crates/kiri-cli/tests/` |
| `yarn test` | pure TS logic (`src/**/*.test.ts`) and scripts (`scripts/*.test.mjs`) | next to the file under test |
| `yarn typecheck` | types, including missing i18n keys | — |
| `yarn lint` | ESLint + Prettier (`eslint src`) | — |

Run node/yarn through mise (`mise exec -- yarn test`). `cargo test -p <crate>` (`kiri-core`, `kiri-cli`, `kiri-app`) runs one crate while iterating. The Rust tests need the sidecars in `src-tauri/binaries/` ([build-run](./build-run.md#setup)).

## What to test

- **Write the failing test first**, then the code (TDD).
- **Rust:**
  - Test through the domain's public API in `mod.rs`, against `db::open_in_memory()`.
  - Cover the rejection paths too: invalid input saves nothing, and unknown stored values fall back.
  - Engine and pipeline: use `kiri-core`'s `testutil` (shell scripts standing in for yt-dlp/ffmpeg) and `tempfile`.
- **First launch, reopen, import:** use `tempfile::tempdir()` with `db::open(path)`.
- **Frontend:**
  - Extract decisions into pure functions in `src/utils/` (or `src/shared/i18n/locale.ts`) and test those, e.g. `resolveTheme`, `nextPhase`, `resolveLocale`, `mergeSettings`.
  - Components and hooks are not unit-tested; verify them in the running app.
- **Scripts:** `scripts/latest-json.test.mjs` covers `latest-json.mjs`.

## Constraints

- `node --test` runs TS by stripping types. It cannot resolve `@/`, so a tested file may use `@/…` only in `import type` lines ([module-aliases](./conventions/module-aliases.md)).
- **Tauri-dependent code** (`commands.rs`, most of `app/*`) has no unit tests beyond pure helpers. Keep it thin and verify it in `yarn tauri dev`.
- **UI changes:** check them in the running app in light/dark and ko/en ([design](./design.md)).
