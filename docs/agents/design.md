---
title: Design
scope: src/**/*.{tsx,css}
applies_to: any UI work — new screen, component, color, spacing, animation
related:
  - ./libs/tailwind-daisyui.md
  - ./libs/theme.md
  - ./conventions/react-sections.md
---

# Design

> **Read [`docs/design/aurora-glass.md`](../design/aurora-glass.md) before building or changing any UI.** kiri uses "Aurora Glass": glass surfaces over a drifting aurora, kraken brand purple, Pretendard 14px, spring motion that turns off under reduced motion. Colors exist only in `src/app/globals.css`. Check every change in light **and** dark, 한국어 **and** English.

## Sources of truth

1. [`docs/design/aurora-glass.md`](../design/aurora-glass.md) — palette and tokens, glass, aurora background, per-window layout, motion rules. The user approved it as the current standard (2026-10 redesign).
2. [`docs/design/kraken-design.md`](../design/kraken-design.md) — the brand palette Aurora Glass derives from (purple `#7132f5`, near-black text, cool gray neutrals, green success). Where it disagrees with Aurora Glass (fonts, radius, flat white surfaces, shadows), Aurora Glass wins.
3. The running app. When a doc and the app disagree, ask the user. Do not invent a new style.

## Rules

- **Colors live only in `src/app/globals.css`.**
  - The `light`/`dark` daisyUI theme blocks hold the core colors.
  - Aurora Glass tokens (`glass`, `glass-edge`, `aurora-1..3`, `muted`, `track`, `scrim`, `preview-*`, `gradient-progress*`, `shadow-glass`, `shadow-keycap`) live in `@theme static`, with dark overrides in `[data-theme='dark']`.
  - Never write a hex/rgb value in a component.
- **Use semantic classes:**
  - surfaces: `bg-base-100`, `.glass` (+ `shadow-glass` or `.float-shadow`), `bg-secondary` (selected)
  - text: `text-base-content`, `text-muted`, `text-primary`, `text-secondary-content`
  - progress: `bg-track`, `bg-(image:--gradient-progress)`, `bg-(image:--gradient-progress-encoding)`, `bg-success`
- A new token must get a value in **both** themes.
- **Motion** ([aurora-glass §7](../design/aurora-glass.md#7-motion)):
  - Enter 450–650 ms with `ease-spring`; exit 150–250 ms.
  - Animate `transform`/`opacity` only. The allowed exceptions are progress width, the `spring-in` blur, and the `ring`/`key-press` box-shadow.
  - Everything is off under `prefers-reduced-motion: reduce`: add the class to the reduced-motion block in `globals.css` and use `motion-reduce:transition-none` on transitions.
  - Use `useExitAnimation` for exits that unmount.
- **Reuse the settings building blocks** in `src/shared/components/settings/` before writing new ones:
  - `SettingSection`, `SettingRow`
  - `Segmented`: for a pick-one choice of 2–4 options, use this instead of a `<select>`
  - `Switch`: for on/off
  - `Button`
- Every string comes from `useI18n().t` ([i18n](./libs/i18n.md)).
- **Accessibility:**
  - Keep the roles the building blocks use: `radiogroup`/`radio`, `switch`, `tablist`/`tab`, `dialog`, `status`.
  - Keep the `:focus-visible` outline.
  - Icon-only controls need an `aria-label`. Decorative layers (aurora) get `aria-hidden`.
- **Check before you finish:**
  - Look at the screen in Light and Dark (Settings → General → Theme).
  - Look at it in 한국어 and English; Korean strings are often longer.
  - Turn on "Reduce motion" (System Settings → Accessibility → Display) and check the final states.
