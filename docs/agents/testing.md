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

> Rust: `cargo test --manifest-path src-tauri/Cargo.toml --lib` (domain core, DB, errors). Frontend and scripts: `yarn test` (Node built-in `node:test`, no extra libraries). Plus `yarn typecheck` and `yarn lint`. All four must pass before a change is done.

| Command | Covers | Lives in |
| ------- | ------ | -------- |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib` | domain APIs, repositories, migrations, `AppError` serialization, `UpdateStatus` shape, i18n resolution | `#[cfg(test)] mod tests` at the bottom of each Rust file |
| `yarn test` | pure TS logic (`src/**/*.test.ts`) and scripts (`scripts/*.test.mjs`) | next to the file under test |
| `yarn typecheck` | types, including missing i18n keys | — |
| `yarn lint` | ESLint + Prettier | — |

## What to test

- **Write the failing test first**, then the code (TDD).
- **Rust:**
  - Test through the domain's public API in `mod.rs`, against `db::open_in_memory()`.
  - Cover the rejection paths too: invalid input saves nothing, and unknown stored values fall back.
- **First launch and reopen:** use `tempfile::tempdir()` with `db::open(path)`.
- **Frontend:**
  - Extract decisions into pure functions in `src/utils/` (or `src/shared/i18n/locale.ts`) and test those, e.g. `resolveTheme`, `describeUpdateStatus`, `resolveLocale`.
  - Components and hooks are not unit-tested; verify them in the running app.
- **Shell scripts:** run them against fixture copies in a temp dir. See `scripts/bump-version-up.test.mjs`.

## Constraints

- `node --test` runs TS by stripping types. It cannot resolve `@/`, so a tested file may use `@/…` only in `import type` lines ([module-aliases](./conventions/module-aliases.md)).
- **Tauri-dependent code** (`commands.rs`, `app/*`) has no unit tests. Keep it thin and verify it in `yarn tauri dev`.
- **UI changes:** check them in the running app in light/dark and ko/en ([design](./design.md)).
