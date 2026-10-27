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

> App config is `src-tauri/tauri.conf.json`. Webview permissions are in `src-tauri/capabilities/default.json`. Secrets (signing keys, Apple credentials, npm token) live **outside the repo**: the login keychain, `~/.config/apple/` and the `GITHUB_NPM_AUTH_TOKEN` env var. **Never commit a secret.**

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
| `APPLE_SIGNING_IDENTITY` (Developer ID) | `~/.config/apple/.env` (shared by all apps) | `scripts/release-macos.sh` (code signing) |
| App Store Connect API key ID / issuer ID | login keychain, services `tauri-apple-api-key` / `tauri-apple-api-issuer` | `scripts/release-macos.sh` (notarization) |
| App Store Connect API private key | `~/.config/apple/AuthKey_<key id>.p8` | `scripts/release-macos.sh` (`APPLE_API_KEY_PATH`) |
| `TAURI_SIGNING_PRIVATE_KEY` (+ password) | login keychain, services `tauri-updater-kiri` / `tauri-updater-kiri-pw`, exported by `source scripts/load-updater-key-macos.sh` | `yarn tauri build`, `scripts/release-macos.sh` |
| `GITHUB_NPM_AUTH_TOKEN` | shell env | `yarn install` (`@bob-park/*` from GitHub Packages) |
| `GITHUB_TOKEN` | Actions-provided | `bump-version.yml` |

- Don't print secrets in logs, scripts, or commit messages.
- If the stored key is not the key **content** (a path, a truncated value), the build fails late with `Invalid symbol` ([release](../release.md#문제-해결)).
