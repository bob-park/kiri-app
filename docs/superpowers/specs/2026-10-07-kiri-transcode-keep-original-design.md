# kiri: 원본 유지 변환 · CLI 트랜스코딩 · 업데이트 키 검사 설계

- 날짜: 2026-10-07
- 브랜치: `feature/transcoding`
- 선행 스펙: `2026-10-06-kiri-downloader-cli-updater-design.md`

## 0. 배경과 범위

요구사항은 세 가지입니다.

1. **자동 업데이트가 진행되지 않음:** 다운로드 후 minisign 오류가 납니다.
2. **원본과 다른 포맷으로 받을 때:** 원본은 그대로 두고, 다른 포맷 파일을 `{원본파일명}-{index}.{확장자}`로 추가로 만듭니다.
3. **CLI에 트랜스코딩 명령 추가:** 입력(source), 결과 파일 위치, 포맷, 화질을 받습니다.

### 1번 조사 결과 (코드 변경 대상 아님)

- 공개된 `v0.1.1`의 `kiri.app.tar.gz`와 `latest.json` 서명을 `minisign-verify` 0.2.5로 저장소 공개키(key id `1B14787C3B027D17`)에 대해 검증했고, 통과했습니다.
- 사용자가 설치한 `/Applications/kiri.app`(0.1.0)은 정식 릴리즈 dmg가 아닙니다. 업데이트 키를 설정하기 전에 만든 첫 서명 테스트 빌드(2026-10-07 00:00)이고, 실행 파일에 **업데이터 공개키가 비어 있습니다**. 그래서 서명 검증 단계에서 실패합니다.
- **조치:** 사용자가 `v0.1.1` dmg로 한 번 직접 교체합니다.
- **재발 방지:** 4절의 빌드 검사를 추가합니다.

### 범위 밖

- 앱 UI에서 로컬 파일 변환을 시작하는 기능(파일 드롭 등). 요청에 없습니다.
- 변환 작업의 이어서 인코딩. ffmpeg가 지원하지 않으므로 재시작하면 처음부터 다시 인코딩합니다.

## 1. 원본 유지와 이름 규칙

### 1.1 YouTube 다운로드 작업

프리셋이 "원본 유지"가 아니면:

1. 받은 원본 미디어를 지금처럼 저장 폴더로 옮깁니다(`files::unique_path`). 예: `제목.webm`. 같은 이름이 있으면 `제목 (1).webm`이 됩니다.
2. 변환본은 `{원본의 최종 파일명(stem)}-{index}.{확장자}`로 저장합니다. 예: `제목-1.mp4`.
3. 자막 `.srt`는 지금처럼 원본의 최종 stem을 따릅니다. 예: `제목.ko.srt`.
4. 오디오 프리셋(`mp3`, `m4a`)은 지금처럼 `bestaudio`만 받습니다. 원본은 받은 오디오 파일(예: `제목.webm`)이고, 변환본은 `제목-1.mp3`입니다.
5. 작업의 `output`(Finder에서 보기 대상)은 **변환본**입니다.

프리셋이 "원본 유지"이면 동작은 지금과 같습니다.

### 1.2 index 규칙

```rust
/// dir 안에서 `{stem}-{n}.*` (n ≥ 1 정수, 확장자 무관) 중 가장 큰 n + 1. 없으면 1.
pub fn next_variant_index(dir: &Path, stem: &str) -> u32
pub fn variant_path(dir: &Path, stem: &str, ext: &str) -> PathBuf   // dir/{stem}-{index}.{ext}
```

- 파일 이름이 정확히 `{stem}-{숫자}.{확장자}` 형태인 것만 셉니다. `제목2-1.mp4`나 `제목-1a.mp4`는 `제목`의 번호가 아닙니다. stem에 `-`가 들어 있어도(`a-b`) `a-b-3.mp4`는 `a-b`의 3번으로 셉니다.
- 디렉터리(`.kiripart` 포함)도 이름이 맞으면 셉니다. 진행 중인 변환과 번호가 겹치지 않게 하기 위해서입니다.
- 그래도 이미 존재하는 경로가 나오는 경합에 대비해, 최종 이동 직전에 다시 계산합니다.

### 1.3 로컬 파일 변환 작업 (2절)

- 결과 폴더(`output_dir`, 없으면 원본 폴더)에서 원본 stem 기준으로 index를 매깁니다.
- 원본 파일은 읽기만 하고 절대 옮기거나 바꾸지 않습니다.

## 2. 작업 모델과 파이프라인

### 2.1 모델 (`kiri-core::model`)

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JobSource {
    Youtube { url: String },
    File { path: PathBuf, output_dir: Option<PathBuf> },  // None = 원본과 같은 폴더
}
```

- `Job.url`과 `NewJob.url`을 `source: JobSource`로 바꿉니다.
- `JobOptions`에 `#[serde(default)] pub max_height: Option<u32>`를 추가합니다. 화질 상한이고 `None`이면 원본 해상도를 유지합니다.
- **호환성:** `queue.json`에 `source` 없이 `url`만 있는 작업은 `Youtube { url }`로 읽습니다. `QueueState::load`에서 변환하고, 다음 저장부터 새 형식이 됩니다. 테스트로 고정합니다.

### 2.2 파일 변환 작업 추가 (`Engine::add_file`)

```rust
pub async fn add_file(&self, path: PathBuf, output_dir: Option<PathBuf>, preset: &str, quality: Option<String>) -> Result<Job, EngineError>
```

1. **검증:**
   - `path`가 존재하는 일반 파일이 아니면 `SourceNotFound`
   - 프리셋을 해석할 수 없거나 `original`이면 `BadPreset`
   - 화질이 `original | <N>p`가 아니면 `BadQuality`
2. **길이 확인:** 번들 ffprobe로 `ffprobe -v error -show_entries format=duration -of csv=p=0 -- <path>`를 실행합니다. 실패하면 `InvalidMedia(ffprobe stderr 마지막 줄)`입니다. ffprobe 경로는 `Tools`에 `ffprobe: PathBuf`를 추가해 받습니다(앱에서는 sidecar `ffprobe`).
3. **작업 생성:**
   - 제목: 파일 이름
   - 썸네일: 없음
   - `quality_label`: `1080p` 또는 `original`
   - `options`: `{ format_id: None, preset, subtitles: [], auto_subtitles: false, max_height }`
   - `source`: `File { path, output_dir }`

### 2.3 파이프라인 (`pipeline::run`)

- **Youtube:** 다운로드 → (원본 유지가 아니면) 인코딩 → 1.1의 규칙으로 원본과 변환본을 모두 저장합니다.
- **File:**
  1. 결과 폴더를 `check_writable`로 확인합니다. 실패하면 `error.download_dir_unwritable`입니다.
  2. 결과 폴더에 `{원본 stem}-{index}.kiripart` 패키지를 만들고(엔진의 `work_dir` 할당 규칙 재사용), 그 안의 `out/`에서 ffmpeg로 인코딩합니다.
  3. 끝나면 `variant_path(결과 폴더, 원본 stem, 확장자)`로 옮기고 패키지를 지웁니다. 이동에 실패하면 패키지를 남깁니다.
- **화질 상한:** 비디오 프리셋이고 `max_height`가 있으면 `-vf scale=-2:'min(<높이>,ih)'`를 추가합니다. 원본보다 키우지 않고, 너비는 비율에 맞춘 짝수입니다. `ffmpeg::encode_args`에 `max_height: Option<u32>` 인자를 추가합니다. 오디오 프리셋에는 넣지 않습니다.
- **VideoToolbox 폴백, 진행률(`-progress`), 취소, `.kiripart` 처리, `shutdown_for_exit`:** 기존 인코딩 단계를 재사용합니다.
- **재시작:** 파일 작업은 패키지 안의 이전 출력을 지우고 처음부터 인코딩합니다.

## 3. CLI · 프로토콜 · 앱 표시

### 3.1 CLI (`crates/kiri-cli`)

```
kiri transcode <source> --format <preset> [--quality original|2160p|1440p|1080p|720p|480p] [--output <폴더>] [--json]
```

- `--format`은 필수입니다. `original`은 받지 않습니다.
- `--quality`의 기본값은 `original`입니다.
- `<source>`와 `--output`은 CLI가 `std::fs::canonicalize`로 절대 경로로 바꿔 보냅니다. 앱의 작업 디렉터리가 다르기 때문입니다. 원본을 해석할 수 없으면 앱에 보내지 않고 바로 `error: source not found`를 출력하고 종료 코드 1로 끝냅니다. `--output` 폴더가 없을 때는 그대로 보내서, 앱이 작업 시작 시 오류로 처리합니다.
- 출력과 종료 코드는 `add`와 같습니다.
  - 성공: `added #<id>: <제목>`
  - 요청 오류: 1
  - 연결 실패: 2
  - 앱이 꺼져 있으면 실행한 뒤 넣습니다.
- 표 출력(`list`/`status`)은 그대로 씁니다. 제목 열에는 파일 이름이 나옵니다.

### 3.2 프로토콜 (`kiri-core::ipc`)

- `Request::Transcode { source: String, output: Option<String>, format: String, quality: Option<String> }`를 추가합니다. 응답은 `Added { job }`이고, `dispatch`는 `engine.add_file`을 호출합니다.
- `PROTOCOL_VERSION`을 **2**로 올립니다. 작업 JSON에서 `url`이 `source`로 바뀌기 때문입니다.
- README의 CLI 절에 `transcode` 사용법과 `--json`의 `source` 형태를 추가합니다.

### 3.3 앱 표시 (프론트엔드)

- `src/lib/types.ts`에 `JobSource`를 추가하고, `Job`/`NewJob`의 `url`을 `source`로 바꿉니다.
- `buildNewJob`(옵션 시트)은 `source: { kind: "youtube", url }`을 만듭니다.
- 파일 변환 작업은 목록 행에서 이렇게 보입니다.
  - 썸네일 자리에 파일 아이콘(`Icon` 컴포넌트에 `file` 추가)을 둡니다.
  - 둘째 줄 앞에 "변환" 라벨을 붙입니다(`job.transcode`: 변환 / Transcode / 変換).
- 중지, 다시 시작, 삭제, Finder에서 보기, 완료 항목 정리, 트레이 요약, "큐가 끝나면 재시작"은 바꾸지 않습니다.

## 4. 업데이트 키 빌드 검사

- `src-tauri/build.rs`에서 `PROFILE=release`일 때 `tauri.conf.json`의 `plugins.updater.pubkey`를 읽습니다. 비어 있거나 `<PUBKEY>`이면 `panic!`으로 빌드를 실패시킵니다. 메시지: `updater pubkey is empty — see docs/development.md`.
- 디버그 빌드, `yarn tauri dev`, `yarn tauri build --debug`는 영향을 받지 않습니다.
- 판정 로직은 작은 함수로 분리해 테스트합니다. `build.rs`에서는 같은 함수를 `include!` 등으로 공유합니다.

## 5. 에러 처리

| 상황 | 처리 |
|---|---|
| 원본이 없거나 폴더임 | `EngineError::SourceNotFound` (`source_not_found`), CLI 종료 코드 1, 작업 만들지 않음 |
| ffprobe가 읽지 못함 | `EngineError::InvalidMedia(msg)` (`invalid_media`) |
| `original`이거나 알 수 없는 포맷 | `bad_preset` |
| 알 수 없는 화질 | `bad_quality` |
| 결과 폴더가 없거나 쓰기 불가 | 작업 시작 시 `Failed(error.download_dir_unwritable)` |
| 인코딩 실패 | 기존과 동일: VideoToolbox → 소프트웨어 1회 재시도, 그래도 실패하면 `Failed`. 패키지 유지 |
| 작업 도중 원본이 사라짐 | ffmpeg 오류로 `Failed`. 원본 쪽에는 쓰지 않음 |
| 결과 이동 실패 | 패키지를 지우지 않음 |
| 예전 `queue.json` | `Youtube { url }`로 변환해 유지 |
| `pubkey` 빈 채로 release 빌드 | 빌드 실패 |

새 오류 코드 `source_not_found`, `invalid_media`의 ko/en/ja 문구를 `error.*`에 추가합니다.

## 6. 테스트

- **index 규칙:** 확장자와 상관없이 최대 + 1, 기존 번호가 없으면 1, 비슷한 이름은 제외, stem에 `-`가 있는 경우, 패키지 디렉터리도 셈
- **파이프라인 (가짜 스크립트):**
  - 다운로드 + 변환이면 원본과 `-1` 변환본이 모두 남음
  - 다시 변환하면 `-2`
  - 오디오 프리셋이면 원본 오디오 + `-1.mp3`
  - 파일 변환이면 원본의 내용과 수정 시간이 그대로이고 결과는 `원본-1.<ext>`. `output_dir`을 주면 그 폴더에 생김
- **ffmpeg 인자:** `max_height` 비디오 → `-vf scale=-2:'min(1080,ih)'`, 오디오 → 없음, `None` → 없음
- **엔진:**
  - `add_file` 검증 4종(없는 파일, 폴더, original, 잘못된 화질)과 ffprobe 실패
  - 파일 작업의 중지·재시작·삭제와 패키지 처리
  - 예전 `queue.json` 마이그레이션
- **프로토콜·CLI:**
  - `Transcode` 직렬화, 버전 2와 버전 불일치
  - CLI 통합 테스트: 상대 경로가 절대 경로로 바뀜, 없는 원본이면 종료 코드 1
- **빌드 검사:** pubkey 판정 함수(빈 값, `<PUBKEY>`, 정상)
- **프론트엔드:** `buildNewJob`의 `source`, `jobDetail`의 "변환" 라벨, 로케일 키 일치
- **수동:**
  - 실제 YouTube 영상을 `mp4-h264`로 받아 원본과 `-1.mp4`가 모두 생기는지
  - 로컬 mkv에 `kiri transcode --format mp4-hevc --quality 720p`를 실행해 결과 해상도를 ffprobe로 확인
