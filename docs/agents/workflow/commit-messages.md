---
title: Commit Message Convention
scope: "**"
applies_to: writing a commit message
related:
  - ./branching.md
  - ./pr.md
---

# Commit Message Convention

> `<prefix>: <subject>` — one line, imperative mood, lowercase prefix. Optional body after a blank line explains *why*.

| Prefix | Use for |
| ------ | ------- |
| `feat` | New feature. |
| `fix` | Bug fix on existing functionality. |
| `refactor` | Restructuring with no behavior change. |
| `build` | Build tooling, dependencies, release/version scripts, CI config. |
| `docs` | Documentation or comment-only changes. |
| `test` | Adding or modifying tests. |
| `chore` | Maintenance with no code impact — e.g. the bot's `chore: bump version to X.Y.Z [skip ci]`. |

Examples:

```text
feat: add note search command
fix: keep theme when stored value is unknown
build: bump tauri to 2.13
docs: document sqlite migration rules
test: cover set_locale rejection
```

- `[skip ci]` is reserved for the version-bump bot; it stops the bump workflow from re-triggering.
- Body lines wrap at ~72 characters.
