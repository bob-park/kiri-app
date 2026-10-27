# Tools In-App Install and Manual Update — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** FFmpeg leaves the app bundle. On startup the app installs any missing tool (ffmpeg, ffprobe, yt-dlp, Deno) into `app_data_dir/bin/`. After that, updates happen only when the user picks them in Settings → Updates, or from a "newer yt-dlp" toast after a failed download. Pasting a link while offline is refused.

**Architecture:** `kiri-core/src/tools.rs` holds tool names, release resolution (pure parsers plus reqwest glue) and staged installs. `src-tauri/src/domain/tools` keeps per-tool `{current, latest}` in `ToolsState` and exposes four thin commands; `tools://changed` still syncs the windows. The frontend adds a "Components" section (mockup 1-C) and a yt-dlp hint toast (mockup 2-C), and gates paste on `navigator.onLine`.

**Tech Stack:** Rust (`reqwest` with its built-in redirect policy, `sha2`, `/usr/bin/ditto`), Tauri 2, React 19 + React Query, zustand toasts, Tailwind 4/daisyUI 5.

**Spec:** `docs/superpowers/specs/2026-10-27-tools-manual-update-design.md`

## Global Constraints

- No dependency changes in `Cargo.toml` / `package.json`.
- Sources:
  - FFmpeg: `https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release/{ffmpeg,ffprobe}.zip`. Read the 302 `Location` from a GET without following it (HEAD gives 404). The target is `…/download/macos/arm64/<id>_<ver>/<name>.zip`, and the checksum is `<target>.sha256` (`<64 hex>  ffmpeg.zip`).
  - yt-dlp: `https://github.com/yt-dlp/yt-dlp/releases/latest` → `/releases/tag/<ver>`. Asset `…/releases/download/<ver>/yt-dlp_macos`, sums `…/releases/download/<ver>/SHA2-256SUMS`.
  - Deno: `https://github.com/denoland/deno/releases/latest` → `/releases/tag/v<ver>`. Asset `…/releases/download/v<ver>/deno-aarch64-apple-darwin.zip` + `.sha256sum`.
- A `Release` is valid only if all of these hold:
  - the version is digits and dots only (a leading `v` stripped);
  - every url starts with one of `https://github.com/yt-dlp/yt-dlp/releases/download/`, `https://github.com/denoland/deno/releases/download/`, `https://ffmpeg.martin-riedl.de/download/`;
  - every sha256 is 64 hex chars.
- Updating downloads the exact url that the check resolved (versioned path).
- Installs replace a binary only through `rename`, so running processes keep the old inode. Zips are extracted into a staging dir first.
- Startup installs missing tools only; it never checks for updates. No hourly loop.
- A failed latest-version check is silent: no error, and `latest` keeps its previous value. Only a failed install sets `error`.
- `ready` = ffmpeg, ffprobe, yt-dlp and deno all exist. Not split per tool (user decision).
- `settings.update.last_ytdlp_check` keeps its name and means "last check of all tools". Only `check_tool_updates` writes it.
- yt-dlp hint triggers:
  - a `youtube` job seen non-failed this session that turns `failed`;
  - a probe failure on paste.
  - It fires only when a newer yt-dlp exists, once per latest version per session. The yt-dlp-only check runs at most hourly (the timestamp is kept in memory; `check_tool_updates` counts toward it).
- Failed jobs are never auto-retried after an update.
- Offline (`navigator.onLine === false`):
  - paste → error toast `OFFLINE`, no sheet, no probe;
  - Updates-tab update buttons are disabled;
  - jobs already queued and the CLI path are untouched.
- User-visible text only in `src/shared/i18n/{en,ko}.ts`.
- All four checks pass: `cargo test --workspace`, `yarn test`, `yarn typecheck`, `yarn lint` (through `mise exec --`).

## Review Focus

1. **Offline when the Updates tab opens:** rows show the current version with no update button and no error. (Task 3 test: `check_due`/`ToolInfo::has_update` with `latest: None`.)
2. **Update pressed while an ffmpeg job runs:** the running process keeps working and the new binary lands by rename. (Task 1 test `unpack_zip_replaces_by_rename`.)
3. **martin-riedl serves ffmpeg and ffprobe at different versions (mid-publish):** rejected, nothing installed. (Task 1 test `ffmpeg_release_needs_matching_versions`.)
4. **Failed jobs restored from the last session:** no hint at launch. A job failing now gets a hint once. (Task 5 test `newlyFailedYoutube`.)
5. **A redirect to an unexpected host or a dev tag:** rejected. (Task 1 test `release_validity`.)

---

### Task 1: kiri-core — tool names, release parsing, staged zip install

**Files:**
- Modify: `crates/kiri-core/src/tools.rs` (tests at the bottom of the same file)

**Interfaces — Produces:**
- `#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)] pub enum ToolName { #[serde(rename = "yt-dlp")] Ytdlp, #[serde(rename = "ffmpeg")] Ffmpeg, #[serde(rename = "deno")] Deno }` and `pub const ALL_TOOLS: [ToolName; 3] = [Ytdlp, Ffmpeg, Deno]`.
- `#[derive(Debug, Clone, PartialEq)] pub struct Asset { pub url: String, pub sha256: String }`
- `#[derive(Debug, Clone, PartialEq)] pub struct Release { pub version: String, pub assets: Vec<Asset> }` with `pub fn valid(&self) -> bool` (Global Constraints rule).
- `pub fn parse_version(stdout: &str) -> Option<String>`. Takes the first line and its first whitespace token that starts with a digit, keeps the leading `[0-9.]` run, and trims trailing dots.
- `pub fn parse_tag(location: &str) -> Option<String>`. The part after `/releases/tag/`, leading `v` stripped, digits/dots only.
- `pub fn parse_riedl_version(url: &str) -> Option<String>`. From `…/download/macos/arm64/<id>_<ver>/<name>.zip`, the part after the last `_` in the second-to-last path segment.
- `pub fn ffmpeg_release(ffmpeg: Asset, ffprobe: Asset) -> Option<Release>`. `None` unless both urls parse to the same version. Asset order is `[ffmpeg, ffprobe]`.
- `pub async fn unpack_zip(zip: &[u8], bin: &str, dest: &Path) -> Result<(), String>`. Writes the zip to `<dest.parent>/.staging-<bin>/`, runs `ditto -x -k`, chmods `<staging>/<bin>` to 755, renames it onto `dest`, and removes staging (also on error).

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn parse_version_from_each_tool() {
    assert_eq!(parse_version("ffmpeg version 9.0.2-https://www.martin-riedl.de Copyright (c) 2000-2026\n").as_deref(), Some("9.0.2"));
    assert_eq!(parse_version("2026.10.20\n").as_deref(), Some("2026.10.20"));
    assert_eq!(parse_version("deno 2.5.4 (stable, release, aarch64-apple-darwin)\nv8 14.0").as_deref(), Some("2.5.4"));
    assert_eq!(parse_version(""), None);
}
#[test]
fn parse_tag_and_riedl_urls() {
    assert_eq!(parse_tag("https://github.com/yt-dlp/yt-dlp/releases/tag/2026.10.20").as_deref(), Some("2026.10.20"));
    assert_eq!(parse_tag("https://github.com/denoland/deno/releases/tag/v2.5.4").as_deref(), Some("2.5.4"));
    assert_eq!(parse_tag("https://github.com/denoland/deno/releases/tag/v2.6.0-rc.1"), None);
    assert_eq!(parse_riedl_version("https://ffmpeg.martin-riedl.de/download/macos/arm64/1789931890_9.0.2/ffmpeg.zip").as_deref(), Some("9.0.2"));
    assert_eq!(parse_riedl_version("https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release/ffmpeg.zip"), None);
}
#[test]
fn ffmpeg_release_needs_matching_versions() {
    let a = |id_ver: &str, n: &str| Asset { url: format!("https://ffmpeg.martin-riedl.de/download/macos/arm64/{id_ver}/{n}.zip"), sha256: H1.into() };
    let r = ffmpeg_release(a("1_9.0.2", "ffmpeg"), a("2_9.0.2", "ffprobe")).unwrap();
    assert_eq!(r.version, "9.0.2");
    assert!(r.valid());
    assert_eq!(ffmpeg_release(a("1_9.0.2", "ffmpeg"), a("2_9.0.1", "ffprobe")), None);
}
#[test]
fn release_validity() {
    let ok = Release { version: "9.0.2".into(), assets: vec![Asset { url: "https://ffmpeg.martin-riedl.de/download/x/ffmpeg.zip".into(), sha256: H1.into() }] };
    assert!(ok.valid());
    let mut bad = ok.clone(); bad.assets[0].url = "http://ffmpeg.martin-riedl.de/download/x/ffmpeg.zip".into(); assert!(!bad.valid());
    let mut bad = ok.clone(); bad.assets[0].url = "https://evil.example/download/ffmpeg.zip".into(); assert!(!bad.valid());
    let mut bad = ok.clone(); bad.version = "9.0-dev".into(); assert!(!bad.valid());
    let mut bad = ok.clone(); bad.assets[0].sha256 = "abc".into(); assert!(!bad.valid());
    assert!(!Release { assets: vec![], ..ok }.valid());
}
#[tokio::test]
async fn unpack_zip_replaces_by_rename() {
    // build a zip holding `tool` (`#!/bin/sh\necho new`) with `ditto -c -k <dir>/tool <zip>`;
    // dest pre-exists with "old"; hold an open fd on the old dest.
    // after unpack_zip: dest content is the new script, mode & 0o777 == 0o755,
    // the old fd still reads "old" (other inode), and no `.staging-tool` dir remains.
    // a zip without `tool` → Err and dest still "old".
}
#[test]
fn tool_name_serde() {
    assert_eq!(serde_json::to_value(ToolName::Ytdlp).unwrap(), serde_json::json!("yt-dlp"));
}
```

- [ ] **Step 2:** `cargo test -p kiri-core tools` → FAIL (not defined).
- [ ] **Step 3:** Implement the interfaces above in `tools.rs`.
- [ ] **Step 4:** `cargo test -p kiri-core tools` → PASS.
- [ ] **Step 5:** Commit `feat(tools): FFmpeg 릴리스 파싱과 단계적 zip 설치`.

### Task 2: kiri-core — resolve latest, install, installed version

**Files:**
- Modify: `crates/kiri-core/src/tools.rs` (delete `ensure_ytdlp`, `ytdlp_latest_hash`, `ytdlp_matches`, `install_ytdlp`, `ensure_deno`, the `latest/download` URL constants and `version`)

**Interfaces — Consumes:** Task 1. **Produces:**
- `pub fn redirect_client() -> reqwest::Client`. Like `http_client()`, but with `.redirect(reqwest::redirect::Policy::none())` and a 20 s timeout.
- `pub async fn latest(client: &reqwest::Client, tool: ToolName) -> Result<Release, String>`.
  - Reads the `Location` header via `redirect_client()`.
  - yt-dlp/Deno: `parse_tag`, then the versioned sums file through `find_hash`.
  - FFmpeg: two redirects plus `<url>.sha256` through `find_hash(text, "<name>.zip")`, then `ffmpeg_release`.
  - Returns `Err` unless `valid()`.
- `pub async fn install(client: &reqwest::Client, tool: ToolName, release: &Release, paths: &Tools) -> Result<(), String>`.
  - Fetches each asset and checks its sha (mismatch → `"<tool> checksum mismatch"`).
  - yt-dlp goes through `install_executable(&paths.ytdlp, …)`.
  - Deno goes through `unpack_zip(…, "deno", &paths.deno)`.
  - FFmpeg goes through `unpack_zip` for `ffmpeg` → `paths.ffmpeg` and `ffprobe` → `paths.ffprobe`.
  - Creates `bin/` if missing.
- `pub fn is_installed(tool: ToolName, paths: &Tools) -> bool`. FFmpeg requires both files.
- `pub async fn installed_version(tool: ToolName, paths: &Tools) -> Option<String>`. Runs `runner::output` with `-version` for ffmpeg and `--version` for the others, then `parse_version`.

- [ ] **Step 1: Failing tests:**
  - `is_installed_ffmpeg_needs_both` (tempdir with only ffmpeg → false; add ffprobe → true).
  - `installed_version_runs_the_binary`: a fake `ffmpeg` script echoing `ffmpeg version 9.0.2-x` gives `Some("9.0.2")`, and a missing file gives `None`. Reuse the existing `version_reads_first_line` pattern and replace that test.
  - `#[ignore] installs_latest_of_each_tool`, for the network: `latest` + `install` for all three into a tempdir, then `installed_version == release.version`. For FFmpeg, also assert that `ffmpeg -hide_banner -encoders` lists `libx264 libx265 libvpx-vp9 libmp3lame libopus h264_videotoolbox hevc_videotoolbox prores_videotoolbox prores_ks`. This is the one-off encoder check the spec asks for.
- [ ] **Step 2:** `cargo test -p kiri-core tools` → FAIL.
- [ ] **Step 3:** Implement. Update the module doc comment to say that FFmpeg now comes from martin-riedl.de too.
- [ ] **Step 4:** `cargo test -p kiri-core tools` → PASS. Then `cargo test -p kiri-core tools -- --ignored installs_latest` → PASS (needs network; paste its output into the PR).
- [ ] **Step 5:** Commit `feat(tools): yt-dlp·Deno·FFmpeg 최신 릴리스 조회와 설치`.

### Task 3: app — tools state, commands, unbundle FFmpeg

**Files:**
- Modify: `src-tauri/src/domain/tools/mod.rs`, `src-tauri/src/domain/tools/commands.rs`, `src-tauri/src/lib.rs` (paths, command list, startup)
- Modify: `src-tauri/tauri.conf.json` (`externalBin` → `["binaries/kiri-cli"]`), `package.json` (`"sidecars": "sh scripts/build-cli.sh"`), `scripts/release-macos.sh:45` (comment only)
- Delete: `scripts/fetch-ffmpeg.sh`, `scripts/ffmpeg.lock`
- Docs: `docs/agents/{architecture,build-run,configuration,testing}.md`, `docs/development.md`, `docs/release.md`, `README.md`. Remove the ffmpeg-sidecar/lock lines. Say that tools download to `app_data_dir/bin` on first launch and update from Settings → Updates. Add martin-riedl.de to README's network/privacy text.

**Interfaces — Consumes:** Task 2. **Produces:**
- `#[derive(Serialize, Clone, Default, Debug, PartialEq)] pub struct ToolInfo { pub current: Option<String>, pub latest: Option<String> }` with `pub fn has_update(&self) -> bool` (`latest.is_some() && latest != current`).
- `ToolsStatus { ready: bool, installing: Option<ToolName>, ytdlp: ToolInfo, ffmpeg: ToolInfo, deno: ToolInfo, error: Option<String>, last_check: Option<u64> }` plus `fn info_mut(&mut self, ToolName) -> &mut ToolInfo`.
- `ToolsState` gains `releases: Mutex<HashMap<ToolName, Release>>` and `checked_at: Mutex<Option<u64>>`.
- `pub fn check_due(checked_at: Option<u64>, now: u64) -> bool`. True when `None` or ≥ 1 h old. It replaces `due` and `DAY_MS`.
- `initial_status(paths)`: `ready` uses `tools::is_installed` for all of `ALL_TOOLS`.
- Commands (registered in `lib.rs`; `update_tools` keeps its name with a new signature):
  - `install_tools()`. Installs every missing tool (`latest` then `install`), setting `installing` per tool, then refreshes `current` and `ready`. Runs at startup (it replaces `spawn`'s loop with a single spawned call) and from the failed-toast retry.
  - `check_tool_updates()`. Resolves `latest` for `ALL_TOOLS` concurrently and stores the releases. On failure it leaves the previous `latest` alone. It sets `checked_at = now` and patches `update.last_ytdlp_check = now` through `settings::commands::patch_and_notify`, then emits.
  - `check_ytdlp_update() -> AppResult<Option<String>>`. Runs only if `check_due`; otherwise it reuses the stored result. Sets `checked_at` but not the setting. Returns `ytdlp.latest` when `ytdlp.has_update()`.
  - `update_tools(names: Vec<ToolName>)`. For each name it uses the stored release, or `latest` if none is stored, then installs and refreshes `current`. An install failure sets `error`.
  - All of these share the `busy` guard (a second call returns at once) and emit `tools://changed`.
- `Tools.ffmpeg/ffprobe` = `bin.join("ffmpeg")` / `bin.join("ffprobe")`. Remove `app::sidecar` if nothing else uses it (grep first; kiri-cli install may use it — keep it if so).

- [ ] **Step 1: Failing tests** in `domain/tools/mod.rs`:
  - `check_due_after_an_hour`: `None` → true; 59 min → false; 60 min → true.
  - `has_update_needs_a_different_latest`: `{9.0.1, 9.0.2}` → true; `{9.0.2, 9.0.2}` → false; `{9.0.1, None}` (offline) → false; `{None, 9.0.2}` → true.
  - `initial_status_ready_only_with_all_four`: rewrite the existing test; a missing ffprobe → not ready.
  - `status_json_shape`: `{"ready":false,"installing":null,"ytdlp":{"current":null,"latest":null},"ffmpeg":{…},"deno":{…},"error":null,"last_check":null}`.
- [ ] **Step 2:** `cargo test -p kiri-app tools` → FAIL.
- [ ] **Step 3:** Implement. Edit the config, scripts and docs.
- [ ] **Step 4:** `cargo test --workspace` → PASS; `cargo clippy --workspace --all-targets` → no new warnings.
- [ ] **Step 5:** Commit `feat(tools): 도구 수동 업데이트 명령, 번들에서 FFmpeg 제거`.

### Task 4: frontend — Components section in Settings → Updates (mockup 1-C)

**Files:**
- Modify: `src/domain/tools/apis/tools.dto.ts`, `src/domain/tools/apis/tools.ts`, `src/domain/tools/queries/tools.tsx`, `src/utils/tools.ts` (+ `tools.test.ts`), `src/app/settings/_components/UpdateTab.tsx`, `src/app/App.tsx` (the retry action uses `installTools`), `src/shared/i18n/{en,ko}.ts`
- Create: `src/app/settings/_components/ToolsSection.tsx`, `src/shared/hooks/useOnline.tsx`

**Interfaces — Consumes:** Task 3's commands. **Produces:**
- DTOs:
  - `export type ToolName = 'yt-dlp' | 'ffmpeg' | 'deno'`
  - `export interface ToolInfo { current: string | null; latest: string | null }`
  - `ToolsStatus` mirrors the Rust struct.
- APIs:
  - `installTools(): Promise<void>`
  - `checkToolUpdates(): Promise<void>`
  - `checkYtdlpUpdate(): Promise<string | null>`
  - `updateTools(names: ToolName[]): Promise<void>`
- Queries: `useToolsInstall()`, `useToolUpdateCheck()` (calls `checkToolUpdates` once on mount and swallows errors), and `useToolsUpdate()` whose mutate takes `ToolName[]`.
- `src/utils/tools.ts`:
  - `export const TOOL_ORDER: ToolName[] = ['yt-dlp', 'ffmpeg', 'deno']`
  - `export function toolInfo(s: ToolsStatus, name: ToolName): ToolInfo`
  - `export function pendingUpdates(s: ToolsStatus | null): ToolName[]` (in `TOOL_ORDER` order, `latest !== null && latest !== current`)
  - `toolsToast` keeps its behavior.
- `useOnline(): boolean`. Built on `useSyncExternalStore` over `navigator.onLine` and the `online`/`offline` events.
- `ToolsSection` (`{ tools: ToolsStatus | null }`) uses `SettingSection` (title `t.settings.update.components`) and `SettingRow`.
  - Summary row: label `updatesAvailable(n)` or `allUpToDate`, desc `lastCheck(time)`. When n > 0 it shows a primary `Button` `updateAll` that calls `updateTools(pending)`.
  - One row per tool:
    - label is the tool's display name (`yt-dlp` / `FFmpeg` / `Deno`, from i18n);
    - desc is `versionArrow(current, latest)` when there's an update, else `versionLatest(current)` with a ✓ in `text-success`, or `notInstalled` when `current` is null;
    - the `Button` `update` appears only on rows with an update.
  - While `installing === name`, that row's button becomes `loading loading-spinner loading-xs`.
  - Every button is disabled while busy (`isLoading || installing`) or `!online`.
  - It replaces the yt-dlp `SettingSection` in `UpdateTab`. The error alert stays.
- i18n (both files):
  - add `settings.update.{components, updatesAvailable(n), allUpToDate, updateAll, update, versionArrow(a,b), versionLatest(v), notInstalled, toolLabel: {ytdlp, ffmpeg, deno}}`;
  - remove `ytdlpVersion`, `ytdlpMissing`, `ytdlpUpdate`.
  - Korean copy follows the mockup: "구성 요소", "업데이트 N개 사용 가능", "모두 최신 버전", "모두 업데이트", "업데이트", "설치되지 않음".

- [ ] **Step 1: Failing tests** (`src/utils/tools.test.ts`):
  - Update the `st()` fixture to the new shape.
  - `pendingUpdates lists tools with a different latest, in order`: yt-dlp `{a,b}`, ffmpeg `{x,x}`, deno `{d,e}` → `['yt-dlp','deno']`; `null` → `[]`; `{a, null}` → `[]`.
  - Keep the existing `toolsToast` tests.
- [ ] **Step 2:** `mise exec -- yarn test` → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `mise exec -- yarn test && mise exec -- yarn typecheck && mise exec -- yarn lint` → PASS.
- [ ] **Step 5:** `mise exec -- yarn tauri dev`. Check these:
  - first launch with an empty `bin/` installs all four and the progress toast names each one;
  - the Updates tab matches mockup 1-C;
  - after replacing `bin/yt-dlp` with an older release, the tab shows `old → new`, and both Update and Update all work;
  - turning Wi-Fi off disables the buttons and shows no error;
  - all of it in light/dark and ko/en.
- [ ] **Step 6:** Commit `feat(settings): 구성 요소 버전과 업데이트 버튼`.

### Task 5: frontend — offline paste gate and yt-dlp hint toast (mockup 2-C)

**Files:**
- Modify: `src/app/App.tsx`, `src/utils/tools.ts` (+ test), `src/shared/i18n/{en,ko}.ts`

**Interfaces — Consumes:** `checkYtdlpUpdate`, `useToolsUpdate` (Task 4). **Produces:**
- `export function newlyFailedYoutube(seen: ReadonlyMap<number, string>, jobs: Job[]): { failed: boolean; seen: Map<number, string> }`.
  - `failed` is true when some job has `source.kind === 'youtube'`, `state.kind === 'failed'`, and a recorded kind in `seen` that is not `'failed'`.
  - The returned `seen` maps every current job id to its `state.kind`.
  - `Job` is an `import type` from `queue.dto`.
- `App.tsx`:
  - In `handleSubmit`, right after the url check: `if (!navigator.onLine) return showError({ code: 'OFFLINE', message: '' })`. This also covers the `skip_sheet` path.
  - `maybeHintYtdlp()` awaits `checkYtdlpUpdate()` and ignores errors. If it returns a version that differs from `hinted.current` (a ref), it pushes a toast:
    - `{ key: 'ytdlp-hint', kind: 'info', text: t.app.ytdlpHintTitle, desc: t.app.ytdlpHintDesc, action: { label: t.settings.update.update, run: () => updateTools(['yt-dlp']) } }`
    - and then stores the version in `hinted.current`.
  - It is called from the probe `catch` (only in the `current` branch, next to `showError`) and from an effect on `jobs` that runs `newlyFailedYoutube` against a `seen` ref.
- i18n:
  - `app.ytdlpHintTitle`: "다운로드가 실패했어요" / "Download failed".
  - `app.ytdlpHintDesc`: "새 yt-dlp가 있어요. 업데이트하면 해결될 수 있어요." / "A newer yt-dlp is available. Updating may fix it."
  - `errors.OFFLINE`: "인터넷에 연결되어 있지 않습니다" / "You're offline".

- [ ] **Step 1: Failing tests** for `newlyFailedYoutube`:
  - `restored failed jobs do not count`: empty seen, a failed youtube job → `failed: false`.
  - `a youtube job failing now counts`: seen `{1:'downloading'}`, job 1 failed → `true`; the returned seen is `{1:'failed'}`, and feeding it back → `false`.
  - `file jobs never count`: seen `{2:'encoding'}`, a failed file job 2 → `false`.
- [ ] **Step 2:** `mise exec -- yarn test` → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** All four checks → PASS.
- [ ] **Step 5:** `yarn tauri dev`. Check these:
  - Wi-Fi off + ⌘V shows the OFFLINE toast and no sheet;
  - with an old yt-dlp in `bin/`, a failing download (e.g. a private video URL) shows the hint once, and its Update button updates yt-dlp;
  - a second failure shows no second toast;
  - all of it in light/dark and ko/en.
- [ ] **Step 6:** Commit `feat(app): 오프라인 붙여넣기 차단, yt-dlp 업데이트 안내`.
