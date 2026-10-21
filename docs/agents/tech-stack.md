---
title: Tech Stack
scope: "**"
applies_to: checking a library or tool version, adding a dependency
related:
  - ./overview.md
  - ./build-run.md
---

# Tech Stack

> Versions are pinned in `.mise.toml` (Node/Yarn), `package.json` (frontend), and `src-tauri/Cargo.toml` (Rust). Ask the user before adding, removing, or bumping a dependency.

| Area | Choice | Pinned in |
| ---- | ------ | --------- |
| Toolchain | Node 26, Yarn 4.18.1 (`nodeLinker: node-modules`) via mise | `.mise.toml`, `package.json#packageManager` |
| Rust | stable, edition 2024 | `src-tauri/Cargo.toml` |
| Desktop shell | Tauri 2 (`tray-icon`, `image-png`) | `src-tauri/Cargo.toml` |
| Tauri plugins | single-instance, log, updater, autostart | `src-tauri/Cargo.toml` |
| DB | SQLite via `rusqlite` (`bundled`) | `src-tauri/Cargo.toml` |
| Errors | `thiserror` | `src-tauri/Cargo.toml` |
| OS locale | `sys-locale` (native tray/menu labels) | `src-tauri/Cargo.toml` |
| UI | React 19, TypeScript 6, Vite 8 | `package.json` |
| Styling | Tailwind CSS 4 (`@tailwindcss/vite`) + daisyUI 5, `classnames` | `package.json` |
| Server state | `@tanstack/react-query` 5 (+ devtools in dev) | `package.json` |
| Client state | `zustand` 5 + `immer` | `package.json` |
| Lint/format | ESLint 9 + `@bob-park/eslint-config-bobpark`, Prettier + `@bob-park/prettier-config-bobpark` (GitHub Packages) | `package.json`, `eslint.config.mjs`, `.yarnrc.yml` |
| Tests | `cargo test`, Node built-in `node:test` | — |

Notes:

- TypeScript stays on 6.x: `typescript-eslint` (pulled in by the bobpark ESLint config) does not support TS 7 yet.
- `.yarnrc.yml` sets `npmMinimalAgeGate: 0` (no age gate) and `enableScripts: true` (dependency install scripts run).
