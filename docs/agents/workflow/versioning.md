---
title: Versioning
scope: "**"
applies_to: reading or changing the app version, cutting a release
related:
  - ./pr.md
  - ./release.md
---

# Versioning

> Plain semver `X.Y.Z`. `src-tauri/tauri.conf.json` is the source of truth. `bump-version-up.sh` keeps every file carrying the version in sync; **never edit versions by hand**. GitHub Actions runs it on every PR merge into `master`.

## Files carrying the version

`src-tauri/tauri.conf.json` (source of truth) · `package.json` · `src-tauri/Cargo.toml` (`kiri-app`) · `crates/kiri-core/Cargo.toml` · `crates/kiri-cli/Cargo.toml` · the workspace `Cargo.lock` (entries for the three crates).

All three crates share the app version, so `kiri --version` matches the app.

## `bump-version-up.sh`

```bash
./bump-version-up.sh [patch|minor|major]   # default: patch
```

- The script validates the version and the argument first. It writes all files or none: after writing, it checks each file and restores the backups on any miss, exiting non-zero.
- It uses `perl -i`, so it works on macOS and on the Ubuntu runner.
- It is tested by `scripts/bump-version-up.test.mjs` against fixture copies in a temp dir. Its file lists (`CARGO_TOMLS`, `CRATE_NAMES`) must change when a workspace crate is added or renamed.
- No `-rcN`/date suffixes: the Tauri updater compares semver, and prerelease tags would break update ordering.


## Workflow (`.github/workflows/bump-version.yml`)

1. **Trigger:** a push to `master`, which happens when a PR merges. The bot's own commit carries `[skip ci]` and is skipped.
2. **Reading the PR:** the workflow finds the PR number from the merge or squash commit, then reads the PR title and labels ([pr](pr.md#title)).
3. **Bump:** `major` / `minor` / `patch` from the title or label; every merge bumps.
4. **Commit:** it runs the script and commits `chore: bump version to X.Y.Z [skip ci]` to `master`.
5. **Close merges:** runs are serialized (`concurrency`). Each run checks out the branch head, so a merge that lands right after another bumps on top of it instead of being rejected.

**Repository setup:**

- Settings → Actions → Workflow permissions must be **Read and write**.
- If `master` is protected, allow GitHub Actions to bypass the protection.

## Releases

Release tags are `vX.Y.Z`, cut from `master` ([release](release.md)).
