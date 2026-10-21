---
title: Tailwind 4 + daisyUI 5
scope: src/**/*.{tsx,css}
applies_to: styling a component, adding a color token
related:
  - ../design.md
  - ./theme.md
---

# Tailwind 4 + daisyUI 5

> Tailwind v4 via `@tailwindcss/vite`, configured entirely in `src/app/globals.css` (no `tailwind.config.js`). daisyUI 5 provides the `light`/`dark` themes. Compose conditional classes with `classnames` (`cx`).

## `globals.css` layout

1. `@import "tailwindcss";` and `@plugin "daisyui" { themes: false; }`, which disables the built-in themes.
2. Two `@plugin "daisyui/theme"` blocks, `light` (default) and `dark` (`prefersdark`). They hold the core palette and radius.
3. `[data-theme="light"]` / `[data-theme="dark"]` blocks for the extra tokens: `--vee-card`, `--vee-divider`, `--vee-subtle`, `--vee-muted`, `--vee-accent-subtle`.
4. `@theme inline` maps those to Tailwind colors (`bg-card`, `text-muted`, `border-divider`, `bg-subtle`, `bg-accent-subtle`) and sets `--font-sans`.
5. `@layer base`: body font size (13px), `user-select: none`, and the focus ring.

## Rules

- **Adding a color:** add it to **both** the light and dark blocks, then expose it through `@theme inline`. Never write a literal color in a component.
- **Components:**
  - Prefer the settings building blocks and plain Tailwind utilities.
  - daisyUI component classes (`btn`, `card`, `toggle`, …) are available, but the skeleton uses its own vee-style blocks. Don't mix both styles on one screen.
- **Conditional classes:** use `cx('base', { 'on-class': cond })` from `classnames`. Don't build class strings with template literals.
- **Arbitrary values** (`w-[72px]`, `text-[11px]`): use them only to match a design spec; prefer the scale otherwise.
- **Class order:** Prettier sorts classes. Run `yarn prettier`.
