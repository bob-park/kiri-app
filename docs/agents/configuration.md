---
title: Configuration & Secrets
scope: src-tauri/tauri.conf.json, src-tauri/capabilities/**, .yarnrc.yml, scripts/**
applies_to: changing app config, windows, permissions, or anything touching keys and tokens
related:
  - ./build-run.md
  - ./workflow/release.md
  - ./conventions/tauri-commands.md
---

# Configuration & Secrets

> App config is `src-tauri/tauri.conf.json`. Webview permissions are in `src-tauri/capabilities/default.json`. Secrets (signing keys, Apple credentials, npm token) live **outside the repo**: `~/.config/template-rust-app/sign.env` and the `GITHUB_NPM_AUTH_TOKEN` env var. **Never commit a secret.**

## `tauri.conf.json`

| Field | Notes |
| ----- | ----- |
| `version` | source of truth for the app version — changed only by `bump-version-up.sh` ([versioning](./workflow/versioning.md)) |
| `productName`, `identifier` | placeholders `App` / `com.bobpark.app`; release scripts depend on `productName` for file names |
| `app.windows` | `main` and `settings`, both `create: false` (built in `setup` after state exists) |
| `bundle.createUpdaterArtifacts` | `true` — full builds need the signing key |
| `plugins.updater.pubkey` | public half of the updater key; empty in the template, filled per app ([release](../release.md)) |
| `plugins.updater.endpoints` | `…/releases/latest/download/latest.json` of this repo |

## Capabilities

- `capabilities/default.json` lists the windows (`main`, `settings`) and `core:default`.
- A plugin API **called from JS** needs its permission here. Rust-only plugin use does not.
- A new window label must be added here.

## Secrets

| Secret | Where | Used by |
| ------ | ----- | ------- |
| Apple signing identity, Apple ID, app password, team ID | `~/.config/template-rust-app/sign.env` | `scripts/release.sh` (notarization) |
| `TAURI_SIGNING_PRIVATE_KEY` (+ password) | `sign.env` on each release machine | `yarn tauri build`, both release scripts |
| `GITHUB_NPM_AUTH_TOKEN` | shell env | `yarn install` (`@bob-park/*` from GitHub Packages) |
| `GITHUB_TOKEN` | Actions-provided | `bump-version.yml` |

- Don't print secrets in logs, scripts, or commit messages.
- If a key file path is wrong, the build fails late with `Invalid symbol 46` ([release](../release.md#문제-해결)).
