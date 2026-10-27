# 도구(FFmpeg·yt-dlp·Deno) 앱 내 설치와 수동 업데이트 — 설계

날짜: 2026-10-27 · 브랜치: `feature/tools-manual-update`

## 의도

지금 ffmpeg/ffprobe는 빌드할 때 `scripts/fetch-ffmpeg.sh`가 `scripts/ffmpeg.lock`에 고정된 버전을 받아 앱 번들 안에 sidecar로 넣는다. 그래서 새 FFmpeg를 쓰려면 앱을 다시 릴리스해야 한다. vee-app(`../vee-app/src-tauri/src/app/engines.rs`)처럼 앱이 FFmpeg 최신 릴리스를 직접 받아 쓰게 바꾼다.

동시에 업데이트 방식을 통일한다. 모든 도구(FFmpeg, yt-dlp, Deno)는 **처음에 없으면 자동으로 받고, 그 다음 업데이트는 사용자가 고른다.** 지금 yt-dlp가 하루 한 번 자동으로 업데이트되는 동작은 없앤다. (사용자 결정)

## 목표

1. 앱이 시작할 때 `app_data_dir/bin/`에 ffmpeg, ffprobe, yt-dlp, deno 중 없는 것을 모두 받는다. 지금 yt-dlp/Deno를 설치하는 흐름(`tools::commands::ensure`)과 첫 실행 토스트("구성 요소 내려받는 중…")를 그대로 쓴다.
2. 네 개가 모두 있어야 `ready`다.
3. 설정 → 업데이트 탭에 "구성 요소" 섹션을 둔다(목업 1-C, 사용자 선택).
   - 맨 위 요약 줄: "업데이트 N개 사용 가능" + 마지막 확인 시각 + **모두 업데이트** 버튼. N이 0이면 "모두 최신 버전"이고 버튼은 없다.
   - 도구별 행: 이름, `현재 → 새 버전`(최신이면 `현재 · 최신` + ✓), 새 버전이 있을 때만 **업데이트** 버튼.
   - 행은 yt-dlp, FFmpeg, Deno 세 개다. ffmpeg와 ffprobe는 "FFmpeg" 한 행으로 묶어 같이 업데이트한다.
   - 받는 중인 행은 버튼 자리에 스피너가 돈다.
   - 지금 있는 yt-dlp 섹션은 이 섹션으로 바뀐다.
4. 다운로드가 실패했는데 더 새 yt-dlp가 있으면 메인 창 아래에 토스트를 띄운다(목업 2-C, 사용자 선택). 문구는 "다운로드가 실패했어요 / 새 yt-dlp가 있어요. 업데이트하면 해결될 수 있어요." + **업데이트** 버튼이다. 버튼을 누르면 그 자리에서 yt-dlp를 업데이트한다.
5. 업데이트가 끝나도 실패한 작업을 자동으로 다시 시도하지 않는다. 사용자가 카드의 다시 시도 버튼을 누른다. (내 결정)
6. **오프라인이면 다운로드를 시작할 수 없다.** 첫 실행 때만이 아니라 항상 그렇다. (사용자 결정)
   - 인터넷이 없을 때 ⌘V로 YouTube 링크를 붙여넣으면 옵션 시트를 열지 않고(정보 조회도 하지 않는다) "인터넷 연결이 필요해요" 토스트를 띄운다. 업데이트 탭의 업데이트 버튼도 오프라인이면 비활성화한다.
   - 이미 설치된 FFmpeg만 쓰는 로컬 파일 변환은 오프라인에서도 된다.
   - 판단 기준은 웹뷰의 `navigator.onLine`과 `online`/`offline` 이벤트다. 새 코드나 의존성 없이 쓸 수 있는 가장 싼 방법이다. 공용 Wi-Fi 로그인 페이지처럼 "연결은 됐지만 인터넷은 안 되는" 경우는 잡지 못하는데, 이때는 지금처럼 정보 조회가 실패한다. (내 결정)
   - 오프라인이 되기 전에 이미 큐에 있던 작업은 건드리지 않는다. 시작하면 지금처럼 실패하고, 다시 시도 버튼으로 재개한다. CLI(`kiri`)로 추가하는 경로도 바꾸지 않는다. (내 결정)

## 비목표

- 자동 업데이트를 켜고 끄는 설정은 두지 않는다(YAGNI).
- 특정 버전 고정이나 이전 버전으로 되돌리기는 하지 않는다.
- 첫 실행 마법사 같은 새 화면은 만들지 않는다.
- Intel Mac이나 다른 OS는 지원하지 않는다(kiri는 Apple Silicon 전용).

## 하면 안 되는 것

- 체크섬 검증 없이 실행 파일을 설치하는 것.
- 작업이 돌고 있는 동안 업데이트하다가 그 작업을 깨뜨리는 것. 설치는 지금처럼 임시 파일에 쓴 뒤 `rename`으로 바꿔서, 이미 실행 중인 프로세스는 옛 파일(inode)을 계속 쓰게 한다(`install_executable`).
- 오프라인일 때 최신 버전 확인이 실패했다고 오류를 띄우는 것. 확인 실패는 조용히 넘기고 "현재 버전"만 보여준다. 설치가 실패한 경우만 오류를 보인다.

## 출처와 검증

| 도구 | 최신 버전 확인 | 받는 파일 | 체크섬 |
|------|---------------|-----------|--------|
| FFmpeg (ffmpeg, ffprobe) | `https://ffmpeg.martin-riedl.de/redirect/latest/macos/arm64/release/{ffmpeg,ffprobe}.zip`가 `…/download/macos/arm64/<id>_<ver>/<name>.zip`로 리다이렉트된다. 경로에서 `<ver>`를 꺼낸다. 이 redirect는 HEAD에 404를 주므로 GET의 `Location`만 읽는다(`fetch-ffmpeg.sh` 참고). 두 파일의 버전이 같아야 한다. | 리다이렉트된 zip | `<zip url>.sha256` (`<64 hex>  ffmpeg.zip`) |
| yt-dlp | `https://github.com/yt-dlp/yt-dlp/releases/latest`가 `/tag/<ver>`로 리다이렉트된다 | 지금과 같다(`yt-dlp_macos`) | 지금과 같다(`SHA2-256SUMS`) |
| Deno | `https://github.com/denoland/deno/releases/latest` → `/tag/v<ver>` | 지금과 같다 | 지금과 같다(`.sha256sum`) |

- 받은 버전 문자열은 숫자와 점만 허용하고(앞의 `v`는 뗀다), 다운로드 URL은 위 호스트의 `https://`만 허용한다. vee-app의 `Release::valid` 규칙과 같다.
- 업데이트할 때는 확인할 때 얻은 그 URL(버전이 박힌 경로)로 받는다. 확인과 다운로드 사이에 새 릴리스가 나와도 보여준 버전과 받는 버전이 같다.
- **현재 버전**은 실행 파일에 물어본다. `ffmpeg -version`의 첫 줄 `ffmpeg version 9.0.2 …`에서 버전을 뽑고, `yt-dlp --version`과 `deno --version`도 같은 식이다(`tools::version`을 확장). "업데이트 가능"은 현재 버전 문자열과 최신 버전 문자열이 다를 때다.
- HTTP는 지금의 `tools::http_client()`(reqwest, https only)를 쓴다. 새 의존성은 없다.

## 언제 확인하나

- **시작할 때:** 없는 도구만 설치한다. 최신 버전 확인은 하지 않는다.
- **업데이트 탭을 열 때:** 세 도구의 최신 버전을 확인한다. 결과는 메모리(`ToolsState`)에만 두고 마지막 확인 시각을 저장한다. 기존 `settings.update.last_ytdlp_check` 필드를 "마지막 확인"으로 그대로 쓰며, 이름은 바꾸지 않는다(설정 JSON 호환).
- **다운로드가 실패했을 때:** 아직 확인한 적이 없거나 확인한 지 1시간이 넘었으면 yt-dlp 최신 버전만 확인한다. 더 새 버전이 있으면 토스트를 띄운다. 토스트는 같은 최신 버전에 대해 앱 세션당 한 번만 뜬다. 여러 작업이 실패해도 한 번이다. (내 결정)
  - "실패"는 **이번 세션에서 실패한 `youtube` 작업**과 **URL을 붙여넣었을 때 정보 조회(`probe`) 실패**를 말한다. `file` 작업은 yt-dlp와 상관없으니 제외하고, 이전 세션에서 이미 실패해 있던 작업도 제외한다. (내 결정)
  - 1시간 기준 시각은 메모리에만 둔다. 실패 때 하는 yt-dlp 확인은 설정의 "마지막 확인" 시각을 바꾸지 않는다. 업데이트 탭을 열어서 한 확인은 yt-dlp도 확인하므로 1시간 기준에 포함한다. (내 결정)
- 매시간 돌던 백그라운드 루프(`tools::commands::spawn`의 `RECHECK`)는 시작할 때 한 번 설치하는 것으로 줄인다.

## 상태 모양 (제안, 빌더 재량)

`ToolsStatus`가 도구별 정보를 갖는다. 예를 들면 `tools: { ytdlp | ffmpeg | deno: { current: Option<String>, latest: Option<String> } }`에 `installing`, `error`, `last_check`, `ready`를 더한다. `tools://changed` 이벤트로 모든 창이 같은 상태를 보는 구조는 그대로 둔다. 명령 이름 예시는 `check_tool_updates`, `update_tool(name)`, `update_all_tools`이고, 기존 `update_tools`는 대체한다.

## 번들에서 FFmpeg 제거

- `tauri.conf.json` `externalBin`에서 `binaries/ffmpeg`, `binaries/ffprobe`를 뺀다(`kiri-cli`는 남긴다).
- `scripts/fetch-ffmpeg.sh`, `scripts/ffmpeg.lock`을 지우고, `package.json`의 `sidecars` 스크립트는 `build-cli.sh`만 부른다.
- `lib.rs`의 `Tools.ffmpeg/ffprobe`는 `bin.join("ffmpeg")`, `bin.join("ffprobe")`로 바꾼다.
- 기존 사용자는 이 버전으로 업데이트한 뒤 첫 실행에서 FFmpeg를 한 번 받는다. 받는 동안은 지금 첫 실행 때처럼 `ready=false`이고 토스트가 보인다.
- 그 첫 실행이 오프라인이면 인터넷에 연결될 때까지 아무것도 할 수 없다. 로컬 파일 변환도 마찬가지이고, 지금의 "준비 실패" 토스트가 보인다. 이건 받아들인다(사용자 결정). `ready`를 도구별로 나누지 않고, FFmpeg를 비상용으로 번들에 남기지도 않는다.
- `fetch-ffmpeg.sh`가 하던 인코더 확인(libx264, libx265, libvpx-vp9, libmp3lame, libopus, videotoolbox, prores)은 런타임으로 옮기지 않는다. martin-riedl 빌드에 들어 있다는 걸 이번에 한 번 손으로 확인하고 끝낸다. (내 결정)
- 문서를 갱신한다: `docs/agents/architecture.md`(sidecar 줄), `build-run.md`, `configuration.md`, `docs/development.md`, `docs/release.md`, README에서 ffmpeg 번들을 언급한 곳. README의 네트워크·개인정보 문구에 martin-riedl.de도 넣는다.

## 제약

- 의존성(`Cargo.toml`, `package.json`)을 바꾸지 않는다.
- `kiri-core`에 다운로드·파싱 로직을 둔다(지금 `tools.rs`와 같은 자리). 파싱은 순수 함수로 두고 단위 테스트한다. `src-tauri`는 얇은 명령과 상태만 맡는다.
- 사용자 문구는 `src/shared/i18n/{en,ko}.ts`에만 둔다.
- UI는 Aurora Glass와 설정 빌딩 블록(`SettingSection`, `SettingRow`, `Button`)을 쓰고, 토스트는 기존 `Toasts`를 쓴다. 라이트/다크와 한국어/English를 확인한다.
- 네 가지 검사(`cargo test --workspace`, `yarn test`, `yarn typecheck`, `yarn lint`)를 통과해야 한다.

## 빌더에게 맡기는 것

- `ToolsStatus`의 정확한 필드 구성과 명령 이름.
- 버전 파싱 세부(예: `ffmpeg version n9.0.2-…` 같은 접두어 처리).
- 요약 줄의 정확한 문구(i18n 키 이름 규칙은 따른다).
- 업데이트 중에 다른 업데이트 버튼을 막는 방식. 지금의 `busy` 플래그를 쓰면 된다.
