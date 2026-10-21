---
title: Release
scope: scripts/**, docs/release.md
applies_to: cutting a release, touching release scripts or updater config
related:
  - ./versioning.md
  - ../configuration.md
---

# Release

> Releases are built, signed and uploaded **locally**: `scripts/release.sh` (macOS: sign and notarize) and `scripts/release.ps1` (Windows: updater `.sig` only). `scripts/latest-json.mjs` then assembles `latest.json`. There is no CI build. The full procedure, in Korean, is in [docs/release.md](../../release.md).

## Steps

1. The version is already bumped on `develop` ([versioning](versioning.md)). Merge `develop` into `main`.
2. On macOS (Apple Silicon), run `scripts/release.sh`. It uploads the dmg, `App_<v>_aarch64.app.tar.gz` and its `.sig` to the draft release `v<v>`.
3. On Windows, run `pwsh scripts/release.ps1`. It uploads `App_<v>_x64-setup.exe` and its `.sig`.
4. Write the release notes; they become the update notes. Then run `node scripts/latest-json.mjs v<v>`.
5. Publish with `gh release edit v<v> --draft=false`.

## Rules for agents

- **Never run the release scripts or `gh release` commands without the user's explicit request.** They publish to users.
- If you change `productName`, update the file names in `release.sh` and `release.ps1`. `latest-json.mjs` matches on suffix only.
- Both scripts refuse to run when `v<v>` is already **published** with that platform's build. A shipped version is never overwritten; bump the version instead (hotfix flow in [versioning](versioning.md)). Adding a platform that is still missing from a published release is allowed.
- Releases are created with `--target main`, so the tag points at `main` even when `develop` is the repo's default branch.
- `latest-json.mjs` refuses to run while `plugins.updater.pubkey` is empty. Don't remove that guard.
- The updater only accepts a `latest.json` whose listed platforms are all complete. That's why the script includes only the platforms that were uploaded.
