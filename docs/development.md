# kiri 개발 · 빌드 · 릴리즈

kiri는 **Apple Silicon Mac 전용**이고, 빌드와 릴리즈는 로컬에서 합니다(CI 없음). Windows 빌드는 [7. Windows 빌드](#7-windows-빌드)를 보세요.

## 1. 개발 환경 구성 (한 번)

### 필요한 도구

| 도구 | 버전 | 설치 |
|---|---|---|
| macOS | 14.2 이상, Apple Silicon | |
| Xcode Command Line Tools | 최신 | `xcode-select --install` |
| Rust | stable (edition 2024, 1.85 이상) | `curl https://sh.rustup.rs -sSf \| sh` |
| Node.js | 24 | `mise use node@24` 또는 공식 설치본 |
| Yarn | 4 (Berry) | `corepack enable` (package.json의 `packageManager`를 따름) |
| GitHub CLI | 최신 (릴리즈할 때만) | `brew install gh` 후 `gh auth login` |

### 저장소 준비

```sh
git clone git@github.com:bob-park/kiri-app.git
cd kiri-app
yarn install
yarn sidecars
```

`yarn sidecars`는 두 가지를 `src-tauri/binaries/`에 준비합니다.
- ffmpeg/ffprobe arm64 정적 빌드: `scripts/fetch-ffmpeg.sh`가 받습니다. URL과 SHA256은 `scripts/ffmpeg.lock`에 고정되어 있고, 필요한 인코더가 모두 들어 있는지도 검사합니다.
- `kiri` CLI 릴리즈 빌드: `scripts/build-cli.sh`가 만들어 `kiri-cli-aarch64-apple-darwin`으로 복사합니다.

> **새로 clone 했다면 `cargo build`, `cargo test`, `yarn tauri dev`보다 먼저 `yarn sidecars`를 실행하세요.** Tauri가 빌드할 때 `externalBin` 파일이 있는지 검사하므로, 없으면 빌드가 실패합니다. `src-tauri/binaries/`는 git에 넣지 않습니다.

ffmpeg 버전을 올릴 때는 `sh scripts/fetch-ffmpeg.sh --update`로 `scripts/ffmpeg.lock`을 갱신하고 커밋합니다.

## 2. 프로젝트 구조

```
crates/kiri-core/   Tauri 비의존 코어: 작업 모델, yt-dlp/ffmpeg 인자·파서, 프로세스 실행,
                    다운로드 파이프라인, 큐 엔진, CLI 소켓 프로토콜, yt-dlp·Deno 설치
crates/kiri-cli/    `kiri` CLI 바이너리
src-tauri/          Tauri 앱 셸: 설정, 명령, 메뉴 막대, 종료 처리, 업데이터, CLI 설치, Info.plist
src/                React 프론트엔드: 메인 창, 옵션 시트, 환경설정 창, 다국어(locales), 테마
scripts/            fetch-ffmpeg.sh, build-cli.sh, latest-json.mjs
docs/superpowers/   설계 스펙과 구현 계획
```

## 3. 개발

```sh
yarn tauri dev                      # 앱 실행 (프론트엔드 핫 리로드)
cargo test --workspace && yarn test # 전체 테스트
yarn build                          # 타입 검사 + 프론트엔드 빌드
```

- **도구 위치:** 앱이 처음 실행될 때 yt-dlp와 Deno를 `~/Library/Application Support/org.bobpark.kiri/bin/`에 받습니다. yt-dlp는 하루에 한 번 갱신을 확인합니다.
- **데이터 위치:**
  - 설정: `~/Library/Application Support/org.bobpark.kiri/settings.json`
  - 큐: `~/Library/Application Support/org.bobpark.kiri/queue.json`
  - 작업별 로그: `~/Library/Logs/org.bobpark.kiri/jobs/<id>.log`
- **CLI를 dev 앱에 붙이기:** `./target/release/kiri list`. 소켓 경로는 환경변수 `KIRI_SOCKET`으로 바꿀 수 있습니다.
- **`.kiripart` 표시:** `yarn tauri dev`는 `.app` 번들이 아니라서 `.kiripart`가 폴더로 보입니다. 패키지(파일 하나)로 보이는지 확인하려면 번들을 만들어 macOS에 등록하세요.
  ```sh
  yarn tauri build --debug --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}'
  /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f target/debug/bundle/macos/kiri.app
  ```

## 4. 서명 준비 (한 번)

### Apple Developer ID

- "Developer ID Application" 인증서가 키체인에 있어야 합니다. `security find-identity -v -p codesigning`으로 확인합니다.
- 공증에 쓸 앱 전용 암호를 https://account.apple.com 에서 만듭니다.

### 업데이트 서명 키

```sh
yarn tauri signer generate -w ~/.tauri/kiri.key   # 암호를 정해 입력
```

- 공개키(`~/.tauri/kiri.key.pub` 내용)는 `src-tauri/tauri.conf.json`의 `plugins.updater.pubkey`에 넣습니다. 현재 저장소에는 이미 들어 있습니다.
- 개인키와 암호를 잃어버리면 이미 설치된 앱에 더 이상 업데이트를 보낼 수 없습니다. 따로 안전하게 보관하세요.
- 공개키가 비어 있으면 `scripts/latest-json.mjs`가 릴리즈를 거부합니다.
- 릴리즈 빌드(`--debug` 없는 `yarn tauri build`)는 `src-tauri/tauri.conf.json`의 `plugins.updater.pubkey`가 비어 있거나 `<PUBKEY>`이면 `updater pubkey is empty — see docs/development.md` 오류로 실패합니다.

### `~/.config/kiri/sign.env` (커밋 금지)

```sh
APPLE_SIGNING_IDENTITY="Developer ID Application: <이름> (<TEAM_ID>)"
APPLE_ID="<apple id 이메일>"
APPLE_PASSWORD="<앱 전용 암호>"
APPLE_TEAM_ID="<TEAM_ID>"
TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/kiri.key"
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="<업데이트 키 암호>"
```

키 암호가 맞는지 미리 확인하려면 작은 파일에 서명해 봅니다. `.sig` 파일이 생기면 정상입니다.

```sh
echo test > /tmp/t && ( set -a; . ~/.config/kiri/sign.env; set +a;
  TAURI_SIGNING_PRIVATE_KEY="$(cat "$TAURI_SIGNING_PRIVATE_KEY")" yarn tauri signer sign /tmp/t ) && ls /tmp/t.sig
```

## 5. 릴리즈

예시는 버전 `0.2.0` 기준입니다. 모든 명령은 같은 셸에서 이어서 실행합니다.

1. **버전 올리기:** 아래 5곳의 `version`을 같은 값으로 바꾸고, 바뀐 `Cargo.lock`과 함께 커밋합니다.
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`
   - `crates/kiri-core/Cargo.toml`
   - `crates/kiri-cli/Cargo.toml`
2. **테스트와 sidecar:**
   ```sh
   cargo test --workspace && yarn test
   yarn sidecars
   ```
3. **빌드, 서명, `.app` 공증, 업데이트 파일 생성:** 업데이트 키는 파일 경로가 아니라 **파일 내용**으로 넘깁니다.
   ```sh
   set -a; . ~/.config/kiri/sign.env; set +a
   TAURI_SIGNING_PRIVATE_KEY="$(cat "$TAURI_SIGNING_PRIVATE_KEY")" yarn tauri build --bundles app,dmg
   ```
   `target/release/bundle/`에 결과물이 생깁니다.
   - `macos/kiri.app`
   - `macos/kiri.app.tar.gz`, `macos/kiri.app.tar.gz.sig` (업데이트용)
   - `dmg/kiri_0.2.0_aarch64.dmg`
4. **`.dmg` 공증과 확인:**
   ```sh
   DMG=target/release/bundle/dmg/kiri_0.2.0_aarch64.dmg
   xcrun notarytool submit "$DMG" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" --wait
   xcrun stapler staple "$DMG"
   spctl --assess --type open --context context:primary-signature -vv "$DMG"
   spctl --assess --type execute -vv target/release/bundle/macos/kiri.app
   ```
   두 `spctl` 모두 `accepted`, `source=Notarized Developer ID`가 나와야 합니다.
5. **GitHub Release:**
   ```sh
   B=target/release/bundle/macos
   gh release create v0.2.0 --target master --title "kiri v0.2.0" "$DMG" "$B/kiri.app.tar.gz" "$B/kiri.app.tar.gz.sig" --notes "…"
   ```
6. **업데이트 매니페스트:**
   ```sh
   node scripts/latest-json.mjs v0.2.0
   ```
   릴리즈에 올라간 `.sig`로 `latest.json`을 만들어 같은 릴리즈에 올립니다. 앱은 `https://github.com/bob-park/kiri-app/releases/latest/download/latest.json`을 확인합니다.

## 6. 릴리즈 후 확인

- 새로 받은 dmg를 열어 Gatekeeper 경고 없이 설치·실행되는지 확인합니다.
- 이전 버전 앱에서 설정 > 업데이트 > 지금 확인 → 배너 → 설치 후 재시작 → 새 버전으로 바뀌는지 확인합니다.

## 7. Windows 빌드

### 현재 상태: 아직 지원하지 않음

kiri는 macOS 전용으로 설계되어 있어서, **지금 코드는 Windows에서 그대로 빌드되거나 동작하지 않습니다.** Windows 설치 파일을 만들려면 먼저 아래 부분을 Windows용으로 옮겨야 합니다.

| 영역 | 지금 (macOS) | 위치 | Windows에서 필요한 것 |
|---|---|---|---|
| 다운로드 중지 | Unix 프로세스 그룹 kill (`process_group`, `killpg`) | `crates/kiri-core/src/runner.rs` | Job Object로 자식 프로세스 트리 종료 |
| CLI ↔ 앱 통신 | Unix 소켓 (`tokio::net::UnixListener`) | `crates/kiri-core/src/ipc.rs`, `crates/kiri-cli` | Named pipe (`tokio::net::windows::named_pipe`) |
| 실행 권한·파일 처리 | `PermissionsExt`(chmod), `/usr/bin/ditto`로 zip 풀기 | `crates/kiri-core/src/tools.rs`, `files.rs` | chmod 불필요, zip 풀기는 `zip` 크레이트 등 |
| 외부 도구 | `yt-dlp_macos`, `deno-aarch64-apple-darwin.zip` | `crates/kiri-core/src/tools.rs` | `yt-dlp.exe`, `deno-x86_64-pc-windows-msvc.zip` |
| ffmpeg sidecar | martin-riedl.de macOS arm64 빌드 | `scripts/fetch-ffmpeg.sh`, `scripts/ffmpeg.lock` | Windows x64 정적 빌드와 이를 받는 PowerShell 스크립트 |
| 하드웨어 인코딩 | VideoToolbox (`*_videotoolbox`, `-hwaccel videotoolbox`) | `crates/kiri-core/src/ffmpeg.rs` | NVENC / QSV / AMF 중 선택, 또는 소프트웨어만 |
| CLI 설치 | `/usr/local/bin/kiri` 링크 + `osascript` 관리자 권한 | `src-tauri/src/cli_install.rs` | 설치 폴더를 사용자 PATH에 추가 |
| 창·트레이 | 메뉴 막대, Dock 숨기기(activation policy), 앱 메뉴 | `src-tauri/src/windows.rs`, `tray.rs` | 시스템 트레이 (Dock·앱 메뉴 코드는 `#[cfg(target_os = "macos")]`로 분리) |
| `.kiripart` 표시 | Info.plist 패키지 등록 | `src-tauri/Info.plist` | 같은 개념이 없음 (폴더로 보임) |
| 기본 경로 | `$HOME/Movies/kiri`, `~/Library/...` | `src-tauri/src/settings.rs`, `ipc.rs` | `%USERPROFILE%\Videos\kiri`, `dirs` 크레이트 |
| 번들 설정 | `targets: ["app","dmg"]`, macOS 서명 설정 | `src-tauri/tauri.conf.json` | `tauri.windows.conf.json`에 `nsis`, `windows.certificateThumbprint` 등 |

이 작업은 따로 설계(스펙)부터 잡아서 진행하는 것을 권장합니다.

### 옮긴 뒤의 빌드 절차

Tauri의 Windows 설치 파일(NSIS `.exe`, `.msi`)은 **Windows에서 빌드**해야 합니다. macOS에서 교차 빌드(`cargo-xwin`)도 가능은 하지만 실험적이고, 코드 서명은 어차피 Windows 환경이 필요합니다. Windows PC를 쓰거나 GitHub Actions의 `windows-latest` 러너를 씁니다.

**1. 도구 준비 (Windows 10/11 x64)**

| 도구 | 설치 |
|---|---|
| Visual Studio Build Tools | "C++를 사용한 데스크톱 개발" 워크로드 (MSVC, Windows SDK) |
| WebView2 런타임 | Windows 10/11에는 대부분 이미 있음. 없으면 Microsoft에서 설치 |
| Rust | `rustup` 설치 후 `rustup default stable-msvc` |
| Node.js 24, Yarn 4 | Node 공식 설치본 + `corepack enable` |
| Git, GitHub CLI | `winget install Git.Git GitHub.cli` |

**2. 저장소 준비**

```powershell
git clone https://github.com/bob-park/kiri-app.git
cd kiri-app
yarn install
# ffmpeg/ffprobe Windows 빌드를 src-tauri\binaries\ffmpeg-x86_64-pc-windows-msvc.exe 등으로 준비
cargo build -p kiri-cli --release
copy target\release\kiri.exe src-tauri\binaries\kiri-cli-x86_64-pc-windows-msvc.exe
```

sidecar 이름은 `<이름>-x86_64-pc-windows-msvc.exe` 형식이어야 Tauri가 찾습니다.

**3. 빌드와 서명**

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $HOME\.tauri\kiri.key -Raw   # 업데이트 서명 키 (macOS와 같은 키)
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "<업데이트 키 암호>"
yarn tauri build --bundles nsis
```

- 결과물은 `target\release\bundle\nsis\kiri_<버전>_x64-setup.exe`와 그 `.sig`(업데이트용)입니다.
- 코드 서명 없이도 설치는 되지만, SmartScreen 경고가 뜹니다. 서명하려면 코드 서명 인증서(OV/EV) 또는 Azure Trusted Signing을 준비하고, `tauri.windows.conf.json`의 `bundle.windows`(`certificateThumbprint`, `digestAlgorithm`, `timestampUrl` 또는 `signCommand`)에 설정합니다.
- 업데이트 설치 방식은 `plugins.updater.windows.installMode`(예: `"passive"`)로 정합니다.

**4. 릴리즈와 업데이트 매니페스트**

- macOS와 같은 GitHub Release에 `kiri_<버전>_x64-setup.exe`와 `.sig`를 함께 올립니다.
- `scripts/latest-json.mjs`의 `PLATFORMS`에 Windows 항목을 추가해야 `latest.json`에 Windows 플랫폼이 들어갑니다.
  ```js
  const PLATFORMS = [
    { suffix: ".app.tar.gz.sig", key: "darwin-aarch64" },
    { suffix: "-setup.exe.sig", key: "windows-x86_64" },
  ];
  ```
- 두 플랫폼 파일을 모두 올린 뒤 `node scripts/latest-json.mjs v<버전>`을 한 번 실행합니다.
