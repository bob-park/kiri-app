# kiri

YouTube 링크를 붙여넣으면 원하는 화질과 자막을 골라 내려받는 macOS 앱입니다. [Downie](https://software.charliemonroe.net/downie/)처럼 가볍게 쓰는 것을 목표로 합니다.

- **대상:** Apple Silicon Mac, macOS 14.2 이상
- **최신 버전:** [Releases](https://github.com/bob-park/kiri-app/releases/latest)

## 설치

1. [Releases](https://github.com/bob-park/kiri-app/releases/latest)에서 `kiri_<버전>_aarch64.dmg`를 받습니다.
2. dmg를 열고 kiri를 Applications 폴더로 옮깁니다.
3. 실행합니다. Apple 공증을 받은 앱이라 경고 없이 열립니다.

처음 실행하면 "다운로드 준비 중"이 잠시 표시됩니다. 다운로드에 필요한 구성 요소(yt-dlp, Deno)를 자동으로 받는 중입니다.

## 사용법

### 다운로드

1. 브라우저에서 YouTube 링크를 복사한 뒤 kiri 창에서 **⌘V**를 누릅니다. 링크를 창으로 끌어다 놓아도 됩니다.
2. 옵션 시트에서 고릅니다.
   - **화질:** YouTube가 실제로 제공하는 해상도와 예상 용량이 나옵니다. "오디오만"도 고를 수 있습니다.
   - **출력 포맷:** 아래 표에서 고릅니다.
   - **자막:** 수동 자막과 자동 생성 자막 중에서 여러 언어를 고를 수 있고, `.srt` 파일로 함께 저장됩니다.
3. **다운로드**를 누릅니다. "다음부터 이 설정으로 바로 시작"을 켜 두면 다음부터는 시트 없이 바로 받습니다.

| 출력 포맷 | 설명 |
|---|---|
| 원본 유지 | 재인코딩 없이 받은 그대로 (가장 빠름) |
| MP4 · H.264 | 호환성 최우선 |
| MP4 · HEVC | 용량 대비 화질이 좋음 |
| MOV · ProRes 422 | 편집용 |
| WebM · VP9 | |
| MP3 / M4A | 오디오만 |

H.264, HEVC, ProRes는 가능하면 Mac의 하드웨어 인코더(VideoToolbox)를 씁니다.

"원본 유지"가 아닌 포맷을 고르면 받은 원본(`제목.webm` 등)은 그대로 두고 변환본을 `제목-1.mp4`처럼 따로 만듭니다. 같은 영상을 다시 변환하면 `제목-2.mp4`가 됩니다.

### 다운로드 중

- 받는 동안 저장 폴더(기본 `~/Movies/kiri`)에 **`제목.kiripart`**가 생기고 점점 커집니다. 다 받으면 `제목.mp4` 같은 완성 파일로 바뀝니다.
- 목록의 버튼으로 **중지·다시 시작·삭제**하거나, 완료된 파일을 **Finder에서 보기**로 열 수 있습니다.
- 중지하거나 앱을 꺼도 다음에 다시 시작하면 이어서 받습니다.
- 헤더의 **완료 항목 정리** 버튼으로 끝난 항목을 목록에서 한 번에 지울 수 있습니다. 받은 파일은 지워지지 않습니다.

### 메뉴 막대

- 창을 닫으면 앱이 꺼지지 않고 메뉴 막대로 숨어서, 다운로드가 계속됩니다.
- 메뉴 막대 아이콘을 **더블클릭**하면 창이 다시 열립니다. **우클릭**하면 진행 상황, 업데이트 확인, 종료 메뉴가 나옵니다.
- 다운로드 중에 ⌘Q로 종료하면 한 번 더 묻습니다.

### 환경설정 (⌘,)

| 탭 | 항목 |
|---|---|
| 일반 | 언어(시스템 / 한국어 / English / 日本語), 테마(시스템 / 라이트 / 다크), 창을 닫으면 메뉴 막대로 숨기기 |
| 다운로드 | 저장 위치, 기본 화질·포맷·자막, 옵션을 묻지 않고 바로 시작, 동시 다운로드 수, 하드웨어 가속 |
| CLI | `kiri` 명령줄 도구 설치·제거 |
| 업데이트 | 앱 버전과 업데이트 확인, 자동 확인, yt-dlp 버전과 지금 업데이트 |

## 명령줄 도구 (CLI)

환경설정 > CLI에서 **CLI 설치**를 누르면 `/usr/local/bin/kiri`가 생깁니다(관리자 암호를 묻습니다). 실행 중인 kiri 앱의 큐를 다른 앱이나 스크립트에서 제어할 수 있습니다.

```sh
kiri status                  # 진행 중인 작업
kiri list                    # 전체 큐
kiri add <url> [--quality best|1080p|720p|audio] [--format original|mp4-h264|mp4-hevc|mov-prores|webm-vp9|mp3|m4a] [--subs ko,en]
kiri stop <id>               # 작업 중지
kiri remove <id>             # 작업 삭제 (받은 파일은 남음)
kiri transcode <파일> --format mp4-h264|mp4-hevc|mov-prores|webm-vp9|mp3|m4a [--quality original|2160p|1440p|1080p|720p|480p] [--output <폴더>]
kiri list --json             # 다른 프로그램이 읽기 좋은 JSON 출력
```

- `kiri transcode`는 원본을 그대로 두고 `<원본 이름>-<번호>.<확장자>`를 만듭니다(예: `clip.mkv` → `clip-1.mp4`, 다시 변환하면 `clip-2.mp4`). 결과는 `--output` 폴더(없으면 만듭니다), 생략하면 원본과 같은 폴더에 생깁니다. `--quality`는 높이 상한이며 원본보다 키우지 않습니다(기본 `original`).
- `--json` 출력의 작업에는 출처가 `"source": {"kind": "youtube", "url": …}` 또는 `{"kind": "file", "path": …, "output_dir": …}`로 들어 있습니다.
- 생략한 옵션은 앱 설정의 기본값을 따릅니다.
- `kiri add`와 `kiri transcode`는 앱이 꺼져 있으면 앱을 실행한 뒤 추가합니다. 나머지 명령은 앱이 꺼져 있으면 종료 코드 2로 끝납니다.
- 종료 코드: 성공 0, 요청 오류 1, 앱 연결 실패 2

## 업데이트

- **앱:** 실행 후 10초 뒤, 그리고 하루에 한 번 새 버전을 확인합니다. 새 버전이 있으면 창 위쪽에 배너가 뜨고, 눌러서 설치하면 재시작됩니다. 다운로드 중이면 "큐가 끝나면 재시작"을 고를 수 있습니다.
- **yt-dlp:** YouTube 변경에 대응하도록 하루에 한 번 최신 버전을 확인해 자동으로 바꿉니다.

## 데이터 위치

| 내용 | 경로 |
|---|---|
| 설정, 큐, 구성 요소(yt-dlp, Deno) | `~/Library/Application Support/org.bobpark.kiri/` |
| 작업별 로그 | `~/Library/Logs/org.bobpark.kiri/jobs/` |
| 받은 파일 (기본값) | `~/Movies/kiri/` |

## 개발

빌드, 개발 환경 구성, 서명, 릴리즈 절차는 [docs/development.md](docs/development.md)를 보세요.

kiri는 [Tauri](https://tauri.app) v2(Rust) + React로 만들었고, 다운로드에는 [yt-dlp](https://github.com/yt-dlp/yt-dlp), 인코딩에는 [FFmpeg](https://ffmpeg.org)를 씁니다.
