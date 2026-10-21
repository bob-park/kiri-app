---
title: SQLite Migrations
scope: src-tauri/src/shared/**, src-tauri/src/domain/*/repository.rs
applies_to: adding or changing a table, column, index, or seed row
related:
  - ./rust-modules.md
  - ../testing.md
---

# SQLite Migrations

> One SQL file per schema version in `src-tauri/src/shared/migrations/`, appended to `MIGRATIONS` in `shared/db.rs`. `PRAGMA user_version` records how many have run. Never edit a migration that has shipped.

## Adding a migration

1. Create `src-tauri/src/shared/migrations/000N_<what>.sql`, where N is the next number. Example: `0002_notes.sql`.
2. Append it to `MIGRATIONS`:

   ```rust
   const MIGRATIONS: &[&str] = &[
       include_str!("migrations/0001_init.sql"),
       include_str!("migrations/0002_notes.sql"),
   ];
   ```

3. `db::migrate` runs each pending file in its own transaction and then sets `user_version = index + 1`.

If the DB's `user_version` is **higher** than `MIGRATIONS.len()` (an older build reinstalled over newer data), `migrate` returns `AppError::SchemaTooNew` and the app stops instead of running against an unknown schema.

## Rules

- **Never edit, reorder, or delete a shipped file.** Installed apps have already recorded that version. To fix a mistake, add a new migration.
- Don't put `BEGIN`/`COMMIT` or `PRAGMA user_version` inside the SQL file; `migrate` handles both.
- Seed defaults in the migration only when every install needs the row. Otherwise, fall back in code. For example, `settings::auto_update` treats a missing row as `true`.
- Readers must tolerate unknown or old values. `settings::theme` maps garbage to `System` instead of failing.
- A destructive change (dropping or renaming a column) needs the user's confirmation first, because it touches real user data.

## Tests

- Use `db::open_in_memory()` (migrated, `#[cfg(test)]` only) for domain tests.
- Use `tempfile::tempdir()` with `db::open(path)` when the test is about files: first launch, reopen, parent dir creation.
- Every new migration needs a test that reopens a DB at the previous version (or a fresh one) and checks the new shape and seed rows.
