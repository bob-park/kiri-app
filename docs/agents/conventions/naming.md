---
title: Naming
scope: "**"
applies_to: creating any file, type, function, command, or event
related:
  - ./module-aliases.md
  - ./tauri-commands.md
  - ../structure.md
---

# Naming

> Domains share one lowercase name across `src/domain/<name>` and `src-tauri/src/domain/<name>`. Rust is snake_case. TS functions are camelCase and mirror the command (`get_settings` → `getSettings`). Components use `PascalCase.tsx`, hooks `useXxx.tsx`, DTOs `*.dto.ts`.

## Frontend files

| Kind | Pattern | Example |
| ---- | ------- | ------- |
| Command wrappers | `apis/<domain>.ts` | `apis/settings.ts` |
| Wire types | `apis/<domain>.dto.ts` | `apis/updater.dto.ts` |
| Query hooks | `queries/<domain>.tsx` | `queries/settings.tsx` |
| Components | `PascalCase.tsx`, default export | `JobRow.tsx` |
| Hooks | `useXxx.tsx`, default export | `useExitAnimation.tsx` |
| Zustand | `store/slice.ts` + `store/<domain>.state.ts` | — |
| Pure utils | `camelCase.ts` + `camelCase.test.ts` | `utils/theme.ts` |
| Screen | `<Name>App.tsx` under `src/app/<screen>/` | `settings/SettingsApp.tsx` |

Sub-resources get their own files with the domain prefix: `apis/noteTags.ts`, `queries/noteTags.tsx`, and query key `['notes', 'tags', …]`.

## Identifiers

- Query hooks: `useXxx` for reads (`useSettings`, `useJobs`) and `useXxx<Verb>` for mutations (`useSettingsPatch`, `useJobAdd`, `useUpdateCheck`). A mutation hook returns `{ <verb>Xxx, isLoading }` (`{ patchSettings, isLoading }`).
- Query keys: `[domain, resource, ...specifier]`, e.g. `['queue', 'jobs']`, `['tools', 'status']`. Export a `XXX_QUERY_KEY` const when other code (providers, `main.tsx`, event listeners) needs it.
- i18n keys: nested by area (`t.settings.tab.general`, `t.errors.INVALID_URL`). camelCase leaves (error codes stay SCREAMING_SNAKE); functions for interpolation (`version: (v) => …`).

## Rust

- Modules and files: snake_case. Types: PascalCase. Functions: snake_case.
- Commands: `verb_noun`, unique app-wide (`get_settings`, `patch_settings`, `add_job`, `check_update`). Domain API functions read as verbs on the domain (`settings::get`, `settings::patch`, `queue::import_json`).
- Events: `<domain>://<kebab-event>`, kept in a `const` (`SETTINGS_CHANGED` = `settings://changed`).
- Settings: the `settings` table key is a section name (`general`, `download`, `update`); fields inside the section JSON are snake_case (`ui_language`, `max_concurrent`, `auto_check`), and the frontend DTO keeps them as is.
