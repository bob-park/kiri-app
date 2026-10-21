---
title: Tailwind 4 + daisyUI 5
scope: src/**/*.{tsx,css}
applies_to: styling a component, adding a color token or animation
related:
  - ../design.md
  - ./theme.md
---

# Tailwind 4 + daisyUI 5

> Tailwind v4 via `@tailwindcss/vite`, configured entirely in `src/app/globals.css` (no `tailwind.config.js`). daisyUI 5 provides the `light`/`dark` themes; Aurora Glass tokens and animations sit next to them. Compose conditional classes with `classnames` (`cx`).

## `globals.css` layout

1. `@import 'tailwindcss'`, the Pretendard variable font CSS, and `@plugin 'daisyui' { themes: false; exclude: rootcolor, glass; }` — built-in themes off, daisyUI's `glass` replaced by ours.
2. Two `@plugin 'daisyui/theme'` blocks, `dark` and `light` (default). They hold the core palette and radius.
3. `@theme` sets `--font-sans` (Pretendard first).
4. `@theme static` defines the Aurora Glass tokens with light values (`--color-glass`, `--color-aurora-1..3`, `--color-muted`, `--color-track`, `--color-scrim`, `--color-preview-*`, `--gradient-progress*`, `--shadow-glass`, `--shadow-keycap`, `--ease-spring`, `--animate-*`). `static` keeps every variable in the output even when no class uses it.
5. `[data-theme='dark']` overrides the tokens that differ in dark.
6. Base: `body` background/text, font, 14px, `user-select: none`.
7. `@layer components`: `.float-shadow`, `.chip*`, `.glass`, `.aurora` / `.aurora-blob`.
8. `@keyframes` for every `--animate-*`, toast animations, and the `prefers-reduced-motion: reduce` block that turns them all off.

## Rules

- **Adding a color:** add it in `@theme static` (light) **and** `[data-theme='dark']`, or say in a comment why one value serves both (the `preview-*` tokens). Never write a literal color in a component.
- **Gradients** are variables, used as `bg-(image:--gradient-progress)`.
- **Adding an animation:** define `--animate-x` + `@keyframes x` here, follow the motion rules in [design](../design.md), and add `.animate-x` to the reduced-motion block.
- **Components:**
  - Prefer the settings building blocks and plain Tailwind utilities with the tokens.
  - daisyUI component classes (`btn`, `card`, `toggle`, …) are available, but kiri uses its own Aurora Glass blocks. Don't mix both styles on one screen.
- **Conditional classes:** use `cx('base', { 'on-class': cond })` from `classnames`. Don't build class strings with template literals.
- **Arbitrary values** (`w-[72px]`, `text-[11px]`): use them only to match a design spec; prefer the scale otherwise.
- **Class order:** Prettier sorts classes. Run `yarn prettier`.
