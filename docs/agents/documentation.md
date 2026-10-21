---
title: Documentation
scope: docs/**, AGENTS.md, CLAUDE.md, README.md
applies_to: adding or updating docs
related:
  - ./structure.md
---

# Documentation

> Agent rules live in `docs/agents/` (English, frontmatter + TL;DR). Human guides are `docs/development.md` and `docs/release.md` (Korean). The design system is in `docs/design/`. Superpowers specs and plans go in `docs/superpowers/{specs,plans}/`.

| Path | Content | Language |
| ---- | ------- | -------- |
| `AGENTS.md` | map of agent docs + task routes | English |
| `CLAUDE.md` | `@AGENTS.md` + Claude Code-only rules | Korean |
| `docs/agents/**` | conventions, libs, workflow | English |
| `docs/design/` | design system prompts/specs — read before UI work | as written |
| `docs/development.md` | dev setup, run, test, build, project tree | Korean |
| `docs/release.md` | release procedure | Korean |
| `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md` | design specs | Korean |
| `docs/superpowers/plans/YYYY-MM-DD-<topic>.md` | implementation plans | English |

## Rules

- **Agent doc format:** every file in `docs/agents/` starts with frontmatter (`title`, `scope`, `applies_to`, `related`), then `# Title`, then a one-line `> TL;DR`.
- **New agent docs:** add each one to the Map in `AGENTS.md`, and to "How to use" if a task should route there.
- **Docs follow the code:** when code changes a documented path, command, or rule, update the doc in the same PR.
- **Links:** use relative links.
