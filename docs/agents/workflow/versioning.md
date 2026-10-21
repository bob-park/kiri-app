---
title: Versioning
scope: "**"
applies_to: reading or changing the app version, cutting a release
related:
  - ./pr.md
  - ./release.md
---

# Versioning

> Plain semver `X.Y.Z`, starting at `0.0.1`. `src-tauri/tauri.conf.json` is the source of truth. `bump-version-up.sh` keeps four files in sync; **never edit versions by hand**. GitHub Actions runs the bump on every merge into `develop`, and on `hotfix/*` merges into `main`.

## Files carrying the version

`src-tauri/tauri.conf.json` (source of truth) · `package.json` · `src-tauri/Cargo.toml` (`[package]`) · `src-tauri/Cargo.lock` (the `template-rust-app` entry).

## `bump-version-up.sh`

```bash
./bump-version-up.sh [patch|minor|major]   # default: patch
```

- The script validates the version and the argument first. It writes all four files or none: after writing, it checks each file. If one wasn't updated (e.g. `CRATE_NAME` not renamed, so `Cargo.lock` had no match), it restores the backups and exits non-zero.
- It uses `perl -i`, so it works on macOS and on the Ubuntu runner.
- It is tested by `scripts/bump-version-up.test.mjs`.
- No `-rcN`/date suffixes: the Tauri updater compares semver, and prerelease tags would break update ordering.

## Workflow (`.github/workflows/bump-version.yml`)

1. **Trigger:** a push to `develop` or `main`, which happens when a PR merges. The bot's own commit carries `[skip ci]` and is skipped.
2. **Reading the PR:** the workflow finds the PR number from the merge or squash commit, then reads the PR title, labels and head branch ([pr](pr.md#title)).
3. **Which merges bump:**

   | Merged into | PR head | Bump? |
   | ----------- | ------- | ----- |
   | `develop` | any feature/day-to-day branch | yes |
   | `develop` | `main` (hotfix back-merge) | no — the hotfix was already bumped on `main` |
   | `main` | `hotfix/*` | yes — so the hotfix ships as a new version |
   | `main` | `develop` (release promotion) | no — already bumped on `develop` |

4. **Commit:** it runs the script and commits `chore: bump version to X.Y.Z [skip ci]` to the same branch.
5. **Close merges:** runs are serialized per branch (`concurrency`). Each run checks out the branch head, so a merge that lands right after another bumps on top of it instead of being rejected.

**Repository setup:**

- Settings → Actions → Workflow permissions must be **Read and write**.
- If `develop` is protected, allow GitHub Actions to bypass the protection.

## Releases

Release tags are `vX.Y.Z`, cut from `main` by the release scripts ([release](release.md)).
