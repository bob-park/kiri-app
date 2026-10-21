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

> Domains share one lowercase name across `src/domain/<name>` and `src-tauri/src/domain/<name>`. Rust is snake_case. TS functions are camelCase and mirror the command (`get_theme` → `getTheme`). Components use `PascalCase.tsx`, hooks `useXxx.tsx`, DTOs `*.dto.ts`.

## Frontend files

| Kind | Pattern | Example |
| ---- | ------- | ------- |
| Command wrappers | `apis/<domain>.ts` | `apis/settings.ts` |
| Wire types | `apis/<domain>.dto.ts` | `apis/updater.dto.ts` |
| Query hooks | `queries/<domain>.tsx` | `queries/settings.tsx` |
| Components | `PascalCase.tsx`, default export | `UpdateSection.tsx` |
| Hooks | `useXxx.tsx`, default export | `useSettingsShortcut.tsx` |
| Zustand | `store/slice.ts` + `store/<domain>.state.ts` | — |
| Pure utils | `camelCase.ts` + `camelCase.test.ts` | `utils/theme.ts` |
| Screen | `<Name>App.tsx` under `src/app/<screen>/` | `settings/SettingsApp.tsx` |

Sub-resources get their own files with the domain prefix: `apis/noteTags.ts`, `queries/noteTags.tsx`, and query key `['notes', 'tags', …]`.

## Identifiers

- Query hooks: `useXxx` for reads (`useThemePref`) and `useXxxUpdate` / verb-based names for mutations (`useThemeUpdate`, `useUpdateCheck`). A mutation hook returns `{ <verb>Xxx, isLoading }`.
- Query keys: `[domain, resource, ...specifier]`, e.g. `['settings', 'theme']`. Export a `XXX_QUERY_KEY` const when other code (providers, `main.tsx`) needs it.
- i18n keys: nested by area (`t.settings.theme`, `t.updates.check`). camelCase leaves; functions for interpolation (`version: (v) => …`).

## Rust

- Modules and files: snake_case. Types: PascalCase. Functions: snake_case.
- Commands: `verb_noun`, unique app-wide. Domain API functions read as nouns or verbs on the domain (`settings::theme`, `settings::set_theme`).
- Events: `<domain>://<kebab-event>`, kept in a `const` (`THEME_CHANGED`).
- Settings keys in the `settings` table: snake_case (`auto_update`, `locale`).
