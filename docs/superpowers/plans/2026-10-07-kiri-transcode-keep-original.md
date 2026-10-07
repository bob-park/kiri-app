# kiri 원본 유지 변환 · CLI 트랜스코딩 · 업데이트 키 검사 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 포맷을 바꿀 때 원본을 유지하고 `{원본}-{index}.{확장자}` 변환본을 추가로 만든다. 로컬 파일을 앱 큐에서 변환하는 `kiri transcode` 명령을 추가한다. 업데이터 공개키가 빈 릴리즈 빌드를 막는다.

**Architecture:**
- `Job`에 `source: JobSource`(`Youtube { url }` 또는 `File { path, output_dir }`)를 둔다. 큐, 동시 실행, 중지·재시작·삭제, 앱 목록, CLI는 그대로 재사용한다.
- 파이프라인은 출처에 따라 갈라진다. Youtube는 다운로드 → 인코딩, File은 바로 인코딩한다. 저장할 때는 원본을 그대로 두고 변환본을 `variant_path`로 만든다.
- CLI는 경로를 절대 경로로 바꿔 `Request::Transcode`를 보내고, 프로토콜은 v2가 된다.

**Tech Stack:** Rust 2024 (kiri-core, kiri-cli, Tauri v2 app), tokio, serde_json, clap, React 19 + TypeScript, vitest, i18next.

**Spec:** `docs/superpowers/specs/2026-10-07-kiri-transcode-keep-original-design.md`

## Global Constraints

- 작업 브랜치는 `feature/transcoding`이다. 브랜치를 새로 만들지 않는다.
- 변환본 이름은 `{원본의 최종 stem}-{index}.{확장자}`이다. index는 같은 폴더에서 `{stem}-{숫자}.*`(확장자 무관, 디렉터리 포함)의 최댓값 + 1이고, 없으면 1이다.
- 원본 파일(로컬 변환의 입력)은 읽기만 하고, 옮기거나 쓰지 않는다.
- 화질 상한은 `original | 2160p | 1440p | 1080p | 720p | 480p`이고 기본값은 `original`이다. ffmpeg 필터는 `scale=-2:'min(<높이>,ih)'`이고 비디오 프리셋에만 적용한다.
- `kiri transcode`의 `--format`은 필수이고 `original`은 받지 않는다.
- `PROTOCOL_VERSION = 2`.
- 새 오류 코드는 `source_not_found`, `invalid_media`이다. ko/en/ja 문구가 모두 있어야 한다(locales 키 집합 일치 테스트가 있다).
- 결과 폴더가 없으면 작업 시작 시 만든다(`files::check_writable`). 만들 수 없거나 쓸 수 없으면 `Failed("error.download_dir_unwritable")`.
- 릴리즈 프로필에서 `plugins.updater.pubkey`가 비어 있거나 `<PUBKEY>`이면 빌드를 실패시킨다. 메시지: `updater pubkey is empty — see docs/development.md`.
- 커밋 메시지 끝에는 아래 두 줄을 붙인다.
  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01C5HD9GpJXcxovP5MWBjd4t
  ```
- 새로 clone한 상태라면 `cargo` 명령 전에 `yarn sidecars`가 필요하다. 이미 되어 있다면 다시 실행할 필요는 없다.

## Review Focus

1. **공백·한글이 든 상대 경로:** `kiri transcode "./내 영상 01.mkv"`는 절대 경로로 바뀌어 정상 처리되어야 한다. Task 7 `transcode_sends_absolute_path_with_spaces`에서 확인한다.
2. **v0.1.1에서 업데이트한 직후의 `queue.json`(`url`만 있음):** 기존 작업이 그대로 보여야 한다. Task 3 `loads_legacy_url_jobs`에서 확인한다.
3. **같은 파일을 연달아 두 번 변환(동시 실행 2):** `-1`과 `-2`가 따로 생겨야 하고 서로 덮어쓰면 안 된다. Task 6 `two_transcodes_of_same_file_get_distinct_indices`에서 확인한다.
4. **큐에서 기다리는 동안 원본이 지워짐:** 앱이 죽지 않고 해당 작업만 `Failed`가 되어야 한다. Task 6 `source_deleted_before_start_fails_job`에서 확인한다.
5. **변환 재시작:** 이전 부분 출력이 섞이지 않고 처음부터 다시 만들어야 한다. Task 6 `restart_file_job_reencodes_from_scratch`에서 확인한다.

## File Structure

| 파일 | 변경 |
|---|---|
| `src-tauri/src/pubkey_check.rs` | 새 파일. `pubkey_missing(conf_json) -> bool` |
| `src-tauri/build.rs`, `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs` | 릴리즈 빌드 검사, build-dependency `serde_json`, 테스트용 모듈 등록 |
| `crates/kiri-core/src/files.rs` | `next_variant_index`, `variant_path` |
| `crates/kiri-core/src/model.rs` | `JobSource`, `Job.source`/`NewJob.source`, `JobOptions.max_height`, `Tools.ffprobe`, `Job::result_dir` |
| `crates/kiri-core/src/queue.rs` | 예전 `url` 작업 마이그레이션 |
| `crates/kiri-core/src/ffmpeg.rs` | `encode_args`의 `max_height` 인자 |
| `crates/kiri-core/src/pipeline.rs` | Youtube 원본 유지, File 변환 분기 |
| `crates/kiri-core/src/engine.rs` | `add` 출처 검증, `add_file`, `pump`/`spawn_job`의 File 처리, 새 오류 |
| `crates/kiri-core/src/ipc.rs` | `Request::Transcode`, v2 |
| `crates/kiri-cli/src/main.rs`, `crates/kiri-cli/tests/cli.rs` | `transcode` 명령 |
| `crates/kiri-core/src/testutil.rs`, 각 테스트의 `Job`/`Tools` 리터럴 | 새 필드 반영 |
| `src-tauri/src/lib.rs` | `Tools.ffprobe = sidecar("ffprobe")` |
| `src/lib/types.ts`, `src/lib/sheet.ts`, `src/lib/format.ts`, `src/components/JobRow.tsx`, `src/components/Icon.tsx`, `src/locales/*.json`, `src/pages/settings/CliTab.tsx`, 관련 테스트 | 프론트엔드 |
| `README.md` | CLI 절 |

---

### Task 1: 업데이터 공개키 릴리즈 빌드 검사

**Files:**
- Create: `src-tauri/src/pubkey_check.rs`
- Modify: `src-tauri/build.rs`, `src-tauri/Cargo.toml` (`[build-dependencies]`), `src-tauri/src/lib.rs` (모듈 등록)

**Interfaces:**
- Produces: `pub fn pubkey_missing(conf_json: &str) -> bool` (설정 JSON 문자열 → 공개키가 비어 있거나 `<PUBKEY>`이면 true. JSON을 읽을 수 없어도 true)

- [ ] **Step 1: 실패하는 테스트 작성**

`src-tauri/src/pubkey_check.rs`:

```rust
//! 릴리즈 빌드에서 업데이터 공개키가 빠지는 실수를 막는다. build.rs 와 테스트가 같이 쓴다.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_missing_pubkey() {
        assert!(pubkey_missing(r#"{"plugins":{"updater":{"pubkey":""}}}"#));
        assert!(pubkey_missing(r#"{"plugins":{"updater":{"pubkey":"<PUBKEY>"}}}"#));
        assert!(pubkey_missing(r#"{"plugins":{}}"#));
        assert!(pubkey_missing("not json"));
        assert!(!pubkey_missing(r#"{"plugins":{"updater":{"pubkey":"dW50cnVzdGVk"}}}"#));
    }
}
```

`src-tauri/src/lib.rs`의 `mod` 목록에 추가한다(앱 코드는 쓰지 않으므로 테스트에서만 컴파일).

```rust
#[cfg(test)]
mod pubkey_check;
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test -p kiri-app pubkey_check`
Expected: 컴파일 에러 `cannot find function pubkey_missing`.

- [ ] **Step 3: 구현**

`pubkey_check.rs`의 테스트 모듈 위에 추가한다.

```rust
pub fn pubkey_missing(conf_json: &str) -> bool {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(conf_json) else {
        return true;
    };
    match v.pointer("/plugins/updater/pubkey").and_then(|k| k.as_str()) {
        Some(k) => k.trim().is_empty() || k == "<PUBKEY>",
        None => true,
    }
}
```

`src-tauri/Cargo.toml`의 `[build-dependencies]`에 `serde_json = "1"`을 추가한다.

`src-tauri/build.rs` 전체:

```rust
#[path = "src/pubkey_check.rs"]
mod pubkey_check;

fn main() {
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        let conf = std::fs::read_to_string("tauri.conf.json").unwrap_or_default();
        if pubkey_check::pubkey_missing(&conf) {
            panic!("updater pubkey is empty — see docs/development.md");
        }
    }
    tauri_build::build()
}
```

- [ ] **Step 4: 통과 확인**

Run: `cargo test -p kiri-app pubkey_check && cargo build -p kiri-app`
Expected: 1 passed. 디버그 빌드도 성공한다.

빌드 검사 자체도 확인한다. 공개키를 잠시 비우고 릴리즈 빌드를 돌려 패닉 메시지를 보고, 바로 되돌린다.

```bash
cp src-tauri/tauri.conf.json /tmp/kiri-conf.bak
sed -i '' -E 's|"pubkey": "[^"]*"|"pubkey": ""|' src-tauri/tauri.conf.json
cargo build -p kiri-app --release 2>&1 | grep -m1 "updater pubkey is empty"
cp /tmp/kiri-conf.bak src-tauri/tauri.conf.json && git diff --exit-code src-tauri/tauri.conf.json
```

Expected: 메시지 한 줄이 출력되고, 마지막 `git diff`는 변경 없음(종료 코드 0).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/pubkey_check.rs src-tauri/build.rs src-tauri/Cargo.toml src-tauri/src/lib.rs Cargo.lock
git commit -m "build: 업데이터 공개키가 비면 릴리즈 빌드를 실패시킨다"
```

---

### Task 2: 변환본 이름 규칙 (`files.rs`)

**Files:**
- Modify: `crates/kiri-core/src/files.rs`

**Interfaces:**
- Produces:
  - `pub fn next_variant_index(dir: &Path, stem: &str) -> u32`
  - `pub fn variant_path(dir: &Path, stem: &str, ext: &str) -> PathBuf` (`dir/{stem}-{index}.{ext}`)

- [ ] **Step 1: 실패하는 테스트 작성**

`files.rs`의 `#[cfg(test)] mod tests` 안에 추가한다.

```rust
    #[test]
    fn variant_index_counts_any_extension_and_dirs() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(next_variant_index(d.path(), "제목"), 1);
        assert_eq!(variant_path(d.path(), "제목", "mp4"), d.path().join("제목-1.mp4"));
        fs::write(d.path().join("제목.webm"), "").unwrap();
        fs::write(d.path().join("제목-1.mp4"), "").unwrap();
        fs::create_dir(d.path().join("제목-3.kiripart")).unwrap(); // 진행 중인 변환도 센다
        assert_eq!(next_variant_index(d.path(), "제목"), 4);
        assert_eq!(variant_path(d.path(), "제목", "mov"), d.path().join("제목-4.mov"));
    }

    #[test]
    fn variant_index_ignores_lookalike_names() {
        let d = tempfile::tempdir().unwrap();
        for n in ["제목2-5.mp4", "제목-1a.mp4", "제목-.mp4", "제목-x-9.mp4", "other-7.mp4"] {
            fs::write(d.path().join(n), "").unwrap();
        }
        assert_eq!(next_variant_index(d.path(), "제목"), 1);
        fs::write(d.path().join("a-b-3.mp4"), "").unwrap(); // stem 에 '-' 가 있어도 된다
        assert_eq!(next_variant_index(d.path(), "a-b"), 4);
        assert_eq!(next_variant_index(d.path(), "a"), 1);
    }
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test -p kiri-core variant_index`
Expected: 컴파일 에러 `cannot find function next_variant_index`.

- [ ] **Step 3: 구현**

`files.rs`에 추가한다(`unique_path` 아래).

```rust
/// dir 안에서 `{stem}-{n}.*` (n ≥ 1, 확장자 무관, 디렉터리 포함) 의 최댓값 + 1. 없으면 1.
pub fn next_variant_index(dir: &Path, stem: &str) -> u32 {
    let prefix = format!("{stem}-");
    let max = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let rest = name.strip_prefix(&prefix)?;
            let num = rest.split_once('.').map_or(rest, |(n, _)| n);
            (!num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()))
                .then(|| num.parse::<u32>().ok())
                .flatten()
        })
        .max()
        .unwrap_or(0);
    max + 1
}

/// `dir/{stem}-{index}.{ext}`
pub fn variant_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    dir.join(format!("{stem}-{}.{ext}", next_variant_index(dir, stem)))
}
```

`"제목-x-9.mp4"`는 `rest = "x-9.mp4"`, `num = "x-9"`이므로 숫자가 아니어서 제외된다. `"a-b-3.mp4"`는 stem `a`의 입장에서 `num = "b-3"`이라 제외되고, stem `a-b`의 입장에서는 3으로 센다.

- [ ] **Step 4: 통과 확인**

Run: `cargo test -p kiri-core variant_index`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/kiri-core/src/files.rs
git commit -m "feat(core): 변환본 이름 규칙 {stem}-{index}.{ext} 추가"
```

---
### Task 3: 작업 출처 `JobSource`, `max_height`, `Tools.ffprobe`, 예전 큐 마이그레이션

동작은 바꾸지 않는 모델 리팩터링이다. 변환 기능은 Task 5·6에서 쓴다.

**Files:**
- Modify:
  - `crates/kiri-core/src/model.rs`, `crates/kiri-core/src/queue.rs`, `crates/kiri-core/src/engine.rs`, `crates/kiri-core/src/pipeline.rs`, `crates/kiri-core/src/testutil.rs`
  - `crates/kiri-core/src/ytdlp.rs`(테스트), `crates/kiri-core/src/ipc.rs`(테스트)
  - `crates/kiri-cli/src/main.rs`(테스트), `crates/kiri-cli/tests/cli.rs`
  - `src-tauri/src/lib.rs`, `src-tauri/src/tray.rs`(테스트), `src-tauri/src/updater.rs`(테스트)

**Interfaces:**
- Produces:
  - `pub enum JobSource { Youtube { url: String }, File { path: PathBuf, output_dir: Option<PathBuf> } }`. serde는 `#[serde(tag = "kind", rename_all = "snake_case")]`이고, JSON은 `{"kind":"youtube","url":…}` 또는 `{"kind":"file","path":…,"output_dir":…}`
  - `Job.source: JobSource`, `NewJob.source: JobSource` (`url` 필드는 없어짐)
  - `JobOptions.max_height: Option<u32>` (`#[serde(default)]`)
  - `Tools.ffprobe: PathBuf`
  - `testutil::tools(...)`가 `ffprobe` 가짜 스크립트(`echo 2.0`)도 만든다
- 리터럴을 만드는 모든 곳에 `max_height: None`과 `source: JobSource::Youtube { url }`을 넣는다.

- [ ] **Step 1: 실패하는 테스트 작성**

`model.rs`의 tests에 추가한다.

```rust
    #[test]
    fn job_source_json_shape() {
        let y = JobSource::Youtube { url: "u".into() };
        assert_eq!(serde_json::to_value(&y).unwrap(), json!({"kind": "youtube", "url": "u"}));
        let f: JobSource = serde_json::from_value(json!({"kind": "file", "path": "/a/b.mkv"})).unwrap();
        assert_eq!(f, JobSource::File { path: "/a/b.mkv".into(), output_dir: None });
    }
```

`queue.rs`의 tests에 추가한다.

```rust
    #[test]
    fn loads_legacy_url_jobs() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("queue.json");
        // v0.1.x 형식: source 없이 url, options 에 max_height 없음
        fs::write(
            &path,
            r#"{"next_id":2,"jobs":[{"id":1,"url":"https://youtu.be/x","title":"t",
               "options":{"format_id":null,"preset":"original","subtitles":[],"auto_subtitles":false},
               "state":{"kind":"completed"}}]}"#,
        )
        .unwrap();
        let (q, trusted) = QueueState::load_checked(&path);
        assert!(trusted);
        assert_eq!(q.jobs.len(), 1);
        assert_eq!(q.jobs[0].source, JobSource::Youtube { url: "https://youtu.be/x".into() });
        assert_eq!(q.jobs[0].options.max_height, None);
        assert!(!path.with_extension("json.bad").exists());
    }
```

`queue.rs` 테스트 모듈 맨 위의 import에 `use crate::model::JobSource;`를 추가한다.

- [ ] **Step 2: 실패 확인**

Run: `cargo test -p kiri-core job_source_json_shape loads_legacy_url_jobs`
Expected: 컴파일 에러(`JobSource` 없음).

- [ ] **Step 3: 모델 구현**

`model.rs`의 `JobOptions` 위에 추가한다.

```rust
/// 작업이 다루는 대상: YouTube 링크를 받거나, 로컬 파일을 변환한다.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JobSource {
    Youtube {
        url: String,
    },
    File {
        path: PathBuf,
        /// None = 원본과 같은 폴더
        #[serde(default)]
        output_dir: Option<PathBuf>,
    },
}
```

`JobOptions`의 마지막 필드 뒤에 추가한다.

```rust
    /// 변환 해상도 상한(높이). None 이면 원본 해상도.
    #[serde(default)]
    pub max_height: Option<u32>,
```

`Job`과 `NewJob`의 `pub url: String,`을 `pub source: JobSource,`로 바꾼다.

`Tools`에 `pub ffprobe: PathBuf,`를 추가한다(`ffmpeg` 다음).

- [ ] **Step 4: 큐 마이그레이션 구현**

`queue.rs`의 `load_checked`에서 파싱 부분을 바꾼다. 기존 `Ok(text) => match serde_json::from_str(&text) {`를 아래로 교체한다(나머지 match 팔은 그대로).

```rust
            Ok(text) => match serde_json::from_str::<serde_json::Value>(&text).and_then(|mut v| {
                migrate_legacy(&mut v);
                serde_json::from_value(v)
            }) {
```

`impl QueueState` 위에 추가한다.

```rust
/// v0.1.x queue.json: 작업에 `source` 대신 `url` 만 있다. YouTube 작업으로 바꿔 읽는다.
fn migrate_legacy(v: &mut serde_json::Value) {
    let Some(jobs) = v.get_mut("jobs").and_then(|j| j.as_array_mut()) else {
        return;
    };
    for job in jobs.iter_mut().filter_map(|j| j.as_object_mut()) {
        if !job.contains_key("source") {
            if let Some(url) = job.remove("url") {
                job.insert("source".into(), serde_json::json!({"kind": "youtube", "url": url}));
            }
        }
    }
}
```

`QueueState::add`의 `url: new.url,`을 `source: new.source,`로 바꾼다.

- [ ] **Step 5: 컴파일 오류가 나는 곳 고치기**

`cargo build --workspace --tests 2>&1 | grep -E "^error" -A6`로 위치를 보면서 아래를 반영한다.

`engine.rs`의 `add`:

```rust
    pub fn add(&self, new: NewJob) -> Result<Job, EngineError> {
        match &new.source {
            JobSource::Youtube { url } if ytdlp::is_youtube_url(url) => {}
            _ => return Err(EngineError::InvalidUrl),
        }
        // (이하 기존 본문 그대로)
```

`engine.rs`의 `add_url`이 만드는 `NewJob`에서 `url: url.trim().to_string(),`을 `source: JobSource::Youtube { url: url.trim().to_string() },`로 바꾸고, `JobOptions`에 `max_height: None,`을 추가한다. `use crate::model::{...}`에 `JobSource`를 추가한다.

`pipeline.rs`의 `run`에서 `ytdlp::download_args(&job.url, ...)` 직전에 넣는다. Task 6에서 이 분기를 실제 변환으로 바꾼다.

```rust
    let url = match &job.source {
        JobSource::Youtube { url } => url.as_str(),
        JobSource::File { .. } => {
            return Err(PipelineError::Failed("file transcode is not supported yet".into()));
        }
    };
```

그리고 `download_args(&job.url, …)`를 `download_args(url, …)`로 바꾸고, import에 `JobSource`를 추가한다.

테스트 리터럴:
- `testutil::job`, `engine.rs` 테스트의 `new_job`, `queue.rs` 테스트의 `new_job`, `kiri-cli/src/main.rs` 테스트의 `job`, `src-tauri/src/tray.rs`와 `updater.rs` 테스트의 `Job`은 모두 이렇게 바꾼다.
  - `url: "…".into(),`를 `source: JobSource::Youtube { url: "…".into() },`로 바꾼다.
  - `JobOptions { … }`에 `max_height: None,`을 추가한다.
  - 각 파일의 import에 `JobSource`를 추가한다.
- `engine.rs` 테스트 `add_rejects_non_youtube_url`의 `j.url = "--exec=touch /tmp/pwned".into();`를 `j.source = JobSource::Youtube { url: "--exec=touch /tmp/pwned".into() };`로 바꾼다.
- `ytdlp.rs` 테스트의 `JobOptions { … }` 세 곳에 `max_height: None,`을 추가한다.

`Tools` 리터럴:
- `ytdlp.rs` 테스트 `tools()`, `ipc.rs` 테스트 `idle_engine`, `kiri-cli/tests/cli.rs`의 `idle_engine`에는 `ffprobe: …`를 같은 패턴으로 추가한다. 예: `ffprobe: dir.join("none/ffprobe")`, ytdlp 테스트는 `"/app/MacOS/ffprobe".into()`.
- `testutil::tools`:
  ```rust
  pub fn tools(dir: &Path, ytdlp_body: &str, ffmpeg_body: &str) -> Tools {
      Tools {
          ytdlp: script(dir, "yt-dlp", ytdlp_body),
          deno: dir.join("deno"),
          ffmpeg: script(dir, "ffmpeg", ffmpeg_body),
          ffprobe: script(dir, "ffprobe", "echo 2.0"),
      }
  }
  ```
- `src-tauri/src/lib.rs`의 `Tools`에 `ffprobe: sidecar("ffprobe"),`를 추가한다.

- [ ] **Step 6: 통과 확인**

Run: `cargo fmt --all && cargo test --workspace`
Expected: 새 테스트 2개를 포함해 모두 통과하고, 경고가 없다.

- [ ] **Step 7: Commit**

```bash
git add crates src-tauri/src
git commit -m "refactor(core): 작업 출처 JobSource 도입, max_height·ffprobe 경로 추가, 예전 queue.json 마이그레이션"
```

---

### Task 4: 화질 상한 ffmpeg 인자 (`ffmpeg.rs`)

**Files:**
- Modify: `crates/kiri-core/src/ffmpeg.rs`, `crates/kiri-core/src/pipeline.rs`(호출부)

**Interfaces:**
- Produces: `pub fn encode_args(p: Preset, hw: bool, max_height: Option<u32>, input: &Path, output: &Path) -> Option<Vec<String>>`. `max_height`이 있고 비디오 프리셋이면 `-vf scale=-2:'min(<h>,ih)'`를 인코더 인자 앞에 넣는다.

- [ ] **Step 1: 실패하는 테스트 작성**

`ffmpeg.rs`의 tests에서 기존 헬퍼 `args`를 아래처럼 바꾸고(기존 호출부는 그대로 동작), 테스트를 추가한다.

```rust
    fn args(p: Preset, hw: bool) -> Vec<String> {
        encode_args(p, hw, None, Path::new("/w/in.webm"), Path::new("/w/out/in.mp4")).unwrap()
    }

    #[test]
    fn max_height_scales_video_only() {
        let a = encode_args(Preset::Mp4H264, true, Some(1080), Path::new("/i.mkv"), Path::new("/o.mp4"))
            .unwrap()
            .join(" ");
        assert!(a.contains("-i /i.mkv -vf scale=-2:'min(1080,ih)' -c:v h264_videotoolbox"), "{a}");
        let mp3 = encode_args(Preset::Mp3, false, Some(720), Path::new("/i.mkv"), Path::new("/o.mp3"))
            .unwrap()
            .join(" ");
        assert!(!mp3.contains("scale"), "{mp3}");
        assert!(!args(Preset::Mp4H264, false).join(" ").contains("scale"));
    }
```

`original_has_no_encode_step` 테스트의 `encode_args(Preset::Original, true, Path::new("a"), Path::new("b"))`는 `encode_args(Preset::Original, true, None, Path::new("a"), Path::new("b"))`로 바꾼다.

- [ ] **Step 2: 실패 확인**

Run: `cargo test -p kiri-core ffmpeg`
Expected: 컴파일 에러(인자 개수 불일치).

- [ ] **Step 3: 구현**

`encode_args`의 시그니처를 바꾸고, `-i input` 다음에 필터를 넣는다.

```rust
pub fn encode_args(
    p: Preset,
    hw: bool,
    max_height: Option<u32>,
    input: &Path,
    output: &Path,
) -> Option<Vec<String>> {
    // (기존 match 그대로)
    // ...
    a.extend(["-i".into(), input.display().to_string()]);
    if let Some(h) = max_height.filter(|_| !p.is_audio_only()) {
        // 원본보다 키우지 않고, 너비는 비율에 맞춘 짝수
        a.extend(["-vf".into(), format!("scale=-2:'min({h},ih)'")]);
    }
    a.extend(video.iter().chain(audio).map(|s| s.to_string()));
    // (이하 그대로)
```

`pipeline.rs`의 `encode_once`에서 호출부를 바꾼다.

```rust
    let args = ffmpeg::encode_args(job.options.preset, hw, job.options.max_height, input, output)
        .expect("preset with extension");
```

- [ ] **Step 4: 통과 확인**

Run: `cargo test -p kiri-core`
Expected: 모두 통과.

- [ ] **Step 5: Commit**

```bash
git add crates/kiri-core/src/ffmpeg.rs crates/kiri-core/src/pipeline.rs
git commit -m "feat(core): 변환 해상도 상한(scale=-2:min(h,ih)) 인자 추가"
```

---

### Task 5: YouTube 다운로드에서 원본 유지 + 변환본 추가

**Files:**
- Modify: `crates/kiri-core/src/pipeline.rs`

**Interfaces:**
- Consumes: `files::variant_path` (Task 2)
- Produces: 원본 유지가 아닌 프리셋이면 `run`이 **변환본 경로**를 돌려준다. 원본은 `download_dir`에 남는다.

- [ ] **Step 1: 실패하는 테스트 작성 (기존 기대값 갱신 + 추가)**

`pipeline.rs` tests에서 바꾸고 추가한다.

```rust
    #[tokio::test]
    async fn encodes_when_preset_needs_it() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, true);
        let (r, reports) = go(&e, Preset::MovProres).await;
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video-1.mov"));
        assert!(e.cfg.download_dir.join("Fake Video.mp4").exists(), "original kept");
        assert!(e.cfg.download_dir.join("Fake Video.ko.srt").exists());
        assert!(
            reports
                .iter()
                .any(|r| r.stage == Stage::Encoding && r.progress == 0.5)
        );
    }

    #[tokio::test]
    async fn second_conversion_gets_next_index() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, true);
        fs::create_dir_all(&e.cfg.download_dir).unwrap();
        fs::write(e.cfg.download_dir.join("Fake Video-1.mov"), "earlier").unwrap();
        let (r, _) = go(&e, Preset::Mp4H264).await;
        // 원본은 같은 이름이 없으니 Fake Video.mp4, 변환본은 기존 -1 다음인 -2
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video-2.mp4"));
        assert!(e.cfg.download_dir.join("Fake Video.mp4").exists());
    }

    #[tokio::test]
    async fn audio_preset_keeps_downloaded_original() {
        let e = env(FAKE_YTDLP_DOWNLOAD, FAKE_FFMPEG, true);
        let (r, _) = go(&e, Preset::Mp3).await;
        assert_eq!(r.unwrap(), e.cfg.download_dir.join("Fake Video-1.mp3"));
        assert!(e.cfg.download_dir.join("Fake Video.mp4").exists());
    }
```

`falls_back_to_software_when_videotoolbox_fails`의 기대값은 `e.cfg.download_dir.join("Fake Video-1.mp4")`로 바꾼다.

- [ ] **Step 2: 실패 확인**

Run: `cargo test -p kiri-core pipeline`
Expected: 위 테스트들이 FAIL. 지금은 `Fake Video.mov`를 돌려주고 원본이 없다.

- [ ] **Step 3: 구현**

`run`의 끝부분(`let media = match encode(...)` 부터)을 아래로 바꾼다.

```rust
    match encode(job, cfg, cancel, &downloaded, &mut report).await? {
        None => finalize(cfg, &downloaded),
        Some(encoded) => {
            // 원본(과 자막)을 먼저 저장 폴더로 옮기고, 그 최종 이름을 기준으로 변환본 이름을 정한다.
            let original = finalize(cfg, &downloaded)?;
            let stem = original
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("video");
            let ext = encoded
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("mp4");
            let dest = files::variant_path(&cfg.download_dir, stem, ext);
            files::move_file(&encoded, &dest).map_err(io_fail)?;
            Ok(dest)
        }
    }
```

- [ ] **Step 4: 통과 확인**

Run: `cargo test -p kiri-core`
Expected: 모두 통과. `existing_file_is_not_overwritten`, `original_downloads_and_moves_with_subtitles`(원본 유지 프리셋)는 그대로 통과한다.

- [ ] **Step 5: Commit**

```bash
git add crates/kiri-core/src/pipeline.rs
git commit -m "feat(core): 포맷을 바꿔 받을 때 원본을 남기고 {원본}-{n} 변환본을 추가"
```

---
### Task 6: 로컬 파일 변환 작업 (`Engine::add_file` + 파이프라인 File 분기)

**Files:**
- Modify: `crates/kiri-core/src/model.rs`, `crates/kiri-core/src/engine.rs`, `crates/kiri-core/src/pipeline.rs`

**Interfaces:**
- Consumes: `JobSource`, `Tools.ffprobe`, `JobOptions.max_height`(Task 3), `encode_args(.., max_height, ..)`(Task 4), `files::next_variant_index`/`variant_path`(Task 2)
- Produces:
  - `impl Job { pub fn result_dir(&self, download_dir: &Path) -> PathBuf }`: 결과 폴더. File이면 `output_dir` 또는 원본 폴더, Youtube면 `download_dir`
  - `pub async fn Engine::add_file(&self, path: PathBuf, output_dir: Option<PathBuf>, preset: &str, quality: Option<String>) -> Result<Job, EngineError>`
  - `EngineError::SourceNotFound(String)` (`code()` = `"source_not_found"`, 메시지 `source not found: {0}`), `EngineError::InvalidMedia(String)` (`"invalid_media"`, 메시지 `{0}`)
  - `Engine::add`는 Youtube 출처만 받는다(File이면 `InvalidUrl`)

- [ ] **Step 1: 실패하는 테스트 작성**

`engine.rs`의 tests에 추가한다(기존 `env`, `engine_at`, `wait_for`, `new_job` 헬퍼 사용).

```rust
    fn source_file(d: &Path, name: &str) -> PathBuf {
        let dir = d.join("clips");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, "orig").unwrap();
        p
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn add_file_validates_input() {
        let e = env("sleep 30", 1);
        let missing = e.d.path().join("nope.mkv");
        assert!(matches!(e.engine.add_file(missing, None, "mp4-h264", None).await, Err(EngineError::SourceNotFound(_))));
        assert!(matches!(
            e.engine.add_file(e.d.path().to_path_buf(), None, "mp4-h264", None).await,
            Err(EngineError::SourceNotFound(_))
        ));
        let src = source_file(e.d.path(), "a.mkv");
        assert!(matches!(e.engine.add_file(src.clone(), None, "original", None).await, Err(EngineError::BadPreset(_))));
        assert!(matches!(e.engine.add_file(src.clone(), None, "avi", None).await, Err(EngineError::BadPreset(_))));
        assert!(matches!(
            e.engine.add_file(src.clone(), None, "mp4-h264", Some("hd".into())).await,
            Err(EngineError::BadQuality(_))
        ));
        testutil::script(&e.d.path().join("bin"), "ffprobe", "echo 'Invalid data found when processing input' >&2; exit 1");
        match e.engine.add_file(src, None, "mp4-h264", None).await {
            Err(EngineError::InvalidMedia(m)) => assert!(m.contains("Invalid data"), "{m}"),
            other => panic!("{other:?}"),
        }
        assert!(e.engine.list().is_empty());
        assert_eq!(EngineError::SourceNotFound("x".into()).code(), "source_not_found");
        assert_eq!(EngineError::InvalidMedia("x".into()).code(), "invalid_media");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn add_rejects_file_source_from_ui_path() {
        let e = env("sleep 30", 1);
        let mut j = new_job("a");
        j.source = JobSource::File { path: "/tmp/x.mkv".into(), output_dir: None };
        assert!(matches!(e.engine.add(j), Err(EngineError::InvalidUrl)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn file_transcode_keeps_source_and_writes_variant() {
        let e = env("sleep 30", 2);
        let src = source_file(e.d.path(), "clip.mkv");
        let before = std::fs::metadata(&src).unwrap().modified().unwrap();
        let j = e.engine.add_file(src.clone(), None, "mp4-h264", Some("720p".into())).await.unwrap();
        assert_eq!(
            (j.title.as_str(), j.quality_label.as_str(), j.options.max_height, j.duration_secs),
            ("clip.mkv", "720p", Some(720), Some(2.0))
        );
        wait_for(&e.engine, |jobs| jobs[0].state == JobState::Completed).await;
        let out = e.d.path().join("clips/clip-1.mp4");
        assert_eq!(e.engine.list()[0].output.as_deref(), Some(out.as_path()));
        assert!(out.exists());
        assert_eq!(std::fs::read_to_string(&src).unwrap(), "orig");
        assert_eq!(std::fs::metadata(&src).unwrap().modified().unwrap(), before);
        assert!(!e.d.path().join("clips/clip-1.kiripart").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn file_transcode_honours_output_dir() {
        let e = env("sleep 30", 1);
        let src = source_file(e.d.path(), "clip.mkv");
        let out_dir = e.d.path().join("converted"); // 없는 폴더는 만든다
        e.engine.add_file(src, Some(out_dir.clone()), "mp3", None).await.unwrap();
        wait_for(&e.engine, |jobs| jobs[0].state == JobState::Completed).await;
        assert!(out_dir.join("clip-1.mp3").exists());
        assert!(!e.d.path().join("clips/clip-1.mp3").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn two_transcodes_of_same_file_get_distinct_indices() {
        let e = env("sleep 30", 2);
        let src = source_file(e.d.path(), "clip.mkv");
        e.engine.add_file(src.clone(), None, "mp4-h264", None).await.unwrap();
        e.engine.add_file(src, None, "mov-prores", None).await.unwrap();
        wait_for(&e.engine, |jobs| jobs.iter().all(|j| j.state == JobState::Completed)).await;
        let clips = e.d.path().join("clips");
        let outs: Vec<_> = e.engine.list().into_iter().map(|j| j.output.unwrap()).collect();
        assert_ne!(outs[0], outs[1]);
        assert!(outs.iter().all(|o| o.exists() && o.parent() == Some(clips.as_path())));
        let mut names: Vec<String> = outs.iter().map(|o| o.file_stem().unwrap().to_string_lossy().into_owned()).collect();
        names.sort();
        assert_eq!(names, vec!["clip-1", "clip-2"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn source_deleted_before_start_fails_job() {
        let e = env("sleep 30", 1);
        let bin = e.d.path().join("bin");
        // ffmpeg 가짜: -i 다음 파일이 없으면 실패
        testutil::script(
            &bin,
            "ffmpeg",
            &format!(
                "prev=\"\"\nfor a; do if [ \"$prev\" = \"-i\" ] && [ ! -f \"$a\" ]; then echo \"$a: No such file or directory\" >&2; exit 1; fi; prev=\"$a\"; done\n{FAKE_FFMPEG}"
            ),
        );
        e.engine.add(new_job("busy")).unwrap(); // 슬롯 점유 (sleep 30)
        let src = source_file(e.d.path(), "clip.mkv");
        e.engine.add_file(src.clone(), None, "mp4-h264", None).await.unwrap();
        std::fs::remove_file(&src).unwrap();
        e.engine.stop(1).unwrap(); // 슬롯을 비워 변환 작업을 시작시킨다
        wait_for(&e.engine, |jobs| matches!(jobs[1].state, JobState::Failed(_))).await;
        let JobState::Failed(m) = &e.engine.list()[1].state else { unreachable!() };
        assert!(m.contains("No such file"), "{m}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn restart_file_job_reencodes_from_scratch() {
        let e = env("sleep 30", 1);
        let bin = e.d.path().join("bin");
        let marker = e.d.path().join("second-run");
        // 첫 실행: 출력에 "partial" 을 쓰고 대기. 두 번째(marker 있음): 정상 완료.
        testutil::script(
            &bin,
            "ffmpeg",
            &format!(
                "for a; do last=\"$a\"; done\nif [ ! -f '{}' ]; then mkdir -p \"$(dirname \"$last\")\"; echo partial > \"$last\"; sleep 30; fi\n{FAKE_FFMPEG}",
                marker.display()
            ),
        );
        let src = source_file(e.d.path(), "clip.mkv");
        e.engine.add_file(src, None, "mp4-h264", None).await.unwrap();
        wait_for(&e.engine, |jobs| jobs[0].state == JobState::Encoding).await;
        let pkg = e.engine.list()[0].work_dir.clone().unwrap();
        e.engine.stop(1).unwrap();
        wait_for(&e.engine, |_| e.engine.running().is_empty() && !e.engine.has_active()).await;
        std::fs::write(&marker, "").unwrap();
        e.engine.restart(1).unwrap();
        wait_for(&e.engine, |jobs| jobs[0].state == JobState::Completed).await;
        let out = e.engine.list()[0].output.clone().unwrap();
        assert_eq!(out, e.d.path().join("clips/clip-1.mp4"));
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "", "must not keep the partial output");
        assert!(!pkg.exists());
    }
```

`engine.rs` 테스트 모듈의 import에 `use std::path::PathBuf;`와 `JobSource`가 없으면 추가한다. `e.engine.running()`이 "실행 중 작업"을 돌려주므로 대기 조건에 쓴다. 이 테스트는 `stop` 뒤 이전 태스크가 끝날 때까지 기다린다.

- [ ] **Step 2: 실패 확인**

Run: `cargo test -p kiri-core engine`
Expected: 컴파일 에러(`add_file`, `SourceNotFound`, `InvalidMedia` 없음).

- [ ] **Step 3: 모델 헬퍼**

`model.rs`에 추가한다.

```rust
impl Job {
    /// 결과물이 놓일 폴더. 파일 변환은 지정 폴더(없으면 원본 폴더), 다운로드는 저장 폴더.
    pub fn result_dir(&self, download_dir: &Path) -> PathBuf {
        match &self.source {
            JobSource::Youtube { .. } => download_dir.to_path_buf(),
            JobSource::File { path, output_dir } => output_dir
                .clone()
                .or_else(|| path.parent().map(Path::to_path_buf))
                .unwrap_or_else(|| download_dir.to_path_buf()),
        }
    }
}
```

`use std::path::{Path, PathBuf};`로 import를 맞춘다.

- [ ] **Step 4: 엔진 구현**

`EngineError`에 두 가지를 추가하고 `code()`에도 반영한다.

```rust
    #[error("source not found: {0}")]
    SourceNotFound(String),
    #[error("{0}")]
    InvalidMedia(String),
```
```rust
            EngineError::SourceNotFound(_) => "source_not_found",
            EngineError::InvalidMedia(_) => "invalid_media",
```

`add`를 검증과 삽입으로 나눈다.

```rust
    pub fn add(&self, new: NewJob) -> Result<Job, EngineError> {
        match &new.source {
            JobSource::Youtube { url } if ytdlp::is_youtube_url(url) => {}
            _ => return Err(EngineError::InvalidUrl),
        }
        Ok(self.insert(new))
    }

    fn insert(&self, new: NewJob) -> Job {
        let job = self.0.state.lock().unwrap().add(new, now_ms());
        self.save();
        self.notify();
        self.pump();
        // pump 가 같은 호출 안에서 상태를 바꿨을 수 있다. 최신 상태를 돌려준다.
        self.list()
            .into_iter()
            .find(|j| j.id == job.id)
            .unwrap_or(job)
    }

    /// CLI `transcode`: 로컬 파일을 변환하는 작업. 원본은 읽기만 한다.
    pub async fn add_file(
        &self,
        path: PathBuf,
        output_dir: Option<PathBuf>,
        preset: &str,
        quality: Option<String>,
    ) -> Result<Job, EngineError> {
        if !path.is_file() {
            return Err(EngineError::SourceNotFound(path.display().to_string()));
        }
        let preset = preset
            .parse::<Preset>()
            .ok()
            .filter(|p| *p != Preset::Original)
            .ok_or_else(|| EngineError::BadPreset(preset.to_string()))?;
        let quality = quality.unwrap_or_else(|| "original".into());
        let max_height = match quality.as_str() {
            "original" => None,
            q => Some(
                q.strip_suffix('p')
                    .and_then(|n| n.parse::<u32>().ok())
                    .filter(|h| *h > 0)
                    .ok_or_else(|| EngineError::BadQuality(quality.clone()))?,
            ),
        };
        let duration_secs = self.media_duration(&path).await?;
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();
        Ok(self.insert(NewJob {
            source: JobSource::File { path, output_dir },
            title,
            thumbnail: None,
            duration_secs,
            quality_label: quality,
            options: JobOptions {
                format_id: None,
                preset,
                subtitles: vec![],
                auto_subtitles: false,
                max_height,
            },
        }))
    }

    /// ffprobe 로 길이를 읽는다. 읽지 못하면 미디어가 아니라고 본다. 길이가 N/A 면 None.
    async fn media_duration(&self, path: &Path) -> Result<Option<f64>, EngineError> {
        let args: Vec<String> = ["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", "-i"]
            .iter()
            .map(|s| s.to_string())
            .chain([path.display().to_string()])
            .collect();
        let out = runner::output(&self.0.tools.ffprobe, &args)
            .await
            .map_err(|e| EngineError::InvalidMedia(e.summary()))?;
        Ok(out.trim().parse::<f64>().ok())
    }
```

`pump`에서 상태와 작업 폴더를 정하는 부분을 바꾼다.

```rust
                let job = st.get_mut(id).expect("queued job exists");
                job.state = match job.source {
                    JobSource::File { .. } => JobState::Encoding,
                    JobSource::Youtube { .. } => JobState::Downloading,
                };
                job.progress = 0.0;
                let out_dir = job.result_dir(&cfg.download_dir);
                // 저장된 패키지가 없거나(지워짐·만들지 못함) 표식이 없으면 결과 폴더에 새로 만든다.
                let work_dir = match &job.work_dir {
                    Some(d) if files::is_part_dir(d) => d.clone(),
                    _ => {
                        let name = match &job.source {
                            JobSource::Youtube { .. } => {
                                format!("{}.{}", files::safe_title(&job.title), files::PART_EXT)
                            }
                            JobSource::File { path, .. } => {
                                let stem = files::safe_title(
                                    path.file_stem().and_then(|s| s.to_str()).unwrap_or("file"),
                                );
                                let n = files::next_variant_index(&out_dir, &stem);
                                format!("{stem}-{n}.{}", files::PART_EXT)
                            }
                        };
                        let dir = files::unique_path(&out_dir, &name);
                        // 바로 만들어 이름을 선점한다(같은 이름의 다음 작업이 다른 이름을 받게).
                        // 실패하면 저장하지 않고, pipeline 이 결과 폴더 오류로 보고한다.
                        if files::make_part_dir(&dir).is_ok() {
                            job.work_dir = Some(dir.clone());
                        }
                        dir
                    }
                };
```

`spawn_job`의 `PipelineCfg`에서 `download_dir: cfg.download_dir,`를 아래로 바꾼다. 작업마다 결과 폴더를 쓴다.

```rust
                download_dir: job.result_dir(&cfg.download_dir),
```

`use std::path::{Path, PathBuf};`와 `crate::model::JobSource` import를 맞춘다.

- [ ] **Step 5: 파이프라인 File 분기**

`pipeline.rs`의 `run`을 아래처럼 바꾼다. 앞부분 공통 처리 뒤 출처별로 나누고, Task 3에서 넣은 임시 오류를 없앤다.

```rust
pub async fn run(
    job: &Job,
    cfg: &PipelineCfg,
    cancel: &CancellationToken,
    mut report: impl FnMut(Report),
) -> Result<PathBuf, PipelineError> {
    if files::check_writable(&cfg.download_dir).is_err() {
        return Err(PipelineError::Failed(ERR_DIR_UNWRITABLE.into()));
    }
    files::make_part_dir(&cfg.work_dir).map_err(io_fail)?;
    match &job.source {
        JobSource::Youtube { url } => download_and_save(job, url, cfg, cancel, &mut report).await,
        JobSource::File { path, .. } => transcode_file(job, path, cfg, cancel, &mut report).await,
    }
}
```

기존 `run`의 다운로드 이후 본문(`report(Report { stage: Stage::Downloading ...`부터 Task 5의 `match encode(...)`까지)을 `download_and_save`로 옮긴다. 시그니처는 `async fn download_and_save(job: &Job, url: &str, cfg: &PipelineCfg, cancel: &CancellationToken, report: &mut impl FnMut(Report)) -> Result<PathBuf, PipelineError>`이다. 본문 안의 `report(...)` 호출과 `&mut report` 전달은 `report`가 이미 `&mut`이므로 `report(...)`와 `report`로 그대로 쓴다.

새 함수를 추가한다.

```rust
/// 로컬 파일 변환: 원본은 읽기만 하고, 결과를 `{원본}-{n}.{확장자}` 로 결과 폴더에 둔다.
async fn transcode_file(
    job: &Job,
    src: &Path,
    cfg: &PipelineCfg,
    cancel: &CancellationToken,
    report: &mut impl FnMut(Report),
) -> Result<PathBuf, PipelineError> {
    // 재시작이면 이전 부분 출력을 버린다(ffmpeg 는 이어서 인코딩하지 못한다).
    let _ = fs::remove_dir_all(cfg.work_dir.join("out"));
    let encoded = encode(job, cfg, cancel, src, report)
        .await?
        .ok_or_else(|| PipelineError::Failed(ERR_NO_OUTPUT.into()))?;
    let ext = encoded
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4");
    // 패키지 이름이 `{stem}-{n}.kiripart` 이므로 같은 번호로 꺼낸다. 이미 있으면 다음 번호.
    let pkg_stem = cfg
        .work_dir
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let mut dest = cfg.download_dir.join(format!("{pkg_stem}.{ext}"));
    if dest.exists() {
        let stem = files::safe_title(src.file_stem().and_then(|s| s.to_str()).unwrap_or("file"));
        dest = files::variant_path(&cfg.download_dir, &stem, ext);
    }
    files::move_file(&encoded, &dest).map_err(io_fail)?;
    Ok(dest)
}
```

`encode`는 `report: &mut impl FnMut(Report)`를 받으므로 그대로 넘긴다.

- [ ] **Step 6: 통과 확인**

Run: `cargo fmt --all && cargo test -p kiri-core` 를 2번 실행한다.
Expected: 2번 모두 통과(엔진 테스트에 새 테스트 7개 포함). `cargo clippy -p kiri-core`에서 새 경고가 없다.

- [ ] **Step 7: Commit**

```bash
git add crates/kiri-core
git commit -m "feat(core): 로컬 파일 변환 작업(add_file) — 원본은 그대로, 결과는 {원본}-{n}"
```

---
### Task 7: 프로토콜 v2 + CLI `kiri transcode`

**Files:**
- Modify: `crates/kiri-core/src/ipc.rs`, `crates/kiri-cli/src/main.rs`, `crates/kiri-cli/tests/cli.rs`, `README.md`, `src/pages/settings/CliTab.tsx`

**Interfaces:**
- Consumes: `Engine::add_file(path, output_dir, preset, quality)`(Task 6), `JobSource`(Task 3)
- Produces:
  - `Request::Transcode { source: String, output: Option<String>, format: String, quality: Option<String> }`. JSON은 `{"type":"transcode","source":…,"output":…,"format":…,"quality":…}`
  - `PROTOCOL_VERSION = 2`
  - CLI `Cmd::into_request(self) -> Result<Request, String>`. `Err`는 앱에 보내기 전의 요청 오류이고, 종료 코드는 1이다.

- [ ] **Step 1: 실패하는 테스트 작성 (ipc)**

`ipc.rs` tests의 `request_json_shape` 끝에 추가한다.

```rust
        let t = Request::Transcode {
            source: "/a/b c.mkv".into(),
            output: None,
            format: "mp4-hevc".into(),
            quality: Some("720p".into()),
        };
        assert_eq!(
            serde_json::to_value(&t).unwrap(),
            json!({"type": "transcode", "source": "/a/b c.mkv", "output": null, "format": "mp4-hevc", "quality": "720p"})
        );
        let t2: Request =
            serde_json::from_value(json!({"type": "transcode", "source": "/x", "format": "mp3"})).unwrap();
        assert_eq!(
            t2,
            Request::Transcode { source: "/x".into(), output: None, format: "mp3".into(), quality: None }
        );
        assert_eq!(PROTOCOL_VERSION, 2);
```

`list_and_errors_over_socket` 끝에 추가한다(idle 엔진이라 원본 확인에서 끝난다).

```rust
        let r = request(
            &path,
            &Request::Transcode {
                source: d.path().join("missing.mkv").display().to_string(),
                output: None,
                format: "mp4-h264".into(),
                quality: None,
            },
        )
        .await
        .unwrap();
        assert!(
            matches!(r, Response::Error { ref code, .. } if code == "source_not_found"),
            "{r:?}"
        );
```

`version_mismatch_is_reported`에서 v1 클라이언트(이전 CLI)도 거부되는지 확인한다. 기존 `v:99` 요청 다음에 한 번 더 보낸다.

```rust
        let mut s = UnixStream::connect(&path).await.unwrap();
        s.write_all(b"{\"v\":1,\"body\":{\"type\":\"list\"}}\n")
            .await
            .unwrap();
        let mut line = String::new();
        BufReader::new(s).read_line(&mut line).await.unwrap();
        assert!(line.contains("version_mismatch"), "{line}");
```

- [ ] **Step 2: 실패하는 테스트 작성 (CLI)**

`main.rs` tests의 `job()` 헬퍼는 Task 3에서 이미 `source`/`max_height`로 바뀌어 있다. `parses_add_args`의 `cli.cmd.into_request()`를 `cli.cmd.into_request().unwrap()`로 바꾸고, 아래 테스트를 추가한다.

```rust
    #[test]
    fn transcode_resolves_paths_and_requires_format() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("내 영상 01.mkv");
        std::fs::write(&src, "x").unwrap();
        let cli = Cli::try_parse_from([
            "kiri",
            "transcode",
            src.to_str().unwrap(),
            "--format",
            "mp4-hevc",
            "--quality",
            "720p",
            "--output",
            d.path().join("new dir").to_str().unwrap(),
        ])
        .unwrap();
        let real = std::fs::canonicalize(d.path()).unwrap();
        assert_eq!(
            cli.cmd.into_request().unwrap(),
            Request::Transcode {
                source: real.join("내 영상 01.mkv").display().to_string(),
                output: Some(d.path().join("new dir").display().to_string()),
                format: "mp4-hevc".into(),
                quality: Some("720p".into()),
            }
        );
        // --format 은 필수
        assert!(Cli::try_parse_from(["kiri", "transcode", "a.mkv"]).is_err());
        // 없는 원본은 앱에 보내기 전에 거부
        let cli = Cli::try_parse_from(["kiri", "transcode", "/nope/x.mkv", "--format", "mp3"]).unwrap();
        assert_eq!(cli.cmd.into_request().unwrap_err(), "source not found: /nope/x.mkv");
    }
```

`kiri-cli/Cargo.toml`의 `[dev-dependencies]`에 `tempfile`이 없으면 추가한다(`tests/cli.rs`가 이미 쓰므로 있을 것이다).

`tests/cli.rs`에 추가한다. 실제 소켓 서버와 가짜 ffprobe가 있는 엔진을 쓰고, 엔진은 `start()`하지 않는다(작업은 대기 상태로 남는다).

```rust
fn engine_with_ffprobe(dir: &Path) -> Engine {
    use std::os::unix::fs::PermissionsExt;
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let ffprobe = bin.join("ffprobe");
    std::fs::write(&ffprobe, "#!/bin/sh\necho 3.5\n").unwrap();
    std::fs::set_permissions(&ffprobe, std::fs::Permissions::from_mode(0o755)).unwrap();
    let tools = Tools {
        ytdlp: dir.join("none/yt-dlp"),
        deno: dir.join("none/deno"),
        ffmpeg: dir.join("none/ffmpeg"),
        ffprobe,
    };
    let paths = EnginePaths {
        queue_file: dir.join("q.json"),
        cache_dir: dir.join("c"),
        log_dir: dir.join("l"),
    };
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

async fn kiri_in(cwd: &Path, socket: &Path, args: &[&str]) -> (i32, String, String) {
    let (cwd, socket) = (cwd.to_path_buf(), socket.to_path_buf());
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    tokio::task::spawn_blocking(move || {
        let out = Command::new(env!("CARGO_BIN_EXE_kiri"))
            .args(&args)
            .current_dir(&cwd)
            .env("KIRI_SOCKET", &socket)
            .output()
            .unwrap();
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
async fn transcode_sends_absolute_path_with_spaces() {
    let d = tempfile::tempdir().unwrap();
    let socket = d.path().join("kiri.sock");
    let listener = ipc::bind(&socket).await.unwrap();
    tokio::spawn(ipc::serve(listener, engine_with_ffprobe(d.path())));
    std::fs::write(d.path().join("내 영상 01.mkv"), "x").unwrap();

    let (code, out, err) = kiri_in(
        d.path(),
        &socket,
        &["--json", "transcode", "./내 영상 01.mkv", "--format", "mp4-hevc", "--quality", "720p"],
    )
    .await;
    assert_eq!(code, 0, "{err}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let real = std::fs::canonicalize(d.path()).unwrap();
    assert_eq!(v["type"], "added");
    assert_eq!(
        v["job"]["source"],
        serde_json::json!({"kind": "file", "path": real.join("내 영상 01.mkv"), "output_dir": null})
    );
    assert_eq!(v["job"]["title"], "내 영상 01.mkv");
    assert_eq!(v["job"]["quality_label"], "720p");

    let (code, out, _) = kiri_in(d.path(), &socket, &["transcode", "./내 영상 01.mkv", "--format", "mp3"]).await;
    assert_eq!(code, 0);
    assert!(out.starts_with("added #2: 내 영상 01.mkv"), "{out}");
}

#[tokio::test(flavor = "multi_thread")]
async fn transcode_missing_source_exits_1_without_app() {
    let d = tempfile::tempdir().unwrap();
    // 앱이 없어도(소켓 없음) 원본 확인이 먼저라 2가 아니라 1로 끝난다.
    let (code, _, err) = kiri(&d.path().join("none.sock"), &["transcode", "/nope/x.mkv", "--format", "mp3"]).await;
    assert_eq!(code, 1);
    assert!(err.contains("source not found"), "{err}");
}
```

- [ ] **Step 3: 실패 확인**

Run: `cargo test -p kiri-core ipc && cargo test -p kiri-cli`
Expected: 컴파일 에러(`Request::Transcode`, `Cmd::Transcode` 없음).

- [ ] **Step 4: ipc 구현**

`PROTOCOL_VERSION`을 `2`로 바꾸고, 주석을 붙인다.

```rust
/// v2: 작업 JSON 의 `url` 이 `source` 로 바뀌고 `transcode` 요청이 생겼다.
pub const PROTOCOL_VERSION: u32 = 2;
```

`Request`의 `Stop` 뒤에 추가한다.

```rust
    /// 로컬 파일 변환. 경로는 절대 경로여야 한다(CLI 가 바꿔 보낸다).
    Transcode {
        source: String,
        #[serde(default)]
        output: Option<String>,
        format: String,
        #[serde(default)]
        quality: Option<String>,
    },
```

`dispatch`에 분기를 추가한다.

```rust
        Request::Transcode {
            source,
            output,
            format,
            quality,
        } => match engine
            .add_file(source.into(), output.map(Into::into), &format, quality)
            .await
        {
            Ok(job) => Response::Added { job },
            Err(e) => e.into(),
        },
```

- [ ] **Step 5: CLI 구현**

`Cmd`의 `Stop` 뒤에 추가한다.

```rust
    /// Transcode a local file; the source is left untouched
    Transcode {
        source: PathBuf,
        /// mp4-h264 | mp4-hevc | mov-prores | webm-vp9 | mp3 | m4a
        #[arg(long)]
        format: String,
        /// original | 2160p | 1440p | 1080p | 720p | 480p (never upscales)
        #[arg(long)]
        quality: Option<String>,
        /// Output folder (default: next to the source; created if missing)
        #[arg(long)]
        output: Option<PathBuf>,
    },
```

`into_request`를 `Result`로 바꾼다. 앱은 작업 디렉터리가 다르므로 경로를 절대 경로로 바꿔 보낸다. 원본은 `canonicalize`(없으면 오류)로, 출력 폴더는 아직 없을 수 있으므로 `std::path::absolute`로 바꾼다.

```rust
impl Cmd {
    /// Err 는 앱에 보내기 전의 요청 오류(종료 코드 1).
    fn into_request(self) -> Result<Request, String> {
        Ok(match self {
            Cmd::Status => Request::Status,
            Cmd::List => Request::List,
            Cmd::Add {
                url,
                quality,
                format,
                subs,
            } => Request::Add {
                url,
                quality,
                preset: format,
                subs,
            },
            Cmd::Remove { id } => Request::Remove { id },
            Cmd::Stop { id } => Request::Stop { id },
            Cmd::Transcode {
                source,
                format,
                quality,
                output,
            } => {
                let src = std::fs::canonicalize(&source)
                    .ok()
                    .filter(|p| p.is_file())
                    .ok_or_else(|| format!("source not found: {}", source.display()))?;
                let output = output
                    .map(|o| std::path::absolute(&o).map_err(|e| format!("{}: {e}", o.display())))
                    .transpose()?;
                Request::Transcode {
                    source: src.display().to_string(),
                    output: output.map(|o| o.display().to_string()),
                    format,
                    quality,
                }
            }
        })
    }
}
```

`use std::{path::Path, …}`를 `use std::{path::{Path, PathBuf}, process::ExitCode, time::Duration};`로 바꾼다.

`run`의 앞부분을 바꾼다. 앱이 꺼져 있으면 실행하는 대상에 `transcode`도 넣는다.

```rust
async fn run(cli: Cli) -> ExitCode {
    let socket = ipc::socket_path();
    let launches = matches!(cli.cmd, Cmd::Add { .. } | Cmd::Transcode { .. });
    let req = match cli.cmd.into_request() {
        Ok(r) => r,
        Err(m) => {
            eprintln!("error: {m}");
            return ExitCode::from(1);
        }
    };
    let mut result = ipc::request(&socket, &req).await;
    if launches && result == Err(ClientError::NotRunning) && launch_and_wait(&socket).await {
        result = ipc::request(&socket, &req).await;
    }
```

나머지(`match result …`)는 그대로 둔다.

- [ ] **Step 6: 문서**

`README.md` CLI 절의 코드 블록에서 `kiri remove` 줄 다음에 추가한다.

```sh
kiri transcode <파일> --format mp4-h264|mp4-hevc|mov-prores|webm-vp9|mp3|m4a [--quality original|2160p|1440p|1080p|720p|480p] [--output <폴더>]
```

코드 블록 아래 목록의 첫 줄 앞에 추가한다.

```markdown
- `kiri transcode`는 원본을 그대로 두고 `<원본 이름>-<번호>.<확장자>`를 만듭니다(예: `clip.mkv` → `clip-1.mp4`, 다시 변환하면 `clip-2.mp4`). 결과는 `--output` 폴더(없으면 만듭니다), 생략하면 원본과 같은 폴더에 생깁니다. `--quality`는 높이 상한이며 원본보다 키우지 않습니다(기본 `original`).
- `--json` 출력의 작업에는 출처가 `"source": {"kind": "youtube", "url": …}` 또는 `{"kind": "file", "path": …, "output_dir": …}`로 들어 있습니다.
```

`- \`kiri add\`는 앱이 꺼져 있으면…` 줄을 `- \`kiri add\`와 \`kiri transcode\`는 앱이 꺼져 있으면 앱을 실행한 뒤 추가합니다. 나머지 명령은 앱이 꺼져 있으면 종료 코드 2로 끝납니다.`로 바꾼다.

README "다운로드" 절의 출력 포맷 표 바로 아래 문단(`H.264, HEVC, ProRes는…`) 다음에 추가한다.

```markdown
"원본 유지"가 아닌 포맷을 고르면 받은 원본(`제목.webm` 등)은 그대로 두고 변환본을 `제목-1.mp4`처럼 따로 만듭니다. 같은 영상을 다시 변환하면 `제목-2.mp4`가 됩니다.
```

`src/pages/settings/CliTab.tsx`의 `USAGE`에서 `kiri remove <id>` 줄 다음에 추가한다.

```
kiri transcode <file> --format mp4-hevc [--quality 720p] [--output <dir>]
```

- [ ] **Step 7: 통과 확인**

Run: `cargo fmt --all && cargo test --workspace && cargo clippy --workspace`
Expected: 모두 통과, 새 경고 없음.

- [ ] **Step 8: Commit**

```bash
git add crates README.md src/pages/settings/CliTab.tsx
git commit -m "feat(cli): kiri transcode 명령과 프로토콜 v2"
```

---

### Task 8: 프론트엔드 — `source` 필드, 변환 작업 표시

**Files:**
- Modify: `src/lib/types.ts`, `src/lib/sheet.ts`, `src/lib/format.ts`, `src/components/JobRow.tsx`, `src/components/Icon.tsx`, `src/locales/{ko,en,ja}.json`
- Test: `src/test/format.test.ts`, `src/test/sheet.test.ts`, `src/test/queue.test.ts`

**Interfaces:**
- Consumes: Rust `Job`/`NewJob` JSON(Task 3): `source: {kind:"youtube",url} | {kind:"file",path,output_dir}`, `options.max_height: number | null`
- Produces: `type JobSource`, `Job.source`, `NewJob.source`, `JobOptions.max_height`, `Icon` 이름 `"file"`, 로케일 키 `job.transcode`, `error.source_not_found`, `error.invalid_media`

Task 3부터 이 Task 전까지는 앱의 옵션 시트 추가(`add_job`)가 `url` 필드 때문에 실패한다. 이 Task가 그 연결을 다시 맞춘다.

- [ ] **Step 1: 실패하는 테스트 작성**

`src/test/format.test.ts`의 `job` 헬퍼에서 `url: "u",`를 `source: { kind: "youtube", url: "u" },`로 바꾸고, `options`에 `max_height: null`을 넣는다(파일 안의 다른 `options` 리터럴에도 넣는다). 테스트를 추가한다.

```ts
  it("labels file transcode jobs", () => {
    const f = job({ kind: "encoding" }, { source: { kind: "file", path: "/a/clip.mkv", output_dir: null }, quality_label: "720p" });
    expect(jobDetail(f, t)).toBe("job.transcode · 720p · preset.mp4-h264 · Subs ko · state.encoding 48%");
  });
```

`src/test/sheet.test.ts`의 `buildNewJob` 기대값에서 `url: "u",`를 `source: { kind: "youtube", url: "u" },`로 바꾸고, `options`에 `max_height: null`을 넣는다.

`src/test/queue.test.ts`의 작업 리터럴도 같은 방식으로 바꾼다(`url: "u"` → `source: { kind: "youtube", url: "u" }`, `options`에 `max_height: null`).

로케일 키 일치는 기존 `src/test/locales.test.ts`가 검사한다.

- [ ] **Step 2: 실패 확인**

Run: `yarn test && yarn tsc --noEmit`
Expected: 타입 오류(`source` 없음)와 `labels file transcode jobs` 실패.

- [ ] **Step 3: 타입과 시트**

`src/lib/types.ts`를 바꾼다.

```ts
export type JobSource =
  | { kind: "youtube"; url: string }
  | { kind: "file"; path: string; output_dir: string | null };

export interface JobOptions {
  format_id: string | null;
  preset: Preset;
  subtitles: string[];
  auto_subtitles: boolean;
  /** 파일 변환의 높이 상한. null = 원본 그대로 */
  max_height: number | null;
}
```

`Job`의 `url: string;`을 `source: JobSource;`로 바꾸고, `NewJob`의 `"url"`을 `"source"`로 바꾼다.

`src/lib/sheet.ts`의 `buildNewJob`에서 `url,`을 `source: { kind: "youtube", url },`로 바꾸고, `options`에 `max_height: null,`을 추가한다.

- [ ] **Step 4: 목록 행 표시**

`src/lib/format.ts`의 `jobDetail` 첫 줄을 바꾼다.

```ts
  const parts = [job.quality_label === "audio" ? t("quality.audio") : job.quality_label, t(`preset.${job.options.preset}`)];
  if (job.source.kind === "file") parts.unshift(t("job.transcode"));
```

`src/components/Icon.tsx`의 `paths`에 추가한다.

```tsx
  file: (
    <>
      <path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" />
      <path d="M14 3v5h5" />
    </>
  ),
```

`src/components/JobRow.tsx`의 썸네일 부분을 바꾼다.

```tsx
      {job.thumbnail ? (
        <img src={job.thumbnail} alt="" className="h-12 w-20 shrink-0 rounded-md object-cover" />
      ) : (
        <div className="flex h-12 w-20 shrink-0 items-center justify-center rounded-md bg-base-300 text-fg-muted">
          {job.source.kind === "file" && <Icon name="file" />}
        </div>
      )}
```

- [ ] **Step 5: 로케일**

`job`에 `transcode`를, `error`에 두 키를 추가한다.

`ko.json`:
```json
    "transcode": "변환"
```
```json
    "source_not_found": "원본 파일을 찾을 수 없습니다",
    "invalid_media": "영상이나 오디오 파일이 아닙니다",
```

`en.json`:
```json
    "transcode": "Transcode"
```
```json
    "source_not_found": "Source file not found",
    "invalid_media": "Not a video or audio file",
```

`ja.json`:
```json
    "transcode": "変換"
```
```json
    "source_not_found": "元のファイルが見つかりません",
    "invalid_media": "動画または音声ファイルではありません",
```

`job.subs` 다음 줄에 넣고(앞 줄에 쉼표 추가), `error`의 두 키는 `"unknown"` 앞에 넣는다.

- [ ] **Step 6: 통과 확인**

Run: `yarn test && yarn tsc --noEmit && yarn build`
Expected: 모두 통과.

수동: `yarn tauri dev`로 YouTube 링크를 `mp4-h264`로 받아 저장 폴더에 원본과 `제목-1.mp4`가 함께 생기는지 확인한다. 이어서 `cargo run -p kiri-cli -- transcode <로컬 mkv> --format mp4-hevc --quality 720p`를 실행하고, 목록에 파일 아이콘과 "변환" 라벨이 보이는지, 완료 후 `ffprobe -v error -show_entries stream=height -of csv=p=0 <결과>`가 720 이하인지 확인한다(`KIRI_SOCKET`을 따로 지정하지 않으면 dev 앱 소켓에 붙는다).

- [ ] **Step 7: Commit**

```bash
git add src
git commit -m "feat(ui): 작업 source 필드, 파일 변환 작업 아이콘·라벨"
```
