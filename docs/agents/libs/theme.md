---
title: Theme (light / dark / system)
scope: src/shared/providers/ThemeProvider.tsx, src/app/settings/_components/ThemePicker.tsx, src/app/main.tsx
applies_to: theme switching, reading the active theme
related:
  - ./tailwind-daisyui.md
  - ./react-query.md
  - ../design.md
---

# Theme

> The preference (`system` | `light` | `dark`) is `settings.general.theme` in SQLite. `ThemeProvider` resolves it with `resolveTheme(pref, prefersDark)` and writes `<html data-theme="light|dark">`. With `system`, it follows `prefers-color-scheme` live. Changes reach every window via `settings://changed`.

## Flow

1. **Startup (`main.tsx`):** awaits `getSettings()`, seeds the query cache, and calls `applyTheme(settings.general.theme)` before the first render, so there is no flash.
2. **`ThemeProvider`:**
   - Applies the theme whenever the preference changes.
   - While the preference is `system`, it subscribes to `matchMedia('(prefers-color-scheme: dark)')`.
   - It owns the `settings://changed` listener for the whole `['settings']` cache (the language follows from the same cache).
3. **`ThemePicker`** (Settings → General): three preview cards (System / Light / Dark) in a `radiogroup` that call `patchSettings({ general: { theme } })`. Rust validates, stores and emits the event.

## Rules

- **Pure logic:** keep it in `src/utils/theme.ts` (`resolveTheme`), tested with `node:test`.
- **Theme names:** the daisyUI themes are `light` and `dark`. Don't add a third.
- **Reading the theme in a component:** read `document.documentElement.dataset.theme`, or `useSettings().settings.general.theme` if you need the preference itself.
- **Theme-specific styling:** use the tokens. Never branch on the theme in JS to choose colors.
