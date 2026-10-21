# kiri Aurora Redesign & Conventions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring kiri onto the `docs/agents/` architecture (SQLite, React Query, typed i18n, domain folders), make tool readiness instant, save the original before converting, and ship the "Aurora Glass" redesign.

**Architecture:** Four phases on one branch, each leaving all checks green: (1) toolchain, (2) Rust refactor into `app/ domain/ shared/` with SQLite, (3) frontend refactor into `src/{app,domain,shared,utils}`, (4) behavior changes (tools readiness, download flow), (5) redesign + docs. `kiri-core` stays a tauri-free crate; `kiri-cli` is untouched except comments.

**Tech Stack:** Tauri 2, Rust (rusqlite bundled, thiserror, log, tauri-plugin-log, tauri-plugin-single-instance), React 19, TanStack Query 5, zustand 5 + immer, Tailwind 4 + daisyUI 5, classnames, ESLint/Prettier (`@bob-park/*`), `node:test`, Vite 8, Node 26, Yarn 4.18.1.

**Spec:** `docs/superpowers/specs/2026-10-21-redesign-and-conventions-design.md`

## Global Constraints

- Branch `feature/aurora-redesign` from `master`. Commits `<prefix>: <subject>` (`docs/agents/workflow/commit-messages.md`), ending with the session's Co-Authored-By/Claude-Session lines.
- Never hand-edit version fields in `package.json` / `Cargo.toml` / `tauri.conf.json` (bump script owns them).
- Every user-visible string lives in `src/shared/i18n/{en,ko}.ts` (frontend) or `src-tauri/src/app/i18n.rs` (native). Languages: `en`, `ko` only. Stored `ja` reads as `system`.
- Colors only in `src/app/globals.css`; no hex/rgb in components.
- Code comments and identifiers in English.
- Rust domain core (`domain/*/{mod,model,repository}.rs`) and `shared/` never `use tauri`.
- Done means: `cargo test --workspace`, `yarn test`, `yarn typecheck`, `yarn lint` all pass. (Until Task 3 finishes, `yarn lint` is run only on `src/app src/domain src/shared src/utils`.)
- macOS arm64 only. No autostart plugin. No play button. No drag-and-drop input.
- Motion: enter 450–650 ms spring `cubic-bezier(.2,.9,.3,1.2)`, exit 150–250 ms; animate `transform`/`opacity` (progress width exempt); everything off under `prefers-reduced-motion: reduce`.

## Review Focus

1. **Upgrading user with `settings.json` (`ui_language: "ja"`) and `queue.json` holding a job mid-download** → after launch settings are in SQLite with `ui_language: "system"`, the job is requeued, both JSON files are renamed `*.json.migrated`. Test in Task 4 / Task 5.
2. **App quits while a job is encoding** → relaunch re-encodes from the saved original without re-downloading; if the user deleted that original, it re-downloads. Test in Task 9.
3. **Original name collides** (`Fake Video.mp4` already exists) → original saved as `Fake Video (1).mp4`, converted file `Fake Video (1)-1.<ext>`, original stays even if encode fails or job is removed. Test in Task 9.
4. **Launch offline with yt-dlp and Deno already installed** → `ready: true` from the first `tools_status` call, no "preparing" toast, check error only shown in Settings → Update. Test in Task 8.
5. **DB written by a newer build (`user_version` > migrations)** → app refuses to start with `SCHEMA_TOO_NEW` logged instead of migrating or wiping. Test in Task 4.

---

## Phase 1 — Toolchain

### Task 1: Toolchain, lint, aliases, node:test

**Files:**
- Modify: `.mise.toml`, `package.json`, `yarn.lock`, `tsconfig.json`, `vite.config.ts`
- Create: `eslint.config.js`, `.prettierrc.cjs` (or the form `@bob-park/prettier-config-bobpark` documents), `.yarnrc.yml` npm scope for `@bob-park` if required (see `docs/agents/build-run.md`, token `GITHUB_NPM_AUTH_TOKEN`)

**Interfaces:**
- Produces: scripts `test` = `node --test "src/**/*.test.ts" "scripts/*.test.mjs"`, `typecheck` = `tsc --noEmit`, `lint` = `eslint src`, `prettier` = `prettier --write src`; alias `@/*` → `src/*` in both tsconfig `paths` and vite `resolve.alias`; `verbatimModuleSyntax: true`, `strict: true`.

- [ ] **Step 1:** `.mise.toml` → `node = '26'`, `yarn = "4.18.1"`; `packageManager` → `yarn@4.18.1`. Run `mise install`.
- [ ] **Step 2:** `mise exec -- yarn add @tanstack/react-query immer classnames` and `mise exec -- yarn add -D @tanstack/react-query-devtools eslint @bob-park/eslint-config-bobpark prettier @bob-park/prettier-config-bobpark vite@^8`. Do not remove i18next/vitest yet (Task 7 does).
- [ ] **Step 3:** Add ESLint config extending `@bob-park/eslint-config-bobpark` plus `no-restricted-imports` banning `../*` patterns; Prettier config from `@bob-park/prettier-config-bobpark`.
- [ ] **Step 4:** Add the alias and compiler options; add the four scripts. Keep `"test:legacy": "vitest run"` until Task 7.
- [ ] **Step 5:** Verify: `mise exec -- yarn typecheck` passes, `mise exec -- yarn test:legacy` passes, `mise exec -- yarn build` passes, `cargo test --workspace` passes.
- [ ] **Step 6:** Commit `chore: Node 26·Yarn 4.18.1·Vite 8, ESLint/Prettier, @/ 별칭`.

---

## Phase 2 — Rust refactor

### Task 2: `shared/` — AppError, db, migrations

**Files:**
- Create: `src-tauri/src/shared/{mod.rs,error.rs,db.rs}`, `src-tauri/src/shared/migrations/0001_init.sql`
- Modify: `src-tauri/Cargo.toml` (add `rusqlite = { version = "0.37", features = ["bundled"] }` or current, `thiserror = "2"`, `log = "0.4"`, `tauri-plugin-log = "2"`, `tauri-plugin-single-instance = "2"`), `src-tauri/src/lib.rs` (`mod shared;`)

**Interfaces:**
- Produces:
  - `pub enum AppError { InvalidInput(String), Db(rusqlite::Error), Io(std::io::Error), SchemaTooNew { found: i64, supported: i64 }, NotFound(String), Failed(String) }`, `impl AppError { pub fn code(&self) -> &'static str }` → `INVALID_INPUT`, `DB`, `IO`, `SCHEMA_TOO_NEW`, `NOT_FOUND`, `FAILED`. Serializes to `{ "code", "message" }`. `pub type AppResult<T> = Result<T, AppError>;`
  - Engine errors map through `impl From<kiri_core::engine::EngineError> for AppError` using `EngineError::code()` uppercased into `code` (keep `message`). Pipeline `error.*` i18n keys keep travelling inside job state, not AppError.
  - `pub fn db::open(path: &Path) -> AppResult<Connection>` (creates parent dir, migrates), `pub fn db::migrate(conn: &mut Connection) -> AppResult<()>`, `#[cfg(test)] pub fn db::open_in_memory() -> Connection`.
  - Schema `0001_init.sql`: `CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);` (value = JSON), `CREATE TABLE jobs (id INTEGER PRIMARY KEY, data TEXT NOT NULL);` (data = serde JSON of `kiri_core::model::Job`), `CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);` (holds `next_job_id`).

- [ ] **Step 1: Failing tests** in `db.rs`: `fresh_db_is_at_latest_version` (`user_version == MIGRATIONS.len()`), `reopen_does_not_rerun` (open twice on tempdir path, insert row between, row survives), `newer_schema_is_refused` (set `user_version = 99`, `migrate` → `Err(AppError::SchemaTooNew{found:99, supported:1})`). In `error.rs`: `serializes_code_and_message` (`AppError::InvalidInput("x".into())` → `{"code":"INVALID_INPUT","message":"x"}`).
- [ ] **Step 2:** `cargo test -p kiri --lib shared` → FAIL (not defined). (Use the actual package name from `src-tauri/Cargo.toml`.)
- [ ] **Step 3:** Implement per `docs/agents/conventions/sqlite-migrations.md` (each migration in its own transaction, then `PRAGMA user_version`).
- [ ] **Step 4:** Tests pass.
- [ ] **Step 5:** Commit `feat: SQLite 기반 shared(db·AppError·migrations)`.

### Task 3: `domain/settings` (SQLite) + JSON import

**Files:**
- Create: `src-tauri/src/domain/mod.rs`, `src-tauri/src/domain/settings/{mod.rs,model.rs,repository.rs,commands.rs}`, `src-tauri/src/app/{mod.rs,state.rs}`
- Delete: `src-tauri/src/settings.rs` (logic moves)

**Interfaces:**
- Consumes: Task 2 `AppResult`, `db::open_in_memory`.
- Produces:
  - `model.rs`: `Settings`, `General`, `Download`, `Update` — same fields and serde shape as today. `General.ui_language` accepts `system|ko|en`; anything else (incl. `ja`) normalizes to `system`.
  - `repository.rs`: `get_all(conn) -> AppResult<serde_json::Map<String, Value>>`, `put(conn, key: &str, value: &Value) -> AppResult<()>`. Keys are the top-level sections: `general`, `download`, `update`.
  - `mod.rs`: `pub fn get(conn: &Connection) -> AppResult<Settings>` (missing rows → defaults), `pub fn patch(conn: &Connection, patch: &Value) -> AppResult<Settings>` (deep merge via existing `merge`, validate, normalize, write changed sections in one transaction; invalid → `InvalidInput`, nothing saved), `pub fn import_json(conn: &Connection, path: &Path) -> AppResult<bool>` (if `settings` table empty and file exists: load like today's `Settings::load`, normalize, save, rename file to `settings.json.migrated`; returns whether imported), `pub fn engine_config(s: &Settings) -> EngineConfig` (moved as-is).
  - `commands.rs`: `get_settings(state) -> AppResult<Settings>`, `patch_settings(app, state, patch: Value) -> AppResult<Settings>` emitting `SETTINGS_CHANGED = "settings://changed"` with the new `Settings`, then the same side effects as today (engine config, tray/menu rebuild, language).
  - `app/state.rs`: `pub struct AppState { pub db: Mutex<Connection> }`.

- [ ] **Step 1: Failing tests** in `domain/settings/mod.rs` against `open_in_memory()`: `defaults_when_empty`, `patch_merges_and_persists` (patch `{"general":{"theme":"dark"}}` → `get().general.theme == "dark"`, other fields default), `ja_reads_as_system` (put `general` with `ui_language:"ja"` → `get()` returns `"system"`), `invalid_patch_saves_nothing` (`{"download":{"max_concurrent":0}}` → `InvalidInput`, `get()` unchanged; use today's validation rules from `settings.rs`), `import_json_moves_file` (tempdir `settings.json` with theme `dark` → `import_json` true, `get().general.theme=="dark"`, `settings.json.migrated` exists, second call false). Port every existing test from `src-tauri/src/settings.rs`.
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3:** Implement; wire `AppState` in `lib.rs` setup: `db::open(app_local_data_dir/"kiri.db")`, then `settings::import_json(conn, app_config_dir/"settings.json")` (use the exact path today's `SettingsState::new` reads). Replace every `SettingsState` use.
- [ ] **Step 4:** `cargo test --workspace` passes; `yarn tauri dev` starts and keeps an existing user's settings.
- [ ] **Step 5:** Commit `refactor: 설정 저장을 SQLite domain/settings 로 이전`.

### Task 4: Queue store in SQLite

**Files:**
- Modify: `crates/kiri-core/src/{queue.rs,engine.rs,lib.rs}`
- Create: `src-tauri/src/domain/queue/{mod.rs,model.rs,repository.rs,commands.rs}`

**Interfaces:**
- Produces:
  - `kiri_core::queue::QueueStore` trait: `fn load(&self) -> (QueueState, bool /*trusted*/); fn save(&self, state: &QueueState) -> std::io::Result<()>;` (+ `Send + Sync`). `JsonQueueStore { path }` implements it with today's `load_checked`/`save` (kept for tests and import).
  - `EnginePaths.queue_file` → `EnginePaths.store: Arc<dyn QueueStore>`.
  - `src-tauri` `domain/queue/repository.rs`: `SqliteQueueStore { db: Arc<Mutex<Connection>> }` implementing `QueueStore` (save = replace all `jobs` rows + `meta.next_job_id` in one transaction; load: corrupt row → skip it, `trusted = false`).
  - `domain/queue/mod.rs`: `pub fn import_json(conn, path) -> AppResult<bool>` (jobs table empty + file exists → load via `JsonQueueStore`, which already runs `migrate_legacy`, save to SQLite, rename to `queue.json.migrated`).
  - `AppState.db` becomes `Arc<Mutex<Connection>>` so the store shares it.
  - `commands.rs`: the existing job commands (`list_jobs`, `probe`, `add_job`, `add_url`, `stop_job`, `remove_job`, `restart_job`, `clear_completed`) returning `AppResult<T>`; event `QUEUE_CHANGED = "queue://changed"`.

- [ ] **Step 1: Failing tests:** kiri-core existing engine tests keep passing through `JsonQueueStore` (update helpers only). New in `domain/queue`: `roundtrip_preserves_jobs_and_next_id`, `corrupt_row_is_untrusted` (insert `data = "{"` → `load().1 == false`, other rows load), `import_json_requeues_active_job` (queue.json with one `downloading` job → after import + `QueueState::recover`, state is `queued`; file renamed).
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** `cargo test --workspace` passes.
- [ ] **Step 5:** Commit `refactor: 작업 큐 저장을 QueueStore·SQLite 로 이전`.

### Task 5: App shell into `app/`, tools domain, events, plugins, English comments

**Files:**
- Move: `windows.rs, tray.rs, updater.rs, quit.rs, i18n.rs, cli_install.rs, ipc_server.rs, pubkey_check.rs` → `src-tauri/src/app/`; add `app/menu.rs` only if the app menu code exists separately today (otherwise keep as is).
- Move: `bootstrap.rs` → `src-tauri/src/domain/tools/{mod.rs,commands.rs}` (status logic tauri-free in `mod.rs`; emit + spawn in `commands.rs`).
- Delete: `src-tauri/src/commands.rs` (split into each domain's `commands.rs`; `open_settings` → `app/windows.rs`, cli commands → `app/cli_install.rs`, update commands → `app/updater.rs`).
- Modify: `lib.rs` assembly only; `capabilities/default.json` if plugins need it.

**Interfaces:**
- Produces event consts: `settings://changed`, `queue://changed`, `tools://changed`, `update://status` (single event carrying the full `UpdateInfo`-based status, replacing `update-available|progress|ready|download-failed|scheduled|error`; payload type `UpdateStatus` = today's union of those payloads as a tagged enum `{"kind": "...", ...}`), `settings://opened`.
- `app::i18n::Lang { Ko, En }`, `resolve(pref)`: `ja`/unknown → OS language rule (`ko*` → Ko else En). Delete Japanese labels.
- Plugins in order: single-instance (second launch → `windows::show_main`), log (replace every `eprintln!` with `log::warn!/error!`), opener, dialog, clipboard, updater.

- [ ] **Step 1: Failing tests:** `app/i18n.rs` `ja_pref_follows_system` (`resolve_with("ja", Some("ko-KR")) == Ko`, `resolve_with("ja", Some("fr")) == En`); `app/updater.rs` `update_status_json_shape` for each variant; keep existing tools tests (`due_after_a_day`, `status_json_shape`).
- [ ] **Step 2:** FAIL. **Step 3:** Move/split; translate all Korean comments in `src-tauri/` and `crates/` to English; add a `//!` line per module.
- [ ] **Step 4:** `cargo test --workspace`; `yarn tauri dev` runs (frontend still on old event names — temporarily keep a compat shim in `src/lib/*` that listens to the new names; Task 6 removes it).
- [ ] **Step 5:** Commit `refactor: Rust 를 app/·domain/·shared/ 구조로 정리`.

---

## Phase 3 — Frontend refactor

### Task 6: Shared infrastructure

**Files (create):**
- `src/shared/api/{call.ts,common.dto.ts}` — `export default async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T>`; rejects with `AppError { code: string; message: string }` (string errors → `{code:'UNKNOWN', message}`).
- `src/shared/i18n/{en.ts,ko.ts,index.ts,locale.ts,locale.test.ts}` — `en` is the base `Dict`; `ko: Dict`. Port every key from `src/locales/{en,ko}.json` (drop `ja.json`). Interpolations become functions (`toolsFailed: (error: string) => ...`). Add `errors` area with `UNKNOWN`, `INVALID_INPUT`, `DB`, `IO`, `NOT_FOUND`, `FAILED` and every engine code (uppercased).
- `src/shared/providers/{I18nProvider.tsx,ThemeProvider.tsx}`, `src/shared/hooks/useI18n.tsx` (default export, returns `{ t, locale }`).
- `src/utils/{theme.ts,theme.test.ts,errorMessage.ts,errorMessage.test.ts}` — `resolveTheme(pref, prefersDark): 'light'|'dark'`; `errorMessage(err: AppError, errors: Dict['errors']): string`.
- `src/shared/components/queries/RQProvider.tsx` (staleTime 60 s, gcTime 120 s, refetchOnWindowFocus false, MutationCache.onError → `useStore.getState().showError`).
- `src/shared/store/{rootStore.ts,types.d.ts}`, `src/shared/store/error/{error.state.ts,slice.ts}`, `src/shared/store/toast/{toast.state.ts,slice.ts}` (port `src/lib/toast.ts` behavior; pure helpers to `src/utils/toast.ts` + test).
- `src/shared/components/error/ErrorAlert.tsx` — renders as a toast (kiri shows errors as toasts; keep that look).

- [ ] **Step 1: Failing tests (`node:test`):** `locale.test.ts`: `resolveLocale('system','ko-KR')==='ko'`, `('system','ja-JP')==='en'`, `('ja' as any,'ko-KR')==='ko'`, `('en','ko-KR')==='en'`. `theme.test.ts`: port `src/test/theme.test.ts`. `toast.test.ts`: port `src/test/toast.test.ts`. `errorMessage.test.ts`: unknown code → `errors.UNKNOWN`.
- [ ] **Step 2:** `yarn test` → FAIL. **Step 3:** Implement. Theme names in CSS become `light`/`dark` (Task 11 rewrites the tokens; here just rename the two daisyUI theme blocks and move `src/index.css` → `src/app/globals.css`).
- [ ] **Step 4:** `yarn test`, `yarn typecheck` pass.
- [ ] **Step 5:** Commit `refactor: shared(api·i18n·providers·store) 기반 추가`.

### Task 7: Domains, screens, removal of legacy

**Files:**
- Create per domain `settings`, `queue`, `tools`, `updater`, `cli`: `src/domain/<d>/apis/{<d>.ts,<d>.dto.ts}`, `queries/<d>.tsx`, `components/`, `store/` (`.gitkeep` where empty).
  - Query keys: `['settings']`, `['queue','jobs']`, `['tools','status']`, `['updater','status']`, `['cli','status']`. Each read hook listens to its `<domain>://…` event once and `setQueryData`.
  - Mutations: `useSettingsPatch`, `useJobAdd`, `useUrlAdd`, `useJobStop`, `useJobRemove`, `useJobRestart`, `useCompletedClear`, `useToolsUpdate`, `useUpdateCheck`, `useUpdateInstall`, `useUpdateDownloadRetry`, `useCliInstall`, `useCliUninstall`; `probe` stays a plain `apis` call (one-shot, sheet-scoped).
- Move pure logic: `src/lib/{format,paste,sheet,queue,tools,update,settings}.ts` → `src/utils/*.ts` (+ `*.test.ts` ported from `src/test/`). `scripts/latest-json` test → `scripts/latest-json.test.mjs`.
- Screens: `src/app/main.tsx` (seed theme + locale + settings before render), `src/app/App.tsx` (was `MainWindow`), `src/app/settings/SettingsApp.tsx` + `src/app/settings/_components/{GeneralTab,DownloadTab,CliTab,UpdateTab}.tsx`.
- `src/shared/components/settings/{SettingSection,SettingRow,Segmented,Switch,Button}.tsx` replacing `Group`/`Row`.
- Components → `src/domain/queue/components/{JobRow,OptionsSheet}.tsx`, `src/shared/components/{Icon,Toasts,StatusBar}.tsx`.
- Delete: `src/lib/`, `src/pages/`, `src/components/`, `src/locales/`, `src/test/`, `src/main.tsx`, `src/index.css`; deps `i18next`, `react-i18next`, `vitest`; script `test:legacy`. Update `index.html` entry to `src/app/main.tsx`.

- [ ] **Step 1:** Port tests first (all of `src/test/*.test.ts` except `locales`/`i18n`, which typecheck now covers) to `node:test` next to their new files; run → FAIL (modules missing).
- [ ] **Step 2:** Move code; every component follows `react-sections.md` section comments, default export, `Readonly` props, `cx`, `@/` imports; no `invoke` outside `call.ts`.
- [ ] **Step 3:** `yarn test`, `yarn typecheck`, `yarn lint` (whole `src`), `cargo test --workspace` pass.
- [ ] **Step 4:** `yarn tauri dev`: add a job, stop/restart/remove, change theme and language in Settings and see the main window follow (cross-window events), ko/en.
- [ ] **Step 5:** Commit `refactor: 프론트엔드를 app·domain·shared 구조와 React Query 로 이전`.

---

## Phase 4 — Behavior

### Task 8: Instant tool readiness

**Files:** `src-tauri/src/domain/tools/{mod.rs,commands.rs}`, `crates/kiri-core/src/tools.rs` (unchanged API), `src/domain/tools/**`, `src/utils/tools.ts`, i18n.

**Interfaces:**
- `ToolsStatus { ready: bool, installing: Option<String> /* "yt-dlp" | "deno" while downloading */, ytdlp_version: Option<String>, error: Option<String>, last_check: Option<u64> }` (DTO mirrors it).
- `pub fn initial_status(paths: &Tools) -> ToolsStatus` — `ready = ytdlp.exists() && deno.exists()`, everything else empty. Used to seed state in setup **before** windows open.
- `ensure` never sets `ready=false` while both files exist; it sets `installing` only right before an actual download (Deno missing, or yt-dlp missing, or hash differs).
- Frontend `toolsToast(s)`: `hidden` when `ready`; `preparing` only when `installing` is set; `failed` when `!ready && error && !installing`. Toast text: `t.app.toolsInstalling(name)` ("yt-dlp 내려받는 중…" / "Downloading yt-dlp…"). When `ready && error`, nothing on the main window; Settings → Update shows the error.

- [ ] **Step 1: Failing tests:** Rust `initial_status_ready_when_files_exist` (tempdir with both files → ready, without deno → not ready). TS `src/utils/tools.test.ts`: `{ready:true,error:'x'}` → `hidden`; `{ready:false,installing:'yt-dlp'}` → `preparing`; `{ready:false,installing:null,error:'x'}` → `failed`; `null` (before first status) → `hidden`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement; split `ensure` so the "is an update needed?" network check runs without touching `ready`.
- [ ] **Step 4:** Tests pass; `yarn tauri dev` with tools present and Wi-Fi off → no toast, paste works.
- [ ] **Step 5:** Commit `fix: 도구가 이미 있으면 바로 준비 완료, 실제 다운로드 때만 안내`.

### Task 9: Save original before converting

**Files:** `crates/kiri-core/src/{model.rs,pipeline.rs,engine.rs}`, frontend DTO + i18n.

**Interfaces:**
- `Job.original: Option<PathBuf>` (`#[serde(default)]`) — final path of the saved original once the download finished.
- `pipeline::Report` gains `pub original: Option<PathBuf>` (set on the one report emitted right after `finalize`); engine stores it on the job and saves the queue.
- `pipeline::run(job, cfg, cancel, report)` behavior for `JobSource::Youtube`:
  1. If `job.original` is `Some(p)` and `p.exists()` → skip download, encode from `p`.
  2. Else download into `work_dir`, `finalize` (move media + subs to `download_dir`) immediately, report `original`.
  3. If preset needs encoding → encode from the saved original into `work_dir/out`, move to `variant_path(download_dir, <original stem>, ext)`.
- Engine `restart` / `remove` / cancel: never delete `job.original`; only `remove_part_dir(work_dir)`. `restart` clears `output` but keeps `original`.
- Detail line while encoding: `t.job.encodingSaved` ("MP4 변환 중 · 원본 저장됨").

- [ ] **Step 1: Failing tests** in `pipeline.rs`: `original_is_in_download_dir_before_encoding_starts` (fake ffmpeg script asserts `"$DOWNLOAD_DIR/Fake Video.mp4"` exists when invoked, else exits 1), `encode_failure_keeps_saved_original`, `resume_with_saved_original_skips_download` (job with `original = Some(existing file)`, fake yt-dlp that `exit 1`s → still `Ok`), `missing_saved_original_redownloads`, `collision_names_variant_after_final_original` (`Fake Video.mp4` pre-exists → original `Fake Video (1).mp4`, result `Fake Video (1)-1.mov`). In `engine.rs`: `remove_while_encoding_keeps_original`, `report_original_is_persisted` (after the report, `QueueStore` sees `original`). Update `encodes_when_preset_needs_it` expectations if needed.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** `cargo test --workspace` passes; manual: MP4 preset download → original playable in Finder while the card shows encoding.
- [ ] **Step 5:** Commit `feat: 원본을 다운로드 직후 저장하고 그 파일로 변환`.

---

## Phase 5 — Aurora Glass redesign & docs

Reference mockup: `.superpowers/brainstorm/99741-1792578681/content/aurora-v2.html` (approved). Port its look, not its demo script.

### Task 10: Tokens and motion primitives

**Files:** `src/app/globals.css`

- [ ] **Step 1:** `light` / `dark` daisyUI themes: keep brand values (light primary `#7132f5`, success `#149e61`, base `#f6f5fb`, content `#101114`; dark primary `#8b5cff`, success `#4cd393`, base `#121318`, content `#ececf2`). Add tokens in both themes: `--color-glass`, `--color-glass-edge`, `--color-aurora-1/2/3`, `--color-muted`, `--color-track`, `--gradient-progress`, `--gradient-progress-encoding`, `--color-scrim` (values from the mockup's `.au` / `.au.dark` blocks).
- [ ] **Step 2:** Components layer: `.glass` (bg glass, `backdrop-filter: blur(18px) saturate(1.4)`, 1px edge), `.aurora` background with three `.aurora-blob` children (12 s drift, max 3). Keyframes: `drift`, `float`, `spring-in` (blur→sharp, translateY −16 px, scale .94), `pop`, `ring`, `flow`, `shimmer`, `collapse-out`. Utility classes `animate-*`. One `@media (prefers-reduced-motion: reduce)` block disabling all of them.
- [ ] **Step 3:** `yarn lint`, `yarn typecheck`; open `yarn tauri dev` light/dark.
- [ ] **Step 4:** Commit `style: 오로라 글래스 토큰과 모션 기본기`.

### Task 11: Main window

**Files:** `src/app/App.tsx`, `src/domain/queue/components/{JobRow,OptionsSheet,EmptyState}.tsx`, `src/shared/components/{Toasts,StatusBar}.tsx`, i18n.

- [ ] **Step 1:** Remove `DropZone` and all drag handlers (`onDragOver/Enter/Leave/Drop`, `dragDepth`). ⌘V stays the only input; ⌘, opens settings.
- [ ] **Step 2:** Header: `kiri.` logo + clear-completed (only when any completed) + settings button (rotate 45° on hover).
- [ ] **Step 3:** `EmptyState`: floating glass orb with download icon, `⌘` `V` glass keycaps (pressed animation on ⌘V keydown), `t.app.emptyHint`.
- [ ] **Step 4:** `OptionsSheet`: floating glass sheet inset 10 px, scrim with blur, spring up; skeleton shimmer while `info === null`; selected chips scale 1.04 + primary; gradient download button with press scale. Keep all existing behavior (focus trap, subtitles, remember, more presets, error inside sheet).
- [ ] **Step 5:** `JobRow`: glass card, `animate-spring-in` on mount, flowing gradient bar (encoding gradient while `encoding`), completed → success bar + ✓ badge with `pop` + `ring`; failed/stopped/queued styled with the same chip language; remove → `collapse-out` before the mutation's list update (keep the card until the animation ends, ≤250 ms). Buttons unchanged; no play button.
- [ ] **Step 6:** Toasts/StatusBar in glass style; keep enter/exit animations.
- [ ] **Step 7:** `yarn tauri dev`: full flow in light/dark × ko/en; macOS "Reduce motion" on → static UI.
- [ ] **Step 8:** Commit `feat(ui): 메인 창 오로라 글래스 리디자인`.

### Task 12: Settings window

**Files:** `src/app/settings/SettingsApp.tsx`, `src/app/settings/_components/*`, `src/shared/components/settings/*`.

- [ ] **Step 1:** Same aurora background + glass sections. Tab bar with a sliding selection indicator (transform-based). `Segmented` with sliding thumb, `Switch` with spring knob. Roles `tablist/tab`, `radiogroup/radio`, `switch` and focus-visible ring kept.
- [ ] **Step 2:** Update tab shows tools check error (from Task 8) under the yt-dlp row.
- [ ] **Step 3:** Remove the Japanese option from the language picker.
- [ ] **Step 4:** `yarn tauri dev` light/dark × ko/en. Commit `feat(ui): 설정 창 오로라 글래스 리디자인`.

### Task 13: Docs

**Files:** `AGENTS.md`, `CLAUDE.md`, `docs/agents/**`, `docs/design/`, `.github/workflows/bump-version.yml`, `docs/development.md`.

- [ ] **Step 1:** Replace template names (`template-rust-app`, `com.bobpark.app`, `template_rust_app_lib`) with `kiri` / `org.bobpark.kiri`; delete the placeholder-rename section, Windows/NSIS/`release.ps1`, autostart.
- [ ] **Step 2:** `structure.md` / `architecture.md` / `rust-modules.md`: add the workspace (`crates/kiri-core`, `crates/kiri-cli`), CLI + ffmpeg sidecars, IPC server, `QueueStore`, actual domains and events from this plan.
- [ ] **Step 3:** Test command → `cargo test --workspace` in `testing.md` and `CLAUDE.md`.
- [ ] **Step 4:** Branch docs (`branching.md`, `pr.md`, `versioning.md`) and `bump-version.yml` → `master` as the only long-lived branch; merges into `master` bump the version per PR title.
- [ ] **Step 5:** `docs/design/aurora-glass.md` (palette, glass, motion rules from the spec §8) and `design.md` pointing to it; note Pretendard 14 px.
- [ ] **Step 6:** Commit `docs: kiri 실제 구조와 오로라 글래스 기준으로 문서 정리`.

### Task 14: Final verification

- [ ] Run `cargo test --workspace`, `mise exec -- yarn test`, `mise exec -- yarn typecheck`, `mise exec -- yarn lint` — all pass (paste output).
- [ ] Upgrade test: copy a 0.3.1 `settings.json` (`ui_language:"ja"`) and `queue.json` into the dev app data dirs, launch, confirm Review Focus 1.
- [ ] `yarn tauri dev` light/dark × ko/en, main + settings windows, reduced motion.
- [ ] Use superpowers:verification-before-completion, then superpowers:finishing-a-development-branch.
