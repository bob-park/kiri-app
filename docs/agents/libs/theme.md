---
title: Theme (light / dark / system)
scope: src/shared/providers/theme/**, src/shared/components/theme/**, src/app/main.tsx
applies_to: theme switching, reading the active theme
related:
  - ./tailwind-daisyui.md
  - ./react-query.md
  - ../design.md
---

# Theme

> The preference (`system` | `light` | `dark`) is stored in SQLite (`settings.theme`). `ThemeProvider` resolves it with `resolveTheme(pref, prefersDark)` and writes `<html data-theme>`. With `system`, it follows `prefers-color-scheme` live. Changes reach every window via `settings://theme-changed`.

## Flow

1. **Startup (`main.tsx`):** awaits `getTheme()`, seeds the query cache, and calls `applyTheme(pref)` before the first render, so there is no flash.
2. **`ThemeProvider`:**
   - Applies the theme whenever the preference changes.
   - While the preference is `system`, it subscribes to `matchMedia('(prefers-color-scheme: dark)')`.
   - It also listens for `settings://theme-changed` and updates the cache.
3. **`ThemeSwitcher`:** a `Segmented` control (System / Light / Dark) that calls `useThemeUpdate`. Rust validates the value, stores it, and emits the event.

## Rules

- **Pure logic:** keep it in `src/utils/theme.ts` (`resolveTheme`), tested with `node:test`.
- **Reading the theme in a component:** read `document.documentElement.dataset.theme`, or call `useThemePref()` if you need the preference itself.
- **Theme-specific styling:** use the tokens. Never branch on the theme in JS to choose colors.
