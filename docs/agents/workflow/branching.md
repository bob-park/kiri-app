---
title: Branch Naming
scope: "**"
applies_to: starting a branch
related:
  - ./pr.md
  - ./versioning.md
  - ./commit-messages.md
---

# Branch Naming

> `main` = released code, `develop` = next release. Work on `feature/<topic>` or `hotfix/<topic>` (lowercase kebab-case) and merge through a PR.

| Branch | Purpose |
| ------ | ------- |
| `main` | Production-ready code. Releases are built and tagged from here (see [release](release.md)). |
| `develop` | Integration branch for the next release. Every merge bumps the version ([versioning](versioning.md)). |
| `feature/<topic>` | Feature work. Branch from `develop`, PR back to `develop`. |
| `hotfix/<topic>` | Urgent fix to a release. Branch from `main`, PR to `main` (the merge bumps the patch version), release, then merge `main` back into `develop`. |

Rules:

- `<topic>` is lowercase kebab-case (`feature/note-search`, `hotfix/updater-signature`).
- No direct commits to `main` or `develop` — always a PR. Base-branch rules: [pr](pr.md#base-branch).
- Release tags are `v<X.Y.Z>` and are created by the release scripts on `main`.
