---
title: Configuration & Secrets
scope: src-tauri/tauri.conf.json, src-tauri/capabilities/**, src-tauri/Info.plist, .yarnrc.yml, scripts/**
applies_to: changing app config, windows, permissions, or anything touching keys and tokens
related:
  - ./build-run.md
  - ./workflow/release.md
  - ./conventions/tauri-commands.md
---

# Configuration & Secrets

> App config is `src-tauri/tauri.conf.json`. Webview permissions are in `src-tauri/capabilities/default.json`. Secrets (Apple credentials, updater signing key, npm token) live **outside the repo**: `~/.config/kiri/sign.env`, `~/.tauri/kiri.key` and the `GITHUB_NPM_AUTH_TOKEN` env var. **Never commit a secret.**

## `tauri.conf.json`

| Field | Notes |
| ----- | ----- |
| `version` | source of truth for the app version — changed only by `bump-version-up.sh` ([versioning](./workflow/versioning.md)) |
| `productName`, `identifier` | `kiri` / `org.bobpark.kiri`; the identifier names every data dir and the CLI socket path, so never change it casually |
| `app.windows` | empty: `main` and `settings` are built in `app/windows.rs` after state exists |
| `app.security.csp` | `img-src` allows `https:` for YouTube thumbnails |
| `bundle.targets` | `app`, `dmg` |
| `bundle.externalBin` | `binaries/ffmpeg`, `binaries/ffprobe`, `binaries/kiri-cli` ([build-run](./build-run.md#setup)) |
| `bundle.macOS` | `minimumSystemVersion` 14.2, hardened runtime |
| `bundle.createUpdaterArtifacts` | `true` — bundle builds need the signing key |
| `plugins.updater.pubkey` | public half of the updater key. `build.rs` fails a release build when it is empty ([release](../release.md)) |
| `plugins.updater.endpoints` | `https://github.com/bob-park/kiri-app/releases/latest/download/latest.json` |

`Info.plist` registers the `.kiripart` document package type.

## Capabilities

- `capabilities/default.json` lists the windows (`main`, `settings`) and: `core:default`, `opener:allow-reveal-item-in-dir`, `dialog:allow-open`, `clipboard-manager:allow-read-text`.
- A plugin API **called from JS** needs its permission here. Rust-only plugin use (updater, log, single-instance) does not.
- A new window label must be added here.

## Secrets

| Secret | Where | Used by |
| ------ | ----- | ------- |
| Apple signing identity, Apple ID, app-specific password, team ID | `~/.config/kiri/sign.env` | `yarn tauri build` (sign + notarize the `.app`), `xcrun notarytool` (dmg) |
| `TAURI_SIGNING_PRIVATE_KEY` (+ password) | `~/.tauri/kiri.key`, path and password in `sign.env` | `yarn tauri build` (updater `.sig`) |
| `GITHUB_NPM_AUTH_TOKEN` | shell env | `yarn install` (`@bob-park/*` from GitHub Packages) |
| `GITHUB_TOKEN` | Actions-provided | `bump-version.yml` |

- Don't print secrets in logs, scripts, or commit messages.
- `TAURI_SIGNING_PRIVATE_KEY` must hold the key **contents** when building (`"$(cat "$TAURI_SIGNING_PRIVATE_KEY")"`); a path to a missing file fails late with `Invalid symbol 46` ([release](../release.md)).
