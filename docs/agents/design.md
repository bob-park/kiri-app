---
title: Design
scope: src/**/*.{tsx,css}
applies_to: any UI work — new screen, component, color, spacing
related:
  - ./libs/tailwind-daisyui.md
  - ./libs/theme.md
  - ./conventions/react-sections.md
---

# Design

> **Read [`docs/design/`](../design/) before building or changing any UI.** Colors exist only in `src/app/globals.css`. Components use semantic classes. Check every change in light **and** dark.

## Sources of truth

1. [`docs/design/`](../design/) — the app's design system (currently `kraken-design.md`: purple brand, near-black text, cool gray neutrals, 12px radius, whisper-level shadows).
2. The existing skeleton UI, which follows vee-app's look: settings tab bar, bordered section cards, label-left/control-right rows, segmented radio groups, switches, tinted buttons, Helvetica Neue at 13px.

When the two disagree, ask the user. Do not invent a third style.

## Rules

- **Colors live only in `src/app/globals.css`.**
  - The `light`/`dark` daisyUI theme blocks hold the core colors.
  - Tokens daisyUI has no slot for live in the `[data-theme]` blocks: `card`, `divider`, `subtle`, `muted`, `accent-subtle`.
  - Never write a hex/rgb value in a component.
- **Use semantic classes:**
  - surfaces: `bg-base-100`, `bg-card`, `bg-subtle`
  - text: `text-base-content`, `text-muted`, `text-primary`
  - borders: `border-base-300`, `border-divider`
- A new token must get a value in **both** the light and dark blocks.
- **Reuse the settings building blocks** in `src/shared/components/settings/` before writing new ones:
  - `SettingSection`, `SettingRow`
  - `Segmented`: for a pick-one choice of 2–4 options, use this instead of a `<select>`
  - `Switch`: for on/off
  - `Button`
- Every string comes from `useI18n().t` ([i18n](./libs/i18n.md)).
- **Accessibility:**
  - Keep the roles the building blocks use: `radiogroup`/`radio`, `switch`, `tablist`/`tab`.
  - Keep the `:focus-visible` ring.
  - Icon-only controls need an `aria-label`.
- **Check before you finish:**
  - Look at the screen in Light and Dark (Settings → Theme).
  - Look at it in 한국어 and English; Korean strings are often longer.
