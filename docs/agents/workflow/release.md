---
title: Release
scope: scripts/latest-json.mjs, docs/release.md, src-tauri/tauri.conf.json
applies_to: cutting a release, touching release scripts or updater config
related:
  - ./versioning.md
  - ../configuration.md
---

# Release

> Releases are built, signed, notarized and uploaded **locally** on an Apple Silicon Mac: `yarn tauri build --bundles app,dmg` with the keys from `~/.config/kiri/sign.env`, `notarytool` for the dmg, `gh release create --target master`, then `scripts/latest-json.mjs` uploads `latest.json`. There is no CI build. The full procedure, in Korean, is in [docs/release.md](../../release.md).

## Steps

1. The version is already bumped on `master` by the merge ([versioning](versioning.md)). Pull `master`.
2. Run the four checks and `yarn sidecars`.
3. Build with the signing env loaded: `.app` (signed + notarized), `.app.tar.gz` + `.sig` (updater), `.dmg`.
4. Notarize and staple the dmg; check both with `spctl`.
5. `gh release create v<v> --target master` with the dmg, `kiri.app.tar.gz` and its `.sig`.
6. `node scripts/latest-json.mjs v<v>` builds `latest.json` from the uploaded `.sig` and uploads it to the same release.

## Rules for agents

- **Never build-and-upload a release, run `scripts/latest-json.mjs`, or run `gh release` commands without the user's explicit request.** They publish to users.
- Never overwrite assets of a published version; bump the version and release again. Installed apps would not see a replaced build.
- `latest-json.mjs` refuses to run while `plugins.updater.pubkey` is empty, and `build.rs` fails a release build in that case. Don't remove those guards.
- `latest-json.mjs` only lists `darwin-aarch64` (`.app.tar.gz.sig`). There is no other platform.
