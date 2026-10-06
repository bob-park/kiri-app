# kiri 다운로더 · CLI · 자동 업데이트 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** YouTube 링크를 ⌘V로 붙여넣어 화질/포맷/자막을 골라 다운로드하고 ffmpeg로 재인코딩하는 macOS Tauri 앱을 만든다. 같은 큐를 제어하는 `kiri` CLI와 GitHub Release 기반 자동 업데이트도 함께 만든다.

**Architecture:** Cargo workspace는 세 crate로 나뉜다.
- `crates/kiri-core`: Tauri에 의존하지 않는다. 작업 모델, yt-dlp/ffmpeg 인자와 파서, 프로세스 실행, 파이프라인, 큐 엔진, 소켓 프로토콜, 도구 설치를 담당한다.
- `crates/kiri-cli`: `kiri` 바이너리.
- `src-tauri`: 앱 셸. 설정, 명령, 트레이, 업데이터를 담당한다.

큐의 소유자는 앱 프로세스의 `Engine` 하나다. React UI(Tauri command)와 CLI(Unix 소켓)가 같은 `Engine`을 호출한다.

**Tech Stack:** Rust 2024, Tauri v2, tokio, React 19 + TypeScript + Vite 7, Tailwind 4 + daisyUI 5, zustand 5, i18next, vitest, yarn 4.

**Spec:** `docs/superpowers/specs/2026-10-06-kiri-downloader-cli-updater-design.md` (목업: `docs/superpowers/specs/2026-10-06-kiri-mockups/`)

**참고 프로젝트:** `/Users/hwpark/Documents/rust-workspace/babelay-app`. 설정 merge, 업데이터, i18n, latest-json 패턴을 그대로 가져온다.

## Global Constraints

- 플랫폼: macOS Apple Silicon(aarch64)만 지원한다. `minimumSystemVersion` `"14.2"`.
- Tauri 식별자 `org.bobpark.kiri`, productName `kiri`, 버전 `0.1.0`.
- Rust edition `2024`. 모든 crate가 workspace에 속한다.
- 프론트엔드: React `^19.1.0`, Vite `^7.0.4`, TypeScript `^5.9`, Tailwind `^4`, daisyUI `^5.7`, zustand `^5`, i18next `^26`, react-i18next `^17`, vitest `^4`, `packageManager: "yarn@4.18.0"`, `nodeLinker: node-modules`.
- 다국어는 ko/en/ja 세 개의 로케일 파일(`src/locales/*.json`)로 하고 키 집합이 같아야 한다. `fallbackLng: "en"`. Rust 쪽 문구(트레이, 대화상자)는 `src-tauri/src/i18n.rs`에 둔다.
- 설정은 `app_config_dir()/settings.json`에 저장한다. 키는 snake_case이고 모든 구조체에 `#[serde(default)]`를 붙인다. 프론트엔드는 patch만 보낸다(깊은 병합).
- 디자인 토큰: 라이트는 `docs/design/kraken-design.md`를 따른다(primary `#7132f5`, radius 12px). 다크는 bg `#16171c`, bg2 `#1d1e25`, fg `#ececf2`, muted `#8b8ea3`, border `#2c2e38`, accent `#8b5cff`.
- 도구 위치:
  - yt-dlp, Deno: `app_data_dir()/bin/{yt-dlp,deno}` (`~/Library/Application Support/org.bobpark.kiri/bin`)
  - ffmpeg, ffprobe, kiri: sidecar(`externalBin`). 런타임에는 `current_exe()`와 같은 디렉터리에 있다.
- 소켓: `~/Library/Application Support/org.bobpark.kiri/kiri.sock`. 환경변수 `KIRI_SOCKET`으로 바꿀 수 있다. 권한 0600.
- 이벤트 이름: `queue-changed`(payload `Job[]`), `settings-changed`, `tools-changed`, `update-available`, `update-progress`, `update-error`, `update-scheduled`.
- 앱이 만든 에러는 `{ code, message }` 형태로 보낸다. 프론트엔드는 `error.<code>` i18n 키로 번역하고, 키가 없으면 message를 보여준다. `JobState::Failed(message)`의 message가 `error.`로 시작하면 i18n 키로 취급한다.
- 커밋 메시지는 이 저장소의 관례(`docs: …`, `feat: …`)를 따른다. 본문 마지막에 아래 두 줄을 붙인다.
  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01X77WqK7NYLRELcuzcd24FW
  ```
  (아래 단계의 `git commit -m` 예시에서는 지면상 생략했으니, 실제로 커밋할 때 붙인다.)
- 작업 브랜치는 `feature/youtube-donwload`(현재 브랜치)다.

## Review Focus

1. **YouTube가 아닌 클립보드를 붙여넣기**(`hello`, `https://vimeo.com/1`, `https://youtube.com.evil.com/watch?v=x`): 작업이 생기지 않고 "YouTube 링크가 아닙니다" 에러가 나야 한다. 테스트는 Task 2 `is_youtube_url`과 Task 7 `probe_rejects_non_youtube_without_running_ytdlp`.
2. **같은 제목의 영상을 두 번 받기**: 기존 파일을 덮어쓰지 않고 `제목 (1).mp4`가 되어야 한다. 테스트는 Task 4 `unique_path_*`와 Task 5 `existing_file_is_not_overwritten`.
3. **중지 직후 바로 다시 시작**(이전 프로세스가 아직 종료 중): 작업이 중복 실행되거나 상태가 덮어써지면 안 되고, 최종적으로 한 번만 `Completed`가 되어야 한다. 테스트는 Task 7 `stop_then_immediate_restart_completes_once`.
4. **실행 중인 작업 삭제**: 목록에서 사라지고, 다시 나타나지 않고, 캐시 폴더도 지워져야 한다. 테스트는 Task 7 `remove_running_job_cleans_up`.
5. **다운로드 중 앱 강제 종료 후 재실행**: 진행 중이던 작업은 `Queued`로 돌아오고, id는 재사용되지 않아야 한다. 테스트는 Task 6 `recover_requeues_active_jobs`, `load_repairs_next_id`와 Task 7 `new_engine_recovers_persisted_jobs`.

## File Structure

```
Cargo.toml                         # [workspace] members
crates/kiri-core/
  Cargo.toml
  src/lib.rs                       # 모듈 선언
  src/model.rs                     # Preset, JobOptions, JobState, Job, NewJob, Tools
  src/ytdlp.rs                     # URL 검사, probe 파싱, 화질 해석, 자막 선택, 인자, 진행률 파서
  src/ffmpeg.rs                    # 프리셋 → 인자, -progress 파서
  src/runner.rs                    # 자식 프로세스 실행/취소(프로세스 그룹), stderr tail
  src/files.rs                     # unique_path, move_file, check_writable, find_media, subtitle_files
  src/pipeline.rs                  # 다운로드 → 인코딩(가속 폴백) → 최종 이동
  src/queue.rs                     # QueueState: 순수 상태 + queue.json 영속화
  src/engine.rs                    # Engine: 동시 실행, stop/remove/restart, probe, add_url, EngineError
  src/ipc.rs                       # Envelope/Request/Response, bind/serve/dispatch, client request
  src/tools.rs                     # yt-dlp/Deno 설치·갱신(SHA256 검증)
  src/testutil.rs                  # (cfg(test)) 가짜 스크립트, 샘플 Job
  tests/fixtures/probe.json
crates/kiri-cli/
  Cargo.toml
  src/main.rs                      # clap 명령, 표 출력, 종료 코드, 앱 실행
  tests/cli.rs
src-tauri/
  Cargo.toml, build.rs, tauri.conf.json, tauri.macos.conf.json, capabilities/default.json
  icons/                           # `yarn tauri icon`으로 생성
  binaries/                        # (gitignore) ffmpeg/ffprobe/kiri sidecar
  src/main.rs, src/lib.rs          # 진입점, setup, run 이벤트
  src/settings.rs                  # Settings + merge + engine_config
  src/commands.rs                  # Tauri commands + CmdError
  src/windows.rs                   # 메인/설정 창, close-to-tray, Dock 표시
  src/bootstrap.rs                 # 도구 상태, 시작 시/주기 설치
  src/ipc_server.rs                # 소켓 서버 기동 + 상태
  src/cli_install.rs               # /usr/local/bin/kiri 링크
  src/tray.rs                      # 메뉴 막대 아이콘, 더블클릭 판정, 요약 라벨
  src/quit.rs                      # 종료 확인
  src/i18n.rs                      # Rust 쪽 문구
  src/updater.rs                   # 앱 업데이트 확인/설치/큐 이후 재시작
package.json, vite.config.ts, tsconfig.json, tsconfig.node.json, index.html, .yarnrc.yml
assets/icon.svg
src/
  main.tsx                         # 창 label로 MainWindow/SettingsWindow 선택
  index.css                        # Tailwind + daisyUI 테마(kiri, kiri-light)
  locales/{ko,en,ja}.json
  lib/types.ts, tauri.ts, theme.ts, i18n.ts, settings.ts, queue.ts, tools.ts, update.ts, toast.ts, format.ts, paste.ts, sheet.ts
  components/JobRow.tsx, OptionsSheet.tsx, UpdateBanner.tsx, Toasts.tsx
  pages/MainWindow.tsx, SettingsWindow.tsx
  pages/settings/GeneralTab.tsx, DownloadTab.tsx, CliTab.tsx, UpdateTab.tsx
  test/*.test.ts
scripts/fetch-ffmpeg.sh, scripts/build-cli.sh, scripts/latest-json.mjs
docs/development.md
```

---
## Phase A: kiri-core

### Task 1: workspace 전환과 작업 모델

**Files:**
- Modify: `Cargo.toml` (`[package]`를 `[workspace]`로 교체)
- Delete: `src/main.rs`
- Modify: `.gitignore`
- Create: `crates/kiri-core/Cargo.toml`, `crates/kiri-core/src/lib.rs`, `crates/kiri-core/src/model.rs`

**Interfaces:**
- Produces:
  - `kiri_core::model::{Preset, JobOptions, JobState, Job, NewJob, Tools}`
  - `Preset::{ALL, id(), is_audio_only(), extension()}`, `impl FromStr for Preset`
  - `JobState::{is_active(), is_pending()}`
  - JSON 계약:
    - `Preset`는 `"mp4-h264"` 같은 kebab-case 문자열
    - `JobState`는 `{"kind":"queued"}` 또는 `{"kind":"failed","message":"…"}`

- [ ] **Step 1: workspace 전환**

`Cargo.toml` 전체를 아래로 교체한다. `crates/kiri-cli`는 Task 10에서, `src-tauri`는 Task 11에서 members에 추가한다.

```toml
[workspace]
members = ["crates/kiri-core"]
resolver = "3"
```

```bash
git rm src/main.rs
```

`.gitignore` 끝에 추가한다.

```
# kiri
node_modules/
dist/
.yarn/
src-tauri/binaries/
src-tauri/gen/
```

- [ ] **Step 2: kiri-core crate 만들기**

`crates/kiri-core/Cargo.toml`:

```toml
[package]
name = "kiri-core"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
libc = "0.2"
sha2 = "0.10"
tokio = { version = "1", features = ["process", "io-util", "net", "rt", "sync", "time", "macros", "fs"] }
tokio-util = "0.7"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }

[dev-dependencies]
tempfile = "3"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "process", "time"] }
```

`crates/kiri-core/src/lib.rs` (이후 Task에서 모듈을 한 줄씩 추가한다):

```rust
//! kiri 의 Tauri 비의존 코어. 앱과 CLI 가 함께 쓴다.
pub mod model;
```

- [ ] **Step 3: 실패하는 테스트 작성**

`crates/kiri-core/src/model.rs`:

```rust
//! 큐와 CLI 가 함께 쓰는 작업 모델. serde 표현이 곧 프론트엔드·CLI 계약이다.
use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf, str::FromStr};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preset_serde_matches_id() {
        for p in Preset::ALL {
            assert_eq!(serde_json::to_value(p).unwrap(), json!(p.id()));
            assert_eq!(p.id().parse::<Preset>(), Ok(p));
        }
        assert_eq!(Preset::Mp4H264.id(), "mp4-h264");
        assert_eq!(Preset::MovProres.id(), "mov-prores");
        assert!("avi".parse::<Preset>().is_err());
    }

    #[test]
    fn preset_properties() {
        assert_eq!(Preset::Original.extension(), None);
        assert_eq!(Preset::Mp4Hevc.extension(), Some("mp4"));
        assert_eq!(Preset::MovProres.extension(), Some("mov"));
        assert_eq!(Preset::WebmVp9.extension(), Some("webm"));
        assert!(Preset::Mp3.is_audio_only() && Preset::M4a.is_audio_only());
        assert!(!Preset::Mp4H264.is_audio_only());
    }

    #[test]
    fn job_state_json_shape() {
        assert_eq!(serde_json::to_value(JobState::Queued).unwrap(), json!({"kind": "queued"}));
        assert_eq!(
            serde_json::to_value(JobState::Failed("boom".into())).unwrap(),
            json!({"kind": "failed", "message": "boom"})
        );
        let back: JobState = serde_json::from_value(json!({"kind": "encoding"})).unwrap();
        assert_eq!(back, JobState::Encoding);
    }

    #[test]
    fn job_state_predicates() {
        assert!(JobState::Downloading.is_active() && JobState::Encoding.is_active());
        assert!(!JobState::Queued.is_active());
        assert!(JobState::Queued.is_pending() && JobState::Encoding.is_pending());
        assert!(!JobState::Stopped.is_pending() && !JobState::Completed.is_pending());
    }
}
```

- [ ] **Step 4: 테스트가 실패하는지 확인**

Run: `cargo test -p kiri-core`
Expected: 컴파일 에러(`cannot find type Preset`).

- [ ] **Step 5: 구현**

`model.rs`의 `#[cfg(test)]` 위에 추가한다.

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    Original,
    Mp4H264,
    Mp4Hevc,
    MovProres,
    WebmVp9,
    Mp3,
    M4a,
}

impl Preset {
    pub const ALL: [Preset; 7] = [
        Preset::Original,
        Preset::Mp4H264,
        Preset::Mp4Hevc,
        Preset::MovProres,
        Preset::WebmVp9,
        Preset::Mp3,
        Preset::M4a,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Preset::Original => "original",
            Preset::Mp4H264 => "mp4-h264",
            Preset::Mp4Hevc => "mp4-hevc",
            Preset::MovProres => "mov-prores",
            Preset::WebmVp9 => "webm-vp9",
            Preset::Mp3 => "mp3",
            Preset::M4a => "m4a",
        }
    }

    pub fn is_audio_only(self) -> bool {
        matches!(self, Preset::Mp3 | Preset::M4a)
    }

    /// 재인코딩 결과 확장자. 원본 유지는 인코딩하지 않으므로 None.
    pub fn extension(self) -> Option<&'static str> {
        match self {
            Preset::Original => None,
            Preset::Mp4H264 | Preset::Mp4Hevc => Some("mp4"),
            Preset::MovProres => Some("mov"),
            Preset::WebmVp9 => Some("webm"),
            Preset::Mp3 => Some("mp3"),
            Preset::M4a => Some("m4a"),
        }
    }
}

impl FromStr for Preset {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        Preset::ALL.into_iter().find(|p| p.id() == s).ok_or(())
    }
}

impl fmt::Display for Preset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JobOptions {
    /// 비디오 format_id. None 이면 오디오만 받는다.
    pub format_id: Option<String>,
    pub preset: Preset,
    pub subtitles: Vec<String>,
    /// subtitles 중 자동 생성 자막이 섞여 있으면 true (--write-auto-subs).
    pub auto_subtitles: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Downloading,
    Encoding,
    Completed,
    Failed(String),
    Stopped,
}

impl JobState {
    /// 프로세스가 돌고 있는 상태.
    pub fn is_active(&self) -> bool {
        matches!(self, JobState::Downloading | JobState::Encoding)
    }
    /// 아직 끝나지 않은 상태(대기 포함).
    pub fn is_pending(&self) -> bool {
        matches!(self, JobState::Queued) || self.is_active()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Job {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub duration_secs: Option<f64>,
    /// 화면 표시용 화질 ("1080p60", 오디오만이면 "audio").
    pub quality_label: String,
    pub options: JobOptions,
    pub state: JobState,
    /// 현재 단계 기준 0.0~1.0
    pub progress: f32,
    pub speed: Option<String>,
    pub eta: Option<String>,
    pub output: Option<PathBuf>,
    /// unix epoch milliseconds
    pub created_at: u64,
}

/// 큐에 넣을 작업. UI 는 probe 결과로, CLI 는 Engine::add_url 이 만든다.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NewJob {
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub duration_secs: Option<f64>,
    pub quality_label: String,
    pub options: JobOptions,
}

/// 외부 실행 파일 위치. 테스트는 가짜 스크립트 경로를 넣는다.
#[derive(Clone, Debug)]
pub struct Tools {
    pub ytdlp: PathBuf,
    pub deno: PathBuf,
    pub ffmpeg: PathBuf,
}
```

- [ ] **Step 6: 테스트 통과 확인**

Run: `cargo test -p kiri-core`
Expected: `4 passed`.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock .gitignore crates/kiri-core
git commit -m "feat(core): workspace 전환과 작업 모델 추가"
```

---

### Task 2: yt-dlp 인자와 파서 (`ytdlp.rs`)

**Files:**
- Create: `crates/kiri-core/src/ytdlp.rs`, `crates/kiri-core/tests/fixtures/probe.json`
- Modify: `crates/kiri-core/src/lib.rs` (`pub mod ytdlp;` 추가)

**Interfaces:**
- Consumes: `model::{JobOptions, Preset, Tools}`
- Produces:
  - `pub fn is_youtube_url(s: &str) -> bool`
  - `pub struct Quality { format_id: String, height: u32, fps: Option<u32>, vcodec: String, filesize: Option<u64>, label: String }` (Serialize/Deserialize)
  - `pub struct VideoInfo { id, title, channel: Option<String>, duration_secs: Option<f64>, thumbnail: Option<String>, qualities: Vec<Quality>, subtitles: Vec<String>, auto_subtitles: Vec<String> }`
  - `pub fn parse_probe(json: &str) -> Result<VideoInfo, String>`
  - `pub fn resolve_quality(qualities: &[Quality], want: &str) -> Result<Option<Quality>, String>`
  - `pub fn pick_subtitles(info: &VideoInfo, langs: &[String]) -> (Vec<String>, bool)`
  - `pub fn probe_args(url: &str, tools: &Tools) -> Vec<String>`
  - `pub fn download_args(url: &str, o: &JobOptions, tools: &Tools, out_dir: &Path) -> Vec<String>`
  - `pub struct Progress { fraction: f32, speed: Option<String>, eta: Option<String> }`, `pub fn parse_progress(line: &str) -> Option<Progress>`

- [ ] **Step 1: fixture 작성**

`crates/kiri-core/tests/fixtures/probe.json`: `yt-dlp -J` 출력에서 이 모듈이 읽는 필드만 남긴 축약본이다. 스토리보드, 오디오 2개, 결합 포맷, 같은 해상도의 AVC/VP9 쌍이 들어 있다.

```json
{
  "id": "abc123",
  "title": "Rust in 100 Seconds",
  "channel": "Fireship",
  "duration": 144,
  "thumbnail": "https://i.ytimg.com/vi/abc123/maxresdefault.jpg",
  "formats": [
    {"format_id": "sb0", "ext": "mhtml", "vcodec": "none", "acodec": "none", "height": 45},
    {"format_id": "139", "ext": "m4a", "vcodec": "none", "acodec": "mp4a.40.5", "abr": 48.0, "filesize": 1000},
    {"format_id": "251", "ext": "webm", "vcodec": "none", "acodec": "opus", "abr": 130.0, "filesize": 3000000},
    {"format_id": "18", "ext": "mp4", "vcodec": "avc1.42001E", "acodec": "mp4a.40.2", "height": 360, "fps": 30, "tbr": 500, "filesize": 20000000},
    {"format_id": "136", "ext": "mp4", "vcodec": "avc1.4d401f", "acodec": "none", "height": 720, "fps": 30, "tbr": 1500, "filesize": 40000000},
    {"format_id": "247", "ext": "webm", "vcodec": "vp9", "acodec": "none", "height": 720, "fps": 30, "tbr": 1400, "filesize": 38000000},
    {"format_id": "299", "ext": "mp4", "vcodec": "avc1.64002a", "acodec": "none", "height": 1080, "fps": 60, "tbr": 5000, "filesize_approx": 85000000},
    {"format_id": "303", "ext": "webm", "vcodec": "vp09.00.41.08", "acodec": "none", "height": 1080, "fps": 60, "tbr": 4500, "filesize": 80000000}
  ],
  "subtitles": {"ko": [{}], "en": [{}], "live_chat": [{}]},
  "automatic_captions": {"en-orig": [{}], "en": [{}], "ja": [{}], "ko": [{}]}
}
```

- [ ] **Step 2: 실패하는 테스트 작성**

`crates/kiri-core/src/ytdlp.rs`:

```rust
//! yt-dlp 호출 인자와 출력 파서. 프로세스 실행은 runner 가 맡는다.
use crate::model::{JobOptions, Tools};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Preset;
    use std::path::PathBuf;

    const FIXTURE: &str = include_str!("../tests/fixtures/probe.json");

    fn tools() -> Tools {
        Tools {
            ytdlp: "/bin/yt-dlp".into(),
            deno: "/bin/deno".into(),
            ffmpeg: "/app/MacOS/ffmpeg".into(),
        }
    }

    #[test]
    fn accepts_youtube_urls() {
        for u in [
            "https://www.youtube.com/watch?v=abc123",
            "https://youtube.com/watch?v=abc123",
            "http://m.youtube.com/watch?v=abc123",
            "https://music.youtube.com/watch?v=abc123",
            "https://youtu.be/abc123",
            "https://www.youtube.com/shorts/abc123",
            "  https://youtu.be/abc123?t=10  ",
        ] {
            assert!(is_youtube_url(u), "{u}");
        }
    }

    #[test]
    fn rejects_other_input() {
        for u in [
            "",
            "hello",
            "youtube.com/watch?v=abc",
            "https://vimeo.com/1",
            "https://youtube.com.evil.com/watch?v=x",
            "https://notyoutube.com/watch?v=x",
            "https://youtube.com/",
            "ftp://youtube.com/watch?v=x",
        ] {
            assert!(!is_youtube_url(u), "{u}");
        }
    }

    #[test]
    fn parses_probe_metadata() {
        let info = parse_probe(FIXTURE).unwrap();
        assert_eq!(info.id, "abc123");
        assert_eq!(info.title, "Rust in 100 Seconds");
        assert_eq!(info.channel.as_deref(), Some("Fireship"));
        assert_eq!(info.duration_secs, Some(144.0));
        assert_eq!(info.subtitles, vec!["en", "ko"]);
        assert_eq!(info.auto_subtitles, vec!["en", "ja", "ko"]);
    }

    #[test]
    fn parses_qualities_best_per_resolution() {
        let q = parse_probe(FIXTURE).unwrap().qualities;
        let ids: Vec<_> = q.iter().map(|q| q.format_id.as_str()).collect();
        assert_eq!(ids, vec!["299", "136", "18"]); // 높은 tbr 승, 스토리보드·오디오 제외
        assert_eq!(q[0].label, "1080p60");
        assert_eq!(q[1].label, "720p");
        assert_eq!(q[0].vcodec, "AVC");
        assert_eq!(q[0].filesize, Some(85_000_000 + 3_000_000)); // 비디오 + bestaudio
        assert_eq!(q[2].filesize, Some(20_000_000)); // 오디오 포함 포맷은 더하지 않음
    }

    #[test]
    fn rejects_garbage_probe_output() {
        assert!(parse_probe("not json").is_err());
    }

    #[test]
    fn resolves_quality() {
        let q = parse_probe(FIXTURE).unwrap().qualities;
        let id = |w: &str| resolve_quality(&q, w).unwrap().map(|q| q.format_id);
        assert_eq!(id("best").as_deref(), Some("299"));
        assert_eq!(id("1080p").as_deref(), Some("299"));
        assert_eq!(id("720p").as_deref(), Some("136"));
        assert_eq!(id("480p").as_deref(), Some("18"));
        assert_eq!(id("144p").as_deref(), Some("18")); // 더 낮은 게 없으면 가장 낮은 것
        assert_eq!(id("audio"), None);
        assert!(resolve_quality(&q, "hd").is_err());
    }

    #[test]
    fn picks_manual_then_auto_subtitles() {
        let info = parse_probe(FIXTURE).unwrap();
        let langs = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(pick_subtitles(&info, &langs(&["ko", "en"])), (langs(&["ko", "en"]), false));
        assert_eq!(pick_subtitles(&info, &langs(&["ko", "ja", "fr"])), (langs(&["ko", "ja"]), true));
        assert_eq!(pick_subtitles(&info, &[]), (vec![], false));
    }

    #[test]
    fn probe_args_use_deno() {
        let a = probe_args("https://youtu.be/x", &tools());
        assert_eq!(a, vec!["-J", "--no-playlist", "--js-runtimes", "deno:/bin/deno", "https://youtu.be/x"]);
    }

    #[test]
    fn download_args_video_with_subs() {
        let o = JobOptions {
            format_id: Some("299".into()),
            preset: Preset::Mp4H264,
            subtitles: vec!["ko".into(), "ja".into()],
            auto_subtitles: true,
        };
        let a = download_args("https://youtu.be/x", &o, &tools(), &PathBuf::from("/c/jobs/1"));
        let s = a.join(" ");
        assert!(s.starts_with("-f 299+bestaudio/299 --no-playlist"), "{s}");
        assert!(s.contains("--ffmpeg-location /app/MacOS"), "{s}");
        assert!(s.contains("--js-runtimes deno:/bin/deno"), "{s}");
        assert!(s.contains("--write-subs --write-auto-subs --sub-langs ko,ja --convert-subs srt"), "{s}");
        assert!(s.contains(&format!("--progress-template {PROGRESS_TEMPLATE}")), "{s}");
        assert_eq!(a[a.len() - 2], "/c/jobs/1/%(title)s.%(ext)s");
        assert_eq!(a.last().unwrap(), "https://youtu.be/x");
    }

    #[test]
    fn download_args_audio_only_without_subs() {
        let o = JobOptions { format_id: Some("299".into()), preset: Preset::Mp3, subtitles: vec![], auto_subtitles: false };
        let a = download_args("u", &o, &tools(), Path::new("/d"));
        assert_eq!(&a[..2], ["-f", "bestaudio"]);
        assert!(!a.iter().any(|x| x.starts_with("--write")));
    }

    #[test]
    fn parses_progress_lines() {
        let p = parse_progress("KIRI|  45.3%|   2.10MiB/s|00:12").unwrap();
        assert!((p.fraction - 0.453).abs() < 1e-4);
        assert_eq!(p.speed.as_deref(), Some("2.10MiB/s"));
        assert_eq!(p.eta.as_deref(), Some("00:12"));
        let done = parse_progress("KIRI|100.0%|Unknown B/s|NA").unwrap();
        assert_eq!((done.fraction, done.speed, done.eta), (1.0, None, None));
        assert!(parse_progress("[download] Destination: x.mp4").is_none());
        assert!(parse_progress("KIRI|  N/A%|x|y").is_none());
    }
}
```

`lib.rs`에 `pub mod ytdlp;`를 추가한다.

- [ ] **Step 3: 테스트가 실패하는지 확인**

Run: `cargo test -p kiri-core ytdlp`
Expected: 컴파일 에러(`cannot find function is_youtube_url`).

- [ ] **Step 4: 구현**

`ytdlp.rs`의 `#[cfg(test)]` 위에 추가한다.

```rust
pub const PROGRESS_TEMPLATE: &str =
    "download:KIRI|%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s";

const HOSTS: [&str; 5] = ["youtube.com", "www.youtube.com", "m.youtube.com", "music.youtube.com", "youtu.be"];

/// http(s) 이고 호스트가 YouTube 이며 경로가 있는 URL.
pub fn is_youtube_url(s: &str) -> bool {
    let s = s.trim();
    let Some(rest) = s.strip_prefix("https://").or_else(|| s.strip_prefix("http://")) else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority.split(':').next().unwrap_or("").to_ascii_lowercase();
    let has_path = rest.len() > authority.len() + 1;
    HOSTS.contains(&host.as_str()) && has_path
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Quality {
    pub format_id: String,
    pub height: u32,
    pub fps: Option<u32>,
    pub vcodec: String,
    /// 예상 용량 (비디오 + bestaudio)
    pub filesize: Option<u64>,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub channel: Option<String>,
    pub duration_secs: Option<f64>,
    pub thumbnail: Option<String>,
    /// 높은 화질부터
    pub qualities: Vec<Quality>,
    pub subtitles: Vec<String>,
    pub auto_subtitles: Vec<String>,
}

fn text(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}
fn num(v: &Value, k: &str) -> f64 {
    v[k].as_f64().unwrap_or(0.0)
}
fn size(f: &Value) -> Option<u64> {
    f["filesize"].as_u64().or(f["filesize_approx"].as_u64())
}
fn has_audio(f: &Value) -> bool {
    f["acodec"].as_str().is_some_and(|a| a != "none")
}
fn is_video(f: &Value) -> bool {
    f["vcodec"].as_str().is_some_and(|v| v != "none")
        && f["height"].as_u64().is_some()
        && f["ext"].as_str() != Some("mhtml")
}
fn is_audio(f: &Value) -> bool {
    f["vcodec"].as_str() == Some("none") && has_audio(f)
}
fn label(height: u32, fps: Option<u32>) -> String {
    match fps {
        Some(f) if f > 30 => format!("{height}p{f}"),
        _ => format!("{height}p"),
    }
}
fn short_codec(vcodec: &str) -> String {
    match vcodec.split('.').next().unwrap_or("") {
        "avc1" | "h264" => "AVC".into(),
        "vp09" | "vp9" => "VP9".into(),
        "av01" => "AV1".into(),
        "hev1" | "hvc1" => "HEVC".into(),
        other => other.to_uppercase(),
    }
}
fn langs(v: &Value, auto: bool) -> Vec<String> {
    let mut out: Vec<String> = v
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    out.retain(|k| k != "live_chat" && !(auto && k.ends_with("-orig")));
    out.sort();
    out
}

pub fn parse_probe(json: &str) -> Result<VideoInfo, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("invalid yt-dlp output: {e}"))?;
    let formats = v["formats"].as_array().cloned().unwrap_or_default();
    let audio_size = formats
        .iter()
        .filter(|f| is_audio(f))
        .max_by(|a, b| num(a, "abr").total_cmp(&num(b, "abr")))
        .and_then(size);

    // (해상도, fps) 마다 tbr 가 가장 높은 포맷 하나
    let mut best: Vec<(Quality, f64)> = Vec::new();
    for f in formats.iter().filter(|f| is_video(f)) {
        let height = f["height"].as_u64().unwrap_or(0) as u32;
        let fps = f["fps"].as_f64().map(|x| x.round() as u32);
        let tbr = num(f, "tbr");
        let extra = if has_audio(f) { 0 } else { audio_size.unwrap_or(0) };
        let q = Quality {
            format_id: text(f, "format_id"),
            height,
            fps,
            vcodec: short_codec(f["vcodec"].as_str().unwrap_or("")),
            filesize: size(f).map(|s| s + extra),
            label: label(height, fps),
        };
        match best.iter_mut().find(|(b, _)| b.height == height && b.fps == fps) {
            Some(slot) if tbr > slot.1 => *slot = (q, tbr),
            Some(_) => {}
            None => best.push((q, tbr)),
        }
    }
    let mut qualities: Vec<Quality> = best.into_iter().map(|(q, _)| q).collect();
    qualities.sort_by(|a, b| b.height.cmp(&a.height).then(b.fps.cmp(&a.fps)));

    Ok(VideoInfo {
        id: text(&v, "id"),
        title: text(&v, "title"),
        channel: v["channel"].as_str().or(v["uploader"].as_str()).map(String::from),
        duration_secs: v["duration"].as_f64(),
        thumbnail: v["thumbnail"].as_str().map(String::from),
        qualities,
        subtitles: langs(&v["subtitles"], false),
        auto_subtitles: langs(&v["automatic_captions"], true),
    })
}

/// "best" | "audio" | "<N>p". 같은 높이가 없으면 그보다 낮은 것 중 최고, 그것도 없으면 가장 낮은 것.
/// Ok(None) 은 오디오만.
pub fn resolve_quality(qualities: &[Quality], want: &str) -> Result<Option<Quality>, String> {
    match want {
        "audio" => Ok(None),
        "best" => Ok(qualities.first().cloned()),
        w => {
            let h: u32 = w
                .strip_suffix('p')
                .and_then(|n| n.parse().ok())
                .ok_or_else(|| w.to_string())?;
            Ok(qualities.iter().find(|q| q.height <= h).or(qualities.last()).cloned())
        }
    }
}

/// 요청 언어 중 수동 자막이 있으면 그것을, 없으면 자동 생성 자막을 고른다. 둘 다 없으면 버린다.
pub fn pick_subtitles(info: &VideoInfo, langs: &[String]) -> (Vec<String>, bool) {
    let mut out = Vec::new();
    let mut auto = false;
    for l in langs {
        if info.subtitles.contains(l) {
            out.push(l.clone());
        } else if info.auto_subtitles.contains(l) {
            out.push(l.clone());
            auto = true;
        }
    }
    (out, auto)
}

fn deno_flag(tools: &Tools) -> String {
    format!("deno:{}", tools.deno.display())
}

pub fn probe_args(url: &str, tools: &Tools) -> Vec<String> {
    vec!["-J".into(), "--no-playlist".into(), "--js-runtimes".into(), deno_flag(tools), url.into()]
}

pub fn download_args(url: &str, o: &JobOptions, tools: &Tools, out_dir: &Path) -> Vec<String> {
    let format = match &o.format_id {
        Some(id) if !o.preset.is_audio_only() => format!("{id}+bestaudio/{id}"),
        _ => "bestaudio".into(),
    };
    // ffmpeg·ffprobe 가 같은 디렉터리에 있으므로 디렉터리를 넘긴다.
    let ffmpeg_dir = tools.ffmpeg.parent().unwrap_or(Path::new("/")).display().to_string();
    let mut a: Vec<String> = vec![
        "-f".into(), format,
        "--no-playlist".into(),
        "--newline".into(),
        "--color".into(), "never".into(),
        "--progress-template".into(), PROGRESS_TEMPLATE.into(),
        "--ffmpeg-location".into(), ffmpeg_dir,
        "--js-runtimes".into(), deno_flag(tools),
    ];
    if !o.subtitles.is_empty() {
        a.push("--write-subs".into());
        if o.auto_subtitles {
            a.push("--write-auto-subs".into());
        }
        a.extend(["--sub-langs".into(), o.subtitles.join(","), "--convert-subs".into(), "srt".into()]);
    }
    a.extend([
        "-o".into(),
        out_dir.join("%(title)s.%(ext)s").display().to_string(),
        url.into(),
    ]);
    a
}

#[derive(Clone, Debug, PartialEq)]
pub struct Progress {
    pub fraction: f32,
    pub speed: Option<String>,
    pub eta: Option<String>,
}

/// PROGRESS_TEMPLATE 이 찍은 줄만 해석한다.
pub fn parse_progress(line: &str) -> Option<Progress> {
    let rest = line.trim().strip_prefix("KIRI|")?;
    let mut parts = rest.split('|');
    let pct: f32 = parts.next()?.trim().trim_end_matches('%').trim().parse().ok()?;
    let clean = |s: Option<&str>| {
        s.map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with("Unknown") && *s != "N/A" && *s != "NA")
            .map(String::from)
    };
    Some(Progress {
        fraction: (pct / 100.0).clamp(0.0, 1.0),
        speed: clean(parts.next()),
        eta: clean(parts.next()),
    })
}
```

- [ ] **Step 5: 테스트 통과 확인**

Run: `cargo test -p kiri-core ytdlp`
Expected: `11 passed`.

- [ ] **Step 6: Commit**

```bash
git add crates/kiri-core
git commit -m "feat(core): yt-dlp 인자 생성과 probe·진행률 파서 추가"
```

---

### Task 3: ffmpeg 프리셋 인자와 진행률 (`ffmpeg.rs`)

**Files:**
- Create: `crates/kiri-core/src/ffmpeg.rs`
- Modify: `crates/kiri-core/src/lib.rs` (`pub mod ffmpeg;`)

**Interfaces:**
- Consumes: `model::Preset`
- Produces:
  - `pub fn hw_capable(p: Preset) -> bool`
  - `pub fn encode_args(p: Preset, hw: bool, input: &Path, output: &Path) -> Option<Vec<String>>` (Original이면 None)
  - `pub fn parse_progress(line: &str, duration_secs: Option<f64>) -> Option<f32>`

- [ ] **Step 1: 실패하는 테스트 작성**

`crates/kiri-core/src/ffmpeg.rs`:

```rust
//! 인코딩 프리셋 → ffmpeg 인자. VideoToolbox(가속) 와 소프트웨어 두 경로를 가진다.
use crate::model::Preset;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    fn args(p: Preset, hw: bool) -> Vec<String> {
        encode_args(p, hw, Path::new("/w/in.webm"), Path::new("/w/out/in.mp4")).unwrap()
    }

    #[test]
    fn original_has_no_encode_step() {
        assert!(encode_args(Preset::Original, true, Path::new("a"), Path::new("b")).is_none());
    }

    #[test]
    fn h264_hw_and_sw() {
        let hw = args(Preset::Mp4H264, true).join(" ");
        assert!(hw.contains("-hwaccel videotoolbox -i /w/in.webm"), "{hw}");
        assert!(hw.contains("-c:v h264_videotoolbox -q:v 65"), "{hw}");
        assert!(hw.contains("-c:a aac -b:a 192k -movflags +faststart"), "{hw}");
        let sw = args(Preset::Mp4H264, false).join(" ");
        assert!(!sw.contains("hwaccel"), "{sw}");
        assert!(sw.contains("-c:v libx264 -crf 20 -preset medium"), "{sw}");
    }

    #[test]
    fn hevc_and_prores_paths() {
        assert!(args(Preset::Mp4Hevc, true).join(" ").contains("-c:v hevc_videotoolbox -q:v 60 -tag:v hvc1"));
        assert!(args(Preset::Mp4Hevc, false).join(" ").contains("-c:v libx265 -crf 24 -tag:v hvc1"));
        assert!(args(Preset::MovProres, true).join(" ").contains("-c:v prores_videotoolbox -profile:v 2 -c:a pcm_s16le"));
        assert!(args(Preset::MovProres, false).join(" ").contains("-c:v prores_ks -profile:v 2"));
    }

    #[test]
    fn vp9_is_software_even_with_hw() {
        let a = args(Preset::WebmVp9, true).join(" ");
        assert!(a.contains("-c:v libvpx-vp9 -crf 32 -b:v 0 -row-mt 1 -c:a libopus"), "{a}");
        assert!(!hw_capable(Preset::WebmVp9));
        assert!(hw_capable(Preset::Mp4H264) && hw_capable(Preset::Mp4Hevc) && hw_capable(Preset::MovProres));
    }

    #[test]
    fn audio_presets_drop_video_and_skip_hwaccel() {
        let mp3 = args(Preset::Mp3, true).join(" ");
        assert!(!mp3.contains("hwaccel") && mp3.contains("-vn -c:a libmp3lame -q:a 2"), "{mp3}");
        assert!(args(Preset::M4a, false).join(" ").contains("-vn -c:a aac -b:a 192k"));
    }

    #[test]
    fn progress_flags_and_output_last() {
        let a = args(Preset::Mp4H264, false);
        assert_eq!(&a[..2], ["-hide_banner", "-y"]);
        assert!(a.join(" ").contains("-progress pipe:1 -nostats /w/out/in.mp4"));
        assert_eq!(a.last().unwrap(), "/w/out/in.mp4");
    }

    #[test]
    fn parses_progress() {
        assert_eq!(parse_progress("out_time_us=72000000", Some(144.0)), Some(0.5));
        assert_eq!(parse_progress("out_time_us=999000000", Some(144.0)), Some(1.0));
        assert_eq!(parse_progress("progress=end", None), Some(1.0));
        assert_eq!(parse_progress("out_time_us=N/A", Some(10.0)), None);
        assert_eq!(parse_progress("out_time_us=100", None), None);
        assert_eq!(parse_progress("frame=10", Some(10.0)), None);
    }
}
```

`lib.rs`에 `pub mod ffmpeg;`를 추가한다.

- [ ] **Step 2: 테스트가 실패하는지 확인**

Run: `cargo test -p kiri-core ffmpeg`
Expected: 컴파일 에러(`cannot find function encode_args`).

- [ ] **Step 3: 구현**

```rust
const AAC_MP4: &[&str] = &["-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart"];

/// VideoToolbox 인코더가 있는 프리셋.
pub fn hw_capable(p: Preset) -> bool {
    matches!(p, Preset::Mp4H264 | Preset::Mp4Hevc | Preset::MovProres)
}

pub fn encode_args(p: Preset, hw: bool, input: &Path, output: &Path) -> Option<Vec<String>> {
    let (video, audio): (&[&str], &[&str]) = match (p, hw) {
        (Preset::Original, _) => return None,
        (Preset::Mp4H264, true) => (&["-c:v", "h264_videotoolbox", "-q:v", "65"], AAC_MP4),
        (Preset::Mp4H264, false) => (&["-c:v", "libx264", "-crf", "20", "-preset", "medium"], AAC_MP4),
        (Preset::Mp4Hevc, true) => (&["-c:v", "hevc_videotoolbox", "-q:v", "60", "-tag:v", "hvc1"], AAC_MP4),
        (Preset::Mp4Hevc, false) => (&["-c:v", "libx265", "-crf", "24", "-tag:v", "hvc1"], AAC_MP4),
        (Preset::MovProres, true) => (&["-c:v", "prores_videotoolbox", "-profile:v", "2"], &["-c:a", "pcm_s16le"]),
        (Preset::MovProres, false) => (&["-c:v", "prores_ks", "-profile:v", "2"], &["-c:a", "pcm_s16le"]),
        // 맥에는 VP9 하드웨어 인코더가 없다. 디코딩만 가속한다.
        (Preset::WebmVp9, _) => (&["-c:v", "libvpx-vp9", "-crf", "32", "-b:v", "0", "-row-mt", "1"], &["-c:a", "libopus"]),
        (Preset::Mp3, _) => (&["-vn"], &["-c:a", "libmp3lame", "-q:a", "2"]),
        (Preset::M4a, _) => (&["-vn"], &["-c:a", "aac", "-b:a", "192k"]),
    };
    let mut a: Vec<String> = vec!["-hide_banner".into(), "-y".into()];
    if hw && !p.is_audio_only() {
        // 지원하지 않는 코덱(AV1 등)이면 ffmpeg 가 알아서 소프트웨어 디코딩한다.
        a.extend(["-hwaccel".into(), "videotoolbox".into()]);
    }
    a.extend(["-i".into(), input.display().to_string()]);
    a.extend(video.iter().chain(audio).map(|s| s.to_string()));
    a.extend(["-progress".into(), "pipe:1".into(), "-nostats".into()]);
    a.push(output.display().to_string());
    Some(a)
}

/// `-progress pipe:1` 의 key=value 줄. 길이를 모르면 진행률도 모른다.
pub fn parse_progress(line: &str, duration_secs: Option<f64>) -> Option<f32> {
    let (k, v) = line.trim().split_once('=')?;
    match k {
        "progress" if v == "end" => Some(1.0),
        "out_time_us" => {
            let d = duration_secs.filter(|d| *d > 0.0)?;
            let us: f64 = v.parse().ok()?;
            Some(((us / 1_000_000.0) / d).clamp(0.0, 1.0) as f32)
        }
        _ => None,
    }
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p kiri-core ffmpeg`
Expected: `7 passed`.

- [ ] **Step 5: Commit**

```bash
git add crates/kiri-core
git commit -m "feat(core): ffmpeg 인코딩 프리셋(VideoToolbox/소프트웨어)과 진행률 파서 추가"
```

---
### Task 4: 프로세스 실행기와 파일 유틸 (`runner.rs`, `files.rs`, `testutil.rs`)

**Files:**
- Create: `crates/kiri-core/src/runner.rs`, `crates/kiri-core/src/files.rs`, `crates/kiri-core/src/testutil.rs`
- Modify: `crates/kiri-core/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub enum RunError { Cancelled, Spawn(String), Failed { tail: Vec<String> } }`, `RunError::summary() -> String`, `impl Display`
  - `pub async fn runner::run(program: &Path, args: &[String], cancel: &CancellationToken, on_line: impl FnMut(&str)) -> Result<(), RunError>`
  - `pub async fn runner::output(program: &Path, args: &[String]) -> Result<String, RunError>`
  - `files::{unique_path(dir, name) -> PathBuf, move_file(from, to) -> io::Result<()>, check_writable(dir) -> io::Result<()>, find_media(dir) -> io::Result<Option<PathBuf>>, subtitle_files(dir) -> io::Result<Vec<PathBuf>>}`
  - (테스트 전용) `testutil::script(dir: &Path, name: &str, body: &str) -> PathBuf`

**왜 프로세스 그룹인가:** `yt-dlp_macos`는 PyInstaller 바이너리라 부트로더가 실제 Python 프로세스를 자식으로 띄운다. 그 Python 프로세스는 병합할 때 다시 ffmpeg를 띄운다. 부모만 kill하면 손자 프로세스가 남아 계속 다운로드한다. 그래서 자식을 새 프로세스 그룹(`process_group(0)`)으로 띄우고, 취소할 때 `killpg`로 그룹 전체를 죽인다.

- [ ] **Step 1: 테스트 유틸 작성**

`crates/kiri-core/src/testutil.rs`:

```rust
//! 테스트 전용 도우미. 실제 yt-dlp/ffmpeg 대신 sh 스크립트를 실행 파일로 쓴다.
use std::{fs, os::unix::fs::PermissionsExt, path::{Path, PathBuf}};

pub fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}
```

`lib.rs`:

```rust
pub mod files;
pub mod runner;
#[cfg(test)]
pub(crate) mod testutil;
```

- [ ] **Step 2: runner 실패 테스트 작성**

`crates/kiri-core/src/runner.rs`:

```rust
//! 자식 프로세스 실행. 줄 단위 출력 콜백, 취소(프로세스 그룹 kill), 실패 시 stderr 꼬리.
use std::{collections::VecDeque, fmt, path::Path, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::script;
    use std::time::Duration;

    #[tokio::test]
    async fn collects_stdout_and_stderr_lines() {
        let d = tempfile::tempdir().unwrap();
        let s = script(d.path(), "ok", "echo out1\necho err1 >&2");
        let mut lines = vec![];
        run(&s, &[], &CancellationToken::new(), |l| lines.push(l.to_string())).await.unwrap();
        lines.sort();
        assert_eq!(lines, vec!["err1", "out1"]);
    }

    #[tokio::test]
    async fn failure_returns_stderr_tail() {
        let d = tempfile::tempdir().unwrap();
        let s = script(d.path(), "fail", "echo noise\necho 'ERROR: Video unavailable' >&2\nexit 3");
        let err = run(&s, &[], &CancellationToken::new(), |_| {}).await.unwrap_err();
        assert_eq!(err, RunError::Failed { tail: vec!["ERROR: Video unavailable".into()] });
        assert_eq!(err.summary(), "ERROR: Video unavailable");
    }

    #[tokio::test]
    async fn spawn_error_for_missing_binary() {
        let err = run(Path::new("/nonexistent/yt-dlp"), &[], &CancellationToken::new(), |_| {}).await.unwrap_err();
        assert!(matches!(err, RunError::Spawn(_)), "{err:?}");
    }

    #[tokio::test]
    async fn cancel_kills_whole_process_group() {
        let d = tempfile::tempdir().unwrap();
        // 손자 프로세스(sleep)의 pid 를 찍고 기다린다.
        let s = script(d.path(), "slow", "sleep 30 &\necho $!\nwait");
        let token = CancellationToken::new();
        let t2 = token.clone();
        let (pid_tx, pid_rx) = std::sync::mpsc::channel::<i32>();
        let task = tokio::spawn(async move {
            run(&s, &[], &t2, |l| {
                if let Ok(pid) = l.trim().parse() {
                    let _ = pid_tx.send(pid);
                }
            })
            .await
        });
        let grandchild = tokio::task::spawn_blocking(move || pid_rx.recv_timeout(Duration::from_secs(5)).unwrap())
            .await
            .unwrap();
        token.cancel();
        let res = tokio::time::timeout(Duration::from_secs(5), task).await.unwrap().unwrap();
        assert_eq!(res, Err(RunError::Cancelled));
        tokio::time::sleep(Duration::from_millis(200)).await;
        let alive = unsafe { libc::kill(grandchild, 0) } == 0;
        assert!(!alive, "grandchild {grandchild} survived cancel");
    }

    #[tokio::test]
    async fn output_returns_stdout() {
        let d = tempfile::tempdir().unwrap();
        let s = script(d.path(), "v", "echo 2026.09.30");
        assert_eq!(output(&s, &[]).await.unwrap(), "2026.09.30\n");
        let f = script(d.path(), "f", "echo bad >&2; exit 1");
        assert_eq!(output(&f, &[]).await.unwrap_err().summary(), "bad");
    }
}
```

- [ ] **Step 3: 테스트가 실패하는지 확인**

Run: `cargo test -p kiri-core runner`
Expected: 컴파일 에러(`cannot find function run`).

- [ ] **Step 4: runner 구현**

`runner.rs`의 `#[cfg(test)]` 위에 추가한다.

```rust
const TAIL: usize = 20;

#[derive(Debug, Clone, PartialEq)]
pub enum RunError {
    Cancelled,
    Spawn(String),
    Failed { tail: Vec<String> },
}

impl RunError {
    /// 사용자에게 보여줄 한 줄: stderr 의 마지막 비어 있지 않은 줄.
    pub fn summary(&self) -> String {
        match self {
            RunError::Cancelled => "cancelled".into(),
            RunError::Spawn(e) => e.clone(),
            RunError::Failed { tail } => tail
                .iter()
                .rev()
                .find(|l| !l.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| "process failed".into()),
        }
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.summary())
    }
}

fn command(program: &Path, args: &[String]) -> Command {
    let mut c = Command::new(program);
    c.args(args)
        .stdin(Stdio::null())
        .process_group(0) // 취소 시 손자까지 한 번에 죽이기 위해
        .kill_on_drop(true);
    c
}

fn kill_group(child: &tokio::process::Child) {
    if let Some(pid) = child.id() {
        // SAFETY: killpg 는 시그널만 보낸다. 실패(이미 종료)는 무시한다.
        unsafe {
            libc::killpg(pid as i32, libc::SIGKILL);
        }
    }
}

pub async fn run(
    program: &Path,
    args: &[String],
    cancel: &CancellationToken,
    mut on_line: impl FnMut(&str),
) -> Result<(), RunError> {
    let mut child = command(program, args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| RunError::Spawn(format!("{}: {e}", program.display())))?;

    let (tx, mut rx) = mpsc::unbounded_channel::<(bool, String)>();
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let tx_err = tx.clone();
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            if tx.send((false, l)).is_err() {
                break;
            }
        }
    });
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            if tx_err.send((true, l)).is_err() {
                break;
            }
        }
    });

    let mut tail: VecDeque<String> = VecDeque::with_capacity(TAIL);
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                kill_group(&child);
                let _ = child.wait().await;
                return Err(RunError::Cancelled);
            }
            msg = rx.recv() => match msg {
                Some((is_err, line)) => {
                    if is_err {
                        if tail.len() == TAIL {
                            tail.pop_front();
                        }
                        tail.push_back(line.clone());
                    }
                    on_line(&line);
                }
                None => break, // 두 스트림 모두 닫힘
            }
        }
    }
    let status = tokio::select! {
        _ = cancel.cancelled() => {
            kill_group(&child);
            let _ = child.wait().await;
            return Err(RunError::Cancelled);
        }
        s = child.wait() => s.map_err(|e| RunError::Spawn(e.to_string()))?,
    };
    if status.success() {
        Ok(())
    } else {
        Err(RunError::Failed { tail: Vec::from(tail) })
    }
}

/// 짧은 명령의 stdout 전체 (probe, --version).
pub async fn output(program: &Path, args: &[String]) -> Result<String, RunError> {
    let out = command(program, args)
        .output()
        .await
        .map_err(|e| RunError::Spawn(format!("{}: {e}", program.display())))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    let lines: Vec<&str> = err.lines().collect();
    let start = lines.len().saturating_sub(TAIL);
    Err(RunError::Failed { tail: lines[start..].iter().map(|s| s.to_string()).collect() })
}
```

- [ ] **Step 5: runner 테스트 통과 확인**

Run: `cargo test -p kiri-core runner`
Expected: `5 passed`.

- [ ] **Step 6: files 실패 테스트 작성**

`crates/kiri-core/src/files.rs`:

```rust
//! 다운로드 결과 파일 다루기.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn unique_path_returns_name_when_free() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(unique_path(d.path(), "a.mp4"), d.path().join("a.mp4"));
    }

    #[test]
    fn unique_path_appends_counter() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("a.mp4"), "").unwrap();
        assert_eq!(unique_path(d.path(), "a.mp4"), d.path().join("a (1).mp4"));
        fs::write(d.path().join("a (1).mp4"), "").unwrap();
        assert_eq!(unique_path(d.path(), "a.mp4"), d.path().join("a (2).mp4"));
        fs::write(d.path().join("noext"), "").unwrap();
        assert_eq!(unique_path(d.path(), "noext"), d.path().join("noext (1)"));
    }

    #[test]
    fn move_file_moves() {
        let d = tempfile::tempdir().unwrap();
        let (a, b) = (d.path().join("a"), d.path().join("b"));
        fs::write(&a, "x").unwrap();
        move_file(&a, &b).unwrap();
        assert!(!a.exists());
        assert_eq!(fs::read_to_string(b).unwrap(), "x");
    }

    #[test]
    fn check_writable_creates_dir_and_detects_readonly() {
        let d = tempfile::tempdir().unwrap();
        let sub = d.path().join("new/dir");
        check_writable(&sub).unwrap();
        assert!(sub.is_dir());
        let ro = d.path().join("ro");
        fs::create_dir(&ro).unwrap();
        fs::set_permissions(&ro, fs::Permissions::from_mode(0o555)).unwrap();
        assert!(check_writable(&ro).is_err());
        fs::set_permissions(&ro, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn finds_media_and_subtitles() {
        let d = tempfile::tempdir().unwrap();
        for n in ["v.ko.srt", "v.mp4.part", ".hidden", "v.webm"] {
            fs::write(d.path().join(n), "").unwrap();
        }
        fs::create_dir(d.path().join("out")).unwrap();
        assert_eq!(find_media(d.path()).unwrap(), Some(d.path().join("v.webm")));
        assert_eq!(subtitle_files(d.path()).unwrap(), vec![d.path().join("v.ko.srt")]);
    }
}
```

- [ ] **Step 7: 테스트가 실패하는지 확인**

Run: `cargo test -p kiri-core files`
Expected: 컴파일 에러(`cannot find function unique_path`).

- [ ] **Step 8: files 구현**

```rust
/// dir 안에서 겹치지 않는 경로. "a.mp4" → "a (1).mp4" → "a (2).mp4" …
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let p = Path::new(name);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = p.extension().and_then(|e| e.to_str());
    (1..)
        .map(|i| match ext {
            Some(e) => dir.join(format!("{stem} ({i}).{e}")),
            None => dir.join(format!("{stem} ({i})")),
        })
        .find(|c| !c.exists())
        .expect("unbounded range")
}

/// rename, 볼륨이 다르면 복사 후 삭제.
pub fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            fs::copy(from, to)?;
            fs::remove_file(from)
        }
        r => r,
    }
}

/// 폴더를 만들고 실제로 파일을 써 본다.
pub fn check_writable(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let probe = dir.join(".kiri-write-test");
    fs::write(&probe, b"")?;
    fs::remove_file(probe)
}

const AUX_EXT: [&str; 8] = ["srt", "vtt", "part", "ytdl", "json", "jpg", "webp", "png"];

fn is_aux(p: &Path) -> bool {
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    name.starts_with('.') || name.contains(".part-Frag") || AUX_EXT.contains(&ext)
}

/// 작업 폴더에서 결과 미디어(자막·임시 파일이 아닌 첫 파일).
pub fn find_media(dir: &Path) -> io::Result<Option<PathBuf>> {
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_file() && !is_aux(&p) {
            return Ok(Some(p));
        }
    }
    Ok(None)
}

pub fn subtitle_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = vec![];
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("srt") {
            out.push(p);
        }
    }
    out.sort();
    Ok(out)
}
```

- [ ] **Step 9: 테스트 통과 확인**

Run: `cargo test -p kiri-core`
Expected: 지금까지의 테스트가 모두 통과한다(runner 5, files 5 포함).

- [ ] **Step 10: Commit**

```bash
git add crates/kiri-core
git commit -m "feat(core): 프로세스 그룹 취소를 지원하는 실행기와 파일 유틸 추가"
```

---

### Task 5: 다운로드 파이프라인 (`pipeline.rs`)

**Files:**
- Create: `crates/kiri-core/src/pipeline.rs`
- Modify: `crates/kiri-core/src/lib.rs` (`pub mod pipeline;`), `crates/kiri-core/src/testutil.rs` (가짜 도구, 샘플 Job)

**Interfaces:**
- Consumes: `ytdlp::{download_args, parse_progress}`, `ffmpeg::{encode_args, hw_capable, parse_progress}`, `runner::{run, RunError}`, `files::*`, `model::{Job, Tools}`
- Produces:
  - `pub struct PipelineCfg { tools: Tools, hw_accel: bool, download_dir: PathBuf, work_dir: PathBuf, log_path: PathBuf }`
  - `pub enum Stage { Downloading, Encoding }`, `pub struct Report { stage, progress: f32, speed: Option<String>, eta: Option<String> }`
  - `pub enum PipelineError { Cancelled, Failed(String) }`
  - `pub const ERR_DIR_UNWRITABLE: &str = "error.download_dir_unwritable"`, `pub const ERR_NO_OUTPUT: &str = "error.no_output"`
  - `pub async fn run(job: &Job, cfg: &PipelineCfg, cancel: &CancellationToken, report: impl FnMut(Report)) -> Result<PathBuf, PipelineError>`: 최종 미디어 경로를 돌려준다.
  - (테스트 전용) `testutil::{FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, FAKE_FFMPEG_VT_FAILS, job(preset) -> Job, tools(dir, ytdlp_body, ffmpeg_body) -> Tools}`

- [ ] **Step 1: 테스트 유틸 확장**

`testutil.rs`에 추가한다.

```rust
use crate::model::{Job, JobOptions, JobState, Preset, Tools};

/// `-o <dir>/%(title)s.%(ext)s` 를 읽어 그 폴더에 결과물과 자막을 만든다.
pub const FAKE_YTDLP_DOWNLOAD: &str = r#"out=""
while [ $# -gt 0 ]; do case "$1" in -o) out="$2"; shift;; esac; shift; done
dir=$(dirname "$out")
echo "KIRI|  50.0%|  1.00MiB/s|00:01"
: > "$dir/Fake Video.mp4"
: > "$dir/Fake Video.ko.srt"
echo "KIRI|100.0%|  1.00MiB/s|00:00""#;

/// 마지막 인자(출력 경로)에 파일을 만든다.
pub const FAKE_FFMPEG: &str = r#"for a; do last="$a"; done
mkdir -p "$(dirname "$last")"
echo "out_time_us=1000000"
echo "progress=end"
: > "$last""#;

/// VideoToolbox 인코더를 쓰면 실패한다.
pub const FAKE_FFMPEG_VT_FAILS: &str = r#"case "$*" in *videotoolbox*) echo "Error: cannot open videotoolbox encoder" >&2; exit 1;; esac
for a; do last="$a"; done
mkdir -p "$(dirname "$last")"
: > "$last""#;

pub fn tools(dir: &Path, ytdlp_body: &str, ffmpeg_body: &str) -> Tools {
    Tools {
        ytdlp: script(dir, "yt-dlp", ytdlp_body),
        deno: dir.join("deno"),
        ffmpeg: script(dir, "ffmpeg", ffmpeg_body),
    }
}

pub fn job(preset: Preset) -> Job {
    Job {
        id: 1,
        url: "https://youtu.be/abc123".into(),
        title: "Fake Video".into(),
        thumbnail: None,
        duration_secs: Some(2.0),
        quality_label: "1080p".into(),
        options: JobOptions { format_id: Some("299".into()), preset, subtitles: vec!["ko".into()], auto_subtitles: false },
        state: JobState::Downloading,
        progress: 0.0,
        speed: None,
        eta: None,
        output: None,
        created_at: 0,
    }
}
```

- [ ] **Step 2: 실패하는 테스트 작성**

`crates/kiri-core/src/pipeline.rs`:

```rust
//! 작업 하나의 전 과정: 다운로드 → (필요하면) 인코딩 → 저장 위치로 이동.
use crate::{
    ffmpeg, files,
    model::{Job, Tools},
    runner::{self, RunError},
    ytdlp,
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::Preset, testutil::{self, *}};
    use std::os::unix::fs::PermissionsExt;

    struct Env {
        _d: tempfile::TempDir,
        cfg: PipelineCfg,
    }

    fn env(ytdlp: &str, ffmpeg: &str, hw: bool) -> Env {
        let d = tempfile::tempdir().unwrap();
        let bin = d.path().join("bin");
        fs::create_dir(&bin).unwrap();
        let cfg = PipelineCfg {
            tools: testutil::tools(&bin, ytdlp, ffmpeg),
            hw_accel: hw,
            download_dir: d.path().join("Movies/kiri"),
            work_dir: d.path().join("cache/jobs/1"),
            log_path: d.path().join("logs/1.log"),
        };
        Env { _d: d, cfg }
    }

    async fn go(e: &Env, preset: Preset) -> (Result<PathBuf, PipelineError>, Vec<Report>) {
        let mut reports = vec![];
        let r = run(&job(preset), &e.cfg, &CancellationToken::new(), |r| reports.push(r)).await;
        (r, reports)
    }

    #[tokio::test]
    async fn original_downloads_and_moves_with_subtitles() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, true);
        let (r, reports) = go(&e, Preset::Original).await;
        let out = r.unwrap();
        assert_eq!(out, e.cfg.download_dir.join("Fake Video.mp4"));
        assert!(out.exists());
        assert!(e.cfg.download_dir.join("Fake Video.ko.srt").exists());
        assert!(reports.iter().any(|r| r.stage == Stage::Downloading && r.progress == 0.5));
        assert!(reports.iter().all(|r| r.stage == Stage::Downloading));
    }

    #[tokio::test]
    async fn encodes_when_preset_needs_it() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, true);
        let (r, reports) = go(&e, Preset::MovProres).await;
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video.mov"));
        assert!(reports.iter().any(|r| r.stage == Stage::Encoding && r.progress == 0.5));
    }

    #[tokio::test]
    async fn falls_back_to_software_when_videotoolbox_fails() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG_VT_FAILS, true);
        let (r, _) = go(&e, Preset::Mp4H264).await;
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video.mp4"));
        let log = fs::read_to_string(&e.cfg.log_path).unwrap();
        assert!(log.contains("videotoolbox"), "{log}");
    }

    #[tokio::test]
    async fn hw_off_never_uses_videotoolbox() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG_VT_FAILS, false);
        let (r, _) = go(&e, Preset::Mp4H264).await;
        assert!(r.is_ok());
        assert!(!e.cfg.log_path.exists()); // 재시도 로그가 없어야 한다
    }

    #[tokio::test]
    async fn download_failure_reports_last_line_and_logs_tail() {
        let e = env("echo 'ERROR: [youtube] abc: Video unavailable' >&2; exit 1", FAKE_FFMPEG, true);
        let (r, _) = go(&e, Preset::Original).await;
        assert_eq!(r, Err(PipelineError::Failed("ERROR: [youtube] abc: Video unavailable".into())));
        assert!(fs::read_to_string(&e.cfg.log_path).unwrap().contains("Video unavailable"));
    }

    #[tokio::test]
    async fn unwritable_download_dir_fails_before_download() {
        let e = env("touch \"$0.ran\"", FAKE_FFMPEG, true);
        fs::create_dir_all(&e.cfg.download_dir).unwrap();
        fs::set_permissions(&e.cfg.download_dir, fs::Permissions::from_mode(0o555)).unwrap();
        let (r, _) = go(&e, Preset::Original).await;
        fs::set_permissions(&e.cfg.download_dir, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(r, Err(PipelineError::Failed(ERR_DIR_UNWRITABLE.into())));
        let ran = e.cfg.tools.ytdlp.with_extension("ran");
        assert!(!ran.exists(), "yt-dlp must not run");
    }

    #[tokio::test]
    async fn existing_file_is_not_overwritten() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, true);
        fs::create_dir_all(&e.cfg.download_dir).unwrap();
        fs::write(e.cfg.download_dir.join("Fake Video.mp4"), "old").unwrap();
        let (r, _) = go(&e, Preset::Original).await;
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video (1).mp4"));
        assert_eq!(fs::read_to_string(e.cfg.download_dir.join("Fake Video.mp4")).unwrap(), "old");
    }

    #[tokio::test]
    async fn cancel_returns_cancelled() {
        let e = env("sleep 30", FAKE_FFMPEG, true);
        let token = CancellationToken::new();
        let t2 = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            t2.cancel();
        });
        let r = run(&job(Preset::Original), &e.cfg, &token, |_| {}).await;
        assert_eq!(r, Err(PipelineError::Cancelled));
    }
}
```

`lib.rs`에 `pub mod pipeline;`를 추가한다.

- [ ] **Step 3: 테스트가 실패하는지 확인**

Run: `cargo test -p kiri-core pipeline`
Expected: 컴파일 에러(`cannot find struct PipelineCfg`).

- [ ] **Step 4: 구현**

```rust
pub const ERR_DIR_UNWRITABLE: &str = "error.download_dir_unwritable";
pub const ERR_NO_OUTPUT: &str = "error.no_output";

#[derive(Clone, Debug)]
pub struct PipelineCfg {
    pub tools: Tools,
    pub hw_accel: bool,
    pub download_dir: PathBuf,
    /// 작업 전용 캐시 폴더 (cache/jobs/<id>)
    pub work_dir: PathBuf,
    pub log_path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stage {
    Downloading,
    Encoding,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub stage: Stage,
    pub progress: f32,
    pub speed: Option<String>,
    pub eta: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum PipelineError {
    Cancelled,
    /// 사용자에게 보일 메시지. "error." 으로 시작하면 i18n 키.
    Failed(String),
}

fn append_log(cfg: &PipelineCfg, lines: &[String]) {
    if let Some(dir) = cfg.log_path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&cfg.log_path) {
        for l in lines {
            let _ = writeln!(f, "{l}");
        }
    }
}

fn fail(cfg: &PipelineCfg, e: RunError) -> PipelineError {
    match e {
        RunError::Cancelled => PipelineError::Cancelled,
        RunError::Failed { ref tail } => {
            append_log(cfg, tail);
            PipelineError::Failed(e.summary())
        }
        RunError::Spawn(ref msg) => {
            append_log(cfg, std::slice::from_ref(msg));
            PipelineError::Failed(e.summary())
        }
    }
}

fn io_fail(e: std::io::Error) -> PipelineError {
    PipelineError::Failed(e.to_string())
}

pub async fn run(
    job: &Job,
    cfg: &PipelineCfg,
    cancel: &CancellationToken,
    mut report: impl FnMut(Report),
) -> Result<PathBuf, PipelineError> {
    if files::check_writable(&cfg.download_dir).is_err() {
        return Err(PipelineError::Failed(ERR_DIR_UNWRITABLE.into()));
    }
    fs::create_dir_all(&cfg.work_dir).map_err(io_fail)?;

    report(Report { stage: Stage::Downloading, progress: 0.0, speed: None, eta: None });
    let args = ytdlp::download_args(&job.url, &job.options, &cfg.tools, &cfg.work_dir);
    runner::run(&cfg.tools.ytdlp, &args, cancel, |line| {
        if let Some(p) = ytdlp::parse_progress(line) {
            report(Report { stage: Stage::Downloading, progress: p.fraction, speed: p.speed, eta: p.eta });
        }
    })
    .await
    .map_err(|e| fail(cfg, e))?;

    let downloaded = files::find_media(&cfg.work_dir)
        .map_err(io_fail)?
        .ok_or_else(|| PipelineError::Failed(ERR_NO_OUTPUT.into()))?;
    let media = match encode(job, cfg, cancel, &downloaded, &mut report).await? {
        Some(encoded) => encoded,
        None => downloaded,
    };
    finalize(cfg, &media)
}

async fn encode(
    job: &Job,
    cfg: &PipelineCfg,
    cancel: &CancellationToken,
    input: &Path,
    report: &mut impl FnMut(Report),
) -> Result<Option<PathBuf>, PipelineError> {
    let preset = job.options.preset;
    let Some(ext) = preset.extension() else {
        return Ok(None);
    };
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let out_dir = cfg.work_dir.join("out");
    fs::create_dir_all(&out_dir).map_err(io_fail)?;
    let output = out_dir.join(format!("{stem}.{ext}"));
    let hw = cfg.hw_accel && ffmpeg::hw_capable(preset);

    report(Report { stage: Stage::Encoding, progress: 0.0, speed: None, eta: None });
    match encode_once(job, cfg, cancel, input, &output, hw, report).await {
        Err(e @ RunError::Failed { .. }) if hw => {
            let mut lines = vec!["videotoolbox encode failed, retrying with software encoder:".to_string()];
            if let RunError::Failed { tail } = &e {
                lines.extend(tail.iter().cloned());
            }
            append_log(cfg, &lines);
            encode_once(job, cfg, cancel, input, &output, false, report).await.map_err(|e| fail(cfg, e))?;
        }
        r => r.map_err(|e| fail(cfg, e))?,
    }
    Ok(Some(output))
}

async fn encode_once(
    job: &Job,
    cfg: &PipelineCfg,
    cancel: &CancellationToken,
    input: &Path,
    output: &Path,
    hw: bool,
    report: &mut impl FnMut(Report),
) -> Result<(), RunError> {
    let args = ffmpeg::encode_args(job.options.preset, hw, input, output).expect("preset with extension");
    let duration = job.duration_secs;
    runner::run(&cfg.tools.ffmpeg, &args, cancel, |line| {
        if let Some(p) = ffmpeg::parse_progress(line, duration) {
            report(Report { stage: Stage::Encoding, progress: p, speed: None, eta: None });
        }
    })
    .await
}

/// 결과물과 자막을 저장 위치로 옮긴다. 같은 이름이 있으면 " (n)" 을 붙인다.
fn finalize(cfg: &PipelineCfg, media: &Path) -> Result<PathBuf, PipelineError> {
    let name = media.file_name().and_then(|n| n.to_str()).unwrap_or("video");
    let dest = files::unique_path(&cfg.download_dir, name);
    files::move_file(media, &dest).map_err(io_fail)?;
    for srt in files::subtitle_files(&cfg.work_dir).unwrap_or_default() {
        if let Some(n) = srt.file_name().and_then(|n| n.to_str()) {
            let _ = files::move_file(&srt, &files::unique_path(&cfg.download_dir, n));
        }
    }
    Ok(dest)
}
```

`FAKE_FFMPEG`는 `out_time_us=1000000`을 찍고 job의 길이는 2.0초이므로 `Encoding`의 0.5 보고가 나온다. `unwritable_download_dir_fails_before_download` 테스트의 가짜 yt-dlp는 `$0.ran`(= `<bin>/yt-dlp.ran`)을 만든다. 그래서 `with_extension("ran")`으로 존재 여부를 확인한다.

- [ ] **Step 5: 테스트 통과 확인**

Run: `cargo test -p kiri-core pipeline`
Expected: `8 passed`.

- [ ] **Step 6: Commit**

```bash
git add crates/kiri-core
git commit -m "feat(core): 다운로드·인코딩·이동 파이프라인과 VideoToolbox 폴백 추가"
```

---
### Task 6: 큐 상태와 영속화 (`queue.rs`)

**Files:**
- Create: `crates/kiri-core/src/queue.rs`
- Modify: `crates/kiri-core/src/lib.rs` (`pub mod queue;`)

**Interfaces:**
- Consumes: `model::{Job, JobState, NewJob}`
- Produces: `pub struct QueueState { pub next_id: u64, pub jobs: Vec<Job> }` (Serialize/Deserialize/Default/Clone), with methods:
  - `add(&mut self, new: NewJob, now_ms: u64) -> Job`
  - `get_mut(&mut self, id: u64) -> Option<&mut Job>`
  - `remove(&mut self, id: u64) -> Option<Job>`
  - `next_queued(&self, busy: impl Fn(u64) -> bool) -> Option<u64>`
  - `recover(&mut self)`
  - `load(path: &Path) -> QueueState`
  - `save(&self, path: &Path) -> io::Result<()>`

- [ ] **Step 1: 실패하는 테스트 작성**

`crates/kiri-core/src/queue.rs`:

```rust
//! 큐의 순수 상태. 동시성·프로세스는 engine 이 맡고, 여기는 목록과 파일만 다룬다.
use crate::model::{Job, JobState, NewJob};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{JobOptions, Preset};

    fn new_job(title: &str) -> NewJob {
        NewJob {
            url: "https://youtu.be/x".into(),
            title: title.into(),
            thumbnail: None,
            duration_secs: Some(10.0),
            quality_label: "720p".into(),
            options: JobOptions { format_id: Some("136".into()), preset: Preset::Original, subtitles: vec![], auto_subtitles: false },
        }
    }

    #[test]
    fn ids_start_at_one_and_are_never_reused() {
        let mut q = QueueState::default();
        assert_eq!(q.add(new_job("a"), 10).id, 1);
        assert_eq!(q.add(new_job("b"), 11).id, 2);
        q.remove(2).unwrap();
        assert_eq!(q.add(new_job("c"), 12).id, 3);
        let j = &q.jobs[0];
        assert_eq!((j.state.clone(), j.progress, j.created_at), (JobState::Queued, 0.0, 10));
    }

    #[test]
    fn next_queued_is_oldest_not_busy() {
        let mut q = QueueState::default();
        for t in ["a", "b", "c"] {
            q.add(new_job(t), 0);
        }
        q.get_mut(1).unwrap().state = JobState::Completed;
        assert_eq!(q.next_queued(|_| false), Some(2));
        assert_eq!(q.next_queued(|id| id == 2), Some(3));
        assert_eq!(q.next_queued(|_| true), None);
    }

    #[test]
    fn recover_requeues_active_jobs() {
        let mut q = QueueState::default();
        for t in ["a", "b", "c"] {
            q.add(new_job(t), 0);
        }
        q.get_mut(1).unwrap().state = JobState::Downloading;
        q.get_mut(1).unwrap().progress = 0.4;
        q.get_mut(2).unwrap().state = JobState::Encoding;
        q.get_mut(3).unwrap().state = JobState::Stopped;
        q.recover();
        assert_eq!(q.jobs[0].state, JobState::Queued);
        assert_eq!(q.jobs[0].progress, 0.0);
        assert_eq!(q.jobs[1].state, JobState::Queued);
        assert_eq!(q.jobs[2].state, JobState::Stopped);
    }

    #[test]
    fn save_load_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("data/queue.json");
        let mut q = QueueState::default();
        q.add(new_job("a"), 5);
        q.save(&path).unwrap();
        assert_eq!(QueueState::load(&path), q);
    }

    #[test]
    fn load_missing_or_corrupt_is_empty() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(QueueState::load(&d.path().join("none.json")), QueueState::default());
        let bad = d.path().join("bad.json");
        fs::write(&bad, "{nope").unwrap();
        assert_eq!(QueueState::load(&bad), QueueState::default());
    }

    #[test]
    fn load_repairs_next_id() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("queue.json");
        let mut q = QueueState::default();
        q.add(new_job("a"), 0);
        q.add(new_job("b"), 0);
        q.next_id = 1; // 손상된 파일
        q.save(&path).unwrap();
        let mut loaded = QueueState::load(&path);
        assert_eq!(loaded.add(new_job("c"), 0).id, 3);
    }
}
```

`lib.rs`에 `pub mod queue;`를 추가한다.

- [ ] **Step 2: 테스트가 실패하는지 확인**

Run: `cargo test -p kiri-core queue`
Expected: 컴파일 에러(`cannot find struct QueueState`).

- [ ] **Step 3: 구현**

```rust
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct QueueState {
    pub next_id: u64,
    pub jobs: Vec<Job>,
}

impl QueueState {
    pub fn add(&mut self, new: NewJob, now_ms: u64) -> Job {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        let job = Job {
            id,
            url: new.url,
            title: new.title,
            thumbnail: new.thumbnail,
            duration_secs: new.duration_secs,
            quality_label: new.quality_label,
            options: new.options,
            state: JobState::Queued,
            progress: 0.0,
            speed: None,
            eta: None,
            output: None,
            created_at: now_ms,
        };
        self.jobs.push(job.clone());
        job
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Job> {
        self.jobs.iter_mut().find(|j| j.id == id)
    }

    pub fn remove(&mut self, id: u64) -> Option<Job> {
        let i = self.jobs.iter().position(|j| j.id == id)?;
        Some(self.jobs.remove(i))
    }

    /// busy(id) 가 아닌 대기 작업 중 가장 오래된 것 (id 가 곧 추가 순서).
    pub fn next_queued(&self, busy: impl Fn(u64) -> bool) -> Option<u64> {
        self.jobs
            .iter()
            .filter(|j| j.state == JobState::Queued && !busy(j.id))
            .map(|j| j.id)
            .min()
    }

    /// 앱이 작업 도중 꺼졌던 경우: 돌던 작업을 다시 대기열로.
    pub fn recover(&mut self) {
        for j in &mut self.jobs {
            if j.state.is_active() {
                j.state = JobState::Queued;
                j.progress = 0.0;
                j.speed = None;
                j.eta = None;
            }
        }
    }

    pub fn load(path: &Path) -> QueueState {
        let mut q: QueueState = match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("kiri queue: parse error, starting empty: {e}");
                QueueState::default()
            }),
            Err(e) => {
                if e.kind() != io::ErrorKind::NotFound {
                    eprintln!("kiri queue: read error, starting empty: {e}");
                }
                QueueState::default()
            }
        };
        let min_next = q.jobs.iter().map(|j| j.id + 1).max().unwrap_or(1);
        q.next_id = q.next_id.max(min_next);
        q
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(self).map_err(io::Error::other)?)?;
        fs::rename(tmp, path)
    }
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p kiri-core queue`
Expected: `6 passed`.

- [ ] **Step 5: Commit**

```bash
git add crates/kiri-core
git commit -m "feat(core): 큐 상태와 queue.json 영속화 추가"
```

---

### Task 7: 큐 엔진 (`engine.rs`)

**Files:**
- Create: `crates/kiri-core/src/engine.rs`
- Modify: `crates/kiri-core/src/lib.rs` (`pub mod engine;`), `crates/kiri-core/src/testutil.rs` (probe도 처리하는 가짜 yt-dlp)

**Interfaces:**
- Consumes: `queue::QueueState`, `pipeline::{run, PipelineCfg, PipelineError, Report, Stage}`, `ytdlp::{is_youtube_url, probe_args, parse_probe, resolve_quality, pick_subtitles, VideoInfo}`, `runner::output`
- Produces:
  - `pub struct EngineConfig { max_concurrent: usize, hw_accel: bool, download_dir: PathBuf, default_quality: String, default_preset: Preset, default_subtitles: Vec<String> }`
  - `pub struct EnginePaths { queue_file: PathBuf, cache_dir: PathBuf, log_dir: PathBuf }`
  - `pub enum EngineError { NotFound(u64), NotStoppable(u64), NotRestartable(u64), YtdlpMissing, InvalidUrl, Probe(String), BadQuality(String), BadPreset(String) }`, `EngineError::code() -> &'static str`
  - `#[derive(Clone)] pub struct Engine`, with methods:
    - `new(paths, tools, config, rt: tokio::runtime::Handle, listener: impl Fn(&[Job]) + Send + Sync + 'static) -> Engine`
    - `start(&self)`
    - `list(&self) -> Vec<Job>`, `running(&self) -> Vec<Job>`, `has_active(&self) -> bool`, `is_idle(&self) -> bool`
    - `config(&self) -> EngineConfig`, `set_config(&self, c: EngineConfig)`, `tools(&self) -> &Tools`
    - `async probe(&self, url: &str) -> Result<VideoInfo, EngineError>`
    - `add(&self, new: NewJob) -> Job`
    - `async add_url(&self, url: &str, quality: Option<String>, preset: Option<String>, subs: Option<Vec<String>>) -> Result<Job, EngineError>`
    - `stop(&self, id: u64) -> Result<(), EngineError>`, `remove(&self, id: u64) -> Result<(), EngineError>`, `restart(&self, id: u64) -> Result<(), EngineError>`
  - `pub fn now_ms() -> u64`

**설계 메모:**
- **락 순서:** `running → state` 순서로만 중첩한다. `state` 락을 잡은 채로 `running` 락을 잡지 않는다.
- **작업 종료 처리:** 끝난 작업은 상태가 아직 `is_active()`일 때만 결과를 반영한다. 그래서 stop이나 restart가 먼저 바꾼 상태를 덮어쓰지 않는다.
- **중복 실행 방지:** `pump`는 `running`에 남아 있는 id(아직 종료 중인 이전 실행)를 건너뛴다. 따라서 같은 작업이 두 번 동시에 돌지 않는다.

- [ ] **Step 1: 테스트 유틸 확장**

`testutil.rs`에 추가한다.

```rust
pub const PROBE_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/probe.json");

/// `-J` 이면 fixture 를 출력하고, 아니면 download_body 를 실행한다.
pub fn ytdlp_with_probe(download_body: &str) -> String {
    format!("case \" $* \" in *\" -J \"*) cat '{PROBE_FIXTURE}'; exit 0;; esac\n{download_body}")
}
```

- [ ] **Step 2: 실패하는 테스트 작성**

`crates/kiri-core/src/engine.rs`:

```rust
//! 앱 하나에 하나. 큐 상태 + 동시 실행 + 프로세스 취소를 묶는다. UI 와 CLI 가 같은 Engine 을 쓴다.
use crate::{
    model::{Job, JobOptions, JobState, NewJob, Preset, Tools},
    pipeline::{self, PipelineCfg, PipelineError, Report, Stage},
    queue::QueueState,
    runner,
    ytdlp::{self, VideoInfo},
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{self, *};
    use std::{fs, path::Path, time::Duration};

    const SLOW_DOWNLOAD: &str = "sleep 1\n";

    struct Env {
        d: tempfile::TempDir,
        engine: Engine,
    }

    fn engine_at(dir: &Path, ytdlp_body: &str, max: usize) -> Engine {
        let bin = dir.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let tools = testutil::tools(&bin, &ytdlp_with_probe(ytdlp_body), FAKE_FFMPEG);
        let paths = EnginePaths {
            queue_file: dir.join("data/queue.json"),
            cache_dir: dir.join("cache"),
            log_dir: dir.join("logs"),
        };
        let cfg = EngineConfig {
            max_concurrent: max,
            hw_accel: true,
            download_dir: dir.join("Movies/kiri"),
            default_quality: "best".into(),
            default_preset: Preset::Original,
            default_subtitles: vec!["ko".into()],
        };
        Engine::new(paths, tools, cfg, Handle::current(), |_| {})
    }

    fn env(ytdlp_body: &str, max: usize) -> Env {
        let d = tempfile::tempdir().unwrap();
        let engine = engine_at(d.path(), ytdlp_body, max);
        engine.start();
        Env { d, engine }
    }

    fn new_job(title: &str) -> NewJob {
        NewJob {
            url: "https://youtu.be/abc123".into(),
            title: title.into(),
            thumbnail: None,
            duration_secs: Some(2.0),
            quality_label: "720p".into(),
            options: JobOptions { format_id: Some("136".into()), preset: Preset::Original, subtitles: vec![], auto_subtitles: false },
        }
    }

    async fn wait_for(e: &Engine, pred: impl Fn(&[Job]) -> bool) {
        for _ in 0..100 {
            if pred(&e.list()) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("timeout waiting; jobs = {:#?}", e.list());
    }

    fn state_of(e: &Engine, id: u64) -> JobState {
        e.list().into_iter().find(|j| j.id == id).unwrap().state
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn probe_rejects_non_youtube_without_running_ytdlp() {
        let e = env("touch \"$0.ran\"", 2);
        assert!(matches!(e.engine.probe("hello").await, Err(EngineError::InvalidUrl)));
        assert!(matches!(e.engine.probe("https://vimeo.com/1").await, Err(EngineError::InvalidUrl)));
        assert!(!e.engine.tools().ytdlp.with_extension("ran").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn probe_requires_ytdlp() {
        let d = tempfile::tempdir().unwrap();
        let engine = engine_at(d.path(), "", 1);
        fs::remove_file(&engine.tools().ytdlp).unwrap();
        assert!(matches!(engine.probe("https://youtu.be/x").await, Err(EngineError::YtdlpMissing)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn probe_parses_fixture() {
        let e = env(FAKE_YTDLP_DOWNLOAD, 2);
        let info = e.engine.probe("https://youtu.be/abc123").await.unwrap();
        assert_eq!(info.title, "Rust in 100 Seconds");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn added_job_runs_to_completion() {
        let e = env(FAKE_YTDLP_DOWNLOAD, 2);
        let job = e.engine.add(new_job("a"));
        wait_for(&e.engine, |j| j[0].state == JobState::Completed).await;
        let done = &e.engine.list()[0];
        assert_eq!(done.id, job.id);
        assert_eq!(done.output.as_deref(), Some(e.d.path().join("Movies/kiri/Fake Video.mp4").as_path()));
        assert!(!e.d.path().join("cache/jobs/1").exists(), "cache cleaned after success");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn respects_max_concurrent() {
        let e = env(&format!("{SLOW_DOWNLOAD}{FAKE_YTDLP_DOWNLOAD}"), 1);
        e.engine.add(new_job("a"));
        e.engine.add(new_job("b"));
        assert_eq!(state_of(&e.engine, 1), JobState::Downloading);
        assert_eq!(state_of(&e.engine, 2), JobState::Queued);
        wait_for(&e.engine, |j| j.iter().all(|j| j.state == JobState::Completed)).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stop_running_job() {
        let e = env("sleep 30", 2);
        e.engine.add(new_job("a"));
        e.engine.stop(1).unwrap();
        assert_eq!(state_of(&e.engine, 1), JobState::Stopped);
        wait_for(&e.engine, |_| !e.engine.has_active()).await;
        assert_eq!(state_of(&e.engine, 1), JobState::Stopped);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stop_queued_job_and_reject_finished() {
        let e = env("sleep 30", 1);
        e.engine.add(new_job("a"));
        e.engine.add(new_job("b"));
        e.engine.stop(2).unwrap();
        assert_eq!(state_of(&e.engine, 2), JobState::Stopped);
        assert!(matches!(e.engine.stop(2), Err(EngineError::NotStoppable(2))));
        assert!(matches!(e.engine.stop(99), Err(EngineError::NotFound(99))));
        e.engine.stop(1).unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stop_then_immediate_restart_completes_once() {
        let e = env(&format!("{SLOW_DOWNLOAD}{FAKE_YTDLP_DOWNLOAD}"), 2);
        e.engine.add(new_job("a"));
        e.engine.stop(1).unwrap();
        e.engine.restart(1).unwrap(); // 이전 프로세스가 아직 종료 중일 수 있다
        wait_for(&e.engine, |j| j[0].state == JobState::Completed).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(state_of(&e.engine, 1), JobState::Completed);
        let outputs = fs::read_dir(e.d.path().join("Movies/kiri")).unwrap().filter(|f| {
            f.as_ref().unwrap().path().extension().is_some_and(|x| x == "mp4")
        }).count();
        assert_eq!(outputs, 1, "job ran twice");
        assert!(matches!(e.engine.restart(1), Err(EngineError::NotRestartable(1))));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn remove_running_job_cleans_up() {
        let e = env("sleep 30", 2);
        e.engine.add(new_job("a"));
        tokio::time::sleep(Duration::from_millis(200)).await;
        e.engine.remove(1).unwrap();
        assert!(e.engine.list().is_empty());
        wait_for(&e.engine, |_| !e.d.path().join("cache/jobs/1").exists()).await;
        assert!(e.engine.list().is_empty(), "removed job resurrected");
        assert!(matches!(e.engine.remove(1), Err(EngineError::NotFound(1))));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn failed_job_keeps_message_and_can_restart() {
        let e = env("echo 'ERROR: boom' >&2; exit 1", 2);
        e.engine.add(new_job("a"));
        wait_for(&e.engine, |j| matches!(j[0].state, JobState::Failed(_))).await;
        assert_eq!(state_of(&e.engine, 1), JobState::Failed("ERROR: boom".into()));
        e.engine.restart(1).unwrap();
        wait_for(&e.engine, |j| matches!(j[0].state, JobState::Failed(_))).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn add_url_resolves_quality_preset_and_subs() {
        let e = env("sleep 30", 1);
        let j = e.engine.add_url("https://youtu.be/abc123", Some("720p".into()), None, Some(vec!["ja".into()])).await.unwrap();
        assert_eq!(j.options.format_id.as_deref(), Some("136"));
        assert_eq!(j.quality_label, "720p");
        assert_eq!((j.options.subtitles.clone(), j.options.auto_subtitles), (vec!["ja".to_string()], true));
        assert_eq!(j.title, "Rust in 100 Seconds");
        let a = e.engine.add_url("https://youtu.be/abc123", None, Some("mp3".into()), None).await.unwrap();
        assert_eq!((a.options.format_id, a.quality_label.as_str(), a.options.preset), (None, "audio", Preset::Mp3));
        assert_eq!(a.options.subtitles, vec!["ko"]); // 기본 자막
        assert!(matches!(e.engine.add_url("https://youtu.be/x", None, Some("avi".into()), None).await, Err(EngineError::BadPreset(_))));
        assert!(matches!(e.engine.add_url("https://youtu.be/x", Some("hd".into()), None, None).await, Err(EngineError::BadQuality(_))));
        for j in e.engine.list() {
            let _ = e.engine.stop(j.id);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn new_engine_recovers_persisted_jobs() {
        let d = tempfile::tempdir().unwrap();
        let mut q = QueueState::default();
        q.add(new_job("a"), 0);
        q.get_mut(1).unwrap().state = JobState::Downloading;
        q.save(&d.path().join("data/queue.json")).unwrap();
        let engine = engine_at(d.path(), "sleep 30", 1); // start() 를 부르지 않는다
        assert_eq!(state_of(&engine, 1), JobState::Queued);
        assert_eq!(engine.add(new_job("b")).id, 2);
    }

    #[test]
    fn error_codes() {
        assert_eq!(EngineError::InvalidUrl.code(), "invalid_url");
        assert_eq!(EngineError::NotFound(3).code(), "not_found");
        assert_eq!(EngineError::NotFound(3).to_string(), "job 3 not found");
        assert_eq!(EngineError::YtdlpMissing.code(), "ytdlp_missing");
    }
}
```

`remove_running_job_cleans_up`에서 캐시 폴더 `cache/jobs/1`은 파이프라인이 yt-dlp를 실행하기 전에 만든다. 그래서 가짜 yt-dlp는 잠들기만 하면 된다.

`lib.rs`에 `pub mod engine;`을 추가한다.

- [ ] **Step 3: 테스트가 실패하는지 확인**

Run: `cargo test -p kiri-core engine`
Expected: 컴파일 에러(`cannot find struct Engine`).

- [ ] **Step 4: 구현**

`engine.rs`의 `#[cfg(test)]` 위에 추가한다.

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct EngineConfig {
    pub max_concurrent: usize,
    pub hw_accel: bool,
    pub download_dir: PathBuf,
    /// CLI add 의 기본값 ("best" | "audio" | "<N>p")
    pub default_quality: String,
    pub default_preset: Preset,
    pub default_subtitles: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct EnginePaths {
    pub queue_file: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("job {0} not found")]
    NotFound(u64),
    #[error("job {0} is not running or queued")]
    NotStoppable(u64),
    #[error("job {0} cannot be restarted")]
    NotRestartable(u64),
    #[error("yt-dlp is not installed yet")]
    YtdlpMissing,
    #[error("not a YouTube URL")]
    InvalidUrl,
    #[error("{0}")]
    Probe(String),
    #[error("unknown quality: {0}")]
    BadQuality(String),
    #[error("unknown format: {0}")]
    BadPreset(String),
}

impl EngineError {
    /// 프론트엔드 i18n 키(error.<code>)와 CLI 응답에 쓰는 고정 코드.
    pub fn code(&self) -> &'static str {
        match self {
            EngineError::NotFound(_) => "not_found",
            EngineError::NotStoppable(_) => "not_stoppable",
            EngineError::NotRestartable(_) => "not_restartable",
            EngineError::YtdlpMissing => "ytdlp_missing",
            EngineError::InvalidUrl => "invalid_url",
            EngineError::Probe(_) => "probe_failed",
            EngineError::BadQuality(_) => "bad_quality",
            EngineError::BadPreset(_) => "bad_preset",
        }
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

type Listener = Box<dyn Fn(&[Job]) + Send + Sync>;

struct Inner {
    state: Mutex<QueueState>,
    running: Mutex<HashMap<u64, CancellationToken>>,
    config: Mutex<EngineConfig>,
    paths: EnginePaths,
    tools: Tools,
    rt: Handle,
    listener: Listener,
}

#[derive(Clone)]
pub struct Engine(Arc<Inner>);

impl Engine {
    pub fn new(
        paths: EnginePaths,
        tools: Tools,
        config: EngineConfig,
        rt: Handle,
        listener: impl Fn(&[Job]) + Send + Sync + 'static,
    ) -> Engine {
        let mut state = QueueState::load(&paths.queue_file);
        state.recover();
        let engine = Engine(Arc::new(Inner {
            state: Mutex::new(state),
            running: Mutex::new(HashMap::new()),
            config: Mutex::new(config),
            paths,
            tools,
            rt,
            listener: Box::new(listener),
        }));
        engine.save();
        engine
    }

    /// 대기 작업 실행을 시작한다. setup 이 끝난 뒤(리스너가 준비된 뒤) 한 번 부른다.
    pub fn start(&self) {
        self.pump();
    }

    pub fn tools(&self) -> &Tools {
        &self.0.tools
    }

    pub fn list(&self) -> Vec<Job> {
        self.0.state.lock().unwrap().jobs.clone()
    }

    pub fn running(&self) -> Vec<Job> {
        self.list().into_iter().filter(|j| j.state.is_active()).collect()
    }

    pub fn has_active(&self) -> bool {
        self.0.state.lock().unwrap().jobs.iter().any(|j| j.state.is_active())
    }

    /// 대기·실행 중인 작업이 하나도 없다.
    pub fn is_idle(&self) -> bool {
        !self.0.state.lock().unwrap().jobs.iter().any(|j| j.state.is_pending())
    }

    pub fn config(&self) -> EngineConfig {
        self.0.config.lock().unwrap().clone()
    }

    pub fn set_config(&self, c: EngineConfig) {
        *self.0.config.lock().unwrap() = c;
        self.pump(); // 동시 실행 수가 늘었을 수 있다
    }

    pub async fn probe(&self, url: &str) -> Result<VideoInfo, EngineError> {
        if !ytdlp::is_youtube_url(url) {
            return Err(EngineError::InvalidUrl);
        }
        let tools = &self.0.tools;
        if !tools.ytdlp.exists() {
            return Err(EngineError::YtdlpMissing);
        }
        let out = runner::output(&tools.ytdlp, &ytdlp::probe_args(url.trim(), tools))
            .await
            .map_err(|e| EngineError::Probe(e.summary()))?;
        ytdlp::parse_probe(&out).map_err(EngineError::Probe)
    }

    pub fn add(&self, new: NewJob) -> Job {
        let job = self.0.state.lock().unwrap().add(new, now_ms());
        self.save();
        self.notify();
        self.pump();
        self.list().into_iter().find(|j| j.id == job.id).unwrap_or(job)
    }

    /// CLI 경로: probe → 화질/프리셋/자막 해석 → add. 생략한 옵션은 설정 기본값.
    pub async fn add_url(
        &self,
        url: &str,
        quality: Option<String>,
        preset: Option<String>,
        subs: Option<Vec<String>>,
    ) -> Result<Job, EngineError> {
        let cfg = self.config();
        let preset = match preset {
            Some(p) => p.parse::<Preset>().map_err(|_| EngineError::BadPreset(p))?,
            None => cfg.default_preset,
        };
        let want = quality.unwrap_or(cfg.default_quality);
        if want != "best" && want != "audio" && want.strip_suffix('p').and_then(|n| n.parse::<u32>().ok()).is_none() {
            return Err(EngineError::BadQuality(want));
        }
        let info = self.probe(url).await?;
        let quality = if preset.is_audio_only() {
            None
        } else {
            ytdlp::resolve_quality(&info.qualities, &want).map_err(EngineError::BadQuality)?
        };
        let langs = subs.unwrap_or(cfg.default_subtitles);
        let (subtitles, auto_subtitles) = ytdlp::pick_subtitles(&info, &langs);
        Ok(self.add(NewJob {
            url: url.trim().to_string(),
            title: info.title,
            thumbnail: info.thumbnail,
            duration_secs: info.duration_secs,
            quality_label: quality.as_ref().map_or("audio".to_string(), |q| q.label.clone()),
            options: JobOptions {
                format_id: quality.map(|q| q.format_id),
                preset,
                subtitles,
                auto_subtitles,
            },
        }))
    }

    pub fn stop(&self, id: u64) -> Result<(), EngineError> {
        {
            let mut st = self.0.state.lock().unwrap();
            let job = st.get_mut(id).ok_or(EngineError::NotFound(id))?;
            if !job.state.is_pending() {
                return Err(EngineError::NotStoppable(id));
            }
            job.state = JobState::Stopped;
            job.speed = None;
            job.eta = None;
        }
        if let Some(t) = self.0.running.lock().unwrap().get(&id) {
            t.cancel();
        }
        self.save();
        self.notify();
        Ok(())
    }

    pub fn remove(&self, id: u64) -> Result<(), EngineError> {
        let removed = self.0.state.lock().unwrap().remove(id);
        removed.ok_or(EngineError::NotFound(id))?;
        let token = self.0.running.lock().unwrap().get(&id).cloned();
        match token {
            Some(t) => t.cancel(), // 작업 태스크가 끝나면서 캐시 폴더를 지운다
            None => {
                let _ = std::fs::remove_dir_all(self.job_dir(id));
            }
        }
        self.save();
        self.notify();
        Ok(())
    }

    pub fn restart(&self, id: u64) -> Result<(), EngineError> {
        {
            let mut st = self.0.state.lock().unwrap();
            let job = st.get_mut(id).ok_or(EngineError::NotFound(id))?;
            if !matches!(job.state, JobState::Stopped | JobState::Failed(_)) {
                return Err(EngineError::NotRestartable(id));
            }
            job.state = JobState::Queued;
            job.progress = 0.0;
        }
        self.save();
        self.notify();
        self.pump();
        Ok(())
    }

    fn job_dir(&self, id: u64) -> PathBuf {
        self.0.paths.cache_dir.join("jobs").join(id.to_string())
    }

    fn save(&self) {
        let st = self.0.state.lock().unwrap();
        if let Err(e) = st.save(&self.0.paths.queue_file) {
            eprintln!("kiri queue: save failed: {e}");
        }
    }

    fn notify(&self) {
        let jobs = self.list();
        (self.0.listener)(&jobs);
    }

    /// 빈 슬롯만큼 대기 작업을 시작한다.
    fn pump(&self) {
        loop {
            let max = self.config().max_concurrent.max(1);
            let next = {
                let mut running = self.0.running.lock().unwrap();
                if running.len() >= max {
                    return;
                }
                let mut st = self.0.state.lock().unwrap();
                let Some(id) = st.next_queued(|id| running.contains_key(&id)) else {
                    return;
                };
                let job = st.get_mut(id).expect("queued job exists");
                job.state = JobState::Downloading;
                job.progress = 0.0;
                let job = job.clone();
                let token = CancellationToken::new();
                running.insert(id, token.clone());
                (job, token)
            };
            self.save();
            self.notify();
            self.spawn_job(next.0, next.1);
        }
    }

    fn spawn_job(&self, job: Job, token: CancellationToken) {
        let me = self.clone();
        self.0.rt.spawn(async move {
            let cfg = me.config();
            let pcfg = PipelineCfg {
                tools: me.0.tools.clone(),
                hw_accel: cfg.hw_accel,
                download_dir: cfg.download_dir,
                work_dir: me.job_dir(job.id),
                log_path: me.0.paths.log_dir.join(format!("{}.log", job.id)),
            };
            let id = job.id;
            let reporter = me.clone();
            let result = pipeline::run(&job, &pcfg, &token, move |r| reporter.on_report(id, r)).await;
            me.finish(id, result);
        });
    }

    fn on_report(&self, id: u64, r: Report) {
        let stage_changed = {
            let mut st = self.0.state.lock().unwrap();
            let Some(job) = st.get_mut(id) else { return };
            if !job.state.is_active() {
                return; // 이미 중지·삭제됨
            }
            let next = match r.stage {
                Stage::Downloading => JobState::Downloading,
                Stage::Encoding => JobState::Encoding,
            };
            let changed = job.state != next;
            job.state = next;
            job.progress = r.progress;
            job.speed = r.speed;
            job.eta = r.eta;
            changed
        };
        if stage_changed {
            self.save();
        }
        self.notify();
    }

    fn finish(&self, id: u64, result: Result<PathBuf, PipelineError>) {
        let still_listed = {
            let mut st = self.0.state.lock().unwrap();
            match st.get_mut(id) {
                None => false,
                Some(job) => {
                    if job.state.is_active() {
                        match &result {
                            Ok(path) => {
                                job.state = JobState::Completed;
                                job.progress = 1.0;
                                job.output = Some(path.clone());
                            }
                            Err(PipelineError::Cancelled) => job.state = JobState::Stopped,
                            Err(PipelineError::Failed(msg)) => job.state = JobState::Failed(msg.clone()),
                        }
                    }
                    job.speed = None;
                    job.eta = None;
                    true
                }
            }
        };
        if !still_listed || result.is_ok() {
            let _ = std::fs::remove_dir_all(self.job_dir(id));
        }
        self.0.running.lock().unwrap().remove(&id);
        self.save();
        self.notify();
        self.pump();
    }
}
```

**`add`가 `list()`에서 다시 찾는 이유:** `pump`가 같은 호출 안에서 상태를 `Downloading`으로 바꿨을 수 있다. 호출자(CLI 응답)에게 최신 상태를 돌려주기 위해서다.

- [ ] **Step 5: 테스트 통과 확인**

Run: `cargo test -p kiri-core engine`
Expected: `13 passed`. 실패하면 락 순서(`running → state`)와 `finish`의 `is_active()` 확인부터 본다.

- [ ] **Step 6: 전체 테스트**

Run: `cargo test -p kiri-core`
Expected: 모두 통과.

- [ ] **Step 7: Commit**

```bash
git add crates/kiri-core
git commit -m "feat(core): 동시 실행·중지·삭제·재시작을 처리하는 큐 엔진 추가"
```

---

### Task 8: 소켓 프로토콜·서버·클라이언트 (`ipc.rs`)

**Files:**
- Create: `crates/kiri-core/src/ipc.rs`
- Modify: `crates/kiri-core/src/lib.rs` (`pub mod ipc;`)

**Interfaces:**
- Consumes: `engine::{Engine, EngineError}`, `model::Job`
- Produces:
  - `pub const PROTOCOL_VERSION: u32 = 1`
  - `pub struct Envelope<T> { v: u32, body: T }`
  - `#[serde(tag = "type", rename_all = "snake_case")] pub enum Request { Status, List, Add { url, quality: Option<String>, preset: Option<String>, subs: Option<Vec<String>> }, Remove { id: u64 }, Stop { id: u64 } }`
  - `#[serde(tag = "type", rename_all = "snake_case")] pub enum Response { Jobs { jobs: Vec<Job> }, Added { job: Job }, Ok, Error { code: String, message: String } }`
  - `pub fn socket_path() -> PathBuf` (`KIRI_SOCKET`이 있으면 그 값)
  - `pub async fn bind(path: &Path) -> io::Result<UnixListener>`: 살아 있는 서버가 있으면 `AddrInUse`
  - `pub async fn serve(listener: UnixListener, engine: Engine)`
  - `pub async fn dispatch(engine: &Engine, req: Request) -> Response`
  - `pub enum ClientError { NotRunning, Io(String) }`, `pub async fn request(path: &Path, req: &Request) -> Result<Response, ClientError>`

- [ ] **Step 1: 실패하는 테스트 작성**

`crates/kiri-core/src/ipc.rs`:

```rust
//! CLI ↔ 앱. 연결당 요청 한 줄, 응답 한 줄 (JSON + \n).
use crate::{
    engine::{Engine, EngineError},
    model::Job,
};
use serde::{Deserialize, Serialize};
use std::{
    io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        engine::{EngineConfig, EnginePaths},
        model::{Preset, Tools},
    };
    use serde_json::json;

    fn idle_engine(dir: &Path) -> Engine {
        let tools = Tools { ytdlp: dir.join("none/yt-dlp"), deno: dir.join("none/deno"), ffmpeg: dir.join("none/ffmpeg") };
        let paths = EnginePaths { queue_file: dir.join("q.json"), cache_dir: dir.join("c"), log_dir: dir.join("l") };
        let cfg = EngineConfig {
            max_concurrent: 1,
            hw_accel: false,
            download_dir: dir.join("out"),
            default_quality: "best".into(),
            default_preset: Preset::Original,
            default_subtitles: vec![],
        };
        Engine::new(paths, tools, cfg, tokio::runtime::Handle::current(), |_| {})
    }

    async fn server(dir: &Path) -> PathBuf {
        let path = dir.join("kiri.sock");
        let listener = bind(&path).await.unwrap();
        tokio::spawn(serve(listener, idle_engine(dir)));
        path
    }

    #[test]
    fn request_json_shape() {
        let env = Envelope { v: 1, body: Request::Stop { id: 3 } };
        assert_eq!(serde_json::to_value(&env).unwrap(), json!({"v": 1, "body": {"type": "stop", "id": 3}}));
        let add: Request = serde_json::from_value(json!({"type": "add", "url": "u"})).unwrap();
        assert_eq!(add, Request::Add { url: "u".into(), quality: None, preset: None, subs: None });
        assert_eq!(serde_json::to_value(Response::Ok).unwrap(), json!({"type": "ok"}));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn list_and_errors_over_socket() {
        let d = tempfile::tempdir().unwrap();
        let path = server(d.path()).await;
        assert_eq!(request(&path, &Request::List).await.unwrap(), Response::Jobs { jobs: vec![] });
        assert_eq!(request(&path, &Request::Status).await.unwrap(), Response::Jobs { jobs: vec![] });
        let Response::Error { code, .. } = request(&path, &Request::Stop { id: 7 }).await.unwrap() else { panic!() };
        assert_eq!(code, "not_found");
        let r = request(&path, &Request::Add { url: "hello".into(), quality: None, preset: None, subs: None }).await.unwrap();
        assert!(matches!(r, Response::Error { ref code, .. } if code == "invalid_url"), "{r:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn version_mismatch_is_reported() {
        let d = tempfile::tempdir().unwrap();
        let path = server(d.path()).await;
        let mut s = UnixStream::connect(&path).await.unwrap();
        s.write_all(b"{\"v\":99,\"body\":{\"type\":\"list\"}}\n").await.unwrap();
        let mut line = String::new();
        BufReader::new(s).read_line(&mut line).await.unwrap();
        let env: Envelope<Response> = serde_json::from_str(&line).unwrap();
        assert!(matches!(env.body, Response::Error { ref code, .. } if code == "version_mismatch"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bind_sets_0600_and_refuses_live_server() {
        let d = tempfile::tempdir().unwrap();
        let path = server(d.path()).await;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(bind(&path).await.unwrap_err().kind(), io::ErrorKind::AddrInUse);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bind_replaces_stale_socket_file() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("kiri.sock");
        drop(std::os::unix::net::UnixListener::bind(&path).unwrap()); // 파일만 남는다
        assert!(bind(&path).await.is_ok());
    }

    #[tokio::test]
    async fn client_reports_not_running() {
        let d = tempfile::tempdir().unwrap();
        let r = request(&d.path().join("none.sock"), &Request::List).await;
        assert_eq!(r, Err(ClientError::NotRunning));
    }

    #[test]
    fn socket_path_env_override() {
        // 다른 테스트와 env 를 공유하므로 이 테스트 안에서만 설정·해제한다.
        unsafe { std::env::set_var("KIRI_SOCKET", "/tmp/x.sock") };
        assert_eq!(socket_path(), PathBuf::from("/tmp/x.sock"));
        unsafe { std::env::remove_var("KIRI_SOCKET") };
        assert!(socket_path().ends_with("Library/Application Support/org.bobpark.kiri/kiri.sock"));
    }
}
```

`lib.rs`에 `pub mod ipc;`를 추가한다.

- [ ] **Step 2: 테스트가 실패하는지 확인**

Run: `cargo test -p kiri-core ipc`
Expected: 컴파일 에러(`cannot find struct Envelope`).

- [ ] **Step 3: 구현**

```rust
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Envelope<T> {
    pub v: u32,
    pub body: T,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Status,
    List,
    Add {
        url: String,
        #[serde(default)]
        quality: Option<String>,
        #[serde(default)]
        preset: Option<String>,
        #[serde(default)]
        subs: Option<Vec<String>>,
    },
    Remove { id: u64 },
    Stop { id: u64 },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Jobs { jobs: Vec<Job> },
    Added { job: Job },
    Ok,
    Error { code: String, message: String },
}

impl From<EngineError> for Response {
    fn from(e: EngineError) -> Self {
        Response::Error { code: e.code().into(), message: e.to_string() }
    }
}

pub fn socket_path() -> PathBuf {
    if let Some(p) = std::env::var_os("KIRI_SOCKET") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join("Library/Application Support/org.bobpark.kiri/kiri.sock")
}

/// 다른 인스턴스가 응답하면 AddrInUse. 응답 없는 파일은 지우고 다시 바인드한다.
pub async fn bind(path: &Path) -> io::Result<UnixListener> {
    if UnixStream::connect(path).await.is_ok() {
        return Err(io::Error::new(io::ErrorKind::AddrInUse, "another kiri instance owns the socket"));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

pub async fn serve(listener: UnixListener, engine: Engine) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let engine = engine.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_conn(stream, engine).await {
                        eprintln!("kiri ipc: {e}");
                    }
                });
            }
            Err(e) => eprintln!("kiri ipc: accept failed: {e}"),
        }
    }
}

async fn handle_conn(stream: UnixStream, engine: Engine) -> io::Result<()> {
    let (r, mut w) = stream.into_split();
    let mut line = String::new();
    BufReader::new(r).read_line(&mut line).await?;
    let bad = |m: String| Response::Error { code: "bad_request".into(), message: m };
    let resp = match serde_json::from_str::<Envelope<serde_json::Value>>(&line) {
        Err(e) => bad(e.to_string()),
        Ok(env) if env.v != PROTOCOL_VERSION => Response::Error {
            code: "version_mismatch".into(),
            message: format!("app speaks protocol v{PROTOCOL_VERSION}, client sent v{}", env.v),
        },
        Ok(env) => match serde_json::from_value::<Request>(env.body) {
            Ok(req) => dispatch(&engine, req).await,
            Err(e) => bad(e.to_string()),
        },
    };
    let mut out = serde_json::to_string(&Envelope { v: PROTOCOL_VERSION, body: resp }).map_err(io::Error::other)?;
    out.push('\n');
    w.write_all(out.as_bytes()).await
}

pub async fn dispatch(engine: &Engine, req: Request) -> Response {
    let done = |r: Result<(), EngineError>| r.map_or_else(Response::from, |_| Response::Ok);
    match req {
        Request::Status => Response::Jobs { jobs: engine.running() },
        Request::List => Response::Jobs { jobs: engine.list() },
        Request::Add { url, quality, preset, subs } => match engine.add_url(&url, quality, preset, subs).await {
            Ok(job) => Response::Added { job },
            Err(e) => e.into(),
        },
        Request::Remove { id } => done(engine.remove(id)),
        Request::Stop { id } => done(engine.stop(id)),
    }
}

#[derive(Debug, PartialEq)]
pub enum ClientError {
    NotRunning,
    Io(String),
}

pub async fn request(path: &Path, req: &Request) -> Result<Response, ClientError> {
    let stream = UnixStream::connect(path).await.map_err(|e| match e.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => ClientError::NotRunning,
        _ => ClientError::Io(e.to_string()),
    })?;
    let io_err = |e: io::Error| ClientError::Io(e.to_string());
    let (r, mut w) = stream.into_split();
    let mut out = serde_json::to_string(&Envelope { v: PROTOCOL_VERSION, body: req })
        .map_err(|e| ClientError::Io(e.to_string()))?;
    out.push('\n');
    w.write_all(out.as_bytes()).await.map_err(io_err)?;
    let mut line = String::new();
    BufReader::new(r).read_line(&mut line).await.map_err(io_err)?;
    let env: Envelope<Response> = serde_json::from_str(&line).map_err(|e| ClientError::Io(e.to_string()))?;
    Ok(env.body)
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p kiri-core ipc`
Expected: `7 passed`.

- [ ] **Step 5: Commit**

```bash
git add crates/kiri-core
git commit -m "feat(core): CLI용 Unix 소켓 프로토콜과 서버·클라이언트 추가"
```

---

### Task 9: yt-dlp·Deno 설치 (`tools.rs`)

**Files:**
- Create: `crates/kiri-core/src/tools.rs`
- Modify: `crates/kiri-core/src/lib.rs` (`pub mod tools;`)

**Interfaces:**
- Consumes: `runner::output`
- Produces:
  - `pub fn sha256_hex(bytes: &[u8]) -> String`
  - `pub fn find_hash(sums: &str, file: &str) -> Option<String>`
  - `pub fn install_executable(dest: &Path, bytes: &[u8]) -> io::Result<()>`
  - `pub async fn ensure_ytdlp(client: &reqwest::Client, dest: &Path) -> Result<bool, String>`: 바꿨으면 true
  - `pub async fn ensure_deno(client: &reqwest::Client, dest: &Path) -> Result<bool, String>`: 새로 설치했으면 true
  - `pub async fn version(bin: &Path) -> Option<String>`
  - `pub fn http_client() -> reqwest::Client`

- [ ] **Step 1: 실패하는 테스트 작성**

`crates/kiri-core/src/tools.rs`:

```rust
//! yt-dlp·Deno 를 GitHub Release 에서 받아 SHA256 으로 검증하고 설치한다.
//! reqwest 로 받은 파일엔 quarantine 속성이 없어 Gatekeeper 에 막히지 않는다.
use crate::runner;
use sha2::{Digest, Sha256};
use std::{fs, io, os::unix::fs::PermissionsExt, path::Path};

#[cfg(test)]
mod tests {
    use super::*;

    const H1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const H2: &str = "2222222222222222222222222222222222222222222222222222222222222222";

    #[test]
    fn sha256_known_vector() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn finds_hash_in_sums_file() {
        let sums = format!("{H1}  yt-dlp\n{H2}  yt-dlp_macos\n{H1}  yt-dlp_macos.zip\n");
        assert_eq!(find_hash(&sums, "yt-dlp_macos").as_deref(), Some(H2));
        assert_eq!(find_hash(&sums, "yt-dlp").as_deref(), Some(H1));
        assert_eq!(find_hash(&sums, "missing"), None);
    }

    #[test]
    fn finds_hash_in_single_file_sum() {
        let zip = "deno-aarch64-apple-darwin.zip";
        assert_eq!(find_hash(&format!("{H2}  {zip}\n"), zip).as_deref(), Some(H2));
        assert_eq!(find_hash(&format!("{H2}\n"), zip).as_deref(), Some(H2));
        assert_eq!(find_hash(&format!("{} *bin\n", H2.to_uppercase()), "bin").as_deref(), Some(H2));
        assert_eq!(find_hash("garbage", "x"), None);
    }

    #[test]
    fn install_executable_replaces_atomically_with_mode() {
        let d = tempfile::tempdir().unwrap();
        let dest = d.path().join("bin/yt-dlp");
        install_executable(&dest, b"v1").unwrap();
        install_executable(&dest, b"v2").unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"v2");
        assert_eq!(fs::metadata(&dest).unwrap().permissions().mode() & 0o777, 0o755);
        assert!(!dest.with_extension("tmp").exists());
    }

    #[tokio::test]
    async fn version_reads_first_line() {
        let d = tempfile::tempdir().unwrap();
        let bin = crate::testutil::script(d.path(), "yt-dlp", "echo 2026.09.30\necho extra");
        assert_eq!(version(&bin).await.as_deref(), Some("2026.09.30"));
        assert_eq!(version(&d.path().join("missing")).await, None);
    }
}
```

`lib.rs`에 `pub mod tools;`를 추가한다.

- [ ] **Step 2: 테스트가 실패하는지 확인**

Run: `cargo test -p kiri-core tools`
Expected: 컴파일 에러(`cannot find function sha256_hex`).

- [ ] **Step 3: 구현**

`tools.rs`의 `#[cfg(test)]` 위에 추가한다.

```rust
const YTDLP_ASSET: &str = "yt-dlp_macos";
const YTDLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos";
const YTDLP_SUMS_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS";
const DENO_ASSET: &str = "deno-aarch64-apple-darwin.zip";
const DENO_URL: &str = "https://github.com/denoland/deno/releases/latest/download/deno-aarch64-apple-darwin.zip";
const DENO_SUM_URL: &str =
    "https://github.com/denoland/deno/releases/latest/download/deno-aarch64-apple-darwin.zip.sha256sum";

pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("kiri/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("static client config")
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// "<hash>  <file>" 줄에서 file 의 해시. 해시만 한 줄 있는 파일(단일 .sha256sum)은 그 해시.
pub fn find_hash(sums: &str, file: &str) -> Option<String> {
    let lines: Vec<&str> = sums.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    for line in &lines {
        let hash: String = line.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        let name = line[hash.len()..].trim_start_matches(|c: char| c.is_whitespace() || c == '*');
        if is_hex64(&hash) && (name == file || (name.is_empty() && lines.len() == 1)) {
            return Some(hash.to_ascii_lowercase());
        }
    }
    None
}

pub fn install_executable(dest: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = dest.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
    fs::rename(tmp, dest) // 실행 중인 프로세스가 연 이전 파일(inode)은 그대로 남는다
}

async fn fetch(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client.get(url).send().await.map_err(|e| format!("{url}: {e}"))?;
    let resp = resp.error_for_status().map_err(|e| format!("{url}: {e}"))?;
    resp.bytes().await.map(|b| b.to_vec()).map_err(|e| format!("{url}: {e}"))
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    String::from_utf8(fetch(client, url).await?).map_err(|e| format!("{url}: {e}"))
}

/// 설치된 파일의 해시가 최신 SHA2-256SUMS 와 같으면 아무것도 하지 않는다.
pub async fn ensure_ytdlp(client: &reqwest::Client, dest: &Path) -> Result<bool, String> {
    let sums = fetch_text(client, YTDLP_SUMS_URL).await?;
    let want = find_hash(&sums, YTDLP_ASSET).ok_or("yt-dlp checksum not found")?;
    if let Ok(current) = fs::read(dest) {
        if sha256_hex(&current) == want {
            return Ok(false);
        }
    }
    let bytes = fetch(client, YTDLP_URL).await?;
    if sha256_hex(&bytes) != want {
        return Err("yt-dlp checksum mismatch".into());
    }
    install_executable(dest, &bytes).map_err(|e| e.to_string())?;
    Ok(true)
}

/// ponytail: 없을 때만 설치한다. 갱신은 yt-dlp 가 새 Deno 를 요구하는 일이 생기면 추가한다.
pub async fn ensure_deno(client: &reqwest::Client, dest: &Path) -> Result<bool, String> {
    if dest.exists() {
        return Ok(false);
    }
    let sum = fetch_text(client, DENO_SUM_URL).await?;
    let want = find_hash(&sum, DENO_ASSET).ok_or("deno checksum not found")?;
    let zip = fetch(client, DENO_URL).await?;
    if sha256_hex(&zip) != want {
        return Err("deno checksum mismatch".into());
    }
    let dir = dest.parent().ok_or("deno dest has no parent")?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let zip_path = dir.join("deno.zip");
    fs::write(&zip_path, &zip).map_err(|e| e.to_string())?;
    let status = tokio::process::Command::new("/usr/bin/ditto")
        .args(["-x", "-k"])
        .arg(&zip_path)
        .arg(dir)
        .status()
        .await
        .map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&zip_path);
    if !status.success() || !dest.exists() {
        return Err("deno unzip failed".into());
    }
    fs::set_permissions(dest, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    Ok(true)
}

pub async fn version(bin: &Path) -> Option<String> {
    let out = runner::output(bin, &["--version".to_string()]).await.ok()?;
    out.lines().next().map(|l| l.trim().to_string()).filter(|l| !l.is_empty())
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cargo test -p kiri-core tools`
Expected: `5 passed`.

실제 네트워크 설치는 자동 테스트에 넣지 않는다. Task 13에서 앱을 처음 실행할 때 확인한다.

- [ ] **Step 5: Commit**

```bash
git add crates/kiri-core
git commit -m "feat(core): yt-dlp·Deno 다운로드와 SHA256 검증 설치 추가"
```

---

## Phase B: CLI

### Task 10: `kiri` CLI

**Files:**
- Modify: `Cargo.toml` (members에 `"crates/kiri-cli"` 추가)
- Create: `crates/kiri-cli/Cargo.toml`, `crates/kiri-cli/src/main.rs`, `crates/kiri-cli/tests/cli.rs`

**Interfaces:**
- Consumes: `kiri_core::ipc::{socket_path, request, Request, Response, ClientError}`, `kiri_core::model::{Job, JobState}`
- Produces: `kiri` 바이너리. 종료 코드는 성공 0, 요청 에러 1, 앱 연결 실패 2. 번들 식별자 상수 `BUNDLE_ID = "org.bobpark.kiri"`.

- [ ] **Step 1: crate 만들기**

`Cargo.toml`:

```toml
[workspace]
members = ["crates/kiri-core", "crates/kiri-cli"]
resolver = "3"
```

`crates/kiri-cli/Cargo.toml`:

```toml
[package]
name = "kiri-cli"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "kiri"
path = "src/main.rs"

[dependencies]
kiri-core = { path = "../kiri-core" }
clap = { version = "4", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt", "net", "time", "macros"] }

[dev-dependencies]
tempfile = "3"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

- [ ] **Step 2: 실패하는 단위 테스트 작성**

`crates/kiri-cli/src/main.rs`:

```rust
//! kiri CLI: 실행 중인 kiri 앱의 큐를 조회·제어한다.
use clap::{Parser, Subcommand};
use kiri_core::{
    ipc::{self, ClientError, Request, Response},
    model::{Job, JobState},
};
use std::{path::Path, process::ExitCode, time::Duration};

const BUNDLE_ID: &str = "org.bobpark.kiri";

#[cfg(test)]
mod tests {
    use super::*;
    use kiri_core::model::{JobOptions, Preset};

    fn job(id: u64, state: JobState, progress: f32) -> Job {
        Job {
            id,
            url: "u".into(),
            title: "Rust in 100 Seconds".into(),
            thumbnail: None,
            duration_secs: None,
            quality_label: "1080p60".into(),
            options: JobOptions { format_id: None, preset: Preset::Original, subtitles: vec![], auto_subtitles: false },
            state,
            progress,
            speed: Some("2.1MiB/s".into()),
            eta: Some("00:12".into()),
            output: None,
            created_at: 0,
        }
    }

    #[test]
    fn formats_table() {
        let out = format_jobs(&[job(1, JobState::Downloading, 0.48), job(12, JobState::Failed("boom".into()), 0.0)]);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "ID  STATE        PROGRESS  SPEED     ETA    TITLE");
        assert_eq!(lines[1], "1   downloading  48%       2.1MiB/s  00:12  Rust in 100 Seconds");
        assert_eq!(lines[2], "12  failed       0%        2.1MiB/s  00:12  Rust in 100 Seconds (boom)");
    }

    #[test]
    fn empty_table_message() {
        assert_eq!(format_jobs(&[]), "no jobs");
    }

    #[test]
    fn parses_add_args() {
        let cli = Cli::try_parse_from(["kiri", "add", "https://youtu.be/x", "--quality", "720p", "--format", "mp3", "--subs", "ko,en", "--json"]).unwrap();
        assert!(cli.json);
        assert_eq!(
            cli.cmd.into_request(),
            Request::Add {
                url: "https://youtu.be/x".into(),
                quality: Some("720p".into()),
                preset: Some("mp3".into()),
                subs: Some(vec!["ko".into(), "en".into()]),
            }
        );
    }
}
```

- [ ] **Step 3: 테스트가 실패하는지 확인**

Run: `cargo test -p kiri-cli`
Expected: 컴파일 에러(`cannot find function format_jobs`).

- [ ] **Step 4: 구현**

`main.rs`의 `#[cfg(test)]` 위에 추가한다.

```rust
#[derive(Parser, Debug)]
#[command(name = "kiri", version, about = "Control the kiri YouTube downloader")]
struct Cli {
    /// Print the raw JSON response
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Jobs currently downloading or encoding
    Status,
    /// All jobs in the queue
    List,
    /// Add a YouTube URL to the queue
    Add {
        url: String,
        /// best | audio | 1080p | 720p | …
        #[arg(long)]
        quality: Option<String>,
        /// original | mp4-h264 | mp4-hevc | mov-prores | webm-vp9 | mp3 | m4a
        #[arg(long)]
        format: Option<String>,
        /// Subtitle languages, comma separated (ko,en)
        #[arg(long, value_delimiter = ',')]
        subs: Option<Vec<String>>,
    },
    /// Remove a job (stops it first if running)
    Remove { id: u64 },
    /// Stop a running or queued job
    Stop { id: u64 },
}

impl Cmd {
    fn into_request(self) -> Request {
        match self {
            Cmd::Status => Request::Status,
            Cmd::List => Request::List,
            Cmd::Add { url, quality, format, subs } => Request::Add { url, quality, preset: format, subs },
            Cmd::Remove { id } => Request::Remove { id },
            Cmd::Stop { id } => Request::Stop { id },
        }
    }
}

fn state_name(s: &JobState) -> &'static str {
    match s {
        JobState::Queued => "queued",
        JobState::Downloading => "downloading",
        JobState::Encoding => "encoding",
        JobState::Completed => "completed",
        JobState::Failed(_) => "failed",
        JobState::Stopped => "stopped",
    }
}

fn format_jobs(jobs: &[Job]) -> String {
    if jobs.is_empty() {
        return "no jobs".into();
    }
    let rows: Vec<[String; 6]> = jobs
        .iter()
        .map(|j| {
            let title = match &j.state {
                JobState::Failed(m) => format!("{} ({m})", j.title),
                _ => j.title.clone(),
            };
            [
                j.id.to_string(),
                state_name(&j.state).into(),
                format!("{}%", (j.progress * 100.0).round() as u32),
                j.speed.clone().unwrap_or_else(|| "-".into()),
                j.eta.clone().unwrap_or_else(|| "-".into()),
                title,
            ]
        })
        .collect();
    let header = ["ID", "STATE", "PROGRESS", "SPEED", "ETA", "TITLE"].map(String::from);
    let widths: Vec<usize> = (0..5)
        .map(|c| rows.iter().map(|r| r[c].len()).chain([header[c].len()]).max().unwrap())
        .collect();
    let line = |r: &[String; 6]| {
        let mut s = String::new();
        for c in 0..5 {
            s.push_str(&format!("{:<w$}  ", r[c], w = widths[c]));
        }
        s.push_str(&r[5]);
        s
    };
    std::iter::once(line(&header)).chain(rows.iter().map(line)).collect::<Vec<_>>().join("\n")
}

/// 응답 출력 후 종료 코드.
fn print_response(resp: &Response, json: bool) -> ExitCode {
    if json {
        println!("{}", serde_json::to_string_pretty(resp).expect("response serializes"));
    } else {
        match resp {
            Response::Jobs { jobs } => println!("{}", format_jobs(jobs)),
            Response::Added { job } => println!("added #{}: {}", job.id, job.title),
            Response::Ok => println!("ok"),
            Response::Error { message, .. } => eprintln!("error: {message}"),
        }
    }
    match resp {
        Response::Error { .. } => ExitCode::from(1),
        _ => ExitCode::SUCCESS,
    }
}

async fn launch_and_wait(socket: &Path) -> bool {
    let launched = std::process::Command::new("open").args(["-b", BUNDLE_ID]).status().is_ok_and(|s| s.success());
    if !launched {
        return false;
    }
    for _ in 0..50 {
        if tokio::net::UnixStream::connect(socket).await.is_ok() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    false
}

async fn run(cli: Cli) -> ExitCode {
    let socket = ipc::socket_path();
    let is_add = matches!(cli.cmd, Cmd::Add { .. });
    let req = cli.cmd.into_request();
    let mut result = ipc::request(&socket, &req).await;
    if is_add && result == Err(ClientError::NotRunning) && launch_and_wait(&socket).await {
        result = ipc::request(&socket, &req).await;
    }
    match result {
        Ok(resp) => print_response(&resp, cli.json),
        Err(ClientError::NotRunning) => {
            eprintln!("kiri app is not running");
            ExitCode::from(2)
        }
        Err(ClientError::Io(e)) => {
            eprintln!("cannot talk to kiri app: {e}");
            ExitCode::from(2)
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(run(cli))
}
```

`Request`와 `ClientError`는 Task 8에서 `PartialEq`를 derive했으므로 비교할 수 있다.

- [ ] **Step 5: 단위 테스트 통과 확인**

Run: `cargo test -p kiri-cli --bin kiri`
Expected: `3 passed`.

- [ ] **Step 6: 통합 테스트 작성**

`crates/kiri-cli/tests/cli.rs`:

```rust
//! 실제 소켓 서버(앱 대신 kiri-core Engine)에 CLI 바이너리를 붙여 본다.
use kiri_core::{
    engine::{Engine, EngineConfig, EnginePaths},
    ipc,
    model::{Preset, Tools},
};
use std::{path::Path, process::Command};

fn idle_engine(dir: &Path) -> Engine {
    let tools = Tools { ytdlp: dir.join("none/yt-dlp"), deno: dir.join("none/deno"), ffmpeg: dir.join("none/ffmpeg") };
    let paths = EnginePaths { queue_file: dir.join("q.json"), cache_dir: dir.join("c"), log_dir: dir.join("l") };
    let cfg = EngineConfig {
        max_concurrent: 1,
        hw_accel: false,
        download_dir: dir.join("out"),
        default_quality: "best".into(),
        default_preset: Preset::Original,
        default_subtitles: vec![],
    };
    Engine::new(paths, tools, cfg, tokio::runtime::Handle::current(), |_| {})
}

async fn kiri(socket: &Path, args: &[&str]) -> (i32, String, String) {
    let socket = socket.to_path_buf();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    tokio::task::spawn_blocking(move || {
        let out = Command::new(env!("CARGO_BIN_EXE_kiri")).args(&args).env("KIRI_SOCKET", &socket).output().unwrap();
        (
            out.status.code().unwrap(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn talks_to_running_app() {
    let d = tempfile::tempdir().unwrap();
    let socket = d.path().join("kiri.sock");
    let listener = ipc::bind(&socket).await.unwrap();
    tokio::spawn(ipc::serve(listener, idle_engine(d.path())));

    let (code, out, _) = kiri(&socket, &["list"]).await;
    assert_eq!((code, out.trim()), (0, "no jobs"));

    let (code, out, _) = kiri(&socket, &["--json", "status"]).await;
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v, serde_json::json!({"type": "jobs", "jobs": []}));

    let (code, _, err) = kiri(&socket, &["stop", "42"]).await;
    assert_eq!(code, 1);
    assert!(err.contains("job 42 not found"), "{err}");

    let (code, _, err) = kiri(&socket, &["add", "hello"]).await;
    assert_eq!(code, 1);
    assert!(err.contains("not a YouTube URL"), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn exit_code_2_when_app_not_running() {
    let d = tempfile::tempdir().unwrap();
    let (code, _, err) = kiri(&d.path().join("none.sock"), &["list"]).await;
    assert_eq!(code, 2);
    assert!(err.contains("not running"), "{err}");
}
```

`crates/kiri-cli/Cargo.toml`의 `[dev-dependencies]`에 `serde_json = "1"`이 필요하다. 이미 `[dependencies]`에 있으므로 통합 테스트에서도 쓸 수 있다.

- [ ] **Step 7: 통합 테스트 통과 확인**

Run: `cargo test -p kiri-cli`
Expected: 단위 테스트 3개와 통합 테스트 2개가 모두 통과한다.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock crates/kiri-cli
git commit -m "feat(cli): kiri status/list/add/remove/stop 명령 추가"
```

---
## Phase C: Tauri 앱

### Task 11: Tauri + React 골격, 테마, 다국어

**Files:**
- Modify: `Cargo.toml` (members에 `"src-tauri"` 추가)
- Create:
  - `package.json`, `.yarnrc.yml`, `vite.config.ts`, `tsconfig.json`, `tsconfig.node.json`, `index.html`, `assets/icon.svg`
  - `src/main.tsx`, `src/index.css`, `src/vite-env.d.ts`
  - `src/lib/types.ts`, `src/lib/theme.ts`, `src/lib/i18n.ts`
  - `src/locales/en.json`, `src/locales/ko.json`, `src/locales/ja.json`
  - `src/test/theme.test.ts`, `src/test/i18n.test.ts`, `src/test/locales.test.ts`
  - `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`
  - `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/windows.rs`
  - `src-tauri/icons/*` (생성)

**Interfaces:**
- Produces:
  - `src/lib/types.ts`의 모든 프론트엔드 타입(이후 Task가 그대로 쓴다)
  - `resolveTheme(pref, systemDark)`, `applyTheme(pref)`
  - `resolveLang(pref, navigatorLang)`, `initI18n(lang)`, `langName(code, uiLang)`
  - `windows::show_main(app) -> Result<(), String>`
  - 패키지/crate 이름: Tauri crate `kiri-app`(lib `kiri_lib`)
- 이름 메모: 앱 실행 파일은 `kiri-app`이고 번들 이름은 productName `kiri`다. CLI sidecar는 번들 안에서 이름이 겹치지 않도록 `kiri-cli`로 넣는다(Task 13).

- [ ] **Step 1: 프론트엔드 패키지 파일 작성**

`package.json`:

```json
{
  "name": "kiri",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "tauri": "tauri",
    "test": "vitest run"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-clipboard-manager": "^2",
    "@tauri-apps/plugin-dialog": "^2",
    "@tauri-apps/plugin-opener": "^2",
    "i18next": "^26",
    "react": "^19.1.0",
    "react-dom": "^19.1.0",
    "react-i18next": "^17",
    "zustand": "^5"
  },
  "devDependencies": {
    "@tailwindcss/vite": "^4",
    "@tauri-apps/cli": "^2",
    "@types/react": "^19.1.8",
    "@types/react-dom": "^19.1.6",
    "@vitejs/plugin-react": "^4.6.0",
    "daisyui": "^5.7",
    "pretendard": "1.3.9",
    "tailwindcss": "^4",
    "typescript": "^5.9",
    "vite": "^7.0.4",
    "vitest": "^4"
  },
  "packageManager": "yarn@4.18.0"
}
```

`.yarnrc.yml`:

```yaml
nodeLinker: node-modules
```

`vite.config.ts`:

```ts
/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**", "**/target/**", "**/crates/**"] },
  },
  test: { environment: "node", include: ["src/test/**/*.test.ts"] },
});
```

`tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

`tsconfig.node.json`:

```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true
  },
  "include": ["vite.config.ts"]
}
```

`index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>kiri</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

`src/vite-env.d.ts`:

```ts
/// <reference types="vite/client" />
```

Run: `yarn install`
Expected: `node_modules/`와 `yarn.lock`이 생긴다.

- [ ] **Step 2: 타입 정의**

`src/lib/types.ts`: Rust serde 표현과 1:1로 맞춘다.

```ts
export type Theme = "system" | "light" | "dark";
export type UiLang = "system" | "ko" | "en" | "ja";
export type Lang = "ko" | "en" | "ja";

export type Preset = "original" | "mp4-h264" | "mp4-hevc" | "mov-prores" | "webm-vp9" | "mp3" | "m4a";
export const PRESETS: Preset[] = ["original", "mp4-h264", "mp4-hevc", "mov-prores", "webm-vp9", "mp3", "m4a"];
export const AUDIO_PRESETS: Preset[] = ["mp3", "m4a"];

export interface Settings {
  version: number;
  general: { ui_language: UiLang; theme: Theme; close_to_tray: boolean };
  download: {
    dir: string;
    quality: string; // "best" | "audio" | "<N>p"
    preset: Preset;
    subtitles: string[];
    skip_sheet: boolean;
    max_concurrent: number;
    hw_accel: boolean;
  };
  update: { auto_check: boolean; last_ytdlp_check: number | null };
}

export type DeepPartial<T> = {
  [K in keyof T]?: T[K] extends unknown[] ? T[K] : T[K] extends object ? DeepPartial<T[K]> : T[K];
};

export type JobState =
  | { kind: "queued" | "downloading" | "encoding" | "completed" | "stopped" }
  | { kind: "failed"; message: string };

export interface JobOptions {
  format_id: string | null;
  preset: Preset;
  subtitles: string[];
  auto_subtitles: boolean;
}

export interface Job {
  id: number;
  url: string;
  title: string;
  thumbnail: string | null;
  duration_secs: number | null;
  quality_label: string;
  options: JobOptions;
  state: JobState;
  progress: number;
  speed: string | null;
  eta: string | null;
  output: string | null;
  created_at: number;
}

export type NewJob = Pick<Job, "url" | "title" | "thumbnail" | "duration_secs" | "quality_label" | "options">;

export interface Quality {
  format_id: string;
  height: number;
  fps: number | null;
  vcodec: string;
  filesize: number | null;
  label: string;
}

export interface VideoInfo {
  id: string;
  title: string;
  channel: string | null;
  duration_secs: number | null;
  thumbnail: string | null;
  qualities: Quality[];
  subtitles: string[];
  auto_subtitles: string[];
}

export interface ToolsStatus {
  ready: boolean;
  installing: boolean;
  ytdlp_version: string | null;
  error: string | null;
  last_check: number | null;
}

export interface CliStatus {
  installed: boolean;
  link: string;
  target: string;
  socket_error: string | null;
}

export interface UpdateInfo {
  version: string;
  notes: string;
}

export interface UpdateProgress {
  received: number;
  total: number | null;
}

/** Rust CmdError */
export interface CmdError {
  code: string;
  message: string;
}
```

- [ ] **Step 3: 실패하는 프론트엔드 테스트 작성**

`src/test/theme.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { resolveTheme } from "../lib/theme";

describe("resolveTheme", () => {
  it("follows the system when pref is system", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
  });
  it("explicit pref wins", () => {
    expect(resolveTheme("dark", false)).toBe("dark");
    expect(resolveTheme("light", true)).toBe("light");
  });
});
```

`src/test/i18n.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { resolveLang } from "../lib/i18n";

describe("resolveLang", () => {
  it("uses the navigator language for system", () => {
    expect(resolveLang("system", "ko-KR")).toBe("ko");
    expect(resolveLang("system", "ja")).toBe("ja");
  });
  it("explicit pref wins", () => {
    expect(resolveLang("ja", "ko-KR")).toBe("ja");
  });
  it("falls back to english", () => {
    expect(resolveLang("system", "de-DE")).toBe("en");
  });
});
```

`src/test/locales.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import ko from "../locales/ko.json";
import en from "../locales/en.json";
import ja from "../locales/ja.json";

function keys(obj: Record<string, unknown>, prefix = ""): string[] {
  return Object.entries(obj).flatMap(([k, v]) =>
    typeof v === "object" && v !== null ? keys(v as Record<string, unknown>, `${prefix}${k}.`) : [`${prefix}${k}`],
  );
}

describe("locale files", () => {
  it("have identical key sets", () => {
    const e = keys(en).sort();
    expect(keys(ko).sort()).toEqual(e);
    expect(keys(ja).sort()).toEqual(e);
  });
  it("have no empty strings", () => {
    for (const f of [ko, en, ja]) expect(JSON.stringify(f)).not.toContain('""');
  });
});
```

Run: `yarn test`
Expected: FAIL. 모듈을 찾을 수 없다(`../lib/theme`, `../locales/ko.json`).

- [ ] **Step 4: 테마·i18n 구현**

`src/lib/theme.ts`:

```ts
import type { Theme } from "./types";

export function resolveTheme(pref: Theme, systemDark: boolean): "dark" | "light" {
  if (pref === "system") return systemDark ? "dark" : "light";
  return pref;
}

// matchMedia 는 호출마다 새 객체를 주므로 하나만 유지한다(핸들러가 쌓이지 않게).
let mq: MediaQueryList | null = null;
let current: Theme = "system";

function apply() {
  const dark = resolveTheme(current, mq?.matches ?? false) === "dark";
  document.documentElement.dataset.theme = dark ? "kiri" : "kiri-light";
}

export function applyTheme(pref: Theme) {
  current = pref;
  if (!mq) {
    mq = window.matchMedia("(prefers-color-scheme: dark)");
    mq.onchange = apply;
  }
  apply();
}
```

`src/lib/i18n.ts`:

```ts
import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import ko from "../locales/ko.json";
import en from "../locales/en.json";
import ja from "../locales/ja.json";
import type { Lang, UiLang } from "./types";

export function resolveLang(pref: UiLang, navigatorLang: string): Lang {
  const code = pref === "system" ? navigatorLang : pref;
  const primary = code.split(/[-_]/)[0]?.toLowerCase();
  return primary === "ko" || primary === "ja" ? primary : "en";
}

/** 자막 언어 코드 → 사람이 읽는 이름 (Intl, 실패하면 코드 그대로). */
export function langName(code: string, uiLang: string): string {
  try {
    return new Intl.DisplayNames([uiLang], { type: "language" }).of(code) ?? code;
  } catch {
    return code;
  }
}

export async function initI18n(lang: Lang) {
  if (i18next.isInitialized) return i18next.changeLanguage(lang);
  return i18next.use(initReactI18next).init({
    lng: lang,
    fallbackLng: "en",
    resources: { ko: { translation: ko }, en: { translation: en }, ja: { translation: ja } },
    interpolation: { escapeValue: false },
  });
}
```

- [ ] **Step 5: 로케일 파일 작성**

`src/locales/en.json`:

```json
{
  "app": {
    "dropHint": "Paste a YouTube link (⌘V) or drop it here",
    "empty": "No downloads yet",
    "settings": "Settings",
    "toolsPreparing": "Preparing yt-dlp…",
    "toolsFailed": "Couldn't prepare yt-dlp: {{error}}",
    "retry": "Retry"
  },
  "job": {
    "stop": "Stop",
    "remove": "Remove",
    "restart": "Restart",
    "reveal": "Show in Finder",
    "subs": "Subs {{langs}}"
  },
  "state": {
    "queued": "Queued",
    "downloading": "Downloading",
    "encoding": "Encoding",
    "completed": "Done",
    "failed": "Failed",
    "stopped": "Stopped"
  },
  "quality": { "best": "Best", "audio": "Audio only" },
  "preset": {
    "original": "Keep original",
    "mp4-h264": "MP4 · H.264",
    "mp4-hevc": "MP4 · HEVC",
    "mov-prores": "MOV · ProRes 422",
    "webm-vp9": "WebM · VP9",
    "mp3": "MP3",
    "m4a": "M4A (AAC)"
  },
  "sheet": {
    "loading": "Reading video info…",
    "quality": "Quality",
    "format": "Output format",
    "subtitles": "Subtitles",
    "autoSubs": "Auto-generated",
    "noSubs": "No subtitles available",
    "remember": "Start right away with these settings next time",
    "cancel": "Cancel",
    "download": "Download"
  },
  "update": {
    "available": "kiri {{version}} is available",
    "install": "Install and restart",
    "installNow": "Restart now",
    "afterQueue": "Restart when queue finishes",
    "scheduled": "kiri will restart when the queue finishes",
    "downloading": "Downloading update… {{pct}}%"
  },
  "settings": {
    "tab": { "general": "General", "download": "Downloads", "cli": "CLI", "update": "Updates" },
    "general": {
      "language": "Language",
      "langSystem": "System",
      "langKo": "한국어",
      "langEn": "English",
      "langJa": "日本語",
      "theme": "Theme",
      "themeSystem": "System",
      "themeLight": "Light",
      "themeDark": "Dark",
      "closeToTray": "Hide to the menu bar when the window closes",
      "closeToTrayDesc": "Downloads keep running. Double-click the menu bar icon to reopen."
    },
    "download": {
      "dir": "Save to",
      "change": "Change…",
      "quality": "Default quality",
      "preset": "Default format",
      "subtitles": "Default subtitles",
      "subtitlesHint": "Comma separated, e.g. ko,en",
      "skipSheet": "Start without asking for options",
      "maxConcurrent": "Simultaneous downloads",
      "hwAccel": "Use hardware acceleration",
      "hwAccelDesc": "Encode with VideoToolbox when possible"
    },
    "cli": {
      "status": "Command-line tool",
      "installed": "Installed at {{path}}",
      "notInstalled": "Not installed",
      "install": "Install CLI",
      "uninstall": "Uninstall",
      "socketError": "CLI connection unavailable: {{error}}",
      "usage": "Usage"
    },
    "update": {
      "appVersion": "kiri {{version}}",
      "check": "Check now",
      "checking": "Checking…",
      "upToDate": "You're up to date",
      "autoCheck": "Check for updates automatically",
      "ytdlpVersion": "yt-dlp {{version}}",
      "ytdlpMissing": "yt-dlp not installed",
      "ytdlpUpdate": "Update now",
      "lastCheck": "Last checked {{time}}",
      "never": "never"
    }
  },
  "error": {
    "invalid_url": "That's not a YouTube link",
    "ytdlp_missing": "yt-dlp is still being prepared",
    "not_found": "That job no longer exists",
    "not_stoppable": "That job isn't running",
    "not_restartable": "That job can't be restarted",
    "bad_quality": "Unknown quality",
    "bad_preset": "Unknown format",
    "download_dir_unwritable": "Can't write to the download folder",
    "no_output": "yt-dlp finished but produced no file",
    "unknown": "Something went wrong"
  }
}
```

`src/locales/ko.json`:

```json
{
  "app": {
    "dropHint": "YouTube 링크를 붙여넣거나(⌘V) 끌어다 놓으세요",
    "empty": "아직 다운로드가 없습니다",
    "settings": "설정",
    "toolsPreparing": "yt-dlp 준비 중…",
    "toolsFailed": "yt-dlp를 준비하지 못했습니다: {{error}}",
    "retry": "재시도"
  },
  "job": {
    "stop": "중지",
    "remove": "삭제",
    "restart": "다시 시작",
    "reveal": "Finder에서 보기",
    "subs": "자막 {{langs}}"
  },
  "state": {
    "queued": "대기",
    "downloading": "다운로드 중",
    "encoding": "인코딩 중",
    "completed": "완료",
    "failed": "실패",
    "stopped": "중지됨"
  },
  "quality": { "best": "최고 화질", "audio": "오디오만" },
  "preset": {
    "original": "원본 유지",
    "mp4-h264": "MP4 · H.264",
    "mp4-hevc": "MP4 · HEVC",
    "mov-prores": "MOV · ProRes 422",
    "webm-vp9": "WebM · VP9",
    "mp3": "MP3",
    "m4a": "M4A (AAC)"
  },
  "sheet": {
    "loading": "영상 정보를 읽는 중…",
    "quality": "화질",
    "format": "출력 포맷",
    "subtitles": "자막",
    "autoSubs": "자동 생성",
    "noSubs": "자막 없음",
    "remember": "다음부터 이 설정으로 바로 시작",
    "cancel": "취소",
    "download": "다운로드"
  },
  "update": {
    "available": "kiri {{version}} 버전을 사용할 수 있습니다",
    "install": "설치 후 재시작",
    "installNow": "지금 재시작",
    "afterQueue": "큐가 끝나면 재시작",
    "scheduled": "큐가 끝나면 kiri가 재시작됩니다",
    "downloading": "업데이트 받는 중… {{pct}}%"
  },
  "settings": {
    "tab": { "general": "일반", "download": "다운로드", "cli": "CLI", "update": "업데이트" },
    "general": {
      "language": "언어",
      "langSystem": "시스템 설정",
      "langKo": "한국어",
      "langEn": "English",
      "langJa": "日本語",
      "theme": "테마",
      "themeSystem": "시스템",
      "themeLight": "라이트",
      "themeDark": "다크",
      "closeToTray": "창을 닫으면 메뉴 막대로 숨기기",
      "closeToTrayDesc": "다운로드는 계속됩니다. 메뉴 막대 아이콘을 더블클릭하면 다시 열립니다."
    },
    "download": {
      "dir": "저장 위치",
      "change": "변경…",
      "quality": "기본 화질",
      "preset": "기본 포맷",
      "subtitles": "기본 자막",
      "subtitlesHint": "쉼표로 구분, 예: ko,en",
      "skipSheet": "옵션을 묻지 않고 바로 시작",
      "maxConcurrent": "동시 다운로드",
      "hwAccel": "하드웨어 가속 사용",
      "hwAccelDesc": "가능하면 VideoToolbox로 인코딩합니다"
    },
    "cli": {
      "status": "명령줄 도구",
      "installed": "{{path}}에 설치됨",
      "notInstalled": "설치되지 않음",
      "install": "CLI 설치",
      "uninstall": "제거",
      "socketError": "CLI 연결 불가: {{error}}",
      "usage": "사용법"
    },
    "update": {
      "appVersion": "kiri {{version}}",
      "check": "지금 확인",
      "checking": "확인 중…",
      "upToDate": "최신 버전입니다",
      "autoCheck": "자동으로 업데이트 확인",
      "ytdlpVersion": "yt-dlp {{version}}",
      "ytdlpMissing": "yt-dlp가 설치되지 않음",
      "ytdlpUpdate": "지금 업데이트",
      "lastCheck": "마지막 확인 {{time}}",
      "never": "없음"
    }
  },
  "error": {
    "invalid_url": "YouTube 링크가 아닙니다",
    "ytdlp_missing": "yt-dlp를 아직 준비하고 있습니다",
    "not_found": "작업이 더 이상 없습니다",
    "not_stoppable": "실행 중인 작업이 아닙니다",
    "not_restartable": "다시 시작할 수 없는 작업입니다",
    "bad_quality": "알 수 없는 화질입니다",
    "bad_preset": "알 수 없는 포맷입니다",
    "download_dir_unwritable": "저장 위치에 쓸 수 없습니다",
    "no_output": "yt-dlp가 끝났지만 파일이 없습니다",
    "unknown": "문제가 발생했습니다"
  }
}
```

`src/locales/ja.json`:

```json
{
  "app": {
    "dropHint": "YouTube のリンクをペースト（⌘V）またはドロップ",
    "empty": "まだダウンロードはありません",
    "settings": "設定",
    "toolsPreparing": "yt-dlp を準備中…",
    "toolsFailed": "yt-dlp を準備できませんでした: {{error}}",
    "retry": "再試行"
  },
  "job": {
    "stop": "停止",
    "remove": "削除",
    "restart": "再開",
    "reveal": "Finder で表示",
    "subs": "字幕 {{langs}}"
  },
  "state": {
    "queued": "待機中",
    "downloading": "ダウンロード中",
    "encoding": "エンコード中",
    "completed": "完了",
    "failed": "失敗",
    "stopped": "停止済み"
  },
  "quality": { "best": "最高画質", "audio": "音声のみ" },
  "preset": {
    "original": "元のまま",
    "mp4-h264": "MP4 · H.264",
    "mp4-hevc": "MP4 · HEVC",
    "mov-prores": "MOV · ProRes 422",
    "webm-vp9": "WebM · VP9",
    "mp3": "MP3",
    "m4a": "M4A (AAC)"
  },
  "sheet": {
    "loading": "動画情報を読み込み中…",
    "quality": "画質",
    "format": "出力フォーマット",
    "subtitles": "字幕",
    "autoSubs": "自動生成",
    "noSubs": "字幕なし",
    "remember": "次回からこの設定ですぐに開始",
    "cancel": "キャンセル",
    "download": "ダウンロード"
  },
  "update": {
    "available": "kiri {{version}} が利用可能です",
    "install": "インストールして再起動",
    "installNow": "今すぐ再起動",
    "afterQueue": "キュー完了後に再起動",
    "scheduled": "キューが終わると kiri が再起動します",
    "downloading": "アップデートをダウンロード中… {{pct}}%"
  },
  "settings": {
    "tab": { "general": "一般", "download": "ダウンロード", "cli": "CLI", "update": "アップデート" },
    "general": {
      "language": "言語",
      "langSystem": "システム",
      "langKo": "한국어",
      "langEn": "English",
      "langJa": "日本語",
      "theme": "テーマ",
      "themeSystem": "システム",
      "themeLight": "ライト",
      "themeDark": "ダーク",
      "closeToTray": "ウィンドウを閉じたらメニューバーに隠す",
      "closeToTrayDesc": "ダウンロードは続きます。メニューバーのアイコンをダブルクリックで再表示します。"
    },
    "download": {
      "dir": "保存先",
      "change": "変更…",
      "quality": "既定の画質",
      "preset": "既定のフォーマット",
      "subtitles": "既定の字幕",
      "subtitlesHint": "カンマ区切り（例: ko,en）",
      "skipSheet": "オプションを聞かずにすぐ開始",
      "maxConcurrent": "同時ダウンロード数",
      "hwAccel": "ハードウェアアクセラレーションを使用",
      "hwAccelDesc": "可能な場合は VideoToolbox でエンコードします"
    },
    "cli": {
      "status": "コマンドラインツール",
      "installed": "{{path}} にインストール済み",
      "notInstalled": "未インストール",
      "install": "CLI をインストール",
      "uninstall": "アンインストール",
      "socketError": "CLI に接続できません: {{error}}",
      "usage": "使い方"
    },
    "update": {
      "appVersion": "kiri {{version}}",
      "check": "今すぐ確認",
      "checking": "確認中…",
      "upToDate": "最新バージョンです",
      "autoCheck": "自動的にアップデートを確認",
      "ytdlpVersion": "yt-dlp {{version}}",
      "ytdlpMissing": "yt-dlp 未インストール",
      "ytdlpUpdate": "今すぐ更新",
      "lastCheck": "最終確認 {{time}}",
      "never": "なし"
    }
  },
  "error": {
    "invalid_url": "YouTube のリンクではありません",
    "ytdlp_missing": "yt-dlp を準備中です",
    "not_found": "そのジョブはもうありません",
    "not_stoppable": "実行中のジョブではありません",
    "not_restartable": "再開できないジョブです",
    "bad_quality": "不明な画質です",
    "bad_preset": "不明なフォーマットです",
    "download_dir_unwritable": "保存先に書き込めません",
    "no_output": "yt-dlp は終了しましたがファイルがありません",
    "unknown": "問題が発生しました"
  }
}
```

- [ ] **Step 6: 프론트엔드 테스트 통과 확인**

Run: `yarn test`
Expected: 3 files, 7 tests passed.

- [ ] **Step 7: 스타일과 진입점**

`src/index.css`:

```css
@import "tailwindcss";
@import "pretendard/dist/web/variable/pretendardvariable.css";
@source not "../docs";
@plugin "daisyui" { themes: false; exclude: rootcolor; }
@plugin "daisyui/theme" {
  name: "kiri"; color-scheme: dark;
  --color-base-100: #16171c; --color-base-200: #1d1e25; --color-base-300: #2c2e38; --color-base-content: #ececf2;
  --color-neutral: #1d1e25; --color-neutral-content: #ececf2;
  --color-primary: #8b5cff; --color-primary-content: #ffffff;
  --color-secondary: rgba(139, 92, 255, 0.22); --color-secondary-content: #c4b0ff;
  --color-accent: #8b5cff; --color-accent-content: #ffffff;
  --color-info: #9b7cff; --color-info-content: #101114;
  --color-success: #4cd393; --color-success-content: #101114;
  --color-warning: #fbbf24; --color-warning-content: #101114;
  --color-error: #f3727f; --color-error-content: #101114;
  --radius-box: 12px; --radius-field: 12px; --radius-selector: 6px;
  --size-field: 0.25rem; --size-selector: 0.25rem; --border: 1px; --depth: 0; --noise: 0;
}
@plugin "daisyui/theme" {
  name: "kiri-light"; default: true; color-scheme: light;
  --color-base-100: #ffffff; --color-base-200: #f6f6f9; --color-base-300: #dedee5; --color-base-content: #101114;
  --color-neutral: #f6f6f9; --color-neutral-content: #101114;
  --color-primary: #7132f5; --color-primary-content: #ffffff;
  --color-secondary: rgba(133, 91, 251, 0.16); --color-secondary-content: #5b1ecf;
  --color-accent: #7132f5; --color-accent-content: #ffffff;
  --color-info: #5741d8; --color-info-content: #ffffff;
  --color-success: #149e61; --color-success-content: #ffffff;
  --color-warning: #f59e0b; --color-warning-content: #101114;
  --color-error: #d33a4a; --color-error-content: #ffffff;
  --radius-box: 12px; --radius-field: 12px; --radius-selector: 6px;
  --size-field: 0.25rem; --size-selector: 0.25rem; --border: 1px; --depth: 0; --noise: 0;
}

@theme {
  --color-fg-muted: #8b8ea3;
  --font-sans: "Pretendard Variable", Pretendard, -apple-system, "Helvetica Neue", Arial, "Hiragino Sans", sans-serif;
}
[data-theme="kiri-light"] { --color-fg-muted: #9497a9; }

html, body, #root { height: 100%; margin: 0; }
body { background: var(--color-base-100); color: var(--color-base-content); font-family: var(--font-sans); font-size: 14px; -webkit-user-select: none; user-select: none; }
```

`src/main.tsx` (임시. Task 15에서 교체한다):

```tsx
import ReactDOM from "react-dom/client";
import "./index.css";
import { applyTheme } from "./lib/theme";
import { initI18n, resolveLang } from "./lib/i18n";

applyTheme("system");
initI18n(resolveLang("system", navigator.language));
ReactDOM.createRoot(document.getElementById("root")!).render(<div className="p-6">kiri</div>);
```

- [ ] **Step 8: 앱 아이콘**

`assets/icon.svg`:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <rect x="64" y="64" width="896" height="896" rx="200" fill="#7132f5"/>
  <path d="M512 250v400M340 490l172 172 172-172" stroke="#fff" stroke-width="80" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M320 780h384" stroke="#fff" stroke-width="80" stroke-linecap="round"/>
</svg>
```

- [ ] **Step 9: Tauri crate**

`Cargo.toml`:

```toml
[workspace]
members = ["crates/kiri-core", "crates/kiri-cli", "src-tauri"]
resolver = "3"
```

`src-tauri/Cargo.toml`:

```toml
[package]
name = "kiri-app"
version = "0.1.0"
edition = "2024"

[lib]
name = "kiri_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
kiri-core = { path = "../crates/kiri-core" }
tauri = { version = "2", features = ["tray-icon", "image-png"] }
tauri-plugin-opener = "2"
tauri-plugin-dialog = "2"
tauri-plugin-clipboard-manager = "2"
tokio = { version = "1", features = ["time", "rt"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sys-locale = "0.3"

[dev-dependencies]
tempfile = "3"
```

`src-tauri/build.rs`:

```rust
fn main() {
    tauri_build::build()
}
```

`src-tauri/tauri.conf.json`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "kiri",
  "version": "0.1.0",
  "identifier": "org.bobpark.kiri",
  "build": {
    "beforeDevCommand": "yarn dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "yarn build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": ["app", "dmg"],
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.png"],
    "macOS": {
      "minimumSystemVersion": "14.2",
      "signingIdentity": null,
      "hardenedRuntime": true
    }
  }
}
```

`src-tauri/capabilities/default.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "kiri windows",
  "windows": ["main", "settings"],
  "permissions": [
    "core:default",
    "opener:allow-reveal-item-in-dir",
    "dialog:allow-open",
    "clipboard-manager:allow-read-text"
  ]
}
```

`src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    kiri_lib::run()
}
```

`src-tauri/src/windows.rs`:

```rust
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const MAIN: &str = "main";

pub fn show_main(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(MAIN) {
        w.show().map_err(|e| e.to_string())?;
        return w.set_focus().map_err(|e| e.to_string());
    }
    WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("/".into()))
        .title("kiri")
        .inner_size(720.0, 520.0)
        .min_inner_size(560.0, 400.0)
        // 링크 드롭을 HTML5 drop 이벤트로 받기 위해 Tauri 의 파일 드롭 처리를 끈다.
        .disable_drag_drop_handler()
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
```

`src-tauri/src/lib.rs`:

```rust
mod windows;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            windows::show_main(app.handle()).map_err(std::io::Error::other)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running kiri");
}
```

Run: `yarn tauri icon assets/icon.svg`
Expected: `src-tauri/icons/`에 png, icns, ico가 생긴다.

- [ ] **Step 10: 빌드와 실행 확인**

Run: `cargo build -p kiri-app`
Expected: 성공.

Run: `yarn tauri dev`
Expected: "kiri"라고 적힌 720×520 창이 뜬다. macOS 다크 모드에서는 배경이 `#16171c`다. 확인했으면 종료한다.

- [ ] **Step 11: Commit**

```bash
git add Cargo.toml Cargo.lock package.json yarn.lock .yarnrc.yml vite.config.ts tsconfig.json tsconfig.node.json index.html assets src src-tauri
git commit -m "feat(app): Tauri + React 골격, Kraken 테마, ko/en/ja 다국어 추가"
```

---

### Task 12: 설정 저장과 설정 스토어

**Files:**
- Create: `src-tauri/src/settings.rs`, `src-tauri/src/commands.rs`, `src/lib/tauri.ts`, `src/lib/toast.ts`, `src/lib/settings.ts`, `src/test/settings.test.ts`, `src/test/toast.test.ts`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `kiri_core::engine::EngineConfig`, `kiri_core::model::Preset`
- Produces:
  - Rust:
    - `settings::{Settings, General, Download, Update, merge, engine_config, SettingsState}`
    - `SettingsState::{new(path), get() -> Settings, set(&AppHandle, Settings) -> Result<(), String>}`. `set`은 저장한 뒤 `settings-changed`를 emit한다.
    - commands `get_settings`, `patch_settings(patch: Value)`
    - `commands::CmdError { code, message }`, `type CmdResult<T>`
  - TS:
    - `api` 객체(`src/lib/tauri.ts`, 이후 Task에서 함수를 추가한다)
    - `useSettings` store(`settings`, `load`, `update(patch)`, `subscribeBackend`)
    - `mergeSettings`, `defaultSettings`
    - `useToasts`, `showError(e)`, `showInfo(text)`, `errorText(e, t)`

- [ ] **Step 1: 실패하는 Rust 테스트 작성**

`src-tauri/src/settings.rs`:

```rust
use kiri_core::{engine::EngineConfig, model::Preset};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Emitter};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_spec() {
        let s = Settings::default();
        assert_eq!(s.version, 1);
        assert_eq!((s.general.ui_language.as_str(), s.general.theme.as_str()), ("system", "system"));
        assert!(s.general.close_to_tray);
        assert!(s.download.dir.ends_with("/Movies/kiri"), "{}", s.download.dir);
        assert_eq!((s.download.quality.as_str(), s.download.preset.as_str()), ("best", "original"));
        assert!(s.download.subtitles.is_empty());
        assert!(!s.download.skip_sheet);
        assert_eq!(s.download.max_concurrent, 2);
        assert!(s.download.hw_accel);
        assert!(s.update.auto_check);
        assert_eq!(s.update.last_ytdlp_check, None);
    }

    #[test]
    fn roundtrip_uses_snake_case_keys() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("settings.json");
        let mut s = Settings::default();
        s.download.max_concurrent = 3;
        s.general.close_to_tray = false;
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"close_to_tray\"") && text.contains("\"max_concurrent\""), "{text}");
    }

    #[test]
    fn missing_fields_and_corrupt_file() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("settings.json");
        fs::write(&path, r#"{"general":{"theme":"dark"}}"#).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.general.theme, "dark");
        assert_eq!(s.download.max_concurrent, 2);
        fs::write(&path, "{nope").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
    }

    #[test]
    fn merge_recurses_and_replaces() {
        let mut base = serde_json::json!({"general": {"theme": "system", "ui_language": "system"}, "download": {"subtitles": ["ko"]}});
        merge(&mut base, &serde_json::json!({"general": {"theme": "dark"}, "download": {"subtitles": []}}));
        assert_eq!(base["general"]["theme"], "dark");
        assert_eq!(base["general"]["ui_language"], "system");
        assert_eq!(base["download"]["subtitles"], serde_json::json!([]));
    }

    #[test]
    fn engine_config_maps_and_clamps() {
        let mut s = Settings::default();
        s.download.max_concurrent = 9;
        s.download.preset = "mp4-hevc".into();
        s.download.subtitles = vec!["ko".into()];
        let c = engine_config(&s);
        assert_eq!(c.max_concurrent, 4);
        assert_eq!(c.default_preset, Preset::Mp4Hevc);
        assert_eq!(c.default_subtitles, vec!["ko"]);
        assert_eq!(c.download_dir, PathBuf::from(&s.download.dir));
        s.download.preset = "bogus".into();
        s.download.max_concurrent = 0;
        let c = engine_config(&s);
        assert_eq!((c.default_preset, c.max_concurrent), (Preset::Original, 1));
    }
}
```

`src-tauri/src/lib.rs` 맨 위에 `mod commands;`와 `mod settings;`를 추가한다.

Run: `cargo test -p kiri-app`
Expected: 컴파일 에러(`cannot find struct Settings`).

- [ ] **Step 2: Rust 설정 구현**

`settings.rs`의 `#[cfg(test)]` 위에 추가한다.

```rust
/// patch 를 base 에 깊게 병합한다. 객체끼리는 재귀, 그 외(배열 포함)는 통째로 치환.
pub fn merge(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                merge(b.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (b, p) => *b = p.clone(),
    }
}

// ponytail: 열거형 대신 String. 프론트 TS 유니온이 값을 제한하고 모르는 값은 기본값으로 취급한다.

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub general: General,
    pub download: Download,
    pub update: Update,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct General {
    pub ui_language: String, // system | ko | en | ja
    pub theme: String,       // system | light | dark
    pub close_to_tray: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Download {
    pub dir: String,
    pub quality: String, // best | audio | <N>p
    pub preset: String,  // Preset id
    pub subtitles: Vec<String>,
    pub skip_sheet: bool,
    pub max_concurrent: u8,
    pub hw_accel: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Update {
    pub auto_check: bool,
    /// unix ms. yt-dlp 최신 여부를 마지막으로 확인한 시각.
    pub last_ytdlp_check: Option<u64>,
}

fn default_download_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    format!("{home}/Movies/kiri")
}

impl Default for Settings {
    fn default() -> Self {
        Self { version: 1, general: General::default(), download: Download::default(), update: Update::default() }
    }
}

impl Default for General {
    fn default() -> Self {
        Self { ui_language: "system".into(), theme: "system".into(), close_to_tray: true }
    }
}

impl Default for Download {
    fn default() -> Self {
        Self {
            dir: default_download_dir(),
            quality: "best".into(),
            preset: "original".into(),
            subtitles: vec![],
            skip_sheet: false,
            max_concurrent: 2,
            hw_accel: true,
        }
    }
}

impl Default for Update {
    fn default() -> Self {
        Self { auto_check: true, last_ytdlp_check: None }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("kiri settings: parse error, using defaults: {e}");
                Settings::default()
            }),
            Err(e) => {
                if e.kind() != io::ErrorKind::NotFound {
                    eprintln!("kiri settings: read error, using defaults: {e}");
                }
                Settings::default()
            }
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(self).map_err(io::Error::other)?)?;
        fs::rename(tmp, path)
    }
}

pub fn engine_config(s: &Settings) -> EngineConfig {
    EngineConfig {
        max_concurrent: s.download.max_concurrent.clamp(1, 4) as usize,
        hw_accel: s.download.hw_accel,
        download_dir: PathBuf::from(&s.download.dir),
        default_quality: s.download.quality.clone(),
        default_preset: s.download.preset.parse().unwrap_or(Preset::Original),
        default_subtitles: s.download.subtitles.clone(),
    }
}

pub struct SettingsState {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsState {
    pub fn new(path: PathBuf) -> Self {
        let current = Mutex::new(Settings::load(&path));
        Self { path, current }
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }

    pub fn set(&self, app: &AppHandle, new: Settings) -> Result<(), String> {
        new.save(&self.path).map_err(|e| e.to_string())?;
        *self.current.lock().unwrap() = new.clone();
        app.emit("settings-changed", &new).map_err(|e| e.to_string())
    }
}
```

`src-tauri/src/commands.rs`:

```rust
use crate::settings::{self, Settings, SettingsState};
use serde::Serialize;
use tauri::{AppHandle, State};

/// 프론트로 가는 오류. code 는 i18n 키 error.<code>.
#[derive(Serialize, Debug)]
pub struct CmdError {
    pub code: String,
    pub message: String,
}

impl From<String> for CmdError {
    fn from(message: String) -> Self {
        Self { code: "unknown".into(), message }
    }
}

pub type CmdResult<T> = Result<T, CmdError>;

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> Settings {
    state.get()
}

/// 부분 갱신. 전체 문서를 쓰면 동시 쓰기가 서로를 덮는다.
#[tauri::command]
pub fn patch_settings(app: AppHandle, state: State<'_, SettingsState>, patch: serde_json::Value) -> CmdResult<()> {
    let mut merged = serde_json::to_value(state.get()).map_err(|e| e.to_string())?;
    settings::merge(&mut merged, &patch);
    let next: Settings = serde_json::from_value(merged).map_err(|e| e.to_string())?;
    state.set(&app, next)?;
    Ok(())
}
```

`src-tauri/src/lib.rs`:

```rust
mod commands;
mod settings;
mod windows;

use settings::SettingsState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            app.manage(SettingsState::new(app.path().app_config_dir()?.join("settings.json")));
            windows::show_main(app.handle()).map_err(std::io::Error::other)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::get_settings, commands::patch_settings])
        .run(tauri::generate_context!())
        .expect("error while running kiri");
}
```

Run: `cargo test -p kiri-app`
Expected: `5 passed`.

- [ ] **Step 3: 실패하는 TS 테스트 작성**

`src/test/settings.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { defaultSettings, mergeSettings } from "../lib/settings";

describe("mergeSettings", () => {
  it("merges nested objects and replaces arrays", () => {
    const base = { ...defaultSettings, download: { ...defaultSettings.download, subtitles: ["ko"] } };
    const next = mergeSettings(base, { general: { theme: "dark" }, download: { subtitles: [] } });
    expect(next.general.theme).toBe("dark");
    expect(next.general.ui_language).toBe("system");
    expect(next.download.subtitles).toEqual([]);
    expect(next.download.max_concurrent).toBe(2);
  });
});
```

`src/test/toast.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { errorText } from "../lib/toast";

const dict: Record<string, string> = { "error.invalid_url": "Not YouTube", "error.unknown": "Oops", "error.no_output": "No file" };
const t = (k: string, o?: { defaultValue?: string }) => dict[k] ?? o?.defaultValue ?? k;

describe("errorText", () => {
  it("translates command errors by code", () => {
    expect(errorText({ code: "invalid_url", message: "not a YouTube URL" }, t)).toBe("Not YouTube");
  });
  it("falls back to the message for unknown codes", () => {
    expect(errorText({ code: "probe_failed", message: "ERROR: private video" }, t)).toBe("ERROR: private video");
    expect(errorText({ code: "probe_failed", message: "" }, t)).toBe("Oops");
  });
  it("translates i18n-key strings and passes raw strings through", () => {
    expect(errorText("error.no_output", t)).toBe("No file");
    expect(errorText(new Error("boom"), t)).toBe("boom");
  });
});
```

Run: `yarn test`
Expected: FAIL. `../lib/settings`와 `../lib/toast`를 찾을 수 없다.

- [ ] **Step 4: TS 구현**

`src/lib/tauri.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import type { DeepPartial, Settings } from "./types";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  patchSettings: (patch: DeepPartial<Settings>) => invoke<void>("patch_settings", { patch }),
};
```

`src/lib/toast.ts`:

```ts
import { create } from "zustand";
import i18next from "i18next";

export interface Toast {
  id: number;
  kind: "error" | "info";
  text: string;
}

interface ToastStore {
  toasts: Toast[];
  push: (kind: Toast["kind"], text: string) => void;
  dismiss: (id: number) => void;
}

let seq = 0;

export const useToasts = create<ToastStore>((set, get) => ({
  toasts: [],
  push: (kind, text) => {
    const id = ++seq;
    set({ toasts: [...get().toasts, { id, kind, text }] });
    setTimeout(() => get().dismiss(id), 5000);
  },
  dismiss: (id) => set({ toasts: get().toasts.filter((t) => t.id !== id) }),
}));

type T = (key: string, opts?: { defaultValue?: string }) => string;

/** invoke 오류 {code,message}, Error, 문자열 무엇이든 사람이 읽을 한 문장으로. */
export function errorText(e: unknown, t: T = (k, o) => i18next.t(k, o) as string): string {
  if (typeof e === "object" && e !== null && "code" in e) {
    const { code, message } = e as { code: string; message?: string };
    return t(`error.${code}`, { defaultValue: message || t("error.unknown") });
  }
  const msg = e instanceof Error ? e.message : String(e);
  return msg.startsWith("error.") ? t(msg, { defaultValue: msg }) : msg;
}

export const showError = (e: unknown) => useToasts.getState().push("error", errorText(e));
export const showInfo = (text: string) => useToasts.getState().push("info", text);
```

`src/lib/settings.ts`:

```ts
import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { showError } from "./toast";
import type { DeepPartial, Settings } from "./types";

export const defaultSettings: Settings = {
  version: 1,
  general: { ui_language: "system", theme: "system", close_to_tray: true },
  download: {
    dir: "",
    quality: "best",
    preset: "original",
    subtitles: [],
    skip_sheet: false,
    max_concurrent: 2,
    hw_accel: true,
  },
  update: { auto_check: true, last_ytdlp_check: null },
};

function isObj(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function merge<T>(base: T, patch: DeepPartial<T>): T {
  const out: Record<string, unknown> = { ...(base as Record<string, unknown>) };
  for (const [k, v] of Object.entries(patch as Record<string, unknown>)) {
    if (v === undefined) continue;
    out[k] = isObj(v) && isObj(out[k]) ? merge(out[k], v) : v;
  }
  return out as T;
}

export const mergeSettings = (base: Settings, patch: DeepPartial<Settings>): Settings => merge(base, patch);

interface SettingsStore {
  settings: Settings | null;
  load: () => Promise<void>;
  update: (patch: DeepPartial<Settings>) => Promise<void>;
  subscribeBackend: () => () => void;
}

// settings-changed 는 호출자에게도 되돌아온다. 아직 디스크에 닿지 않은 패치는 에코 위에 다시 덮는다.
let pending = 0;
let pendingPatch: DeepPartial<Settings> | null = null;

export const useSettings = create<SettingsStore>((set, get) => ({
  settings: null,
  load: async () => {
    try {
      set({ settings: await api.getSettings() });
    } catch (e) {
      set({ settings: get().settings ?? defaultSettings });
      showError(e);
    }
  },
  update: async (patch) => {
    const prev = get().settings;
    set({ settings: mergeSettings(prev ?? defaultSettings, patch) });
    pending++;
    pendingPatch = pendingPatch ? merge(pendingPatch, patch) : patch;
    try {
      await api.patchSettings(patch);
    } catch (e) {
      set({ settings: prev });
      showError(e);
    } finally {
      pending--;
      if (pending === 0) pendingPatch = null;
    }
  },
  subscribeBackend: () => {
    const p = listen<Settings>("settings-changed", (e) => {
      set({ settings: pendingPatch ? mergeSettings(e.payload, pendingPatch) : e.payload });
    });
    return () => {
      p.then((un) => un());
    };
  },
}));
```

- [ ] **Step 5: 테스트 통과 확인**

Run: `yarn test && cargo test -p kiri-app`
Expected: 모두 통과.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src src/lib src/test
git commit -m "feat(app): settings.json 저장, patch 병합, 설정 스토어와 오류 토스트 추가"
```

---

### Task 13: 엔진 연결 — 명령, 이벤트, 소켓 서버, 도구 설치, sidecar

**Files:**
- Create:
  - `src-tauri/src/bootstrap.rs`, `src-tauri/src/ipc_server.rs`
  - `scripts/fetch-ffmpeg.sh`, `scripts/build-cli.sh`, `scripts/ffmpeg.lock`(생성)
  - `src/lib/queue.ts`, `src/lib/tools.ts`, `src/test/queue.test.ts`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/src/commands.rs`, `src-tauri/tauri.conf.json`, `package.json`, `src/lib/tauri.ts`

**Interfaces:**
- Consumes:
  - `kiri_core::engine::{Engine, EnginePaths, EngineError}`
  - `kiri_core::{ipc, tools, model::*, ytdlp::VideoInfo}`
  - `settings::{engine_config, SettingsState}`
- Produces:
  - Rust:
    - `lib.rs`의 `pub(crate) fn sidecar(name: &str) -> PathBuf`, `fn on_queue_change(app, jobs)`
    - `bootstrap::{ToolsStatus, ToolsState, ensure(&AppHandle, force) (async), spawn(AppHandle), due(last, now) -> bool}`
    - `ipc_server::{IpcState, spawn(&AppHandle, Engine)}`
    - commands `list_jobs`, `probe`, `add_job`, `add_url`, `stop_job`, `remove_job`, `restart_job`, `tools_status`, `update_tools`
    - `impl From<EngineError> for CmdError`
  - TS: `api.{listJobs, probe, addJob, addUrl, stopJob, removeJob, restartJob, toolsStatus, updateTools}`, `useQueue` (`jobs`, `bind`), `hasActive(jobs)`, `isIdle(jobs)`, `useTools` (`status`, `bind`)
  - 이벤트 `queue-changed`(`Job[]`), `tools-changed`(`ToolsStatus`)
  - sidecar: `src-tauri/binaries/{ffmpeg,ffprobe,kiri-cli}-aarch64-apple-darwin`

- [ ] **Step 1: sidecar 스크립트**

`scripts/fetch-ffmpeg.sh`:

```sh
#!/bin/sh
# ffmpeg/ffprobe arm64 정적 빌드(ffmpeg.martin-riedl.de, 서명·공증됨)를 sidecar 위치에 받는다.
#   처음 또는 버전을 올릴 때:  sh scripts/fetch-ffmpeg.sh --update   (scripts/ffmpeg.lock 갱신)
#   평소:                     sh scripts/fetch-ffmpeg.sh            (lock 의 URL·SHA256 그대로)
set -eu
cd "$(dirname "$0")/.."
LOCK=scripts/ffmpeg.lock
OUT=src-tauri/binaries
TRIPLE=aarch64-apple-darwin
BASE=https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release

if [ "${1:-}" = "--update" ]; then
  : > "$LOCK.tmp"
  for tool in ffmpeg ffprobe; do
    url=$(curl -fsSIL -o /dev/null -w '%{url_effective}' "$BASE/$tool.zip")
    sha=$(curl -fsSL "$url" | shasum -a 256 | cut -d' ' -f1)
    echo "$tool $url $sha" >> "$LOCK.tmp"
  done
  mv "$LOCK.tmp" "$LOCK"
fi

[ -f "$LOCK" ] || { echo "missing $LOCK — run: sh scripts/fetch-ffmpeg.sh --update" >&2; exit 1; }
mkdir -p "$OUT"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
while read -r tool url sha; do
  curl -fsSL "$url" -o "$tmp/$tool.zip"
  got=$(shasum -a 256 "$tmp/$tool.zip" | cut -d' ' -f1)
  [ "$got" = "$sha" ] || { echo "$tool: checksum mismatch ($got != $sha)" >&2; exit 1; }
  mkdir -p "$tmp/$tool"
  ditto -x -k "$tmp/$tool.zip" "$tmp/$tool"
  install -m 755 "$tmp/$tool/$tool" "$OUT/$tool-$TRIPLE"
done < "$LOCK"

# 프리셋이 쓰는 인코더가 모두 들어 있는지 확인한다.
encoders=$("$OUT/ffmpeg-$TRIPLE" -hide_banner -encoders)
for enc in libx264 libx265 libvpx-vp9 libmp3lame libopus h264_videotoolbox hevc_videotoolbox prores_videotoolbox prores_ks; do
  echo "$encoders" | grep -q " $enc " || { echo "ffmpeg build lacks encoder: $enc" >&2; exit 1; }
done
echo "ffmpeg sidecars ready in $OUT"
```

`scripts/build-cli.sh`:

```sh
#!/bin/sh
# kiri CLI 를 릴리스로 빌드해 sidecar 위치에 둔다. 앱 번들 안에서는 kiri-cli 라는 이름이 된다.
set -eu
cd "$(dirname "$0")/.."
cargo build -p kiri-cli --release
mkdir -p src-tauri/binaries
install -m 755 target/release/kiri src-tauri/binaries/kiri-cli-aarch64-apple-darwin
```

`package.json`의 `scripts`에 추가한다.

```json
"sidecars": "sh scripts/fetch-ffmpeg.sh && sh scripts/build-cli.sh"
```

Run: `sh scripts/fetch-ffmpeg.sh --update && sh scripts/build-cli.sh`
Expected: `scripts/ffmpeg.lock`이 생기고 마지막에 `ffmpeg sidecars ready in src-tauri/binaries`가 출력된다. `src-tauri/binaries/`에 세 파일이 생긴다. 이 배포처의 redirect URL이 바뀌었다면 https://ffmpeg.martin-riedl.de 의 "latest release" 링크로 `BASE`를 고친다. 인코더 확인이 실패하면 다른 정적 빌드(같은 인코더 목록을 만족하는 것)로 `BASE`를 바꾼다.

`src-tauri/tauri.conf.json`의 `bundle`에 추가한다.

```json
"externalBin": ["binaries/ffmpeg", "binaries/ffprobe", "binaries/kiri-cli"],
```

- [ ] **Step 2: 실패하는 Rust 테스트 작성**

`src-tauri/src/bootstrap.rs`:

```rust
//! yt-dlp·Deno 준비 상태. 시작 시 그리고 한 시간마다 확인하고, yt-dlp 는 하루에 한 번 갱신한다.
use crate::settings::SettingsState;
use kiri_core::{
    engine::{now_ms, Engine},
    tools,
};
use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_after_a_day() {
        assert!(due(None, 0));
        assert!(!due(Some(1_000), 1_000 + DAY_MS - 1));
        assert!(due(Some(1_000), 1_000 + DAY_MS));
    }

    #[test]
    fn status_json_shape() {
        let v = serde_json::to_value(ToolsStatus::default()).unwrap();
        assert_eq!(
            v,
            serde_json::json!({"ready": false, "installing": false, "ytdlp_version": null, "error": null, "last_check": null})
        );
    }
}
```

`commands.rs`의 테스트(파일 끝에 추가):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use kiri_core::engine::EngineError;

    #[test]
    fn engine_errors_keep_code_and_message() {
        let e: CmdError = EngineError::NotFound(4).into();
        assert_eq!((e.code.as_str(), e.message.as_str()), ("not_found", "job 4 not found"));
    }
}
```

`lib.rs`에 `mod bootstrap;`과 `mod ipc_server;`를 추가한다(아래 Step 3의 전체 lib.rs).

Run: `cargo test -p kiri-app`
Expected: 컴파일 에러(`cannot find value DAY_MS`, `From<EngineError>` 없음).

- [ ] **Step 3: Rust 구현**

`bootstrap.rs`의 `#[cfg(test)]` 위에 추가한다.

```rust
const DAY_MS: u64 = 24 * 60 * 60 * 1000;
const RECHECK: Duration = Duration::from_secs(60 * 60);

#[derive(Serialize, Clone, Default, Debug, PartialEq)]
pub struct ToolsStatus {
    pub ready: bool,
    pub installing: bool,
    pub ytdlp_version: Option<String>,
    pub error: Option<String>,
    pub last_check: Option<u64>,
}

#[derive(Default)]
pub struct ToolsState {
    status: Mutex<ToolsStatus>,
    busy: AtomicBool,
}

impl ToolsState {
    pub fn get(&self) -> ToolsStatus {
        self.status.lock().unwrap().clone()
    }
}

pub fn due(last: Option<u64>, now: u64) -> bool {
    last.is_none_or(|l| now.saturating_sub(l) >= DAY_MS)
}

fn update(app: &AppHandle, f: impl FnOnce(&mut ToolsStatus)) {
    let state = app.state::<ToolsState>();
    let snapshot = {
        let mut s = state.status.lock().unwrap();
        f(&mut s);
        s.clone()
    };
    let _ = app.emit("tools-changed", snapshot);
}

/// force: 사용자가 "지금 업데이트" 를 눌렀다. 아니면 yt-dlp 는 하루에 한 번만 확인한다.
pub async fn ensure(app: &AppHandle, force: bool) {
    let state = app.state::<ToolsState>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return;
    }
    let paths = app.state::<Engine>().tools().clone();
    let settings = app.state::<SettingsState>();
    let now = now_ms();
    let need_ytdlp = force || !paths.ytdlp.exists() || due(settings.get().update.last_ytdlp_check, now);

    update(app, |s| {
        s.installing = true;
        s.error = None;
    });
    let client = tools::http_client();
    let mut error = tools::ensure_deno(&client, &paths.deno).await.err();
    if need_ytdlp {
        match tools::ensure_ytdlp(&client, &paths.ytdlp).await {
            Ok(_) => {
                let mut s = settings.get();
                s.update.last_ytdlp_check = Some(now);
                if let Err(e) = settings.set(app, s) {
                    eprintln!("kiri tools: cannot save last check: {e}");
                }
            }
            Err(e) => error = error.or(Some(e)),
        }
    }
    let version = tools::version(&paths.ytdlp).await;
    let last_check = settings.get().update.last_ytdlp_check;
    let ready = paths.ytdlp.exists() && paths.deno.exists();
    update(app, |s| {
        *s = ToolsStatus { ready, installing: false, ytdlp_version: version, error, last_check };
    });
    state.busy.store(false, Ordering::SeqCst);
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            ensure(&app, false).await;
            tokio::time::sleep(RECHECK).await;
        }
    });
}
```

`src-tauri/src/ipc_server.rs`:

```rust
//! CLI 소켓 서버. 실패해도 앱은 계속 동작하고, 이유는 CLI 탭에 보여준다.
use kiri_core::{engine::Engine, ipc};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Default)]
pub struct IpcState(pub Mutex<Option<String>>);

pub fn spawn(app: &AppHandle, engine: Engine) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match ipc::bind(&ipc::socket_path()).await {
            Ok(listener) => ipc::serve(listener, engine).await,
            Err(e) => {
                eprintln!("kiri ipc: {e}");
                *app.state::<IpcState>().0.lock().unwrap() = Some(e.to_string());
            }
        }
    });
}
```

`commands.rs`: import를 바꾸고, `patch_settings`가 엔진 설정도 갱신하게 고친 뒤 작업 명령을 추가한다.

```rust
use crate::{
    bootstrap::{self, ToolsState, ToolsStatus},
    settings::{self, Settings, SettingsState},
};
use kiri_core::{
    engine::{Engine, EngineError},
    model::{Job, NewJob},
    ytdlp::VideoInfo,
};
use serde::Serialize;
use tauri::{AppHandle, State};

// (CmdError, From<String>, CmdResult, get_settings 는 그대로)

impl From<EngineError> for CmdError {
    fn from(e: EngineError) -> Self {
        Self { code: e.code().into(), message: e.to_string() }
    }
}

/// 부분 갱신. 전체 문서를 쓰면 동시 쓰기가 서로를 덮는다.
#[tauri::command]
pub fn patch_settings(
    app: AppHandle,
    state: State<'_, SettingsState>,
    engine: State<'_, Engine>,
    patch: serde_json::Value,
) -> CmdResult<()> {
    let mut merged = serde_json::to_value(state.get()).map_err(|e| e.to_string())?;
    settings::merge(&mut merged, &patch);
    let next: Settings = serde_json::from_value(merged).map_err(|e| e.to_string())?;
    state.set(&app, next.clone())?;
    engine.set_config(settings::engine_config(&next));
    Ok(())
}

#[tauri::command]
pub fn list_jobs(engine: State<'_, Engine>) -> Vec<Job> {
    engine.list()
}

#[tauri::command]
pub async fn probe(engine: State<'_, Engine>, url: String) -> CmdResult<VideoInfo> {
    Ok(engine.probe(&url).await?)
}

#[tauri::command]
pub fn add_job(engine: State<'_, Engine>, job: NewJob) -> Job {
    engine.add(job)
}

/// 시트 없이 설정 기본값으로 바로 추가.
#[tauri::command]
pub async fn add_url(engine: State<'_, Engine>, url: String) -> CmdResult<Job> {
    Ok(engine.add_url(&url, None, None, None).await?)
}

#[tauri::command]
pub fn stop_job(engine: State<'_, Engine>, id: u64) -> CmdResult<()> {
    Ok(engine.stop(id)?)
}

#[tauri::command]
pub fn remove_job(engine: State<'_, Engine>, id: u64) -> CmdResult<()> {
    Ok(engine.remove(id)?)
}

#[tauri::command]
pub fn restart_job(engine: State<'_, Engine>, id: u64) -> CmdResult<()> {
    Ok(engine.restart(id)?)
}

#[tauri::command]
pub fn tools_status(state: State<'_, ToolsState>) -> ToolsStatus {
    state.get()
}

#[tauri::command]
pub async fn update_tools(app: AppHandle) {
    bootstrap::ensure(&app, true).await;
}
```

`Serialize` import는 `CmdError`가 쓴다.

`src-tauri/src/lib.rs` 전체:

```rust
mod bootstrap;
mod commands;
mod ipc_server;
mod settings;
mod windows;

use kiri_core::{
    engine::{Engine, EnginePaths},
    model::{Job, Tools},
};
use settings::SettingsState;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

/// externalBin 은 실행 파일 옆(Contents/MacOS, dev 에서는 target/debug)에 놓인다.
pub(crate) fn sidecar(name: &str) -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(name)))
        .unwrap_or_else(|| PathBuf::from(name))
}

fn on_queue_change(app: &AppHandle, jobs: &[Job]) {
    let _ = app.emit("queue-changed", jobs);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let path = app.path();
            app.manage(SettingsState::new(path.app_config_dir()?.join("settings.json")));
            app.manage(bootstrap::ToolsState::default());
            app.manage(ipc_server::IpcState::default());

            let bin = path.app_data_dir()?.join("bin");
            let tools = Tools { ytdlp: bin.join("yt-dlp"), deno: bin.join("deno"), ffmpeg: sidecar("ffmpeg") };
            let paths = EnginePaths {
                queue_file: path.app_local_data_dir()?.join("queue.json"),
                cache_dir: path.app_cache_dir()?,
                log_dir: path.app_log_dir()?.join("jobs"),
            };
            let config = settings::engine_config(&app.state::<SettingsState>().get());
            let rt = tauri::async_runtime::block_on(async { tokio::runtime::Handle::current() });
            let h = handle.clone();
            let engine = Engine::new(paths, tools, config, rt, move |jobs| on_queue_change(&h, jobs));
            app.manage(engine.clone());

            windows::show_main(&handle).map_err(std::io::Error::other)?;
            engine.start();
            ipc_server::spawn(&handle, engine);
            bootstrap::spawn(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::patch_settings,
            commands::list_jobs,
            commands::probe,
            commands::add_job,
            commands::add_url,
            commands::stop_job,
            commands::remove_job,
            commands::restart_job,
            commands::tools_status,
            commands::update_tools,
        ])
        .run(tauri::generate_context!())
        .expect("error while running kiri");
}
```

Run: `cargo test -p kiri-app`
Expected: settings 5개, bootstrap 2개, commands 1개가 통과한다.

- [ ] **Step 4: 실패하는 TS 테스트 작성**

`src/test/queue.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { hasActive, isIdle } from "../lib/queue";
import type { Job, JobState } from "../lib/types";

const job = (state: JobState): Job => ({
  id: 1, url: "u", title: "t", thumbnail: null, duration_secs: null, quality_label: "720p",
  options: { format_id: null, preset: "original", subtitles: [], auto_subtitles: false },
  state, progress: 0, speed: null, eta: null, output: null, created_at: 0,
});

describe("queue helpers", () => {
  it("hasActive only counts running jobs", () => {
    expect(hasActive([job({ kind: "queued" })])).toBe(false);
    expect(hasActive([job({ kind: "encoding" })])).toBe(true);
  });
  it("isIdle is false while anything is queued or running", () => {
    expect(isIdle([job({ kind: "completed" }), job({ kind: "failed", message: "x" })])).toBe(true);
    expect(isIdle([job({ kind: "queued" })])).toBe(false);
    expect(isIdle([])).toBe(true);
  });
});
```

Run: `yarn test`
Expected: FAIL(`../lib/queue` 없음).

- [ ] **Step 5: TS 구현**

`src/lib/tauri.ts` 전체:

```ts
import { invoke } from "@tauri-apps/api/core";
import type { DeepPartial, Job, NewJob, Settings, ToolsStatus, VideoInfo } from "./types";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  patchSettings: (patch: DeepPartial<Settings>) => invoke<void>("patch_settings", { patch }),
  listJobs: () => invoke<Job[]>("list_jobs"),
  probe: (url: string) => invoke<VideoInfo>("probe", { url }),
  addJob: (job: NewJob) => invoke<Job>("add_job", { job }),
  addUrl: (url: string) => invoke<Job>("add_url", { url }),
  stopJob: (id: number) => invoke<void>("stop_job", { id }),
  removeJob: (id: number) => invoke<void>("remove_job", { id }),
  restartJob: (id: number) => invoke<void>("restart_job", { id }),
  toolsStatus: () => invoke<ToolsStatus>("tools_status"),
  updateTools: () => invoke<void>("update_tools"),
};
```

`src/lib/queue.ts`:

```ts
import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { showError } from "./toast";
import type { Job } from "./types";

export const hasActive = (jobs: Job[]) => jobs.some((j) => j.state.kind === "downloading" || j.state.kind === "encoding");
export const isIdle = (jobs: Job[]) => !jobs.some((j) => ["queued", "downloading", "encoding"].includes(j.state.kind));

interface QueueStore {
  jobs: Job[];
  bind: () => () => void;
}

export const useQueue = create<QueueStore>((set) => ({
  jobs: [],
  bind: () => {
    api.listJobs().then((jobs) => set({ jobs })).catch(showError);
    const p = listen<Job[]>("queue-changed", (e) => set({ jobs: e.payload }));
    return () => {
      p.then((un) => un());
    };
  },
}));
```

`src/lib/tools.ts`:

```ts
import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import type { ToolsStatus } from "./types";

interface ToolsStore {
  status: ToolsStatus | null;
  bind: () => () => void;
}

export const useTools = create<ToolsStore>((set) => ({
  status: null,
  bind: () => {
    // 이벤트가 invoke 보다 먼저 올 수 있다. 비어 있을 때만 채운다.
    api.toolsStatus().then((s) => set((cur) => ({ status: cur.status ?? s }))).catch(() => {});
    const p = listen<ToolsStatus>("tools-changed", (e) => set({ status: e.payload }));
    return () => {
      p.then((un) => un());
    };
  },
}));
```

Run: `yarn test`
Expected: 통과.

- [ ] **Step 6: 실제 실행 확인 (수동)**

Run: `yarn tauri dev`

확인할 것:
1. 창이 뜬다.
2. 수십 초 안에 `~/Library/Application Support/org.bobpark.kiri/bin/`에 `yt-dlp`와 `deno`가 생긴다.
3. 다른 터미널에서 `./target/release/kiri list`를 실행하면 `no jobs`가 나온다.
4. `./target/release/kiri add "https://www.youtube.com/watch?v=jNQXAC9IVRw" --format mp4-h264`를 실행하면 `added #1: Me at the zoo`가 나온다.
5. 잠시 뒤 `kiri list`에서 `completed`가 보이고, `~/Movies/kiri/Me at the zoo.mp4`가 재생된다.
6. 실행 로그에 `--js-runtimes` 관련 경고가 없다.

- [ ] **Step 7: Commit**

```bash
git add scripts package.json src-tauri src/lib src/test
git commit -m "feat(app): 엔진·소켓 서버·yt-dlp/Deno 설치 연결과 ffmpeg/CLI sidecar 추가"
```

---
### Task 14: 옵션 시트 (`OptionsSheet`)

**Files:**
- Create: `src/lib/sheet.ts`, `src/lib/format.ts`, `src/components/OptionsSheet.tsx`, `src/test/sheet.test.ts`, `src/test/format.test.ts`

**Interfaces:**
- Consumes:
  - `api.addJob`, `useSettings`, `showError`, `langName`
  - 타입 `Quality`, `VideoInfo`, `Preset`, `PRESETS`, `AUDIO_PRESETS`, `NewJob`, `DeepPartial<Settings>`
- Produces:
  - `pickDefaultQuality(qualities, want) -> Quality | null`
  - `defaultSubtitles(info, langs) -> string[]`
  - `buildNewJob(url, info, quality, preset, subs) -> NewJob`
  - `rememberPatch(quality, preset, subs) -> DeepPartial<Settings>`
  - `formatBytes(n)`, `formatDuration(secs)`, `pct(p)`, `failureText(message, t)`, `jobDetail(job, t)` (`src/lib/format.ts`)
  - `<OptionsSheet url={string} info={VideoInfo | null} onClose={() => void} />`

- [ ] **Step 1: 실패하는 테스트 작성**

`src/test/sheet.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { buildNewJob, defaultSubtitles, pickDefaultQuality, rememberPatch } from "../lib/sheet";
import type { Quality, VideoInfo } from "../lib/types";

const q = (format_id: string, height: number, label: string): Quality => ({ format_id, height, fps: null, vcodec: "AVC", filesize: null, label });
const qualities = [q("299", 1080, "1080p60"), q("136", 720, "720p"), q("18", 360, "360p")];
const info: VideoInfo = {
  id: "abc", title: "Rust", channel: "Fireship", duration_secs: 144, thumbnail: "th",
  qualities, subtitles: ["en", "ko"], auto_subtitles: ["en", "ja", "ko"],
};

describe("pickDefaultQuality", () => {
  it("mirrors the backend resolution rules", () => {
    expect(pickDefaultQuality(qualities, "best")?.format_id).toBe("299");
    expect(pickDefaultQuality(qualities, "720p")?.format_id).toBe("136");
    expect(pickDefaultQuality(qualities, "480p")?.format_id).toBe("18");
    expect(pickDefaultQuality(qualities, "144p")?.format_id).toBe("18");
    expect(pickDefaultQuality(qualities, "audio")).toBeNull();
    expect(pickDefaultQuality([], "best")).toBeNull();
  });
});

describe("subtitles", () => {
  it("keeps only languages the video has", () => {
    expect(defaultSubtitles(info, ["ko", "ja", "fr"])).toEqual(["ko", "ja"]);
  });
});

describe("buildNewJob", () => {
  it("video job marks auto subs", () => {
    const j = buildNewJob("u", info, qualities[1], "mp4-h264", ["ko", "ja"]);
    expect(j).toEqual({
      url: "u", title: "Rust", thumbnail: "th", duration_secs: 144, quality_label: "720p",
      options: { format_id: "136", preset: "mp4-h264", subtitles: ["ko", "ja"], auto_subtitles: true },
    });
  });
  it("audio preset or audio quality drops the video format", () => {
    expect(buildNewJob("u", info, qualities[0], "mp3", []).options.format_id).toBeNull();
    expect(buildNewJob("u", info, null, "original", []).quality_label).toBe("audio");
  });
});

describe("rememberPatch", () => {
  it("stores height-based quality and enables skip_sheet", () => {
    expect(rememberPatch(qualities[0], "mp4-hevc", ["ko"])).toEqual({
      download: { quality: "1080p", preset: "mp4-hevc", subtitles: ["ko"], skip_sheet: true },
    });
    expect(rememberPatch(null, "mp3", []).download?.quality).toBe("audio");
  });
});
```

`src/test/format.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { failureText, formatBytes, formatDuration, jobDetail } from "../lib/format";
import type { Job, JobState } from "../lib/types";

const t = (k: string, o?: Record<string, unknown>) => (o && "langs" in o ? `Subs ${o.langs}` : k);
const job = (state: JobState, extra: Partial<Job> = {}): Job => ({
  id: 1, url: "u", title: "t", thumbnail: null, duration_secs: null, quality_label: "1080p60",
  options: { format_id: "299", preset: "mp4-h264", subtitles: ["ko"], auto_subtitles: false },
  state, progress: 0.48, speed: "2.1MiB/s", eta: "00:12", output: null, created_at: 0, ...extra,
});

describe("format", () => {
  it("formats bytes and durations", () => {
    expect(formatBytes(null)).toBe("");
    expect(formatBytes(500)).toBe("500 B");
    expect(formatBytes(88_000_000)).toBe("83.9 MB");
    expect(formatBytes(3 * 1024 ** 3)).toBe("3.0 GB");
    expect(formatDuration(144)).toBe("2:24");
    expect(formatDuration(3725)).toBe("1:02:05");
    expect(formatDuration(null)).toBe("");
  });
  it("describes a downloading job", () => {
    expect(jobDetail(job({ kind: "downloading" }), t)).toBe("1080p60 · preset.mp4-h264 · Subs ko · 48% · 2.1MiB/s · 00:12");
  });
  it("describes audio, encoding and failure", () => {
    expect(jobDetail(job({ kind: "encoding" }, { quality_label: "audio", options: { format_id: null, preset: "mp3", subtitles: [], auto_subtitles: false } }), t))
      .toBe("quality.audio · preset.mp3 · state.encoding 48%");
    expect(jobDetail(job({ kind: "failed", message: "ERROR: private" }), t)).toBe("1080p60 · preset.mp4-h264 · Subs ko · ERROR: private");
    expect(failureText("error.no_output", t)).toBe("error.no_output");
  });
});
```

Run: `yarn test`
Expected: FAIL(`../lib/sheet`, `../lib/format` 없음).

- [ ] **Step 2: 구현**

`src/lib/sheet.ts`:

```ts
import type { DeepPartial, NewJob, Preset, Quality, Settings, VideoInfo } from "./types";
import { AUDIO_PRESETS } from "./types";

/** Rust ytdlp::resolve_quality 와 같은 규칙. null 은 오디오만. */
export function pickDefaultQuality(qualities: Quality[], want: string): Quality | null {
  if (want === "audio") return null;
  if (qualities.length === 0) return null;
  const h = parseInt(want, 10);
  if (want === "best" || Number.isNaN(h)) return qualities[0];
  return qualities.find((q) => q.height <= h) ?? qualities[qualities.length - 1];
}

export function defaultSubtitles(info: VideoInfo, langs: string[]): string[] {
  return langs.filter((l) => info.subtitles.includes(l) || info.auto_subtitles.includes(l));
}

export function buildNewJob(url: string, info: VideoInfo, quality: Quality | null, preset: Preset, subs: string[]): NewJob {
  const video = quality !== null && !AUDIO_PRESETS.includes(preset) ? quality : null;
  return {
    url,
    title: info.title,
    thumbnail: info.thumbnail,
    duration_secs: info.duration_secs,
    quality_label: video ? video.label : "audio",
    options: {
      format_id: video ? video.format_id : null,
      preset,
      subtitles: subs,
      auto_subtitles: subs.some((s) => !info.subtitles.includes(s)),
    },
  };
}

/** "다음부터 이 설정으로 바로 시작" */
export function rememberPatch(quality: Quality | null, preset: Preset, subs: string[]): DeepPartial<Settings> {
  return { download: { quality: quality ? `${quality.height}p` : "audio", preset, subtitles: subs, skip_sheet: true } };
}
```

`src/lib/format.ts`:

```ts
import type { Job } from "./types";

type T = (key: string, opts?: Record<string, unknown>) => string;

export function formatBytes(n: number | null | undefined): string {
  if (!n) return "";
  const units = ["B", "KB", "MB", "GB"];
  let v = n;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return i === 0 ? `${v} B` : `${v.toFixed(1)} ${units[i]}`;
}

export function formatDuration(secs: number | null | undefined): string {
  if (secs == null) return "";
  const s = Math.round(secs);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = String(s % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${r}` : `${m}:${r}`;
}

export const pct = (p: number) => `${Math.round(p * 100)}%`;

/** "error." 으로 시작하면 i18n 키, 아니면 도구 원문. */
export function failureText(message: string, t: T): string {
  return message.startsWith("error.") ? t(message, { defaultValue: message }) : message;
}

/** 행의 둘째 줄: 화질 · 포맷 · 자막 · 상태별 정보 */
export function jobDetail(job: Job, t: T): string {
  const parts = [job.quality_label === "audio" ? t("quality.audio") : job.quality_label, t(`preset.${job.options.preset}`)];
  if (job.options.subtitles.length) parts.push(t("job.subs", { langs: job.options.subtitles.join(", ") }));
  const s = job.state;
  if (s.kind === "downloading") parts.push([pct(job.progress), job.speed, job.eta].filter(Boolean).join(" · "));
  else if (s.kind === "encoding") parts.push(`${t("state.encoding")} ${pct(job.progress)}`);
  else if (s.kind === "failed") parts.push(failureText(s.message, t));
  return parts.join(" · ");
}
```

`format.test.ts`의 가짜 `t`는 `defaultValue`를 무시하고 키를 그대로 돌려준다. 그래서 `failureText("error.no_output", t)`는 키를 그대로 돌려주고, 테스트는 이 동작을 고정한다.

- [ ] **Step 3: 테스트 통과 확인**

Run: `yarn test`
Expected: 통과.

- [ ] **Step 4: 시트 컴포넌트**

`src/components/OptionsSheet.tsx`:

```tsx
import { useEffect, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { api } from "../lib/tauri";
import { useSettings } from "../lib/settings";
import { showError } from "../lib/toast";
import { langName } from "../lib/i18n";
import { formatBytes, formatDuration } from "../lib/format";
import { buildNewJob, defaultSubtitles, pickDefaultQuality, rememberPatch } from "../lib/sheet";
import { PRESETS, type Preset, type Quality, type VideoInfo } from "../lib/types";

interface Props {
  url: string;
  info: VideoInfo | null; // null = probe 중
  onClose: () => void;
}

function Section({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="mt-3">
      <div className="mb-1 text-[10px] font-semibold uppercase tracking-wide text-fg-muted">{label}</div>
      {children}
    </div>
  );
}

export function OptionsSheet({ url, info, onClose }: Props) {
  const { t, i18n } = useTranslation();
  const defaults = useSettings((s) => s.settings!.download);
  const [quality, setQuality] = useState<Quality | null>(null);
  const [preset, setPreset] = useState<Preset>(defaults.preset);
  const [subs, setSubs] = useState<string[]>([]);
  const [remember, setRemember] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!info) return;
    setQuality(pickDefaultQuality(info.qualities, defaults.quality));
    setSubs(defaultSubtitles(info, defaults.subtitles));
  }, [info]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const toggleSub = (l: string) => setSubs((s) => (s.includes(l) ? s.filter((x) => x !== l) : [...s, l]));
  const autoOnly = info ? info.auto_subtitles.filter((l) => !info.subtitles.includes(l)) : [];

  const confirm = async () => {
    if (!info) return;
    setBusy(true);
    try {
      await api.addJob(buildNewJob(url, info, quality, preset, subs));
      if (remember) await useSettings.getState().update(rememberPatch(quality, preset, subs));
      onClose();
    } catch (e) {
      showError(e);
    } finally {
      setBusy(false);
    }
  };

  const qualityRow = (q: Quality | null) => {
    const on = (q?.format_id ?? null) === (quality?.format_id ?? null);
    return (
      <label
        key={q?.format_id ?? "audio"}
        className={`mb-1 flex cursor-pointer items-center justify-between rounded-lg border px-3 py-1.5 text-sm ${
          on ? "border-primary bg-secondary font-semibold text-secondary-content" : "border-base-300"
        }`}
      >
        <span>
          <input type="radio" name="quality" className="sr-only" checked={on} onChange={() => setQuality(q)} />
          {q ? `${q.label} · ${q.vcodec}` : t("quality.audio")}
        </span>
        <span className="text-xs text-fg-muted">{q ? formatBytes(q.filesize) : ""}</span>
      </label>
    );
  };

  const subChip = (l: string) => (
    <label key={l} className="mr-1 mb-1 inline-flex cursor-pointer items-center gap-1 rounded-lg border border-base-300 px-2 py-0.5 text-xs">
      <input type="checkbox" className="checkbox checkbox-xs checkbox-primary" checked={subs.includes(l)} onChange={() => toggleSub(l)} />
      {langName(l, i18n.language)}
    </label>
  );

  return (
    <div className="fixed inset-0 z-40 flex justify-center bg-black/30" onClick={onClose}>
      <div
        role="dialog"
        aria-modal="true"
        aria-label={info?.title ?? t("sheet.loading")}
        className="h-fit max-h-[88vh] w-[min(520px,94vw)] overflow-y-auto rounded-b-2xl bg-base-100 p-4 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        {!info ? (
          <div className="flex items-center gap-3 py-6 text-sm">
            <span className="loading loading-spinner loading-sm" />
            {t("sheet.loading")}
          </div>
        ) : (
          <>
            <div className="flex items-center gap-3">
              {info.thumbnail && <img src={info.thumbnail} alt="" className="h-12 w-20 rounded-md object-cover" />}
              <div className="min-w-0">
                <div className="truncate font-semibold">{info.title}</div>
                <div className="text-xs text-fg-muted">{[info.channel, formatDuration(info.duration_secs)].filter(Boolean).join(" · ")}</div>
              </div>
            </div>

            <Section label={t("sheet.quality")}>
              {info.qualities.map(qualityRow)}
              {qualityRow(null)}
            </Section>

            <Section label={t("sheet.format")}>
              <select className="select select-sm w-full" value={preset} onChange={(e) => setPreset(e.target.value as Preset)}>
                {PRESETS.map((p) => (
                  <option key={p} value={p}>{t(`preset.${p}`)}</option>
                ))}
              </select>
            </Section>

            <Section label={t("sheet.subtitles")}>
              {info.subtitles.length === 0 && autoOnly.length === 0 && <div className="text-xs text-fg-muted">{t("sheet.noSubs")}</div>}
              <div>{info.subtitles.map(subChip)}</div>
              {autoOnly.length > 0 && (
                <details className="mt-1">
                  <summary className="cursor-pointer text-xs text-fg-muted">{t("sheet.autoSubs")} ({autoOnly.length})</summary>
                  <div className="mt-1 max-h-32 overflow-y-auto">{autoOnly.map(subChip)}</div>
                </details>
              )}
            </Section>

            <label className="mt-3 flex items-center gap-2 text-sm">
              <input type="checkbox" className="checkbox checkbox-sm checkbox-primary" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
              {t("sheet.remember")}
            </label>

            <div className="mt-4 flex justify-end gap-2">
              <button className="btn btn-outline btn-sm" onClick={onClose}>{t("sheet.cancel")}</button>
              <button className="btn btn-primary btn-sm" onClick={confirm} disabled={busy}>{t("sheet.download")}</button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
```

Run: `yarn build`
Expected: 타입 체크와 빌드가 성공한다. 시트는 아직 어디에도 연결되어 있지 않으며, Task 15에서 연결한다.

- [ ] **Step 5: Commit**

```bash
git add src/lib src/components src/test
git commit -m "feat(ui): 화질·포맷·자막 옵션 시트와 표시 포맷 유틸 추가"
```

---

### Task 15: 메인 창 — 큐 리스트, ⌘V/드롭, 토스트

**Files:**
- Create:
  - `src/lib/paste.ts`, `src/test/paste.test.ts`
  - `src/components/JobRow.tsx`, `src/components/Toasts.tsx`
  - `src/pages/MainWindow.tsx`
- Modify: `src/main.tsx` (전체 교체)

**Interfaces:**
- Consumes:
  - `useQueue`, `useTools`, `useSettings`, `api.*`, `showError`
  - `OptionsSheet`, `jobDetail`
  - `readText` (`@tauri-apps/plugin-clipboard-manager`), `revealItemInDir` (`@tauri-apps/plugin-opener`)
- Produces:
  - `extractUrl(text) -> string | null`, `isEditableTarget(target) -> boolean`
  - `<MainWindow />`, `<JobRow job />`, `<Toasts />`
  - 창 label로 페이지를 고르는 `main.tsx`

- [ ] **Step 1: 실패하는 테스트 작성**

`src/test/paste.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { extractUrl } from "../lib/paste";

describe("extractUrl", () => {
  it("finds the first http(s) url", () => {
    expect(extractUrl("https://youtu.be/x")).toBe("https://youtu.be/x");
    expect(extractUrl("  watch this https://www.youtube.com/watch?v=a then")).toBe("https://www.youtube.com/watch?v=a");
    expect(extractUrl("# comment\nhttps://youtu.be/y\n")).toBe("https://youtu.be/y"); // text/uri-list
  });
  it("returns null without a url", () => {
    expect(extractUrl("hello")).toBeNull();
    expect(extractUrl("")).toBeNull();
    expect(extractUrl(null)).toBeNull();
  });
});
```

Run: `yarn test`
Expected: FAIL(`../lib/paste` 없음).

- [ ] **Step 2: 구현**

`src/lib/paste.ts`:

```ts
/** 붙여넣거나 끌어다 놓은 텍스트에서 첫 http(s) URL. YouTube 여부는 백엔드가 판정한다. */
export function extractUrl(text: string | null | undefined): string | null {
  const m = text?.match(/https?:\/\/\S+/);
  return m ? m[0] : null;
}

export function isEditableTarget(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  return !!el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT" || el.isContentEditable);
}
```

Run: `yarn test`
Expected: 통과.

- [ ] **Step 3: 컴포넌트**

`src/components/Toasts.tsx`:

```tsx
import { useToasts } from "../lib/toast";

export function Toasts() {
  const { toasts, dismiss } = useToasts();
  return (
    <div className="toast toast-end toast-bottom z-50">
      {toasts.map((x) => (
        <div
          key={x.id}
          role="alert"
          className={`alert ${x.kind === "error" ? "alert-error" : "alert-info"} cursor-pointer py-2 text-sm`}
          onClick={() => dismiss(x.id)}
        >
          {x.text}
        </div>
      ))}
    </div>
  );
}
```

`src/components/JobRow.tsx`:

```tsx
import { useTranslation } from "react-i18next";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { api } from "../lib/tauri";
import { showError } from "../lib/toast";
import { jobDetail } from "../lib/format";
import type { Job } from "../lib/types";

export function JobRow({ job }: { job: Job }) {
  const { t } = useTranslation();
  const kind = job.state.kind;
  const running = kind === "downloading" || kind === "encoding";
  const act = (f: () => Promise<unknown>) => () => {
    f().catch(showError);
  };
  const badge = kind === "completed" ? "badge-success" : kind === "failed" ? "badge-error" : "badge-ghost";

  return (
    <div className="flex items-center gap-3 border-b border-base-300 px-4 py-2.5">
      {job.thumbnail ? (
        <img src={job.thumbnail} alt="" className="h-9 w-16 shrink-0 rounded-md object-cover" />
      ) : (
        <div className="h-9 w-16 shrink-0 rounded-md bg-base-300" />
      )}
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-semibold">{job.title}</div>
        <div className={`truncate text-xs ${kind === "failed" ? "text-error" : "text-fg-muted"}`}>{jobDetail(job, t)}</div>
        {running && (
          <progress
            className={`progress mt-1 h-1 w-full ${kind === "encoding" ? "progress-info" : "progress-primary"}`}
            value={job.progress * 100}
            max={100}
          />
        )}
      </div>
      {!running && <span className={`badge badge-sm ${badge}`}>{t(`state.${kind}`)}</span>}
      {(running || kind === "queued") && (
        <button className="btn btn-ghost btn-xs" aria-label={t("job.stop")} title={t("job.stop")} onClick={act(() => api.stopJob(job.id))}>⏸</button>
      )}
      {(kind === "stopped" || kind === "failed") && (
        <button className="btn btn-ghost btn-xs" aria-label={t("job.restart")} title={t("job.restart")} onClick={act(() => api.restartJob(job.id))}>↻</button>
      )}
      {job.state.kind === "failed" && job.state.message === "error.download_dir_unwritable" && (
        <button className="btn btn-ghost btn-xs" aria-label={t("app.settings")} title={t("app.settings")} onClick={act(() => api.openSettings())}>⚙</button>
      )}
      {kind === "completed" && job.output && (
        <button className="btn btn-ghost btn-xs" aria-label={t("job.reveal")} title={t("job.reveal")} onClick={act(() => revealItemInDir(job.output!))}>⌕</button>
      )}
      <button className="btn btn-ghost btn-xs" aria-label={t("job.remove")} title={t("job.remove")} onClick={act(() => api.removeJob(job.id))}>✕</button>
    </div>
  );
}
```

`src/pages/MainWindow.tsx`:

```tsx
import { useCallback, useEffect, useState, type DragEvent } from "react";
import { useTranslation } from "react-i18next";
import { readText } from "@tauri-apps/plugin-clipboard-manager";
import { api } from "../lib/tauri";
import { useQueue } from "../lib/queue";
import { useTools } from "../lib/tools";
import { useSettings } from "../lib/settings";
import { showError } from "../lib/toast";
import { extractUrl, isEditableTarget } from "../lib/paste";
import { JobRow } from "../components/JobRow";
import { OptionsSheet } from "../components/OptionsSheet";
import type { VideoInfo } from "../lib/types";

interface SheetState {
  url: string;
  info: VideoInfo | null;
}

function ToolsNotice() {
  const { t } = useTranslation();
  const status = useTools((s) => s.status);
  if (status?.ready) return null;
  const failed = status?.error && !status.installing;
  return (
    <div role="status" className={`mx-3 mt-2 flex items-center gap-2 rounded-xl px-3 py-2 text-sm ${failed ? "bg-error/15 text-error" : "bg-secondary text-secondary-content"}`}>
      {!failed && <span className="loading loading-spinner loading-xs" />}
      <span className="flex-1">{failed ? t("app.toolsFailed", { error: status!.error }) : t("app.toolsPreparing")}</span>
      {failed && (
        <button className="btn btn-xs" onClick={() => api.updateTools().catch(showError)}>{t("app.retry")}</button>
      )}
    </div>
  );
}

export default function MainWindow() {
  const { t } = useTranslation();
  const jobs = useQueue((s) => s.jobs);
  const [sheet, setSheet] = useState<SheetState | null>(null);

  const submit = useCallback(async (raw: string | null | undefined) => {
    const url = extractUrl(raw);
    if (!url) return showError({ code: "invalid_url" });
    if (!useTools.getState().status?.ready) return showError({ code: "ytdlp_missing" });
    if (useSettings.getState().settings?.download.skip_sheet) {
      return api.addUrl(url).then(() => undefined, showError);
    }
    setSheet({ url, info: null });
    try {
      const info = await api.probe(url);
      setSheet((cur) => (cur?.url === url ? { url, info } : cur));
    } catch (e) {
      setSheet(null);
      showError(e);
    }
  }, []);

  useEffect(() => {
    const onKey = async (e: KeyboardEvent) => {
      if (!e.metaKey) return;
      if (e.key === ",") {
        e.preventDefault();
        api.openSettings().catch(showError);
      } else if (e.key.toLowerCase() === "v" && !isEditableTarget(e.target)) {
        e.preventDefault();
        submit(await readText().catch(() => null));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [submit]);

  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    submit(e.dataTransfer.getData("text/uri-list") || e.dataTransfer.getData("text/plain"));
  };

  return (
    <div className="flex h-full flex-col" onDragOver={(e) => e.preventDefault()} onDrop={onDrop}>
      <header className="flex items-center gap-2 border-b border-base-300 bg-base-200 px-4 py-2">
        <span className="font-bold tracking-tight">kiri</span>
        <span className="flex-1 truncate text-center text-xs text-fg-muted">{t("app.dropHint")}</span>
        <button className="btn btn-ghost btn-sm" aria-label={t("app.settings")} title={t("app.settings")} onClick={() => api.openSettings().catch(showError)}>⚙</button>
      </header>
      <ToolsNotice />
      <main className="flex-1 overflow-y-auto">
        {jobs.length === 0 ? (
          <div className="grid h-full place-items-center text-sm text-fg-muted">{t("app.empty")}</div>
        ) : (
          [...jobs].reverse().map((j) => <JobRow key={j.id} job={j} />)
        )}
      </main>
      {sheet && <OptionsSheet url={sheet.url} info={sheet.info} onClose={() => setSheet(null)} />}
    </div>
  );
}
```

`api.openSettings`는 Task 18에서 Rust 명령과 함께 추가한다. 지금 타입 체크가 통과하도록 `src/lib/tauri.ts`의 `api`에 미리 한 줄을 추가한다.

```ts
  openSettings: () => invoke<void>("open_settings"),
```

Task 18 전에 ⌘,를 누르면 "command open_settings not found" 토스트가 뜨는데, 이는 정상이다.

`src/main.tsx` 전체 교체:

```tsx
import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./index.css";
import { useSettings } from "./lib/settings";
import { useQueue } from "./lib/queue";
import { useTools } from "./lib/tools";
import { applyTheme } from "./lib/theme";
import { initI18n, resolveLang } from "./lib/i18n";
import { Toasts } from "./components/Toasts";

const MainWindow = React.lazy(() => import("./pages/MainWindow"));
const label = getCurrentWindow().label;

function Root() {
  const { settings, load, subscribeBackend } = useSettings();
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const unsubs = [subscribeBackend(), useQueue.getState().bind(), useTools.getState().bind()];
    load().then(() => setReady(true));
    return () => unsubs.forEach((u) => u());
  }, []);

  useEffect(() => {
    if (!settings) return;
    applyTheme(settings.general.theme);
    initI18n(resolveLang(settings.general.ui_language, navigator.language));
  }, [settings?.general.theme, settings?.general.ui_language]);

  if (!ready || !settings) return null;
  return (
    <React.Suspense fallback={null}>
      {label === "main" && <MainWindow />}
      <Toasts />
    </React.Suspense>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
```

- [ ] **Step 4: 빌드와 수동 확인**

Run: `yarn test && yarn build`
Expected: 통과.

Run: `yarn tauri dev`

확인할 것:
1. 빈 창에 "아직 다운로드가 없습니다"가 보인다(시스템 언어가 한국어일 때).
2. `hello`를 복사하고 창에서 ⌘V를 누르면 "YouTube 링크가 아닙니다" 토스트가 뜨고, 작업은 생기지 않는다.
3. YouTube URL을 복사하고 ⌘V를 누르면 시트에 로딩 표시가 나온 뒤 화질 목록이 뜬다. 다운로드를 누르면 행이 생기고 진행 바가 움직이다가 완료 배지로 바뀐다. ⌕ 버튼을 누르면 Finder가 열린다.
4. 브라우저 주소창의 URL을 창에 드래그해도 같은 시트가 뜬다.
5. 다운로드 중 ⏸를 누르면 "중지됨"이 되고, ↻를 누르면 이어받는다. ✕를 누르면 행이 사라진다.
6. ⌘V가 JS keydown에 도달하지 않는다면(시트가 안 뜸) 이 사실을 기록하고, 해결책으로 Tauri 메뉴에 Paste 대신 `Edit` 메뉴 커스텀 항목을 다는 것을 계획 수정 사항으로 올린다.

- [ ] **Step 5: Commit**

```bash
git add src
git commit -m "feat(ui): 메인 창 큐 리스트, ⌘V·드래그 추가, 오류 토스트 추가"
```

---

### Task 16: 메뉴 막대(트레이), 창 닫기 → 숨기기, 종료 확인

**Files:**
- Create: `src-tauri/src/i18n.rs`, `src-tauri/src/tray.rs`, `src-tauri/src/quit.rs`
- Modify: `src-tauri/src/windows.rs`, `src-tauri/src/settings.rs` (`set`에서 트레이 라벨 갱신), `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `Engine::{list, has_active}`, `SettingsState`, `tauri_plugin_dialog`
- Produces:
  - `i18n::{Lang, resolve(pref), resolve_with(pref, system), labels(lang) -> Labels}`
  - `Labels { open, quit, check_update, install_update, idle, active, settings_title, quit_title, quit_message, quit_ok, quit_cancel }`
  - `tray::{build, relabel, relabel_queue, summary, DoubleClick}`
  - `quit::{allow_exit, needs_confirm, on_run_event}`
  - `windows::{show_main, set_dock_visible}`

**왜 직접 더블클릭을 판정하나:** Tauri는 macOS에서 `TrayIconEvent::DoubleClick`을 내보내지 않는다(babelay `tray.rs` 주석 참고). 그래서 `show_menu_on_left_click(false)`로 왼쪽 클릭이 메뉴를 열지 않게 하고, 왼쪽 클릭 Up 두 번이 400ms 안에 오면 창을 연다. 메뉴는 우클릭(또는 control-클릭)으로 연다.

- [ ] **Step 1: 실패하는 테스트 작성**

`src-tauri/src/i18n.rs`:

```rust
//! Rust 쪽에서 직접 그리는 문구(메뉴 막대, 네이티브 대화상자, 창 제목).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_language() {
        assert_eq!(resolve_with("ja", Some("ko-KR")), Lang::Ja);
        assert_eq!(resolve_with("system", Some("ko-KR")), Lang::Ko);
        assert_eq!(resolve_with("system", Some("de")), Lang::En);
        assert_eq!(resolve_with("system", None), Lang::En);
    }

    #[test]
    fn labels_are_localized() {
        assert_eq!(labels(Lang::Ko).quit, "종료");
        assert_eq!(labels(Lang::En).open, "Open kiri");
        assert_eq!(labels(Lang::Ja).idle, "待機中のジョブはありません");
        assert!(labels(Lang::Ko).active.contains("{n}") && labels(Lang::Ko).active.contains("{p}"));
    }
}
```

`src-tauri/src/tray.rs`:

```rust
use crate::{
    i18n::{self, Labels},
    settings::Settings,
    windows,
};
use kiri_core::{engine::Engine, model::Job};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};

#[cfg(test)]
mod tests {
    use super::*;
    use kiri_core::model::{JobOptions, JobState, Preset};

    fn job(state: JobState, progress: f32) -> Job {
        Job {
            id: 1, url: "u".into(), title: "t".into(), thumbnail: None, duration_secs: None,
            quality_label: "720p".into(),
            options: JobOptions { format_id: None, preset: Preset::Original, subtitles: vec![], auto_subtitles: false },
            state, progress, speed: None, eta: None, output: None, created_at: 0,
        }
    }

    #[test]
    fn double_click_within_window() {
        let d = DoubleClick::default();
        let t0 = Instant::now();
        assert!(!d.click(t0));
        assert!(d.click(t0 + Duration::from_millis(300)));
        assert!(!d.click(t0 + Duration::from_millis(400))); // 판정 후 초기화
        assert!(!d.click(t0 + Duration::from_millis(1000))); // 600ms 간격
        assert!(d.click(t0 + Duration::from_millis(1350)));
    }

    #[test]
    fn summary_text() {
        let l = i18n::labels(i18n::Lang::Ko);
        assert_eq!(summary(&l, &[job(JobState::Completed, 1.0)]), "대기 중인 작업 없음");
        assert_eq!(
            summary(&l, &[job(JobState::Downloading, 0.4), job(JobState::Encoding, 0.6), job(JobState::Queued, 0.0)]),
            "진행 중 2개 · 50%"
        );
    }
}
```

`src-tauri/src/quit.rs`:

```rust
//! 종료 확인. 다운로드가 돌고 있으면 바로 끄지 않고 묻는다.
use crate::{i18n, windows};
use kiri_core::engine::Engine;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, RunEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirm_only_when_active_and_not_confirmed() {
        assert!(needs_confirm(false, true));
        assert!(!needs_confirm(true, true));
        assert!(!needs_confirm(false, false));
    }
}
```

`lib.rs`에 `mod i18n;`, `mod quit;`, `mod tray;`를 추가한다.

Run: `cargo test -p kiri-app`
Expected: 컴파일 에러(`cannot find function resolve_with` 등).

- [ ] **Step 2: i18n 구현**

`i18n.rs`의 `#[cfg(test)]` 위에 추가한다.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Ko,
    En,
    Ja,
}

pub fn resolve(pref: &str) -> Lang {
    resolve_with(pref, sys_locale::get_locale().as_deref())
}

pub fn resolve_with(pref: &str, system: Option<&str>) -> Lang {
    let code = if pref == "system" { system.unwrap_or("en") } else { pref };
    match code.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase().as_str() {
        "ko" => Lang::Ko,
        "ja" => Lang::Ja,
        _ => Lang::En,
    }
}

pub struct Labels {
    pub open: &'static str,
    pub quit: &'static str,
    pub check_update: &'static str,
    /// `{}` 자리에 버전
    pub install_update: &'static str,
    pub idle: &'static str,
    /// `{n}` 실행 중 개수, `{p}` 평균 진행률
    pub active: &'static str,
    pub settings_title: &'static str,
    pub quit_title: &'static str,
    pub quit_message: &'static str,
    pub quit_ok: &'static str,
    pub quit_cancel: &'static str,
}

pub fn labels(lang: Lang) -> Labels {
    match lang {
        Lang::Ko => Labels {
            open: "kiri 열기",
            quit: "종료",
            check_update: "업데이트 확인",
            install_update: "v{} 설치",
            idle: "대기 중인 작업 없음",
            active: "진행 중 {n}개 · {p}%",
            settings_title: "kiri 설정",
            quit_title: "kiri를 종료할까요?",
            quit_message: "진행 중인 다운로드가 있습니다. 종료하면 다음 실행 때 이어서 받습니다.",
            quit_ok: "종료",
            quit_cancel: "취소",
        },
        Lang::En => Labels {
            open: "Open kiri",
            quit: "Quit",
            check_update: "Check for Updates",
            install_update: "Install v{}",
            idle: "No active jobs",
            active: "{n} running · {p}%",
            settings_title: "kiri Settings",
            quit_title: "Quit kiri?",
            quit_message: "Downloads are in progress. They will resume the next time kiri starts.",
            quit_ok: "Quit",
            quit_cancel: "Cancel",
        },
        Lang::Ja => Labels {
            open: "kiri を開く",
            quit: "終了",
            check_update: "アップデートを確認",
            install_update: "v{} をインストール",
            idle: "待機中のジョブはありません",
            active: "実行中 {n} 件 · {p}%",
            settings_title: "kiri 設定",
            quit_title: "kiri を終了しますか？",
            quit_message: "ダウンロード中です。次回起動時に再開します。",
            quit_ok: "終了",
            quit_cancel: "キャンセル",
        },
    }
}

pub fn current(app: &tauri::AppHandle) -> Labels {
    use tauri::Manager;
    labels(resolve(&app.state::<crate::settings::SettingsState>().get().general.ui_language))
}
```

- [ ] **Step 3: 트레이 구현**

`tray.rs`의 `#[cfg(test)]` 위에 추가한다.

```rust
/// macOS 에서 Tauri 가 DoubleClick 을 내지 않으므로 왼쪽 클릭 두 번을 직접 판정한다.
#[derive(Default)]
pub struct DoubleClick {
    last: Mutex<Option<Instant>>,
}

impl DoubleClick {
    const WINDOW: Duration = Duration::from_millis(400);

    pub fn click(&self, now: Instant) -> bool {
        let mut last = self.last.lock().unwrap();
        match *last {
            Some(prev) if now.duration_since(prev) <= Self::WINDOW => {
                *last = None;
                true
            }
            _ => {
                *last = Some(now);
                false
            }
        }
    }
}

pub fn summary(l: &Labels, jobs: &[Job]) -> String {
    let active: Vec<&Job> = jobs.iter().filter(|j| j.state.is_active()).collect();
    if active.is_empty() {
        return l.idle.to_string();
    }
    let avg = active.iter().map(|j| j.progress).sum::<f32>() / active.len() as f32;
    l.active
        .replace("{n}", &active.len().to_string())
        .replace("{p}", &((avg * 100.0).round() as u32).to_string())
}

pub struct TrayItems {
    pub summary: MenuItem<Wry>,
    pub open: MenuItem<Wry>,
    pub quit: MenuItem<Wry>,
}

/// 설정이 저장될 때. 트레이가 아직 없으면 아무것도 하지 않는다.
pub fn relabel(app: &AppHandle, settings: &Settings) {
    let Some(items) = app.try_state::<TrayItems>() else { return };
    let l = i18n::labels(i18n::resolve(&settings.general.ui_language));
    let _ = items.open.set_text(l.open);
    let _ = items.quit.set_text(l.quit);
    if let Some(engine) = app.try_state::<Engine>() {
        let _ = items.summary.set_text(summary(&l, &engine.list()));
    }
}

/// 큐가 바뀔 때.
pub fn relabel_queue(app: &AppHandle, jobs: &[Job]) {
    let Some(items) = app.try_state::<TrayItems>() else { return };
    let _ = items.summary.set_text(summary(&i18n::current(app), jobs));
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let l = i18n::current(app);
    let summary_item = MenuItem::with_id(app, "summary", l.idle, false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", l.open, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", l.quit, true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .items(&[&summary_item])
        .separator()
        .items(&[&open])
        .separator()
        .items(&[&quit])
        .build()?;

    let clicks = DoubleClick::default();
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("bundle icon"))
        .tooltip("kiri")
        .menu(&menu)
        .show_menu_on_left_click(false) // 왼쪽 클릭은 더블클릭 판정용, 메뉴는 우클릭
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                if clicks.click(Instant::now()) {
                    let _ = windows::show_main(tray.app_handle());
                }
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                let _ = windows::show_main(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    app.manage(TrayItems { summary: summary_item, open, quit });
    relabel_queue(app, &app.state::<Engine>().list());
    Ok(())
}
```

`settings.rs`의 `SettingsState::set` 끝을 바꿔서 트레이 라벨도 갱신한다.

```rust
    pub fn set(&self, app: &AppHandle, new: Settings) -> Result<(), String> {
        new.save(&self.path).map_err(|e| e.to_string())?;
        *self.current.lock().unwrap() = new.clone();
        app.emit("settings-changed", &new).map_err(|e| e.to_string())?;
        crate::tray::relabel(app, &new); // 트레이가 아직 없으면 무시된다
        Ok(())
    }
```

- [ ] **Step 4: 종료 확인과 창 닫기 구현**

`quit.rs`의 `#[cfg(test)]` 위에 추가한다.

```rust
static CONFIRMED: AtomicBool = AtomicBool::new(false);

/// 이후의 종료 요청은 묻지 않는다(확인 대화상자의 "종료", 업데이트 재시작).
pub fn allow_exit() {
    CONFIRMED.store(true, Ordering::SeqCst);
}

pub fn needs_confirm(confirmed: bool, has_active: bool) -> bool {
    !confirmed && has_active
}

pub fn on_run_event(app: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::ExitRequested { api, .. } => {
            let active = app.try_state::<Engine>().is_some_and(|e| e.has_active());
            if needs_confirm(CONFIRMED.load(Ordering::SeqCst), active) {
                api.prevent_exit();
                confirm(app);
            }
        }
        // Dock 아이콘 클릭
        RunEvent::Reopen { .. } => {
            let _ = windows::show_main(app);
        }
        _ => {}
    }
}

fn confirm(app: &AppHandle) {
    let l = i18n::current(app);
    let handle = app.clone();
    app.dialog()
        .message(l.quit_message)
        .title(l.quit_title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(l.quit_ok.into(), l.quit_cancel.into()))
        .show(move |ok| {
            if ok {
                allow_exit();
                handle.exit(0);
            }
        });
}
```

`windows.rs` 전체:

```rust
use crate::settings::SettingsState;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub const MAIN: &str = "main";

/// 창을 숨기면 Dock 에서도 빠지고 메뉴 막대에만 남는다.
pub fn set_dock_visible(app: &AppHandle, visible: bool) {
    let policy = if visible { tauri::ActivationPolicy::Regular } else { tauri::ActivationPolicy::Accessory };
    let _ = app.set_activation_policy(policy);
}

pub fn show_main(app: &AppHandle) -> Result<(), String> {
    set_dock_visible(app, true);
    if let Some(w) = app.get_webview_window(MAIN) {
        w.show().map_err(|e| e.to_string())?;
        return w.set_focus().map_err(|e| e.to_string());
    }
    let w = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("/".into()))
        .title("kiri")
        .inner_size(720.0, 520.0)
        .min_inner_size(560.0, 400.0)
        // 링크 드롭을 HTML5 drop 이벤트로 받기 위해 Tauri 의 파일 드롭 처리를 끈다.
        .disable_drag_drop_handler()
        .build()
        .map_err(|e| e.to_string())?;
    let handle = w.clone();
    let app_handle = app.clone();
    w.on_window_event(move |e| {
        if let WindowEvent::CloseRequested { api, .. } = e {
            // 창은 항상 살려 둔다. 닫기 = 숨기기 또는 앱 종료(확인 포함).
            api.prevent_close();
            if app_handle.state::<SettingsState>().get().general.close_to_tray {
                let _ = handle.hide();
                set_dock_visible(&app_handle, false);
            } else {
                app_handle.exit(0);
            }
        }
    });
    Ok(())
}
```

`src-tauri/src/lib.rs` 전체(모듈 추가, 큐 변경 시 트레이 갱신, 트레이 생성, run 이벤트 처리):

```rust
mod bootstrap;
mod commands;
mod i18n;
mod ipc_server;
mod quit;
mod settings;
mod tray;
mod windows;

use kiri_core::{
    engine::{Engine, EnginePaths},
    model::{Job, Tools},
};
use settings::SettingsState;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

/// externalBin 은 실행 파일 옆(Contents/MacOS, dev 에서는 target/debug)에 놓인다.
pub(crate) fn sidecar(name: &str) -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(name)))
        .unwrap_or_else(|| PathBuf::from(name))
}

fn on_queue_change(app: &AppHandle, jobs: &[Job]) {
    let _ = app.emit("queue-changed", jobs);
    tray::relabel_queue(app, jobs);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let path = app.path();
            app.manage(SettingsState::new(path.app_config_dir()?.join("settings.json")));
            app.manage(bootstrap::ToolsState::default());
            app.manage(ipc_server::IpcState::default());

            let bin = path.app_data_dir()?.join("bin");
            let tools = Tools { ytdlp: bin.join("yt-dlp"), deno: bin.join("deno"), ffmpeg: sidecar("ffmpeg") };
            let paths = EnginePaths {
                queue_file: path.app_local_data_dir()?.join("queue.json"),
                cache_dir: path.app_cache_dir()?,
                log_dir: path.app_log_dir()?.join("jobs"),
            };
            let config = settings::engine_config(&app.state::<SettingsState>().get());
            let rt = tauri::async_runtime::block_on(async { tokio::runtime::Handle::current() });
            let h = handle.clone();
            let engine = Engine::new(paths, tools, config, rt, move |jobs| on_queue_change(&h, jobs));
            app.manage(engine.clone());

            windows::show_main(&handle).map_err(std::io::Error::other)?;
            tray::build(&handle)?;
            engine.start();
            ipc_server::spawn(&handle, engine);
            bootstrap::spawn(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::patch_settings,
            commands::list_jobs,
            commands::probe,
            commands::add_job,
            commands::add_url,
            commands::stop_job,
            commands::remove_job,
            commands::restart_job,
            commands::tools_status,
            commands::update_tools,
        ])
        .build(tauri::generate_context!())
        .expect("error while building kiri")
        .run(|app, event| quit::on_run_event(app, event));
}
```

`AppHandle::set_activation_policy`가 없다는 컴파일 에러가 나면(Tauri 버전 차이), `windows::set_dock_visible`의 본문을 `let _ = app.set_dock_visibility(visible);`(Tauri 2.5+)로 바꾼다.

- [ ] **Step 5: 테스트 통과 확인**

Run: `cargo test -p kiri-app`
Expected: i18n 2개, tray 2개, quit 1개를 포함해 모두 통과한다.

- [ ] **Step 6: 수동 확인**

Run: `yarn tauri dev`

확인할 것:
1. 메뉴 막대에 kiri 아이콘이 보인다.
2. 창의 빨간 버튼을 누르면 창과 Dock 아이콘이 사라지고 메뉴 막대 아이콘만 남는다.
3. 메뉴 막대 아이콘을 빠르게 두 번 클릭하면 창이 다시 뜨고 Dock 아이콘이 돌아온다. 한 번만 클릭하면 아무 일도 없다.
4. 우클릭하면 메뉴(작업 요약, kiri 열기, 종료)가 뜬다. 다운로드 중에는 요약이 "진행 중 1개 · 37%"처럼 바뀐다.
5. 다운로드 중에 메뉴의 "종료"와 ⌘Q를 각각 눌러 본다. 둘 다 확인 대화상자가 뜨고, 취소하면 계속 받고, 종료하면 앱이 끝난다. 다시 실행하면 그 작업이 이어서 진행된다.
6. 다운로드가 없을 때 ⌘Q를 누르면 바로 종료된다.

⌘Q가 확인 없이 바로 종료된다면(`ExitRequested`를 거치지 않는 경우) 이 사실을 기록하고 사용자에게 알린다.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src
git commit -m "feat(app): 메뉴 막대 아이콘(더블클릭 열기), 닫으면 숨기기, 종료 확인 추가"
```

---
### Task 17: 앱 자동 업데이트 (GitHub Release)

**Files:**
- Create: `src-tauri/src/updater.rs`, `src/lib/update.ts`, `src/components/UpdateBanner.tsx`
- Modify:
  - `src-tauri/Cargo.toml` (`tauri-plugin-updater`)
  - `src-tauri/tauri.conf.json` (`createUpdaterArtifacts`, `plugins.updater`)
  - `src-tauri/src/tray.rs` (업데이트 항목), `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`
  - `src/lib/tauri.ts`, `src/pages/MainWindow.tsx`

**Interfaces:**
- Consumes: `Engine::is_idle`, `quit::allow_exit`, `SettingsState` (`update.auto_check`), `i18n::Labels::{check_update, install_update}`
- Produces:
  - Rust:
    - `updater::{UpdateState, UpdateInfo, UpdateProgress, status, check, install, request_install, should_install, on_queue_change, spawn_periodic, on_tray_click}`
    - commands `update_status`, `check_update`, `install_update(after_queue: bool)`
  - 이벤트 `update-available`(`UpdateInfo`), `update-progress`(`UpdateProgress`), `update-error`(string), `update-scheduled`(bool)
  - TS: `useUpdate` (`info`, `progress`, `scheduled`, `checking`, `checked`, `check()`, `install(afterQueue)`, `subscribe()`), `<UpdateBanner />`

- [ ] **Step 1: 서명 키 만들기 (한 번, 수동)**

Run: `yarn tauri signer generate -w ~/.tauri/kiri.key`

비밀번호를 정하고, 출력된 **public key**를 복사한다. 개인키와 비밀번호는 Task 19의 `~/.config/kiri/sign.env`에 넣는다(`TAURI_SIGNING_PRIVATE_KEY`는 키 파일 경로나 내용).

`src-tauri/Cargo.toml`의 `[dependencies]`에 추가한다.

```toml
tauri-plugin-updater = "2"
```

`src-tauri/tauri.conf.json`의 `bundle`에 `"createUpdaterArtifacts": true,`를 추가하고, 최상위에 `plugins`를 추가한다(`<PUBKEY>` 자리에 위에서 복사한 공개키 문자열을 그대로 붙인다).

```json
"plugins": {
  "updater": {
    "pubkey": "<PUBKEY>",
    "endpoints": ["https://github.com/bob-park/kiri-app/releases/latest/download/latest.json"]
  }
}
```

- [ ] **Step 2: 실패하는 Rust 테스트 작성**

`src-tauri/src/updater.rs`:

```rust
//! GitHub Releases 의 latest.json 으로 새 버전을 확인하고 설치한다.
//! 창이 닫혀도 앱은 메뉴 막대에 살아 있으므로 주기 확인은 Rust 에서 돈다.
use crate::{quit, settings::SettingsState};
use kiri_core::{engine::Engine, model::Job};
use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

#[cfg(test)]
mod tests {
    use super::*;
    use kiri_core::model::{JobOptions, JobState, Preset};

    fn job(state: JobState) -> Job {
        Job {
            id: 1, url: "u".into(), title: "t".into(), thumbnail: None, duration_secs: None,
            quality_label: "720p".into(),
            options: JobOptions { format_id: None, preset: Preset::Original, subtitles: vec![], auto_subtitles: false },
            state, progress: 0.0, speed: None, eta: None, output: None, created_at: 0,
        }
    }

    #[test]
    fn installs_after_queue_only_when_pending_and_idle() {
        assert!(!should_install(false, &[]));
        assert!(should_install(true, &[job(JobState::Completed), job(JobState::Failed("x".into()))]));
        assert!(!should_install(true, &[job(JobState::Queued)]));
        assert!(!should_install(true, &[job(JobState::Encoding)]));
    }

    #[test]
    fn info_serializes_with_frontend_keys() {
        let v = serde_json::to_value(UpdateInfo { version: "0.2.0".into(), notes: "fix".into() }).unwrap();
        assert_eq!(v, serde_json::json!({"version": "0.2.0", "notes": "fix"}));
        let p = serde_json::to_value(UpdateProgress { received: 5, total: None }).unwrap();
        assert_eq!(p, serde_json::json!({"received": 5, "total": null}));
    }
}
```

`lib.rs`에 `mod updater;`를 추가한다.

Run: `cargo test -p kiri-app updater`
Expected: 컴파일 에러(`cannot find function should_install`).

- [ ] **Step 3: Rust 구현**

`updater.rs`의 `#[cfg(test)]` 위에 추가한다.

```rust
#[derive(Default)]
pub struct UpdateState(Mutex<Option<Update>>);

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
}

impl UpdateInfo {
    fn from_update(u: &Update) -> Self {
        Self { version: u.version.clone(), notes: u.body.clone().unwrap_or_default() }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct UpdateProgress {
    pub received: u64,
    pub total: Option<u64>,
}

const FIRST_CHECK: Duration = Duration::from_secs(10);
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// "큐가 끝나면 재시작" 을 고른 상태.
static RESTART_AFTER_QUEUE: AtomicBool = AtomicBool::new(false);

pub fn should_install(pending: bool, jobs: &[Job]) -> bool {
    pending && !jobs.iter().any(|j| j.state.is_pending())
}

pub fn status(app: &AppHandle) -> Option<UpdateInfo> {
    app.try_state::<UpdateState>()?.0.lock().unwrap().as_ref().map(UpdateInfo::from_update)
}

pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let found = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?;
    let info = found.as_ref().map(UpdateInfo::from_update);
    *app.state::<UpdateState>().0.lock().unwrap() = found;
    if let Some(i) = &info {
        let _ = app.emit("update-available", i);
    }
    crate::tray::relabel_update(app);
    Ok(info)
}

/// 받고 → 설치 → 재시작. 실패해도 보관한 Update 는 남겨 다시 시도할 수 있다.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    static INSTALLING: AtomicBool = AtomicBool::new(false);
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return Err("busy".into());
    }
    let fail = |e: String| {
        INSTALLING.store(false, Ordering::SeqCst);
        e
    };
    let update = app
        .state::<UpdateState>()
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| fail("no_update".to_string()))?;
    let mut received: u64 = 0;
    let mut last_pct = u64::MAX;
    let bytes = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                let pct = total.map_or(0, |t| received * 100 / t.max(1));
                if pct != last_pct {
                    last_pct = pct;
                    let _ = app.emit("update-progress", UpdateProgress { received, total });
                }
            },
            || {},
        )
        .await
        .map_err(|e| fail(e.to_string()))?;
    update.install(bytes).map_err(|e| fail(e.to_string()))?;
    quit::allow_exit(); // 재시작이 종료 확인에 막히지 않게
    app.restart()
}

/// after_queue 이고 큐가 남아 있으면 예약만 한다.
pub async fn request_install(app: &AppHandle, after_queue: bool) -> Result<(), String> {
    if after_queue && !app.state::<Engine>().is_idle() {
        RESTART_AFTER_QUEUE.store(true, Ordering::SeqCst);
        let _ = app.emit("update-scheduled", true);
        return Ok(());
    }
    install(app).await
}

/// 큐가 바뀔 때마다. 예약돼 있고 큐가 비면 설치한다.
pub fn on_queue_change(app: &AppHandle, jobs: &[Job]) {
    if should_install(RESTART_AFTER_QUEUE.load(Ordering::SeqCst), jobs) {
        RESTART_AFTER_QUEUE.store(false, Ordering::SeqCst);
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = install(&app).await {
                let _ = app.emit("update-error", e);
            }
        });
    }
}

/// setup 에서 한 번. 설정을 매 주기 다시 읽으므로 토글이 바로 반영된다. 오프라인 실패는 로그만.
pub fn spawn_periodic(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            if app.state::<SettingsState>().get().update.auto_check {
                if let Err(e) = check(&app).await {
                    eprintln!("kiri update: check failed: {e}");
                }
            }
            tokio::time::sleep(INTERVAL).await;
        }
    });
}

/// 트레이 항목 하나가 "확인" 과 "설치" 를 겸한다. 설치는 큐가 끝난 뒤로 예약한다.
pub fn on_tray_click(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let r = if status(&app).is_some() { request_install(&app, true).await } else { check(&app).await.map(|_| ()) };
        if let Err(e) = r {
            let _ = app.emit("update-error", e);
        }
    });
}
```

`tray.rs`를 고친다. 업데이트 항목을 추가한다.

```rust
pub struct TrayItems {
    pub summary: MenuItem<Wry>,
    pub open: MenuItem<Wry>,
    pub update: MenuItem<Wry>,
    pub quit: MenuItem<Wry>,
}

fn update_label(l: &Labels, pending: Option<crate::updater::UpdateInfo>) -> String {
    match pending {
        Some(info) => l.install_update.replace("{}", &info.version),
        None => l.check_update.to_string(),
    }
}

/// 업데이트 확인이 끝날 때마다.
pub fn relabel_update(app: &AppHandle) {
    let Some(items) = app.try_state::<TrayItems>() else { return };
    let _ = items.update.set_text(update_label(&i18n::current(app), crate::updater::status(app)));
}
```

`relabel` 안에 한 줄을 추가한다.

```rust
    let _ = items.update.set_text(update_label(&l, crate::updater::status(app)));
```

`build`에서는 항목을 만들어 메뉴에 넣고, 메뉴 이벤트와 `manage`에 반영한다.

```rust
    let update = MenuItem::with_id(app, "update", update_label(&l, crate::updater::status(app)), true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .items(&[&summary_item])
        .separator()
        .items(&[&open, &update])
        .separator()
        .items(&[&quit])
        .build()?;
    // …
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                let _ = windows::show_main(app);
            }
            "update" => crate::updater::on_tray_click(app.clone()),
            "quit" => app.exit(0),
            _ => {}
        })
    // …
    app.manage(TrayItems { summary: summary_item, open, update, quit });
```

`tray.rs` 테스트에 추가한다.

```rust
    #[test]
    fn update_label_follows_pending_state() {
        let l = i18n::labels(i18n::Lang::Ko);
        assert_eq!(update_label(&l, None), "업데이트 확인");
        let info = crate::updater::UpdateInfo { version: "0.2.0".into(), notes: String::new() };
        assert_eq!(update_label(&l, Some(info)), "v0.2.0 설치");
    }
```

`commands.rs`에 추가한다.

```rust
#[tauri::command]
pub fn update_status(app: AppHandle) -> Option<crate::updater::UpdateInfo> {
    crate::updater::status(&app)
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<crate::updater::UpdateInfo>, String> {
    crate::updater::check(&app).await
}

#[tauri::command]
pub async fn install_update(app: AppHandle, after_queue: bool) -> Result<(), String> {
    crate::updater::request_install(&app, after_queue).await
}
```

`src-tauri/src/lib.rs` 전체(Task 16 버전에 updater 플러그인, 상태, 큐 변경 훅, 주기 확인, 명령 3개를 더한 것):

```rust
mod bootstrap;
mod commands;
mod i18n;
mod ipc_server;
mod quit;
mod settings;
mod tray;
mod updater;
mod windows;

use kiri_core::{
    engine::{Engine, EnginePaths},
    model::{Job, Tools},
};
use settings::SettingsState;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

/// externalBin 은 실행 파일 옆(Contents/MacOS, dev 에서는 target/debug)에 놓인다.
pub(crate) fn sidecar(name: &str) -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(name)))
        .unwrap_or_else(|| PathBuf::from(name))
}

fn on_queue_change(app: &AppHandle, jobs: &[Job]) {
    let _ = app.emit("queue-changed", jobs);
    tray::relabel_queue(app, jobs);
    updater::on_queue_change(app, jobs);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            let path = app.path();
            app.manage(SettingsState::new(path.app_config_dir()?.join("settings.json")));
            app.manage(bootstrap::ToolsState::default());
            app.manage(ipc_server::IpcState::default());
            app.manage(updater::UpdateState::default());

            let bin = path.app_data_dir()?.join("bin");
            let tools = Tools { ytdlp: bin.join("yt-dlp"), deno: bin.join("deno"), ffmpeg: sidecar("ffmpeg") };
            let paths = EnginePaths {
                queue_file: path.app_local_data_dir()?.join("queue.json"),
                cache_dir: path.app_cache_dir()?,
                log_dir: path.app_log_dir()?.join("jobs"),
            };
            let config = settings::engine_config(&app.state::<SettingsState>().get());
            let rt = tauri::async_runtime::block_on(async { tokio::runtime::Handle::current() });
            let h = handle.clone();
            let engine = Engine::new(paths, tools, config, rt, move |jobs| on_queue_change(&h, jobs));
            app.manage(engine.clone());

            windows::show_main(&handle).map_err(std::io::Error::other)?;
            tray::build(&handle)?;
            engine.start();
            ipc_server::spawn(&handle, engine);
            bootstrap::spawn(handle.clone());
            updater::spawn_periodic(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::patch_settings,
            commands::list_jobs,
            commands::probe,
            commands::add_job,
            commands::add_url,
            commands::stop_job,
            commands::remove_job,
            commands::restart_job,
            commands::tools_status,
            commands::update_tools,
            commands::update_status,
            commands::check_update,
            commands::install_update,
        ])
        .build(tauri::generate_context!())
        .expect("error while building kiri")
        .run(|app, event| quit::on_run_event(app, event));
}
```

Run: `cargo test -p kiri-app`
Expected: 모두 통과(updater 2개, tray 3개 포함).

- [ ] **Step 4: 프론트엔드 스토어와 배너**

`src/lib/tauri.ts`의 `api`에 추가한다(타입 import에 `UpdateInfo`도 추가한다).

```ts
  updateStatus: () => invoke<UpdateInfo | null>("update_status"),
  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  installUpdate: (afterQueue: boolean) => invoke<void>("install_update", { afterQueue }),
```

Tauri는 Rust의 `after_queue`를 JS의 `afterQueue`로 받는다(camelCase 변환).

`src/lib/update.ts`:

```ts
import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { showError } from "./toast";
import type { UpdateInfo, UpdateProgress } from "./types";

interface UpdateStore {
  info: UpdateInfo | null;
  progress: UpdateProgress | null;
  scheduled: boolean;
  checking: boolean;
  checked: boolean; // 수동 확인을 끝낸 뒤에만 "최신 버전" 을 말한다
  check: () => Promise<void>;
  install: (afterQueue: boolean) => Promise<void>;
  subscribe: () => () => void;
}

// 설치 중 두 번째 요청. 사용자에게 알릴 일이 아니다.
const isBusy = (e: unknown) => (e instanceof Error ? e.message : String(e)) === "busy";

export const useUpdate = create<UpdateStore>((set) => ({
  info: null,
  progress: null,
  scheduled: false,
  checking: false,
  checked: false,
  check: async () => {
    set({ checking: true });
    try {
      set({ info: await api.checkUpdate(), checked: true });
    } catch (e) {
      showError(e);
    } finally {
      set({ checking: false });
    }
  },
  install: async (afterQueue) => {
    if (!afterQueue) set((s) => ({ progress: s.progress ?? { received: 0, total: null } }));
    try {
      await api.installUpdate(afterQueue);
    } catch (e) {
      if (isBusy(e)) return;
      set({ progress: null });
      showError(e);
    }
  },
  subscribe: () => {
    api.updateStatus().then((info) => set((s) => ({ info: s.info ?? info }))).catch(() => {});
    const subs = [
      listen<UpdateInfo>("update-available", (e) => set({ info: e.payload })),
      listen<UpdateProgress>("update-progress", (e) => set({ progress: e.payload, scheduled: false })),
      listen<boolean>("update-scheduled", () => set({ scheduled: true })),
      listen<string>("update-error", (e) => {
        if (isBusy(e.payload)) return;
        set({ progress: null });
        showError(e.payload);
      }),
    ];
    return () => {
      for (const p of subs) p.then((un) => un()).catch(() => {});
    };
  },
}));
```

`src/components/UpdateBanner.tsx`:

```tsx
import { useTranslation } from "react-i18next";
import { useUpdate } from "../lib/update";
import { isIdle, useQueue } from "../lib/queue";

export function UpdateBanner() {
  const { t } = useTranslation();
  const { info, progress, scheduled, install } = useUpdate();
  const busy = useQueue((s) => !isIdle(s.jobs));
  if (!info) return null;
  const pct = progress?.total ? Math.round((progress.received / progress.total) * 100) : 0;
  const text = progress
    ? t("update.downloading", { pct })
    : scheduled
      ? t("update.scheduled")
      : t("update.available", { version: info.version });
  return (
    <div role="status" className="mx-3 mt-2 flex items-center gap-2 rounded-xl bg-secondary px-3 py-2 text-sm text-secondary-content">
      <span className="flex-1">{text}</span>
      {!progress && !scheduled && busy && (
        <>
          <button className="btn btn-ghost btn-xs" onClick={() => install(false)}>{t("update.installNow")}</button>
          <button className="btn btn-primary btn-xs" onClick={() => install(true)}>{t("update.afterQueue")}</button>
        </>
      )}
      {!progress && !scheduled && !busy && (
        <button className="btn btn-primary btn-xs" onClick={() => install(false)}>{t("update.install")}</button>
      )}
    </div>
  );
}
```

`src/pages/MainWindow.tsx`에서 import를 추가하고 `<ToolsNotice />` 위에 배너를 넣는다.

```tsx
import { UpdateBanner } from "../components/UpdateBanner";
// …
      <UpdateBanner />
      <ToolsNotice />
```

`src/main.tsx`의 첫 `useEffect` 구독 목록에 업데이트 구독을 추가한다.

```tsx
import { useUpdate } from "./lib/update";
// …
    const unsubs = [subscribeBackend(), useQueue.getState().bind(), useTools.getState().bind(), useUpdate.getState().subscribe()];
```

- [ ] **Step 5: 확인**

Run: `yarn test && yarn build && cargo test -p kiri-app`
Expected: 통과.

실제 업데이트 동작(0.1.0 → 0.1.1)은 첫 릴리즈 후 Task 19의 Step 5에서 확인한다.

- [ ] **Step 6: Commit**

```bash
git add src-tauri src
git commit -m "feat(app): GitHub Release 자동 업데이트와 큐 종료 후 재시작 추가"
```

---

### Task 18: 환경설정 창 (일반/다운로드/CLI/업데이트)과 CLI 설치

**Files:**
- Create:
  - `src-tauri/src/cli_install.rs`
  - `src/pages/SettingsWindow.tsx`
  - `src/pages/settings/Row.tsx`, `src/pages/settings/GeneralTab.tsx`, `src/pages/settings/DownloadTab.tsx`, `src/pages/settings/CliTab.tsx`, `src/pages/settings/UpdateTab.tsx`
- Modify: `src-tauri/src/windows.rs` (`show_settings`), `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` (모듈·명령 등록), `src/lib/tauri.ts`, `src/main.tsx`

**Interfaces:**
- Consumes: `i18n::current(app).settings_title`, `ipc_server::IpcState`, `sidecar("kiri-cli")`, `useSettings`, `useTools`, `useUpdate`, `open`(`@tauri-apps/plugin-dialog`), `getVersion`(`@tauri-apps/api/app`)
- Produces:
  - Rust:
    - `windows::show_settings(app)`
    - `cli_install::{LINK, CliStatus, is_installed(target, link), shell_quote(path), install(target, link), uninstall(link)}`
    - commands `open_settings`, `cli_status`, `install_cli`, `uninstall_cli`
  - TS: `api.{openSettings (Task 15에서 추가됨), cliStatus, installCli, uninstallCli}`, `<SettingsWindow />`

- [ ] **Step 1: 실패하는 Rust 테스트 작성**

`src-tauri/src/cli_install.rs`:

```rust
//! /usr/local/bin/kiri → 번들 안 kiri-cli 심볼릭 링크.
use serde::Serialize;
use std::{
    fs, io,
    path::Path,
    process::Command,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_and_uninstall_in_writable_dir() {
        let d = tempfile::tempdir().unwrap();
        let target = d.path().join("kiri-cli");
        fs::write(&target, "").unwrap();
        let link = d.path().join("bin/kiri");
        fs::create_dir(d.path().join("bin")).unwrap();
        assert!(!is_installed(&target, &link));
        install(&target, &link).unwrap();
        assert!(is_installed(&target, &link));
        install(&target, &link).unwrap(); // 다시 설치해도 된다
        uninstall(&link).unwrap();
        assert!(!is_installed(&target, &link));
        uninstall(&link).unwrap(); // 없으면 성공
    }

    #[test]
    fn link_to_other_target_is_not_installed() {
        let d = tempfile::tempdir().unwrap();
        let link = d.path().join("kiri");
        std::os::unix::fs::symlink("/somewhere/else", &link).unwrap();
        assert!(!is_installed(&d.path().join("kiri-cli"), &link));
    }

    #[test]
    fn shell_quote_rejects_dangerous_paths() {
        assert_eq!(shell_quote(Path::new("/Applications/kiri.app/Contents/MacOS/kiri-cli")).unwrap(), "'/Applications/kiri.app/Contents/MacOS/kiri-cli'");
        assert!(shell_quote(Path::new("/tmp/a'b")).is_err());
        assert!(shell_quote(Path::new("/tmp/a\"b")).is_err());
        assert!(shell_quote(Path::new("/tmp/a\\b")).is_err());
    }
}
```

`lib.rs`에 `mod cli_install;`을 추가한다.

Run: `cargo test -p kiri-app cli_install`
Expected: 컴파일 에러(`cannot find function is_installed`).

- [ ] **Step 2: Rust 구현**

`cli_install.rs`의 `#[cfg(test)]` 위에 추가한다.

```rust
pub const LINK: &str = "/usr/local/bin/kiri";

#[derive(Serialize, Debug)]
pub struct CliStatus {
    pub installed: bool,
    pub link: String,
    pub target: String,
    pub socket_error: Option<String>,
}

pub fn is_installed(target: &Path, link: &Path) -> bool {
    fs::read_link(link).is_ok_and(|p| p == target)
}

/// AppleScript 문자열 안의 sh 명령에 넣을 경로. 따옴표·역슬래시가 있으면 거부한다.
pub fn shell_quote(p: &Path) -> Result<String, String> {
    let s = p.to_str().ok_or("non-UTF-8 path")?;
    if s.contains(['\'', '"', '\\']) {
        return Err(format!("unsupported path: {s}"));
    }
    Ok(format!("'{s}'"))
}

fn admin(sh: &str) -> Result<(), String> {
    let script = format!("do shell script \"{sh}\" with administrator privileges");
    let out = Command::new("osascript").args(["-e", &script]).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn needs_admin(e: &io::Error) -> bool {
    matches!(e.kind(), io::ErrorKind::PermissionDenied | io::ErrorKind::NotFound)
}

pub fn install(target: &Path, link: &Path) -> Result<(), String> {
    let _ = fs::remove_file(link);
    match std::os::unix::fs::symlink(target, link) {
        Ok(()) => Ok(()),
        Err(e) if needs_admin(&e) => {
            let dir = link.parent().ok_or("link has no parent")?;
            admin(&format!(
                "mkdir -p {} && ln -sf {} {}",
                shell_quote(dir)?,
                shell_quote(target)?,
                shell_quote(link)?
            ))
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn uninstall(link: &Path) -> Result<(), String> {
    match fs::remove_file(link) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => admin(&format!("rm -f {}", shell_quote(link)?)),
        Err(e) => Err(e.to_string()),
    }
}
```

`windows.rs`에 추가한다.

```rust
pub const SETTINGS: &str = "settings";

pub fn show_settings(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(SETTINGS) {
        w.show().map_err(|e| e.to_string())?;
        return w.set_focus().map_err(|e| e.to_string());
    }
    WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("/".into()))
        .title(crate::i18n::current(app).settings_title)
        .inner_size(560.0, 460.0)
        .resizable(false)
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
```

`commands.rs`에 추가한다.

```rust
use crate::cli_install::{self, CliStatus};
use std::path::Path;

#[tauri::command]
pub fn open_settings(app: AppHandle) -> CmdResult<()> {
    Ok(crate::windows::show_settings(&app)?)
}

#[tauri::command]
pub fn cli_status(ipc: State<'_, crate::ipc_server::IpcState>) -> CliStatus {
    let target = crate::sidecar("kiri-cli");
    let link = Path::new(cli_install::LINK);
    CliStatus {
        installed: cli_install::is_installed(&target, link),
        link: cli_install::LINK.into(),
        target: target.display().to_string(),
        socket_error: ipc.0.lock().unwrap().clone(),
    }
}

#[tauri::command]
pub fn install_cli() -> CmdResult<()> {
    Ok(cli_install::install(&crate::sidecar("kiri-cli"), Path::new(cli_install::LINK))?)
}

#[tauri::command]
pub fn uninstall_cli() -> CmdResult<()> {
    Ok(cli_install::uninstall(Path::new(cli_install::LINK))?)
}
```

`lib.rs`: `mod cli_install;`을 추가하고, `generate_handler!` 목록 끝에 네 줄을 추가한다.

```rust
            commands::open_settings,
            commands::cli_status,
            commands::install_cli,
            commands::uninstall_cli,
```

Run: `cargo test -p kiri-app`
Expected: 통과(cli_install 3개 포함).

- [ ] **Step 3: 프론트엔드 설정 창**

`src/lib/tauri.ts`의 `api`에 추가한다(`CliStatus` 타입 import).

```ts
  cliStatus: () => invoke<CliStatus>("cli_status"),
  installCli: () => invoke<void>("install_cli"),
  uninstallCli: () => invoke<void>("uninstall_cli"),
```

`src/pages/settings/Row.tsx`:

```tsx
import type { ReactNode } from "react";

export function Row({ label, desc, children }: { label: string; desc?: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-base-300 py-2.5 last:border-none">
      <div className="min-w-0">
        <div className="text-sm font-medium">{label}</div>
        {desc && <div className="mt-0.5 text-xs text-fg-muted">{desc}</div>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}
```

`src/pages/settings/GeneralTab.tsx`:

```tsx
import { useTranslation } from "react-i18next";
import { useSettings } from "../../lib/settings";
import type { Theme, UiLang } from "../../lib/types";
import { Row } from "./Row";

const LANGS: [UiLang, string][] = [
  ["system", "settings.general.langSystem"],
  ["ko", "settings.general.langKo"],
  ["en", "settings.general.langEn"],
  ["ja", "settings.general.langJa"],
];
const THEMES: [Theme, string][] = [
  ["system", "settings.general.themeSystem"],
  ["light", "settings.general.themeLight"],
  ["dark", "settings.general.themeDark"],
];

export function GeneralTab() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const g = settings!.general;
  return (
    <div>
      <Row label={t("settings.general.language")}>
        <select className="select select-sm w-40" value={g.ui_language} onChange={(e) => update({ general: { ui_language: e.target.value as UiLang } })}>
          {LANGS.map(([v, k]) => (
            <option key={v} value={v}>{t(k)}</option>
          ))}
        </select>
      </Row>
      <Row label={t("settings.general.theme")}>
        <div className="join">
          {THEMES.map(([v, k]) => (
            <button key={v} className={`btn btn-sm join-item ${g.theme === v ? "btn-primary" : ""}`} aria-pressed={g.theme === v} onClick={() => update({ general: { theme: v } })}>
              {t(k)}
            </button>
          ))}
        </div>
      </Row>
      <Row label={t("settings.general.closeToTray")} desc={t("settings.general.closeToTrayDesc")}>
        <input type="checkbox" className="toggle toggle-primary toggle-sm" checked={g.close_to_tray} onChange={(e) => update({ general: { close_to_tray: e.target.checked } })} />
      </Row>
    </div>
  );
}
```

`src/pages/settings/DownloadTab.tsx`:

```tsx
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { useSettings } from "../../lib/settings";
import { showError } from "../../lib/toast";
import { PRESETS, type Preset } from "../../lib/types";
import { Row } from "./Row";

const QUALITIES = ["best", "2160p", "1440p", "1080p", "720p", "480p", "audio"];

export function DownloadTab() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const d = settings!.download;
  const [subs, setSubs] = useState(d.subtitles.join(","));
  useEffect(() => setSubs(d.subtitles.join(",")), [d.subtitles.join(",")]);

  const pickDir = async () => {
    try {
      const dir = await open({ directory: true, defaultPath: d.dir });
      if (typeof dir === "string") await update({ download: { dir } });
    } catch (e) {
      showError(e);
    }
  };
  const commitSubs = () => update({ download: { subtitles: subs.split(",").map((s) => s.trim()).filter(Boolean) } });
  const qLabel = (q: string) => (q === "best" ? t("quality.best") : q === "audio" ? t("quality.audio") : q);

  return (
    <div>
      <Row label={t("settings.download.dir")} desc={d.dir}>
        <button className="btn btn-sm btn-secondary" onClick={pickDir}>{t("settings.download.change")}</button>
      </Row>
      <Row label={t("settings.download.quality")}>
        <select className="select select-sm w-40" value={d.quality} onChange={(e) => update({ download: { quality: e.target.value } })}>
          {QUALITIES.map((q) => (
            <option key={q} value={q}>{qLabel(q)}</option>
          ))}
        </select>
      </Row>
      <Row label={t("settings.download.preset")}>
        <select className="select select-sm w-40" value={d.preset} onChange={(e) => update({ download: { preset: e.target.value as Preset } })}>
          {PRESETS.map((p) => (
            <option key={p} value={p}>{t(`preset.${p}`)}</option>
          ))}
        </select>
      </Row>
      <Row label={t("settings.download.subtitles")} desc={t("settings.download.subtitlesHint")}>
        <input className="input input-sm w-40" value={subs} onChange={(e) => setSubs(e.target.value)} onBlur={commitSubs} onKeyDown={(e) => e.key === "Enter" && commitSubs()} />
      </Row>
      <Row label={t("settings.download.skipSheet")}>
        <input type="checkbox" className="toggle toggle-primary toggle-sm" checked={d.skip_sheet} onChange={(e) => update({ download: { skip_sheet: e.target.checked } })} />
      </Row>
      <Row label={t("settings.download.maxConcurrent")}>
        <select className="select select-sm w-20" value={d.max_concurrent} onChange={(e) => update({ download: { max_concurrent: Number(e.target.value) } })}>
          {[1, 2, 3, 4].map((n) => (
            <option key={n} value={n}>{n}</option>
          ))}
        </select>
      </Row>
      <Row label={t("settings.download.hwAccel")} desc={t("settings.download.hwAccelDesc")}>
        <input type="checkbox" className="toggle toggle-primary toggle-sm" checked={d.hw_accel} onChange={(e) => update({ download: { hw_accel: e.target.checked } })} />
      </Row>
    </div>
  );
}
```

`src/pages/settings/CliTab.tsx`:

```tsx
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api } from "../../lib/tauri";
import { showError } from "../../lib/toast";
import type { CliStatus } from "../../lib/types";
import { Row } from "./Row";

const USAGE = `kiri status            # running jobs
kiri list              # whole queue
kiri add <url> [--quality 1080p] [--format mp4-h264] [--subs ko,en]
kiri stop <id>
kiri remove <id>
kiri list --json       # machine-readable`;

export function CliTab() {
  const { t } = useTranslation();
  const [st, setSt] = useState<CliStatus | null>(null);
  const refresh = () => api.cliStatus().then(setSt).catch(showError);
  useEffect(() => {
    refresh();
  }, []);
  const run = (f: () => Promise<void>) => () => f().then(refresh).catch(showError);

  return (
    <div>
      <Row label={t("settings.cli.status")} desc={st?.installed ? t("settings.cli.installed", { path: st.link }) : t("settings.cli.notInstalled")}>
        {st?.installed ? (
          <button className="btn btn-sm btn-outline" onClick={run(api.uninstallCli)}>{t("settings.cli.uninstall")}</button>
        ) : (
          <button className="btn btn-sm btn-primary" onClick={run(api.installCli)}>{t("settings.cli.install")}</button>
        )}
      </Row>
      {st?.socket_error && (
        <div role="alert" className="alert alert-warning mt-2 py-2 text-sm">{t("settings.cli.socketError", { error: st.socket_error })}</div>
      )}
      <div className="mt-3 mb-1 text-xs font-semibold text-fg-muted">{t("settings.cli.usage")}</div>
      <pre className="overflow-x-auto rounded-lg bg-base-200 p-3 text-xs select-text">{USAGE}</pre>
    </div>
  );
}
```

`src/pages/settings/UpdateTab.tsx`:

```tsx
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { api } from "../../lib/tauri";
import { useSettings } from "../../lib/settings";
import { useTools } from "../../lib/tools";
import { useUpdate } from "../../lib/update";
import { showError } from "../../lib/toast";
import { Row } from "./Row";

export function UpdateTab() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const { info, checking, checked, check } = useUpdate();
  const tools = useTools((s) => s.status);
  const [version, setVersion] = useState("");
  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  const appDesc = info ? t("update.available", { version: info.version }) : checked ? t("settings.update.upToDate") : undefined;
  const last = tools?.last_check ? new Date(tools.last_check).toLocaleString() : t("settings.update.never");

  return (
    <div>
      <Row label={t("settings.update.appVersion", { version })} desc={appDesc}>
        <button className="btn btn-sm btn-secondary" disabled={checking} onClick={check}>
          {checking ? t("settings.update.checking") : t("settings.update.check")}
        </button>
      </Row>
      <Row label={t("settings.update.autoCheck")}>
        <input type="checkbox" className="toggle toggle-primary toggle-sm" checked={settings!.update.auto_check} onChange={(e) => update({ update: { auto_check: e.target.checked } })} />
      </Row>
      <Row
        label={tools?.ytdlp_version ? t("settings.update.ytdlpVersion", { version: tools.ytdlp_version }) : t("settings.update.ytdlpMissing")}
        desc={t("settings.update.lastCheck", { time: last })}
      >
        <button className="btn btn-sm btn-secondary" disabled={tools?.installing} onClick={() => api.updateTools().catch(showError)}>
          {tools?.installing ? <span className="loading loading-spinner loading-xs" /> : t("settings.update.ytdlpUpdate")}
        </button>
      </Row>
      {tools?.error && !tools.installing && <div role="alert" className="alert alert-error mt-2 py-2 text-sm">{tools.error}</div>}
    </div>
  );
}
```

`src/pages/SettingsWindow.tsx`:

```tsx
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { GeneralTab } from "./settings/GeneralTab";
import { DownloadTab } from "./settings/DownloadTab";
import { CliTab } from "./settings/CliTab";
import { UpdateTab } from "./settings/UpdateTab";

const TABS = [
  { key: "general", icon: "⚙", Body: GeneralTab },
  { key: "download", icon: "⬇", Body: DownloadTab },
  { key: "cli", icon: "⌘", Body: CliTab },
  { key: "update", icon: "↻", Body: UpdateTab },
] as const;

export default function SettingsWindow() {
  const { t } = useTranslation();
  const [tab, setTab] = useState<(typeof TABS)[number]["key"]>("general");
  const Body = TABS.find((x) => x.key === tab)!.Body;
  return (
    <div className="flex h-full flex-col">
      <nav role="tablist" className="flex justify-center gap-1 border-b border-base-300 bg-base-200 p-2">
        {TABS.map((x) => (
          <button
            key={x.key}
            role="tab"
            aria-selected={tab === x.key}
            className={`flex w-20 flex-col items-center rounded-xl px-2 py-1 text-xs ${tab === x.key ? "bg-secondary font-semibold text-secondary-content" : "text-fg-muted"}`}
            onClick={() => setTab(x.key)}
          >
            <span className="text-base" aria-hidden>{x.icon}</span>
            {t(`settings.tab.${x.key}`)}
          </button>
        ))}
      </nav>
      <div className="flex-1 overflow-y-auto px-5 py-3">
        <Body />
      </div>
    </div>
  );
}
```

`src/main.tsx`에서 설정 창을 연결한다.

```tsx
const SettingsWindow = React.lazy(() => import("./pages/SettingsWindow"));
// …
      {label === "main" && <MainWindow />}
      {label === "settings" && <SettingsWindow />}
```

- [ ] **Step 4: 확인**

Run: `yarn test && yarn build && cargo test -p kiri-app`
Expected: 통과.

Run: `yarn tauri dev`

확인할 것:
1. ⌘,(또는 ⚙)로 설정 창이 뜬다. 탭 네 개가 보이고 창 제목이 현지화되어 있다.
2. 일반 탭: 언어를 English로 바꾸면 메인 창, 설정 창, 메뉴 막대 메뉴가 모두 영어로 바뀐다. 테마 다크/라이트가 즉시 반영된다. "창을 닫으면 메뉴 막대로 숨기기"를 끄고 메인 창을 닫으면 앱이 종료된다(다운로드 중이면 확인 대화상자가 뜬다).
3. 다운로드 탭: "변경…"으로 폴더를 고르면 경로가 바뀌고, 다음 다운로드부터 그 폴더에 저장된다. 동시 다운로드를 1로 바꾸면 이후 작업이 하나씩 진행된다. "옵션을 묻지 않고 바로 시작"을 켜면 ⌘V가 시트 없이 바로 큐에 추가한다.
4. CLI 탭: "CLI 설치"를 누르면 관리자 암호를 묻고, 설치 후 터미널에서 `kiri list`가 동작한다. "제거"를 누르면 링크가 사라진다.
5. 업데이트 탭: yt-dlp 버전과 마지막 확인 시각이 보인다. "지금 업데이트"를 누르면 스피너가 돈 뒤 끝난다.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src src
git commit -m "feat(ui): 탭형 환경설정 창과 CLI 설치/제거 추가"
```

---

### Task 19: 릴리즈 도구 — latest.json, 서명·공증 절차 문서

**Files:**
- Create: `scripts/latest-json.mjs`, `src/test/latest-json.test.ts`, `docs/development.md`

**Interfaces:**
- Produces:
  - `buildManifest(tag, sigs, now?) -> { version, pub_date, platforms }` (`scripts/latest-json.mjs`)
  - 릴리즈 절차 문서

- [ ] **Step 1: 실패하는 테스트 작성**

`src/test/latest-json.test.ts`:

```ts
import { describe, it, expect } from "vitest";
// @ts-expect-error -- 타입 없는 .mjs 스크립트
import { buildManifest } from "../../scripts/latest-json.mjs";

describe("buildManifest", () => {
  it("maps the mac .sig to darwin-aarch64", () => {
    const m = buildManifest("v0.2.0", [{ name: "kiri.app.tar.gz.sig", body: "SIG\n" }], new Date("2026-10-06T00:00:00Z"));
    expect(m).toEqual({
      version: "0.2.0",
      pub_date: "2026-10-06T00:00:00.000Z",
      platforms: {
        "darwin-aarch64": {
          signature: "SIG",
          url: "https://github.com/bob-park/kiri-app/releases/download/v0.2.0/kiri.app.tar.gz",
        },
      },
    });
  });
  it("is empty without signatures", () => {
    expect(buildManifest("v0.2.0", []).platforms).toEqual({});
  });
});
```

`vite.config.ts`의 test include는 `src/test/**`이므로 이 파일도 포함된다.

Run: `yarn test`
Expected: FAIL(`../../scripts/latest-json.mjs` 없음).

- [ ] **Step 2: 스크립트 구현**

`scripts/latest-json.mjs`:

```js
#!/usr/bin/env node
// 릴리스에 올라온 .app.tar.gz.sig 로 latest.json 을 만들어 같은 릴리스에 올린다.
// 사용: node scripts/latest-json.mjs v0.2.0
import { execFileSync } from "node:child_process";
import { mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO = "bob-park/kiri-app";
const PLATFORMS = [{ suffix: ".app.tar.gz.sig", key: "darwin-aarch64" }];

/**
 * @param {string} tag
 * @param {{name: string, body: string}[]} sigs
 * @param {Date} [now]
 */
export function buildManifest(tag, sigs, now = new Date()) {
  /** @type {Record<string, {signature: string, url: string}>} */
  const platforms = {};
  for (const { suffix, key } of PLATFORMS) {
    const sig = sigs.find((s) => s.name.endsWith(suffix));
    if (!sig) continue;
    const asset = sig.name.slice(0, -".sig".length);
    platforms[key] = {
      signature: sig.body.trim(),
      url: `https://github.com/${REPO}/releases/download/${tag}/${asset}`,
    };
  }
  return { version: tag.replace(/^v/, ""), pub_date: now.toISOString(), platforms };
}

const isMain = process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1]);
if (isMain) {
  const tag = process.argv[2];
  if (!tag) {
    console.error("usage: node scripts/latest-json.mjs <tag>");
    process.exit(1);
  }
  // 공개키가 비어 있으면 그 빌드는 업데이트를 검증하지 못한다. 받은 뒤에야 실패하므로 여기서 막는다.
  const conf = JSON.parse(readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
  if (!conf.plugins?.updater?.pubkey || conf.plugins.updater.pubkey === "<PUBKEY>") {
    console.error("src-tauri/tauri.conf.json: plugins.updater.pubkey is empty (see docs/development.md)");
    process.exit(1);
  }
  const dir = mkdtempSync(join(tmpdir(), "kiri-sig-"));
  execFileSync("gh", ["release", "download", tag, "-R", REPO, "-p", "*.sig", "-D", dir], { stdio: "inherit" });
  const sigs = readdirSync(dir).map((name) => ({ name, body: readFileSync(join(dir, name), "utf8") }));
  const manifest = buildManifest(tag, sigs);
  if (Object.keys(manifest.platforms).length === 0) {
    console.error(`no .app.tar.gz.sig on release ${tag}`);
    process.exit(1);
  }
  const out = join(dir, "latest.json");
  writeFileSync(out, JSON.stringify(manifest, null, 2) + "\n");
  execFileSync("gh", ["release", "upload", tag, "-R", REPO, out, "--clobber"], { stdio: "inherit" });
  console.log(`latest.json uploaded to ${tag}`);
}
```

Run: `yarn test`
Expected: 통과.

- [ ] **Step 3: 개발·릴리즈 문서**

`docs/development.md`:

````markdown
# kiri 개발 · 릴리즈

빌드는 로컬에서만 한다(CI 없음). Apple Silicon 전용.

## 준비 (한 번)

```sh
yarn install
yarn sidecars          # ffmpeg/ffprobe(scripts/ffmpeg.lock 고정) + kiri CLI 를 src-tauri/binaries 에
```

ffmpeg 버전을 올릴 때: `sh scripts/fetch-ffmpeg.sh --update` → `scripts/ffmpeg.lock` 커밋.

## 개발

```sh
yarn tauri dev
cargo test --workspace && yarn test
```

- yt-dlp·Deno 는 앱이 `~/Library/Application Support/org.bobpark.kiri/bin/` 에 받는다.
- 큐: `~/Library/Application Support/org.bobpark.kiri/queue.json`, 작업 로그: `~/Library/Logs/org.bobpark.kiri/jobs/<id>.log`
- CLI 를 dev 앱에 붙이기: `./target/release/kiri list`

## 서명 정보

`~/.config/kiri/sign.env` (커밋 금지):

```sh
APPLE_SIGNING_IDENTITY="Developer ID Application: <이름> (<TEAM_ID>)"
APPLE_ID="<apple id 이메일>"
APPLE_PASSWORD="<앱 전용 암호>"
APPLE_TEAM_ID="<TEAM_ID>"
TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/kiri.key"
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="<키 암호>"
```

- Developer ID 인증서는 키체인에 있어야 한다(`security find-identity -v -p codesigning`).
- 업데이트 서명 키: `yarn tauri signer generate -w ~/.tauri/kiri.key`. 공개키는 `src-tauri/tauri.conf.json` 의 `plugins.updater.pubkey`.

## 릴리즈

1. 버전 올리기: `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` 의 `version` 을 같은 값으로 (예: 0.2.0) → 커밋.
2. sidecar 갱신: `yarn sidecars`
3. 빌드·서명·`.app` 공증:
   ```sh
   set -a; source ~/.config/kiri/sign.env; set +a
   yarn tauri build
   ```
4. `.dmg` 공증:
   ```sh
   DMG=target/release/bundle/dmg/kiri_0.2.0_aarch64.dmg
   xcrun notarytool submit "$DMG" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" --wait
   xcrun stapler staple "$DMG"
   spctl --assess --type open --context context:primary-signature -v "$DMG"
   ```
5. GitHub Release:
   ```sh
   B=target/release/bundle/macos
   gh release create v0.2.0 "$DMG" "$B/kiri.app.tar.gz" "$B/kiri.app.tar.gz.sig" --title v0.2.0 --notes "…"
   ```
6. `node scripts/latest-json.mjs v0.2.0`

## 확인

- 새로 받은 dmg 를 열어 Gatekeeper 경고 없이 실행되는지.
- 이전 버전 앱에서 설정 > 업데이트 > 지금 확인 → 배너 → 설치 후 재시작 → 새 버전.
````

- [ ] **Step 4: 전체 테스트**

Run: `cargo test --workspace && yarn test && yarn build`
Expected: 모두 통과.

- [ ] **Step 5: 첫 릴리즈와 업데이트 확인 (수동, 서명 정보 필요)**

1. `docs/development.md`의 절차로 `v0.1.0`을 릴리즈한다. dmg를 설치하고 Gatekeeper 경고 없이 실행되는지 확인한다.
2. 서명된 앱에서 YouTube 다운로드 1건과 MP4·HEVC 인코딩이 동작하는지 확인한다. 이것으로 yt-dlp와 Deno가 번들 밖에서 실행될 때 entitlements가 필요 없음을 확인한다. 문제가 생기면 Console.app의 `kiri` 로그와 함께 사용자에게 보고한다.
3. 버전을 `0.1.1`로 올려 다시 릴리즈한 뒤, 설치된 0.1.0에서 설정 > 업데이트 > 지금 확인 → 배너 → 설치 후 재시작 → 0.1.1이 되는지 확인한다.

- [ ] **Step 6: Commit**

```bash
git add scripts/latest-json.mjs src/test/latest-json.test.ts docs/development.md
git commit -m "docs: 서명·공증·릴리즈 절차와 latest.json 생성 스크립트 추가"
```
