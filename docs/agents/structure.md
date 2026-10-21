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

> Frontend: `src/{app,domain/<name>/{apis,queries,components,store},shared,utils}`. Rust: `src-tauri/src/{lib.rs,app,domain/<name>/{mod,model,repository,commands}.rs,shared}`. A domain has the **same name** on both sides.

## Tree

```text
src/
├── app/                      # Screens (one per window label) + entry
│   ├── main.tsx              # seeds theme/locale, picks the screen by window label, mounts providers
│   ├── App.tsx               # `main` window
│   ├── globals.css           # Tailwind + daisyUI themes — the only place colors are defined
│   └── settings/
│       ├── SettingsApp.tsx   # `settings` window
│       └── _components/      # settings-window-only components (Tabs, AutostartSwitch)
├── domain/<name>/
│   ├── apis/                 # <name>.ts (call() per command) + <name>.dto.ts (wire types)
│   ├── queries/              # <name>.tsx — React Query hooks
│   ├── components/           # domain-scoped components
│   └── store/                # Zustand slice (slice.ts + <name>.state.ts), or .gitkeep
├── shared/
│   ├── api/                  # call<T>() + AppError
│   ├── components/           # cross-domain UI (settings/*, theme/*, i18n/*, error/ErrorAlert, queries/RQProvider)
│   ├── hooks/                # reusable hooks (useSettingsShortcut)
│   ├── i18n/                 # en.ts (base Dict), ko.ts, locale.ts
│   ├── providers/            # ThemeProvider, I18nProvider
│   ├── queries/              # QueryMutationHandle
│   └── store/                # rootStore (combines slices), SlicePattern augmentation, error/ (cross-cutting error slice)
└── utils/                    # pure functions only (no React, no I/O) + *.test.ts

src-tauri/
├── tauri.conf.json  capabilities/default.json  icons/
└── src/
    ├── main.rs  lib.rs       # lib.rs assembles: plugins, setup, generate_handler!
    ├── app/                  # shell: state, window, menu, tray, updater, autostart, i18n
    ├── domain/<name>/        # mod.rs (public API) · model.rs · repository.rs · commands.rs
    └── shared/               # db.rs, error.rs, migrations/000N_*.sql
```

## Adding a domain

Do it in this order. Create all four frontend sub-folders even if some stay empty (`.gitkeep`) — a predictable layout beats a minimal file count.

**Rust**

1. `src-tauri/src/domain/<name>/model.rs` — types (`serde`), parsing/validation helpers.
2. If it needs tables: `shared/migrations/000N_<name>.sql`, appended to `MIGRATIONS` ([sqlite-migrations](./conventions/sqlite-migrations.md)).
3. `repository.rs` — SQL only, takes `&Connection`.
4. `mod.rs` — the public API other code calls; validation lives here. Unit-test it with `db::open_in_memory()`.
5. `commands.rs` — thin `#[tauri::command]`s; register them in `lib.rs` `generate_handler!` ([tauri-commands](./conventions/tauri-commands.md)).
6. `pub mod <name>;` in `domain/mod.rs`.

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
- Rust: anything that is not a business domain (windows, menus, tray, updates, OS integration) goes in `app/`, not `domain/`. The updater is such a case — its frontend half lives in `src/domain/updater` because the UI groups it like a domain, while the Rust half is `app/updater.rs`.
