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

> Run `yarn sidecars` once after cloning (ffmpeg/ffprobe + the `kiri` CLI), then `yarn tauri dev` runs the app with hot reload. `yarn tauri build --no-bundle` checks a release compile without signing keys. Releases go through `scripts/release-macos.sh` ([release](./workflow/release.md)).

## Setup

```bash
mise install
export GITHUB_NPM_AUTH_TOKEN=…   # read:packages, for @bob-park/* lint configs
mise exec -- yarn install
mise exec -- yarn sidecars       # src-tauri/binaries/: ffmpeg, ffprobe, kiri-cli
```

- Always use Yarn 4.18.1 (mise). A global Yarn 1 rewrites `yarn.lock`.
- Tauri checks `externalBin` at build time, so `cargo build`, `cargo test` and `yarn tauri dev` fail until `yarn sidecars` has run.
- To bump ffmpeg: `sh scripts/fetch-ffmpeg.sh --update` rewrites `scripts/ffmpeg.lock`; commit it.

## Commands

| Command | What it does |
| ------- | ------------ |
| `yarn tauri dev` | Vite on :1420 + debug Rust app; frontend hot-reloads, Rust/config changes rebuild and restart |
| `yarn tauri build --no-bundle` | release compile, no `.app`/`.dmg`, no signing key needed |
| `yarn tauri build --bundles app,dmg` | `.app` + `.dmg` + updater artifacts; needs `TAURI_SIGNING_PRIVATE_KEY` (first `source scripts/load-updater-key-macos.sh`) |
| `scripts/release-macos.sh --build-only` | Developer ID signed `.app` + `.dmg`, both notarized and stapled with the App Store Connect API key; no upload ([configuration](./configuration.md#secrets)) |
| `yarn build` | frontend only (`tsc` + `vite build`) |
| `yarn sidecars` | `scripts/fetch-ffmpeg.sh` + `scripts/build-cli.sh` |
| `yarn prettier` | format `src/` |

Build output goes to the workspace `target/` (not `src-tauri/target/`).

## Gotchas

- **It's a menu-bar app:** closing the main window doesn't quit (with "close to menu bar" on). Quit from the tray before rerunning `yarn tauri dev`; otherwise single-instance just focuses the old process.
- **Real user data:** dev builds use the same identifier as the installed app, so `yarn tauri dev` reads and writes `~/Library/Application Support/org.bobpark.kiri/` (`kiri.db`, `bin/`, `kiri.sock`). Back it up before experimenting, or quit the installed app first — only one instance can own the CLI socket.
- **CLI against dev:** `./target/release/kiri list` (or `target/debug/kiri`). `KIRI_SOCKET` overrides the socket path.
- **`.kiripart` packages** show as folders under `yarn tauri dev`, because the package type is registered by the bundled `Info.plist`. Build a debug `.app` and register it with `lsregister -f` to check.
- **After adding a JS dependency:** the first dev load may show a transient "Invalid hook call" while Vite re-optimizes deps. It is gone after the automatic reload.
