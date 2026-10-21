# Aurora Glass

kiri's visual language since the 2026-10 redesign: translucent glass surfaces floating over a slowly drifting aurora of brand-colored light. Both windows (main and settings) use it. Brand colors come from [kraken-design.md](./kraken-design.md); where the two disagree (fonts, radius, surfaces, shadows), **this file wins**.

All values below live in `src/app/globals.css`. Components use the token classes, never literal colors.

## 1. Palette

### Core (daisyUI `light` / `dark` themes)

| Role | Light | Dark | Class |
| ---- | ----- | ---- | ----- |
| Page background | `#f6f5fb` | `#121318` | `bg-base-100` |
| Raised neutral | `#f6f6f9` | `#1d1e25` | `bg-base-200` |
| Border | `#dedee5` | `#2c2e38` | `border-base-300` |
| Text | `#101114` | `#ececf2` | `text-base-content` |
| Brand (primary, accent) | `#7132f5` | `#8b5cff` | `bg-primary`, `text-primary` |
| Brand tint (selected tab/chip) | `rgba(133,91,251,.16)` on `#5b1ecf` text | `rgba(139,92,255,.22)` on `#c4b0ff` | `bg-secondary text-secondary-content` |
| Info | `#5741d8` | `#9b7cff` | `text-info` |
| Success (completed) | `#149e61` | `#4cd393` | `bg-success`, `text-success` |
| Warning | `#f59e0b` | `#fbbf24` | `text-warning` |
| Error (failed) | `#d33a4a` | `#f3727f` | `text-error` |

Radius: boxes 14px, fields 12px, selectors 6px. No daisyUI depth or noise.

### Aurora Glass tokens (`@theme static`, dark overrides in `[data-theme='dark']`)

| Token | Light | Dark | Use |
| ----- | ----- | ---- | --- |
| `--color-glass` | `rgba(255,255,255,.58)` | `rgba(30,31,40,.55)` | glass fill |
| `--color-glass-edge` | `rgba(255,255,255,.85)` | `rgba(255,255,255,.08)` | glass hairline border |
| `--color-aurora-1` / `-2` / `-3` | `#a98bff` / `#7fd4ff` / `#ffb3d9` | `#5b3bd6` / `#1f6f9e` / `#7a2d63` | the three background blobs (purple, sky, pink) |
| `--color-muted` | `#686b82` | `#8b8ea3` | secondary text (`text-muted`) |
| `--color-track` | `rgba(113,50,245,.12)` | `rgba(139,92,255,.18)` | progress track, skeleton base |
| `--color-scrim` | `rgba(246,245,251,.35)` | `rgba(0,0,0,.3)` | modal backdrop (`backdrop:bg-scrim`) |
| `--gradient-progress` | `#7132f5 → #5fc3ff` | `#8b5cff → #5fc3ff` | downloading bar, primary CTA, thumbnail placeholder |
| `--gradient-progress-encoding` | `#5741d8 → #7fd4ff` | `#6f63ff → #7fd4ff` | converting bar (a different tone from downloading) |
| `--shadow-glass` | `0 4px 24px rgba(16,17,20,.05)` | same | resting glass |
| `--shadow-keycap` | `0 2px 0 rgba(16,17,20,.06)` | same | ⌘ / V keycaps |
| `--color-preview-light`, `-light-bar`, `-light-edge` | `#f6f5fb`, `#ffffff`, `#e6e4f0` | same | theme picker thumbnail of the light theme |
| `--color-preview-dark`, `-dark-bar` | `#121318`, `#262833` | same | theme picker thumbnail of the dark theme |

The `--color-preview-*` tokens are deliberately **not** overridden in dark: each thumbnail depicts one fixed theme, whichever theme is active.

A new token gets a value for both themes (or a comment saying why one value serves both).

## 2. Typography

- Pretendard Variable (npm `pretendard`), falling back to `-apple-system`, Helvetica Neue. Chosen for Korean rendering.
- Body 14px. Small UI text uses arbitrary sizes from the mockups (10–13px) where needed.
- The logo is `kiri.` with the dot in the brand color.

## 3. Glass

- `.glass` = `--color-glass` fill + 1px `--color-glass-edge` border + `backdrop-filter: blur(18px) saturate(1.4)`. daisyUI's own `glass` class is excluded so ours wins.
- Pair it with `shadow-glass` at rest; floating layers (sheet, toasts, popovers) use `.float-shadow` instead.
- Used for: job cards, the options sheet, toasts, the status bar, the settings tab bar and sections, keycaps, the empty-state icon pedestal.

## 4. Aurora background

```html
<div class="aurora" aria-hidden="true">
  <span class="aurora-blob"></span><span class="aurora-blob"></span><span class="aurora-blob"></span>
</div>
```

- Placed first inside a `relative` window root; content after it sits above (`z-index: 0` on the aurora).
- Three blurred (`46px`) blobs drift on a 12 s loop with staggered delays. **Never more than three** — `.aurora-blob:nth-child(n+4)` is hidden. Lower the count or blur if performance drops.

## 5. Main window

- **Header:** the `kiri.` logo and a settings button whose gear turns on hover. "Clear completed" stays available.
- **Empty state:** a floating download icon on a glass pedestal, `⌘` `V` keycaps and the hint to paste a YouTube link. Pressing ⌘V plays the keycap press.
- **Options sheet:** a `<dialog>` modal floating 10px from the window edges (`inset-x-2.5 bottom-2.5`, radius 20px) over a blurred scrim, springing up from the bottom. Loading video info shows a shimmering skeleton. Selected quality/format chips grow slightly and switch to the brand fill. The download button uses `--gradient-progress` and scales down when pressed.
- **Job card:** one glass card with thumbnail, title, status text and percent. The bar flows with `--gradient-progress` (downloading) or `--gradient-progress-encoding` (converting), turns `bg-success` when completed, and a ✓ badge pops in with a spreading ring. New cards drop in with the spring; removed cards collapse out.
- **Toasts and status bar:** glass, with enter/exit animations.
- Input is ⌘V only: no drop zone, no drag-and-drop, no play button.

## 6. Settings window

- Same aurora and glass. A glass tab bar (General / Download / CLI / Updates) whose selection pill slides between tabs.
- Rows use the building blocks in `src/shared/components/settings/` (`SettingSection`, `SettingRow`, `Segmented`, `Switch`, `Button`).
- **Theme picker:** three preview cards (System / Light / Dark) drawn with the `--color-preview-*` tokens; System shows the dark thumbnail over the light one, cut along a diagonal. The selected card lifts with a primary border and its dot springs in. It is a `radiogroup` with arrow-key navigation.

## 7. Motion

| Kind | Duration | Easing |
| ---- | -------- | ------ |
| Enter (cards, sheet, badges, tab pill, theme card) | 450–650 ms | spring `cubic-bezier(.2,.9,.3,1.2)` (`ease-spring`) |
| Exit (card collapse, toast out, sheet close) | 150–250 ms | `ease-in` |
| Hover/press feedback | 150–250 ms | default |
| Ambient loops | drift 12 s, float 4 s, flow 1.6 s, shimmer 1.2 s | linear / ease-in-out |

Keyframes and `--animate-*` utilities: `drift`, `float`, `spring-in`, `pop`, `ring`, `flow`, `shimmer`, `sheet-up`, `key-press`, `collapse-out`, plus `toast-in` / `toast-out`.

Rules:

- Animate only `transform` and `opacity`. Allowed exceptions:
  - progress bar **width** (and the `background-position` of `flow` / `shimmer`);
  - `filter: blur()` in the first frame of `spring-in` (the card sharpens as it lands);
  - `box-shadow` in `ring` (the completion ring) and `key-press` (the keycap loses its shadow while pressed).
- Exits that remove an element go through `useExitAnimation(name)` so the element stays mounted until its animation ends.
- **Reduced motion:** under `prefers-reduced-motion: reduce` every animation above is off — no drift, float, spring or flow — and only the final state is shown. Exit animations jump straight to hidden, and `useExitAnimation` runs its callback at once. Transitions use `motion-reduce:transition-none`.

## 8. Do / Don't

- Do keep every color, gradient and shadow in `globals.css`, defined for light **and** dark.
- Do check light and dark, Korean and English (Korean strings run longer).
- Don't add a fourth aurora blob, a second glass recipe, or a literal color in a component.
- Don't animate layout properties (height, top, margin) except the exceptions above.
