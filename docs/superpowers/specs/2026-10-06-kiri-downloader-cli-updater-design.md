# kiri: YouTube 다운로더 · CLI · 자동 업데이트 설계

- 날짜: 2026-10-06
- 브랜치: `feature/youtube-donwload`
- 범위: 코어 + 다운로드, CLI, 자동 업데이트. **AI 요약/하이라이트는 별도 스펙**으로 진행한다.
- 참고 프로젝트: `/Users/hwpark/Documents/rust-workspace/babelay-app` (공증, updater, i18n, 설정 저장 패턴)
- 목업: `2026-10-06-kiri-mockups/` (main-layout, paste-flow, settings, architecture)

## 1. 목표와 전제

### 목표
- Downie 같은 macOS 앱. YouTube 링크를 ⌘V로 붙여넣으면 실제 제공 화질/자막을 고르고 다운로드한다.
- ffmpeg로 원하는 포맷으로 재인코딩한다. 가능하면 하드웨어 가속(VideoToolbox)을 쓴다.
- 다른 앱에서 쓸 수 있는 CLI(`kiri`)로 작업을 조회/추가/삭제/중지한다.
- GitHub Release 기반으로 자동 업데이트를 확인하고 설치한다. 배포물은 서명과 공증을 거친다.

### 전제
- 플랫폼: macOS, Apple Silicon(aarch64)만 지원한다. `minimumSystemVersion` 14.2.
- 개인 도구 성격이다. 배포는 GitHub Release(`bob-park/kiri-app`)로 한다.
- YouTube 처리는 yt-dlp에 맡긴다. 직접 구현하지 않는다.
- 다국어(ko/en/ja)와 테마(시스템/라이트/다크)를 기본 지원한다.

### 범위 밖
- AI 기능(모델 관리, STT/LLM/얼굴인식, 요약, 하이라이트 합본). 다음 스펙에서 다룬다.
- 재생목록 일괄 다운로드. 재생목록 URL은 해당 영상 하나만 받는다.
- 로그인 시 실행, Windows/Intel 지원, CI 빌드.

## 2. 기술 스택

| 영역 | 선택 |
|---|---|
| 앱 | Tauri v2 |
| 프론트엔드 | React 19 + TypeScript + Vite, Tailwind 4, zustand, i18next + react-i18next |
| 패키지 매니저 | yarn 4 (babelay와 동일) |
| 다운로드 | yt-dlp (`yt-dlp_macos`). 런타임에 설치하고 갱신한다 |
| 인코딩 | ffmpeg / ffprobe 정적 arm64 빌드(libx264, libx265, libvpx, lame 포함). sidecar로 번들한다 |
| CLI | Rust, `clap`(derive) |
| 디자인 | `docs/design/kraken-design.md` (Kraken Purple `#7132f5`, 12px radius) |

## 3. 구조

```
kiri-app/
├─ Cargo.toml              # workspace: src-tauri, crates/*
├─ crates/
│  ├─ kiri-core/           # Tauri 비의존: queue, ytdlp, ffmpeg, tools, protocol
│  └─ kiri-cli/            # `kiri` 바이너리 (kiri-core::protocol 사용)
├─ src-tauri/              # Tauri 앱: commands, ipc_server, settings, updater, tray, i18n
├─ src/                    # React 프론트엔드
│  └─ locales/{ko,en,ja}.json
├─ scripts/
│  ├─ fetch-ffmpeg.sh      # sidecar 준비 (체크섬 고정)
│  └─ latest-json.mjs      # updater latest.json 생성/업로드
└─ docs/development.md     # 빌드 · 서명 · 릴리즈 절차
```

- 기존 `src/main.rs`와 루트 `[package]`는 workspace 구성으로 대체한다.
- `kiri-core`는 Tauri에 의존하지 않는다. CLI는 프로토콜 타입만 공유하고, 큐와 파서는 Tauri 없이 테스트한다.
- 큐의 소유자는 앱 프로세스 하나다. UI(Tauri command)와 CLI(소켓)가 같은 `Queue` 핸들을 호출한다. 큐가 바뀌면 `queue-changed` 이벤트를 프론트엔드로 보낸다.

## 4. UI

### 4.1 메인 창 (목업 `main-layout.html` A안)
- Downie 스타일 단일 리스트. **URL 입력창은 없다.**
- 창 어디서든 ⌘V를 누르거나 링크를 드래그하면 추가된다.
- 각 행에는 썸네일, 제목, `화질 · 포맷 · 자막 · 진행 정보`, 진행 바, 일시정지(중지)와 삭제 버튼이 있다.
- 상태 배지: 대기 / 다운로드 중 / 인코딩 중 / 완료 / 실패 / 중지됨.
- 실패와 중지된 행에는 "다시 시작" 버튼이 있다. 완료된 행은 Finder에서 보기를 지원한다.
- 툴바에는 설정(⚙) 버튼이 있다. AI 버튼은 AI 스펙에서 추가한다.
- 앱 업데이트가 있으면 상단에 배너를 띄운다(7.2절).
- yt-dlp가 아직 없으면 "yt-dlp 준비 중…"을 표시하고 그동안 붙여넣기를 막는다.

### 4.2 붙여넣기 이후 옵션 시트 (목업 `paste-flow.html` 1안)
- 붙여넣으면 `probe(url)`를 실행하고, 결과로 시트를 띄운다. 시트 구성:
  - 헤더: 썸네일, 제목, 채널, 길이
  - 화질 목록: YouTube가 실제 제공하는 해상도·fps·코덱·예상 용량, 그리고 "오디오만"
  - 출력 포맷 드롭다운 (5절의 프리셋)
  - 자막 다중 선택. 수동 자막과 자동 생성 자막을 구분하고, 자동 생성 자막에는 "자동" 라벨을 붙인다.
  - "다음부터 이 설정으로 바로 시작" 체크박스
  - 버튼: 취소 / 다운로드
- "바로 시작"이 켜져 있으면 시트 없이 설정의 기본값으로 추가한다. 이 옵션은 설정의 다운로드 탭에서 다시 끌 수 있다.
- probe 중에는 시트에 로딩 상태를 보여준다. 실패하면 시트 대신 에러 토스트를 띄운다(9절).

### 4.3 환경설정 창 (목업 `settings.html` A안)
- ⌘,로 여는 별도 창이다. 상단 아이콘 탭으로 나뉜다.

| 탭 | 항목 |
|---|---|
| 일반 | 언어(시스템/한국어/English/日本語), 테마(시스템/라이트/다크 세그먼트), 창을 닫으면 메뉴 막대로 숨기기(토글, 기본 켬) |
| 다운로드 | 저장 위치(기본 `~/Movies/kiri`, "변경…" 폴더 선택), 기본 화질(최고/1080p/720p/오디오만), 기본 포맷, 기본 자막 언어, 시트 없이 바로 시작(토글), 동시 다운로드 수(1~4, 기본 2), 하드웨어 가속 사용(토글, 기본 켬) |
| CLI | 설치 상태와 경로, "CLI 설치"/"제거" 버튼, 소켓 상태, 사용 예시 |
| 업데이트 | 앱 버전과 "지금 확인", 자동으로 업데이트 확인(토글, 기본 켬), yt-dlp 버전과 "지금 업데이트", 마지막 확인 시각 |

### 4.4 테마
- CSS 변수로 라이트와 다크 토큰을 정의한다. 다크 테마 토큰:

| 토큰 | 값 |
|---|---|
| bg | `#16171c` |
| bg2 | `#1d1e25` |
| fg | `#ececf2` |
| muted | `#8b8ea3` |
| border | `#2c2e38` |
| accent | `#8b5cff` |
| accent-subtle | `rgba(139,92,255,0.22)` |

- "시스템"이면 `prefers-color-scheme`을 따른다. 프론트엔드가 `<html data-theme>`을 갱신한다.

### 4.5 다국어
- 프론트엔드는 i18next를 쓰고, `src/locales/{ko,en,ja}.json`을 정적으로 번들한다. `fallbackLng: "en"`.
- 설정 `ui_language`는 `system | ko | en | ja` 중 하나다. `system`이면 `navigator.language`의 기본 서브태그를 쓰고, 지원하지 않는 언어면 `en`으로 간다.
- 트레이 메뉴와 네이티브 대화상자 문구는 Rust `src-tauri/src/i18n.rs`에 ko/en/ja 문자열로 둔다. 언어는 설정값 또는 `sys_locale`로 정한다.
- 앱이 만든 에러 문구는 i18n 키로 관리한다. yt-dlp와 ffmpeg 원문 메시지는 번역하지 않는다.

### 4.6 트레이(메뉴 막대)와 창 닫기
- 메뉴 막대 아이콘은 항상 표시한다.
- `close_to_tray`가 켜져 있을 때:
  - 창 닫기(빨간 버튼, ⌘W)를 누르면 `CloseRequested`를 막고 창을 `hide()`한다. 이어서 `ActivationPolicy::Accessory`로 바꿔 Dock 아이콘을 숨긴다.
  - 창을 다시 보이면 `Regular`로 바꾼다.
- `close_to_tray`가 꺼져 있을 때: 창을 닫으면 종료 절차로 간다.
- 아이콘을 **더블클릭**하면 메인 창을 열고 포커스한다. Tauri는 macOS에서 `TrayIconEvent::DoubleClick`을 내보내지 않는다(babelay `tray.rs`에서 확인). 그래서 왼쪽 클릭(Up) 두 번이 400ms 안에 오면 더블클릭으로 판정한다.
- 우클릭으로 여는 메뉴(왼쪽 클릭은 더블클릭 판정에 쓰므로 메뉴를 띄우지 않는다): "kiri 열기", 실행 중 작업 요약(예: "다운로드 중 2개 · 48%", 비활성 항목), "업데이트 설치"(업데이트가 있을 때만), "업데이트 확인", "종료".
- **종료(⌘Q 또는 메뉴):** 실행 중인 작업이 있으면 확인 대화상자를 띄운다. 종료한 작업은 다음 실행 때 이어서 진행한다.
- **검증 항목:** ⌘Q가 `RunEvent::ExitRequested`를 거쳐 종료 확인 대화상자를 띄우는지 수동으로 확인한다.

## 5. 다운로드 파이프라인 (`kiri-core`)

### 5.1 yt-dlp · Deno 관리 (`tools`)
- 설치 위치: `~/Library/Application Support/org.bobpark.kiri/bin/{yt-dlp,deno}` (Tauri `app_data_dir()/bin`).
- **Deno가 필요한 이유:** yt-dlp 2025.11.12부터 YouTube를 제대로 받으려면 외부 JS 런타임이 필요하다. 공식 `yt-dlp_macos`에는 `yt-dlp-ejs`가 들어 있으므로, Deno 바이너리만 받아 `--js-runtimes deno:<경로>`로 넘긴다(참고: yt-dlp/yt-dlp#15012).
- **yt-dlp 갱신:** 앱 시작 시, 그리고 마지막 확인 후 24시간이 지났을 때 확인한다. 업데이트 탭의 "지금 업데이트"로도 실행한다.
  1. `releases/latest/download/SHA2-256SUMS`를 받아 `yt-dlp_macos`의 해시를 얻는다.
  2. 설치된 파일의 SHA256이 같으면 최신이므로 끝낸다. GitHub API나 버전 문자열 비교는 쓰지 않는다.
  3. 다르면 `releases/latest/download/yt-dlp_macos`를 받아 해시를 검증한다.
  4. `bin/yt-dlp.tmp`로 저장하고 chmod 755를 한 뒤 원자적으로 rename한다.
- **Deno 설치:** 없을 때만 설치한다(이후 자동 갱신은 하지 않는다).
  1. `denoland/deno` `releases/latest/download/deno-aarch64-apple-darwin.zip`과 `.zip.sha256sum`을 받아 검증한다.
  2. `/usr/bin/ditto -x -k`로 압축을 푼다.
- rename은 실행 중인 프로세스가 이미 연 파일에 영향을 주지 않는다. 그래서 작업이 실행 중이어도 바로 교체한다.
- reqwest로 받은 파일에는 quarantine 속성이 붙지 않으므로 Gatekeeper에 막히지 않는다.

### 5.2 probe
- `yt-dlp -J --no-playlist <url>`을 실행한다.
- 결과에서 제목, 채널, 길이, 썸네일, 화질 목록, 수동 자막 언어, 자동 생성 자막 언어를 추출한다.
- **화질 목록:** 비디오 포맷 중 해상도·fps별로 최적 하나씩 고른다. 예상 용량은 `filesize` 또는 `filesize_approx` + bestaudio.

### 5.3 다운로드
```
yt-dlp -f <video_format_id>+bestaudio        # 오디오만이면 -f bestaudio
  --no-playlist --ffmpeg-location <sidecar dir>
  --newline --progress-template "download:KIRI|%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s"
  [--write-subs] [--write-auto-subs] --sub-langs <langs> --convert-subs srt
  -o <cache>/jobs/<id>/%(title)s.%(ext)s <url>
```
- stdout에서 `KIRI|`로 시작하는 줄만 파싱해 진행률, 속도, ETA를 갱신한다.
- 비디오와 오디오가 따로 받아지므로 진행률은 단계별 퍼센트를 그대로 표시한다.
- 자막은 영상 옆에 별도 `.srt` 파일로 저장한다. 결과 폴더로 함께 옮긴다.

### 5.4 인코딩 프리셋 (`ffmpeg`)
- 비디오 프리셋은 입력 앞에 `-hwaccel videotoolbox`를 붙인다. 지원하지 않는 코덱은 ffmpeg가 소프트웨어 디코딩으로 처리한다.
- 진행률은 `-progress pipe:1 -nostats`의 `out_time_us`를 probe의 길이로 나눈 값이다.
- VideoToolbox 인코딩이 에러로 끝나면 소프트웨어 인자로 1회 재시도하고 로그에 남긴다.
- 설정 "하드웨어 가속 사용"이 꺼져 있으면 처음부터 소프트웨어 인자와 `-hwaccel` 없이 실행한다.

| id | 프리셋 | 가속 경로 | 소프트웨어 경로 | 오디오 |
|---|---|---|---|---|
| `original` | 원본 유지 | 인코딩 생략 | 인코딩 생략 | — |
| `mp4-h264` | MP4 · H.264 | `-c:v h264_videotoolbox -q:v 65` | `-c:v libx264 -crf 20 -preset medium` | `-c:a aac -b:a 192k -movflags +faststart` |
| `mp4-hevc` | MP4 · HEVC | `-c:v hevc_videotoolbox -q:v 60 -tag:v hvc1` | `-c:v libx265 -crf 24 -tag:v hvc1` | `-c:a aac -b:a 192k -movflags +faststart` |
| `mov-prores` | MOV · ProRes 422 | `-c:v prores_videotoolbox -profile:v 2` | `-c:v prores_ks -profile:v 2` | `-c:a pcm_s16le` |
| `webm-vp9` | WebM · VP9 | (없음) | `-c:v libvpx-vp9 -crf 32 -b:v 0 -row-mt 1` | `-c:a libopus` |
| `mp3` | MP3 | — | `-vn` | `-c:a libmp3lame -q:a 2` |
| `m4a` | M4A | — | `-vn` | `-c:a aac -b:a 192k` |

- `mp3`와 `m4a`는 다운로드 단계에서 `-f bestaudio`만 받는다.
- 원본 코덱이 대상과 같을 때 `-c copy`로 넘기는 최적화는 하지 않는다. 인코딩 속도가 문제가 되면 추가한다.

### 5.5 ffmpeg sidecar
- `scripts/fetch-ffmpeg.sh`가 체크섬이 고정된 arm64 정적 빌드(ffmpeg, ffprobe)를 `src-tauri/binaries/`에 받는다. 이 디렉터리는 git에 넣지 않는다.
- `tauri.conf.json`의 `bundle.externalBin`에 `binaries/ffmpeg`, `binaries/ffprobe`, `binaries/kiri`(CLI)를 등록한다. 앱과 함께 서명된다.

## 6. 작업 큐 (`kiri-core::queue`)

### 6.1 모델
```rust
struct Job {
    id: u64,                     // 1부터 증가. queue.json에 next_id 저장
    url: String,
    title: String,
    thumbnail: Option<String>,
    duration_secs: Option<f64>,
    options: JobOptions,
    state: JobState,
    progress: f32,               // 현재 단계 기준 0.0~1.0
    speed: Option<String>,
    eta: Option<String>,
    output: Option<PathBuf>,
    created_at: DateTime<Utc>,
}
struct JobOptions {
    format_id: Option<String>,   // 비디오 format_id. None이면 오디오만
    preset: Preset,              // 5.4의 id
    subtitles: Vec<String>,      // 언어 코드
    auto_subtitles: bool,
}
enum JobState { Queued, Downloading, Encoding, Completed, Failed(String), Stopped }
```

### 6.2 상태 전이
- 정상 흐름: `Queued → Downloading → (preset != original) Encoding → Completed`
- 실행 중 실패는 `Failed(메시지)`, stop은 `Stopped`가 된다.
- `Stopped`나 `Failed`에서 재시작하면 `Queued`가 된다. 캐시 폴더를 유지하므로 yt-dlp가 `.part`를 이어받는다.
- 동시 실행은 `max_concurrent`(기본 2)로 제한한다. `Downloading`과 `Encoding` 모두 슬롯을 차지한다.
- 슬롯이 비면 `created_at` 순으로 다음 `Queued` 작업을 시작한다.

### 6.3 명령
| 명령 | 동작 |
|---|---|
| `add(url, options)` | 작업을 만들어 `Queued`로 넣는다 |
| `stop(id)` | 자식 프로세스를 kill하고 `Stopped`로 바꾼다. 이미 끝난 작업이면 에러 |
| `remove(id)` | 실행 중이면 stop한 뒤 캐시 폴더를 지우고 목록에서 뺀다. 완료된 결과 파일은 지우지 않는다 |
| `restart(id)` | `Stopped`나 `Failed`인 작업을 `Queued`로 되돌린다 (UI 전용) |
| `list()` / `running()` | 전체 목록 / `Downloading`·`Encoding` 상태인 작업 |

### 6.4 파일
- 작업 중에는 `app_cache_dir/jobs/<id>/`에서 작업한다.
- 완료되면 결과 파일과 `.srt`를 설정의 저장 위치로 옮긴다. 같은 이름이 있으면 ` (1)`, ` (2)` 순으로 붙인다.
- 작업을 시작하기 전에 저장 위치가 존재하고 쓰기 가능한지 검사한다.

### 6.5 영속화
- 상태가 바뀔 때마다 `app_local_data_dir/queue.json`에 저장한다(임시 파일에 쓴 뒤 rename). 진행률만 바뀔 때는 저장하지 않는다.
- 앱을 다시 켜면 `Downloading`이나 `Encoding`이던 작업을 `Queued`로 되돌려 이어서 진행한다.

## 7. 설정과 업데이트

### 7.1 설정 (`src-tauri/settings.rs`)
- serde 구조체로 정의하고 `app_config_dir()/settings.json`에 저장한다. 모든 필드는 `#[serde(default)]`.
- 프론트엔드가 보낸 patch를 깊은 병합(deep merge)한다(babelay의 `merge()`와 같은 방식).
```rust
struct Settings {
    general: General {
        ui_language: String,        // "system"
        theme: String,              // "system" | "light" | "dark"
        close_to_tray: bool,        // true
    },
    download: Download {
        dir: PathBuf,               // ~/Movies/kiri
        quality: String,            // "best" | "1080p" | "720p" | "audio"
        preset: String,             // "original"
        subtitles: Vec<String>,     // []
        skip_sheet: bool,           // false
        max_concurrent: u8,         // 2 (1~4)
        hw_accel: bool,             // true
    },
    update: Update {
        auto_check: bool,           // true
        last_ytdlp_check: Option<DateTime<Utc>>,
    },
}
```

### 7.2 앱 자동 업데이트 (`src-tauri/updater.rs`)
- `tauri-plugin-updater`를 쓴다. 설정:
  - `createUpdaterArtifacts: true`
  - endpoint: `https://github.com/bob-park/kiri-app/releases/latest/download/latest.json`
  - `pubkey`: `yarn tauri signer generate -w ~/.tauri/kiri.key`로 만든 공개키
- `auto_check`가 켜져 있으면 시작 10초 후, 그리고 24시간마다 확인한다.
- 새 버전이 있으면 메인 창 배너와 트레이 메뉴에 "업데이트 설치"를 표시한다. **설치는 항상 사용자가 클릭한다.**
- 실행 중인 작업이 있으면 "지금 재시작"과 "큐가 끝나면 재시작" 중에서 고른다. 뒤쪽을 고르면 큐가 비는 순간 설치하고 재시작한다.

### 7.3 빌드 · 서명 · 공증 · 릴리즈 (로컬, CI 없음)
1. `~/.config/kiri/sign.env`를 준비한다. 값: `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`(앱 전용 암호), `APPLE_TEAM_ID`, `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Developer ID 인증서는 키체인에 있는 것을 쓴다.
2. `scripts/fetch-ffmpeg.sh`로 sidecar를 받는다. `cargo build -p kiri-cli --release`를 실행하고, 결과물을 `src-tauri/binaries/kiri-aarch64-apple-darwin`으로 복사한다.
3. `set -a; source ~/.config/kiri/sign.env; set +a; yarn tauri build`를 실행한다. `.app`과 sidecar가 서명되고, `.app`은 공증과 staple까지 된다.
4. `.dmg`는 직접 처리한다: `xcrun notarytool submit <dmg> --apple-id … --password … --team-id … --wait`, `xcrun stapler staple <dmg>`, `spctl --assess`.
5. `gh release create vX.Y.Z`로 dmg, `.app.tar.gz`, `.app.tar.gz.sig`를 올린다.
6. `node scripts/latest-json.mjs vX.Y.Z`를 실행한다. `.sig`를 내려받아 `darwin-aarch64` 항목으로 `latest.json`을 만들고 `gh release upload --clobber`로 올린다. pubkey가 비어 있으면 중단한다.

- `tauri.conf.json` `bundle.macOS`: `signingIdentity: null`(환경변수에서 읽음), `hardenedRuntime: true`, `minimumSystemVersion: "14.2"`. 앱이 특별한 권한을 쓰지 않으므로 entitlements 파일은 두지 않는다. `tauri.macos.conf.json`의 targets는 `["app","dmg"]`.
- yt-dlp와 Deno는 번들 밖(Application Support)에서 각자의 서명으로 실행된다. 그래서 앱 entitlements에 예외가 필요 없다고 본다. 첫 서명 빌드에서 확인한다.
- 위 절차는 `docs/development.md`에 문서로 남긴다.

## 8. CLI

### 8.1 프로토콜 (`kiri-core::protocol`)
- 연결 하나에 요청 1줄, 응답 1줄을 주고받는다(JSON + `\n`).
```rust
struct Envelope<T> { v: u32, body: T }      // v = 1. 다르면 Error { code: "version_mismatch" }
enum Request {
    Status,
    List,
    Add { url: String, quality: Option<String>, preset: Option<String>, subs: Option<Vec<String>> },
    Remove { id: u64 },
    Stop { id: u64 },
}
enum Response { Jobs { jobs: Vec<Job> }, Added { job: Job }, Ok, Error { code: String, message: String } }  // #[serde(tag = "type")]
```

### 8.2 서버 (`src-tauri/ipc_server.rs`)
- 앱 시작 시 `~/Library/Application Support/org.bobpark.kiri/kiri.sock`(환경변수 `KIRI_SOCKET`으로 바꿀 수 있음)에 바인드한다. 먼저 연결을 시도해 보고, 다른 인스턴스가 응답하면 바인드하지 않는다. 응답이 없는 파일만 지우고 다시 바인드한다. 권한은 0600.
- `Add`는 서버에서 probe를 수행하고 화질을 해석한 뒤 큐에 넣는다.
- **화질 해석:**
  - `best`: 가장 높은 화질
  - `audio`: 오디오만
  - `1080p` 등: 같은 높이 중 최고. 없으면 그보다 낮은 화질 중 최고. 더 낮은 화질도 없으면 가장 낮은 화질
  - 생략하면 설정 기본값
- 바인드에 실패해도 앱은 정상 동작한다. CLI 탭에 이유를 표시한다.

### 8.3 명령어 (`crates/kiri-cli`)
```
kiri status            # 실행 중 작업
kiri list              # 전체 큐 (실행 중 포함)
kiri add <url> [--quality best|1080p|720p|audio]
               [--format original|mp4-h264|mp4-hevc|mov-prores|webm-vp9|mp3|m4a]
               [--subs ko,en]
kiri remove <id>
kiri stop <id>
공통 옵션: --json
```
- 기본 출력은 표 형식(id, 제목, 상태, 진행률, 속도, ETA)이다. `--json`이면 응답 body를 그대로 출력한다.
- **종료 코드:** 성공 0, 요청 에러 1, 앱 연결 실패 2.
- **앱이 꺼져 있을 때:**
  - `add`: `open -b <bundle id>`로 앱을 실행하고, 소켓이 열릴 때까지 최대 10초 기다린 뒤 요청한다.
  - 그 외 명령: "kiri 앱이 실행 중이 아닙니다"를 출력하고 종료 코드 2로 끝낸다.

### 8.4 설치
- `kiri` 바이너리는 sidecar로 번들된다.
- CLI 탭의 "CLI 설치"를 누르면 `/usr/local/bin/kiri`에 번들 안 바이너리로 가는 심볼릭 링크를 만든다. 권한이 필요하면 `osascript … with administrator privileges`로 처리한다.
- "제거"는 링크를 지운다.

## 9. 에러 처리

| 상황 | 처리 |
|---|---|
| 붙여넣은 값이 YouTube URL이 아님 | 작업을 만들지 않고 토스트 "YouTube 링크가 아닙니다" |
| 재생목록 URL | `--no-playlist`로 해당 영상만 처리 |
| probe 실패 (비공개, 연령 제한, 삭제, 네트워크) | yt-dlp stderr 마지막 줄로 에러 토스트. CLI는 종료 코드 1 |
| 다운로드나 인코딩 실패 | `Failed(메시지)`, 빨간 배지와 "다시 시작" 버튼. stderr 마지막 20줄을 `app_log_dir/jobs/<id>.log`에 저장 |
| VideoToolbox 인코딩 실패 | 소프트웨어 인자로 1회 재시도. 그래도 실패하면 `Failed` |
| yt-dlp 설치나 업데이트 실패 | 기존 바이너리가 있으면 그대로 쓰고 업데이트 탭에 경고. 바이너리가 없으면 메인 창에 "재시도" 버튼 |
| 저장 위치가 없거나 쓰기 불가 | `Failed("저장 위치에 쓸 수 없습니다")`와 설정으로 가는 링크 |
| 디스크 부족 | 도구 에러를 그대로 `Failed`로 표시. 사전 용량 계산은 하지 않음 |
| 소켓 바인드 실패 | 앱은 정상 동작. CLI 탭에 "CLI 연결 불가: <이유>" |
| 앱 업데이트 확인 실패 | 조용히 로그만 남김. "지금 확인"으로 실행했을 때만 에러 표시 |

## 10. 테스트

### kiri-core 단위 테스트
- 큐 상태 전이: 동시 실행 제한, stop, remove, restart, 재시작 시 복원
- yt-dlp `KIRI|` 진행률 파서
- `-J` JSON에서 화질과 자막 목록 추출. 실제 출력을 fixture로 저장해 쓴다
- ffmpeg `-progress` 파서
- 화질 해석 규칙 (8.2)
- 프리셋별 ffmpeg 인자 생성 (가속 켬/끔)
- 프로토콜 직렬화 왕복과 버전 불일치

### 프로세스 경계
- yt-dlp와 ffmpeg 실행 파일 경로를 주입할 수 있게 한다.
- 정해진 진행률 줄을 출력하고 지정한 종료 코드로 끝나는 가짜 스크립트로 파이프라인 전체를 테스트한다. 대상: 성공, 실패, 가속 실패 후 폴백, stop

### CLI
- 임시 소켓에 core 큐를 붙인 서버를 띄우고 `kiri --json list/add/stop/remove`를 왕복 테스트한다.

### 프론트엔드 (vitest)
- 스토어 동작
- ko, en, ja 로케일 파일의 키 집합이 같은지 검사

### 수동 검증
- 실제 YouTube 다운로드 1건
- 각 프리셋 인코딩 (가속 켬/끔)
- 트레이 더블클릭과 close-to-tray
- 서명·공증된 빌드의 Gatekeeper 통과
- 업데이트 1회 (0.1.0 → 0.1.1)
