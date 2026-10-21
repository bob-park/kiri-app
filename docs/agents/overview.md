---
title: Project Overview
scope: "**"
applies_to: onboarding, deciding whether a change fits the template
related:
  - ./tech-stack.md
  - ./architecture.md
  - ./structure.md
---

# Project Overview

> Tauri 2 desktop-app template (Rust + React). It ships a runnable skeleton — main window, settings window, tray, SQLite, auto-update, i18n — plus release/versioning scripts. New apps are built by adding domains on top of it.

## What it is

- A **template repository**: clone it, rename the placeholders (README → "새 앱으로 만들 때 바꿀 것"), then build features.
- The skeleton is intentionally minimal but complete end to end. Every layer has one working example to copy:
  - frontend → command → domain → SQLite: `settings` (greeting, theme, language, auto-update)
  - app shell: window lifecycle, app menu, tray, updater, launch at login
  - release: `scripts/release.sh`, `scripts/release.ps1`, `scripts/latest-json.mjs`
  - versioning: `bump-version-up.sh` + `.github/workflows/bump-version.yml`

## What it is not

- Not a component library. UI pieces exist only where the skeleton needs them (`src/shared/components/settings/*`).
- No HTTP backend. The "backend" is the Rust side of the same app, reached through Tauri commands.
- No CI build. Releases are built, signed and uploaded from personal machines ([release](./workflow/release.md)).

## Platforms

- macOS: Apple Silicon only, signed and notarized.
- Windows: x64 (NSIS installer). No code signing; only the updater `.sig`.

## Placeholder names

`App` (productName) · `com.bobpark.app` (identifier) · `template-rust-app` / `template_rust_app_lib` (crate) · `bob-park/template-rust-app` (repo). The full rename checklist is in [README.md](../../README.md).
