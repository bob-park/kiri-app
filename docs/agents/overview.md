---
title: Project Overview
scope: "**"
applies_to: onboarding, deciding whether a change fits kiri
related:
  - ./tech-stack.md
  - ./architecture.md
  - ./structure.md
---

# Project Overview

> kiri is a macOS (Apple Silicon) menu-bar app that downloads YouTube videos: paste a link with ⌘V, pick quality/format/subtitles, and a queue downloads with yt-dlp and converts with ffmpeg. A `kiri` CLI controls the running app's queue. Tauri 2 (Rust) + React.

## What it is

- **The app** (`src-tauri` + `src/`): a `main` window (queue, options sheet) and a `settings` window (General / Download / CLI / Updates), a menu-bar tray, auto-update from GitHub Releases.
- **The engine** (`crates/kiri-core`): Tauri-free queue, download pipeline (yt-dlp → ffmpeg → move), tool installer (yt-dlp, FFmpeg, Deno; missing ones installed at startup, updates manual), CLI socket protocol.
- **The CLI** (`crates/kiri-cli`, binary `kiri`): bundled as a sidecar and linked to `/usr/local/bin/kiri` from Settings → CLI. It talks to the running app over a Unix socket.
- **Storage:** one SQLite file, `kiri.db`, for settings and the queue.

## What it is not

- Not cross-platform. No Windows or Linux build; macOS arm64 only (minimum macOS 14.2).
- No HTTP backend. The "backend" is the Rust side of the same app, reached through Tauri commands (UI) or the IPC socket (CLI).
- No CI build. Releases are built, signed, notarized and uploaded from a personal Mac ([release](./workflow/release.md)). GitHub Actions only bumps the version.
- No launch-at-login (no autostart plugin), no drag-and-drop input, no in-app player.

## Names

| Thing | Value |
| ----- | ----- |
| Product name / window title | `kiri` |
| Bundle identifier | `org.bobpark.kiri` |
| Cargo packages | `kiri-app` (lib `kiri_lib`, in `src-tauri`), `kiri-core`, `kiri-cli` (binary `kiri`) |
| npm package | `kiri` |
| Repository | `bob-park/kiri-app` |
