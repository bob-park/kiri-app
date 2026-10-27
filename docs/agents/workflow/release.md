---
title: Release
scope: scripts/release-macos.sh, scripts/load-updater-key-macos.sh, scripts/latest-json.mjs, docs/release.md, src-tauri/tauri.conf.json
applies_to: cutting a release, touching release scripts or updater config
related:
  - ./versioning.md
  - ../configuration.md
---

# Release

> Releases are built, signed, notarized and uploaded **locally** on an Apple Silicon Mac by `scripts/release-macos.sh`: Developer ID signing, App Store Connect API key notarization of the `.app` and `.dmg`, updater key from the login keychain via `scripts/load-updater-key-macos.sh`, upload to a draft release. `scripts/latest-json.mjs` then uploads `latest.json`. There is no CI build. The full procedure, in Korean, is in [docs/release.md](../../release.md).

## Steps

1. The version is already bumped on `master` by the merge ([versioning](versioning.md)). Pull `master` and run the four checks.
2. Run `scripts/release-macos.sh`. It runs `yarn sidecars`, builds, notarizes and uploads the dmg, `kiri_<v>_aarch64.app.tar.gz` and its `.sig` to the draft release `v<v>` (`--target master`).
3. Write the release notes, then run `node scripts/latest-json.mjs v<v>`.
4. Publish with `gh release edit v<v> --draft=false`.

## Updater signing key

The key pair already exists. The private half lives only in the login keychain (`tauri-updater-kiri` / `tauri-updater-kiri-pw`, from `APP_NAME` in `scripts/load-updater-key-macos.sh`). The public half is `plugins.updater.pubkey`.

- **Store it on a new Mac:** `security add-generic-password -a "$USER" -s tauri-updater-kiri -w '<content>'` and `-s tauri-updater-kiri-pw -w '<password>'`. The value is the key **content**, not a path.
- **Sign a file by hand:** `source scripts/load-updater-key-macos.sh`, then `yarn tauri signer sign --app-version <v> <file>`. `yarn tauri build` already signs the updater artifacts, so this is only for re-signing.

## Rules for agents

- **Never run `scripts/release-macos.sh`, `scripts/latest-json.mjs`, or `gh release` commands without the user's explicit request.** They publish to users.
- **Never run `yarn tauri signer generate` (or change `plugins.updater.pubkey`) without the user's explicit request.** Installed apps only trust the current public key: after a rotation they reject every update, so users must reinstall by hand.
- The updater signing key comes from the login keychain through `scripts/load-updater-key-macos.sh` (sourced, not run as a child process). Don't go back to a key file or `sign.env`, and don't print the values.
- Notarization uses the **API key** method only (`APPLE_API_KEY`, `APPLE_API_ISSUER`, `APPLE_API_KEY_PATH`, set by `release-macos.sh`). Don't reintroduce `APPLE_ID`/`APPLE_PASSWORD`: if `APPLE_ID` is set, Tauri notarizes with it instead.
- Tauri notarizes only the `.app`; `release-macos.sh` then notarizes and staples the `.dmg` with `notarytool`. Keep that step, or Gatekeeper rejects the downloaded dmg.
- `scripts/release-macos.sh --build-only` builds and notarizes but uploads nothing. It still talks to Apple, so ask before running it.
- If you change `productName`, update the file names in `release-macos.sh`. `latest-json.mjs` matches on suffix only.
- The script refuses to run when `v<v>` is already **published**. A shipped version is never overwritten; bump the version and release again.
- `latest-json.mjs` refuses to run while `plugins.updater.pubkey` is empty, and `build.rs` fails a release build in that case. Don't remove those guards.
- `latest-json.mjs` only lists `darwin-aarch64` (`.app.tar.gz.sig`). There is no other platform.
