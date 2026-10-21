---
title: Pull Requests
scope: "**"
applies_to: opening a PR or titling one
related:
  - ./branching.md
  - ./versioning.md
  - ./commit-messages.md
---

# Pull Requests

> PRs target `develop` (hotfixes and release promotion target `main`). The PR title (or a `major`/`minor` label) decides the version bump when it merges into `develop`.

## Base Branch

| Source branch | PR base |
| ------------- | ------- |
| `feature/**` and other day-to-day branches | `develop` |
| `hotfix/**` | `main` (then back-merge into `develop`) |
| release promotion (`develop` → `main`) | `main` |

## Title

A short, imperative summary. **Do not** prefix it with `feat:`/`fix:` — those belong on commits.

Merging into `develop` (or a `hotfix/*` PR into `main`) runs `.github/workflows/bump-version.yml`:

| PR title contains / label | Bump |
| ------------------------- | ---- |
| `[major]` or label `major` | `major` += 1, minor and patch reset to 0 |
| `[minor]` or label `minor` | `minor` += 1, patch reset to 0 |
| anything else | `patch` += 1 |

Examples:

```text
add note search
[minor] add notes domain with sync
[major] migrate settings storage to a new schema
```

Both merge commits (`Merge pull request #N …`) and squash merges (`Title (#N)`) are recognized.
