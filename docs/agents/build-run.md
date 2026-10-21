---
title: Build & Run
scope: "**"
applies_to: running the app, building, packaging
related:
  - ./testing.md
  - ./configuration.md
  - ./workflow/release.md
---

# Build & Run

> `yarn tauri dev` runs the app with hot reload. `yarn tauri build --no-bundle` checks a release compile without signing keys. A full `yarn tauri build` needs the updater signing key. Releases go through the scripts in [release](./workflow/release.md).

## Setup

```bash
mise install
export GITHUB_NPM_AUTH_TOKEN=…   # read:packages, for @bob-park/* lint configs
yarn install
```

Always use Yarn 4.18.1 (mise). A global Yarn 1 rewrites `yarn.lock`.

## Commands

| Command | What it does |
| ------- | ------------ |
| `yarn tauri dev` | Vite on :1420 + debug Rust app; frontend hot-reloads, Rust/config changes rebuild and restart |
| `yarn tauri build --no-bundle` | release compile, no `.app`/installer, no signing key needed |
| `yarn tauri build` | bundles (`.app`, `.dmg`, NSIS) + updater artifacts; needs `TAURI_SIGNING_PRIVATE_KEY` |
| `yarn build` | frontend only (`tsc` + `vite build`) |
| `yarn prettier` | format `src/` |

## Gotchas

- **It's a tray app:** closing windows doesn't quit. Quit from the tray before rerunning `yarn tauri dev`; otherwise single-instance just focuses the old process.
- **The updater** only runs in release builds.
- **After adding a JS dependency:** the first dev load may show a transient "Invalid hook call" while Vite re-optimizes deps. It is gone after the automatic reload.
- **Dev data** lives in `~/Library/Application Support/com.bobpark.app/app.db` (macOS) or `%APPDATA%\com.bobpark.app\` (Windows). Delete it to start fresh.
