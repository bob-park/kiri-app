---
title: Tech Stack
scope: "**"
applies_to: checking a library or tool version, adding a dependency
related:
  - ./overview.md
  - ./build-run.md
---

# Tech Stack

> Versions are pinned in `.mise.toml` (Node/Yarn), `package.json` (frontend), and the Cargo manifests (`Cargo.toml` workspace, `src-tauri/Cargo.toml`, `crates/*/Cargo.toml`). Ask the user before adding, removing, or bumping a dependency.

| Area | Choice | Pinned in |
| ---- | ------ | --------- |
| Toolchain | Node 26, Yarn 4.18.1 (`nodeLinker: node-modules`) via mise | `.mise.toml`, `package.json#packageManager` |
| Rust | stable, edition 2024, Cargo workspace (`resolver = "3"`) | `Cargo.toml`, each crate's `Cargo.toml` |
| Desktop shell | Tauri 2 (`tray-icon`, `image-png`) | `src-tauri/Cargo.toml` |
| Tauri plugins | single-instance, log, opener, dialog, clipboard-manager, updater | `src-tauri/Cargo.toml`, `package.json` (JS halves) |
| DB | SQLite via `rusqlite` (`bundled`) | `src-tauri/Cargo.toml` |
| Errors | `thiserror` | `src-tauri/Cargo.toml`, `crates/kiri-core/Cargo.toml` |
| OS locale | `sys-locale` (native tray/menu labels) | `src-tauri/Cargo.toml` |
| Engine | `tokio` (process, net, time), `reqwest` (rustls), `sha2` | `crates/kiri-core/Cargo.toml` |
| CLI | `clap` 4 (derive) | `crates/kiri-cli/Cargo.toml` |
| External tools | yt-dlp, FFmpeg (ffmpeg + ffprobe) and Deno, downloaded at runtime into the app data dir | `crates/kiri-core/src/tools.rs` |
| UI | React 19, TypeScript 6, Vite 8 | `package.json` |
| Styling | Tailwind CSS 4 (`@tailwindcss/vite`) + daisyUI 5, `classnames`, Pretendard (`pretendard` npm package) | `package.json` |
| Server state | `@tanstack/react-query` 5 (+ devtools in dev) | `package.json` |
| Client state | `zustand` 5 + `immer` | `package.json` |
| Lint/format | ESLint 9 + `@bob-park/eslint-config-bobpark`, Prettier + `@bob-park/prettier-config-bobpark` (GitHub Packages) | `package.json`, `eslint.config.js`, `.yarnrc.yml` |
| Tests | `cargo test --workspace`, Node built-in `node:test` | — |

Notes:

- ESLint stays on 9.x (`^9.39`) to match the bobpark config; move to 10 only together with that config.
- TypeScript stays on 6.x: `typescript-eslint` (pulled in by the bobpark ESLint config) does not support TS 7 yet.
- `.yarnrc.yml` points the `@bob-park` scope at GitHub Packages, so `yarn install` needs `GITHUB_NPM_AUTH_TOKEN`.
