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

> `master` is the only long-lived branch. Work on `feature/<topic>` or `hotfix/<topic>` (lowercase kebab-case), branched from `master`, and merge back through a PR. Every merge into `master` bumps the version.

| Branch | Purpose |
| ------ | ------- |
| `master` | The released line. Every PR merge bumps the version ([versioning](versioning.md)); releases are built and tagged from here ([release](release.md)). |
| `feature/<topic>` | Feature work, refactors, docs. Branch from `master`, PR back to `master`. |
| `hotfix/<topic>` | Urgent fix to a shipped version. Same flow as a feature; the name only signals urgency. |

Rules:

- `<topic>` is lowercase kebab-case (`feature/aurora-redesign`, `hotfix/updater-signature`).
- No direct commits to `master` — always a PR. The version-bump bot is the only exception.
- Release tags are `v<X.Y.Z>`, created on `master` when a release is published.
