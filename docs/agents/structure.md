---
title: Directory Structure
scope: "**"
applies_to: creating files or folders, adding a domain, placing a component
related:
  - ./architecture.md
  - ./conventions/naming.md
  - ./conventions/module-aliases.md
  - ./conventions/rust-modules.md
---

# Directory Structure

> Workspace: `crates/kiri-core` (engine), `crates/kiri-cli` (CLI), `src-tauri` (app). Frontend: `src/{app,domain/<name>/{apis,queries,components,store},shared,utils}`. App Rust: `src-tauri/src/{lib.rs,app,domain/<name>/{mod,model,repository,commands}.rs,shared}`. A domain has the **same name** on both sides.

## Tree

```text
Cargo.toml                    # workspace: crates/kiri-core, crates/kiri-cli, src-tauri (one Cargo.lock, one target/)
crates/
├── kiri-core/src/            # Tauri-free engine shared by app and CLI
│   ├── engine.rs  queue.rs   # Engine (queue, concurrency, cancellation) · QueueState + QueueStore trait
│   ├── pipeline.rs  runner.rs# one job: download → encode → move · child processes
│   ├── ytdlp.rs  ffmpeg.rs   # args + output parsers
│   ├── tools.rs  files.rs    # yt-dlp/Deno installer · output files and .kiripart packages
│   ├── ipc.rs  model.rs      # CLI socket protocol · Job model (the frontend/CLI contract)
│   └── testutil.rs           # test-only fake tools
└── kiri-cli/
    ├── src/main.rs           # `kiri` binary (clap)
    └── tests/cli.rs          # CLI against a fake socket

src/
├── app/                      # Screens (one per window label) + entry
│   ├── main.tsx              # seeds settings (theme + language), picks the screen by window label, mounts providers
│   ├── App.tsx               # `main` window
│   ├── globals.css           # Tailwind + daisyUI themes + Aurora Glass tokens — the only place colors are defined
│   └── settings/
│       ├── SettingsApp.tsx   # `settings` window
│       └── _components/      # settings-window-only components (GeneralTab, DownloadTab, CliTab, UpdateTab, ThemePicker)
├── domain/<name>/            # settings, queue, tools, updater, cli
│   ├── apis/                 # <name>.ts (call() per command) + <name>.dto.ts (wire types)
│   ├── queries/              # <name>.tsx — React Query hooks (+ event listeners)
│   ├── components/           # domain-scoped components (queue: JobRow, OptionsSheet, EmptyState)
│   └── store/                # Zustand slice (slice.ts + <name>.state.ts), or .gitkeep
├── shared/
│   ├── api/                  # call<T>() + AppError
│   ├── components/           # cross-domain UI (settings/*, Icon, StatusBar, Toasts, error/ErrorAlert, queries/RQProvider)
│   ├── hooks/                # useI18n, useExitAnimation
│   ├── i18n/                 # en.ts (base Dict), ko.ts, locale.ts, index.ts
│   ├── providers/            # ThemeProvider, I18nProvider
│   └── store/                # rootStore (combines slices), SlicePattern augmentation, error/ and toast/ slices
└── utils/                    # pure functions only (no React, no I/O) + *.test.ts

src-tauri/
├── tauri.conf.json  capabilities/default.json  icons/  Info.plist
├── binaries/                 # sidecars from `yarn sidecars` (git-ignored)
└── src/
    ├── main.rs  lib.rs       # lib.rs assembles: plugins, setup, generate_handler!
    ├── app/                  # shell: state, windows, tray (+ app menu), updater, quit, cli_install, ipc_server, i18n
    ├── domain/<name>/        # settings, queue: mod.rs (public API) · model.rs · repository.rs · commands.rs
    │                         # tools: mod.rs · commands.rs (in-memory state, no tables)
    └── shared/               # db.rs, error.rs, migrations/000N_*.sql

scripts/                      # build-cli.sh, release-macos.sh + load-updater-key-macos.sh, latest-json.mjs (+ test)
```

Frontend `updater` and `cli` domains have no Rust `domain/` twin: their Rust halves are `app/updater.rs` and `app/cli_install.rs`, because they are OS integration rather than business data.

## Adding a domain

Do it in this order. Create all four frontend sub-folders even if some stay empty (`.gitkeep`) — a predictable layout beats a minimal file count.

**Rust**

1. If the logic is engine work the CLI also needs (downloading, converting, the queue), put it in `kiri-core` and expose it through `Engine`. Otherwise continue in `src-tauri`.
2. `src-tauri/src/domain/<name>/model.rs` — types (`serde`), parsing/validation helpers.
3. If it needs tables: `shared/migrations/000N_<name>.sql`, appended to `MIGRATIONS` ([sqlite-migrations](./conventions/sqlite-migrations.md)).
4. `repository.rs` — SQL only, takes `&Connection`.
5. `mod.rs` — the public API other code calls; validation lives here. Unit-test it with `db::open_in_memory()`.
6. `commands.rs` — thin `#[tauri::command]`s; register them in `lib.rs` `generate_handler!` ([tauri-commands](./conventions/tauri-commands.md)).
7. `pub mod <name>;` in `domain/mod.rs`.

**Frontend**

1. `src/domain/<name>/apis/<name>.dto.ts` — wire types mirroring the Rust `serde` output.
2. `apis/<name>.ts` — one function per command via `call()`.
3. `queries/<name>.tsx` — React Query hooks ([react-query](./libs/react-query.md)).
4. `components/` — UI, using `useI18n()` for strings and design tokens ([design](./design.md)).
5. Client-only UI state → `store/slice.ts`, registered in `src/shared/store/rootStore.ts` ([zustand-slice](./libs/zustand-slice.md)).

## Placement rules

- `shared/` is for code used by **two or more** domains/screens. Promote from a domain when the second user appears, not before.
- `utils/` holds pure functions only; anything touching React, the DOM or IPC goes elsewhere.
- Screen-only sub-components go in a `_components/` folder next to the screen (`src/app/settings/_components/`). Do not import another screen's `_components/` — promote to `shared/components/` instead.
- Rust: anything that is not a business domain (windows, menus, tray, updates, OS integration, the IPC server) goes in `app/`, not `domain/`.
- Rust: nothing in `kiri-core` may depend on Tauri or SQLite; storage reaches it through a trait (`QueueStore`).
