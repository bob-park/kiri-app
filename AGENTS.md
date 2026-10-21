# AGENTS.md

Agent-agnostic guide for `template-rust-app`, a Tauri 2 desktop-app template (Rust + React). Detailed rules are split into topic files under [`docs/agents/`](docs/agents/), so an agent loads only what a task needs.

Each topic doc opens with YAML frontmatter (`title`, `scope`, `applies_to`, `related`) and a one-line TL;DR blockquote. Skimming the first few lines tells you whether it applies.

Find the matching entry in the Map or in "How to use", open that doc, and check its TL;DR. If nothing matches, start with [overview](docs/agents/overview.md) and [architecture](docs/agents/architecture.md).

## Always

- **Design/UI work:** before building or changing any UI (screens, components, styles, tokens), read [`docs/design/`](docs/design/) and [design](docs/agents/design.md) first.
- **UI mockups:** for any design or UI work, show at least 3 mockup options in the `superpowers` visual companion before implementing. Always use the `ui-ux-pro-max` skill to create them.
- **Node/Yarn versions:** use the versions pinned in [`.mise.toml`](.mise.toml) (Node 26, Yarn 4.18.1). Run `mise install` once, then run every `node`/`yarn` command through mise (activated shell or `mise exec --`). Don't use a global Node or Yarn 1; it rewrites `yarn.lock`.

## Map

### Foundations
- [Project Overview](docs/agents/overview.md): what the template is and isn't, platforms, placeholder names
- [Tech Stack](docs/agents/tech-stack.md): libraries and tools, where versions are pinned

### Architecture & Structure
- [Architecture](docs/agents/architecture.md): request path front → Rust → SQLite, layers, startup, windows, updater
- [Directory Structure](docs/agents/structure.md): canonical tree, **adding a domain** (Rust + frontend order), placement rules

### Design
- [Design](docs/agents/design.md): `docs/design/` first, tokens only in `globals.css`, settings building blocks, light/dark + ko/en check

### Rust Conventions
- [Rust Modules & Layering](docs/agents/conventions/rust-modules.md): dependency direction, no `tauri` in domain core/shared, file roles
- [Tauri Commands & Events](docs/agents/conventions/tauri-commands.md): thin commands, `call()` wrappers, event names, capabilities
- [SQLite Migrations](docs/agents/conventions/sqlite-migrations.md): one file per version, never edit shipped migrations
- [Error Handling](docs/agents/conventions/error-handling.md): `AppError` → `{ code, message }`, frontend handling

### Frontend Conventions
- [Module & Path Aliases](docs/agents/conventions/module-aliases.md): `@/` vs `./`, `import type`, the `node --test` caveat
- [Naming](docs/agents/conventions/naming.md): files, hooks, query keys, commands, events, i18n keys
- [TypeScript](docs/agents/conventions/typescript.md): strict mode, `interface` vs `type`, DTOs mirror serde
- [React Components & Section Comments](docs/agents/conventions/react-sections.md): fixed section order in component bodies

### Library Patterns
- [React Query](docs/agents/libs/react-query.md): read/write hooks, defaults, **multi-window sync via events**, first-paint seeding
- [Zustand Slice](docs/agents/libs/zustand-slice.md): client-only UI state, `SlicePattern`, registering in `rootStore`
- [Tailwind 4 + daisyUI 5](docs/agents/libs/tailwind-daisyui.md): `globals.css` layout, adding tokens, `classnames`
- [Theme](docs/agents/libs/theme.md): light/dark/system flow, `ThemeProvider`
- [i18n](docs/agents/libs/i18n.md): `en.ts`/`ko.ts` dictionaries, `useI18n`, native labels, adding a language

### Workflow
- [Branch Naming](docs/agents/workflow/branching.md): `main` / `develop` / `feature/*` / `hotfix/*`
- [Commit Messages](docs/agents/workflow/commit-messages.md): `<prefix>: <subject>`
- [Pull Requests](docs/agents/workflow/pr.md): base branch, title → version bump
- [Versioning](docs/agents/workflow/versioning.md): semver, `bump-version-up.sh`, the `develop` workflow
- [Release](docs/agents/workflow/release.md): local signed release scripts, `latest.json`

### Ops
- [Build & Run](docs/agents/build-run.md): dev, `--no-bundle`, full build, gotchas
- [Testing](docs/agents/testing.md): `cargo test`, `node:test`, typecheck, lint, what to test
- [Configuration & Secrets](docs/agents/configuration.md): `tauri.conf.json`, capabilities, where secrets live
- [Documentation](docs/agents/documentation.md): where docs go, format, languages

## How to use

- **New feature or domain:** [structure](docs/agents/structure.md#adding-a-domain) → [architecture](docs/agents/architecture.md) → [tauri-commands](docs/agents/conventions/tauri-commands.md) → [react-query](docs/agents/libs/react-query.md)
- **New table or column:** [sqlite-migrations](docs/agents/conventions/sqlite-migrations.md)
- **New UI or screen:** [design](docs/agents/design.md) (read `docs/design/` first) → [react-sections](docs/agents/conventions/react-sections.md) → [tailwind-daisyui](docs/agents/libs/tailwind-daisyui.md)
- **Any user-visible text:** [i18n](docs/agents/libs/i18n.md)
- **New setting in the Settings window:** [design](docs/agents/design.md) (`SettingRow` / `Segmented` / `Switch`) → [structure](docs/agents/structure.md) → [i18n](docs/agents/libs/i18n.md)
- **A value changed in one window must show in another:** [react-query](docs/agents/libs/react-query.md#multiple-windows) → [tauri-commands](docs/agents/conventions/tauri-commands.md#events)
- **Client-only UI state:** [zustand-slice](docs/agents/libs/zustand-slice.md)
- **New error case:** [error-handling](docs/agents/conventions/error-handling.md)
- **Window, menu, tray, or startup change:** [architecture](docs/agents/architecture.md) → [rust-modules](docs/agents/conventions/rust-modules.md)
- **Running, building, or testing:** [build-run](docs/agents/build-run.md) → [testing](docs/agents/testing.md)
- **Config, permissions, or secrets:** [configuration](docs/agents/configuration.md)
- **Starting a branch:** [branching](docs/agents/workflow/branching.md)
- **Writing a commit message:** [commit-messages](docs/agents/workflow/commit-messages.md)
- **Opening a PR:** [pr](docs/agents/workflow/pr.md)
- **Cutting a release:** [release](docs/agents/workflow/release.md) → [versioning](docs/agents/workflow/versioning.md)

## Convention Enforcement

These conventions apply to **new code and to existing code being substantially modified**.

- Don't refactor pre-existing code that doesn't conform as part of an unrelated change. Track it separately, in a follow-up note in the PR description or in its own `refactor:` PR.
- Where an example shows a pattern that existing code doesn't follow yet, the example is the target for new code. It is not a directive to backfill.
- Before calling a change done, run the four checks in [testing](docs/agents/testing.md). For UI changes, also look at the running app in light/dark and ko/en.
