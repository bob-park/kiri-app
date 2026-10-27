# 개발 환경 구성과 빌드

kiri는 **Apple Silicon Mac 전용**이다. 빌드와 릴리스는 개인 장비에서 하고, GitHub Actions는 버전 올림에만 쓴다. 릴리스 절차는 [release.md](release.md)를 본다.

## 필요한 도구

| 도구 | 버전 | 비고 |
|---|---|---|
| macOS | 14.2 이상, Apple Silicon | |
| Xcode Command Line Tools | 최신 | `xcode-select --install` |
| Rust | stable (edition 2024) | [rustup](https://rustup.rs)으로 설치 |
| Node.js | 26 | `.mise.toml`에 고정 |
| Yarn | 4.18.1 | `.mise.toml`에 고정, `nodeLinker: node-modules` |
| [mise](https://mise.jdx.dev) | 최신 | Node/Yarn 버전을 맞춘다. node/yarn 명령은 mise로 실행한다(`mise exec -- yarn …` 또는 mise가 활성화된 셸) |
| GitHub CLI | 최신 | 릴리스할 때만. `brew install gh` 후 `gh auth login` |

## 처음 한 번

```bash
git clone git@github.com:bob-park/kiri-app.git
cd kiri-app
mise install                                  # Node 26, Yarn 4.18.1
export GITHUB_NPM_AUTH_TOKEN=<GitHub PAT>     # read:packages 권한 — @bob-park/* (ESLint/Prettier 설정) 설치용
yarn install
yarn sidecars                                 # kiri CLI 를 src-tauri/binaries/ 에 준비
```

- `@bob-park/*` 패키지는 GitHub Packages에서 받는다(`.yarnrc.yml`의 `npmScopes`). 토큰이 없으면 `yarn install`이 실패한다. 셸 프로필에 `export`해 두면 편하다.
- 전역 Yarn 1.x로 실행하면 `yarn.lock`이 Yarn 1 형식으로 통째로 바뀐다. `yarn --version`이 4.18.1인지 확인한다.
- `yarn sidecars`는 `kiri` CLI 릴리스 빌드를 `src-tauri/binaries/`(git에 넣지 않음)에 준비한다. `scripts/build-cli.sh`가 만들어 `kiri-cli-aarch64-apple-darwin`으로 복사한다.
- yt-dlp, FFmpeg(ffmpeg/ffprobe), Deno는 번들에 넣지 않는다. 앱이 처음 실행될 때 `<app data>/bin/`에 받고(SHA256 확인), 업데이트는 설정 → 업데이트에서 사용자가 직접 한다.
- **새로 clone 했다면 `cargo build`, `cargo test`, `yarn tauri dev`보다 먼저 `yarn sidecars`를 실행한다.** Tauri가 빌드할 때 `externalBin` 파일이 있는지 검사하므로, 없으면 빌드가 실패한다.

## 개발 실행

```bash
yarn tauri dev
```

- Vite 개발 서버(`http://localhost:1420`)와 Rust 앱이 함께 뜬다. 프론트 코드는 즉시 반영되고, Rust 코드나 `tauri.conf.json`을 바꾸면 다시 빌드되고 재시작된다.
- 창은 메인(`main`)과 설정(`settings`) 두 개다. 설정 창은 메인 창에서 `⌘,`로 연다.
- 웹 개발자 도구: 창에서 우클릭 → 검사. React Query devtools는 개발 빌드에서만 화면 우하단에 뜬다.
- **메뉴 막대 상주 앱이라 창을 닫아도 프로세스가 남는다.** `yarn tauri dev`를 다시 실행하기 전에 메뉴 막대 → **종료**를 한다. 남아 있으면 중복 실행 방지 때문에 새 인스턴스는 기존 창만 띄우고 꺼진다.
- **개발 빌드도 설치된 앱과 같은 데이터 폴더를 쓴다**(`org.bobpark.kiri`). 실제 설정과 큐를 건드리므로, 실험 전에 아래 [데이터 위치](#데이터-위치)의 폴더를 백업한다. CLI 소켓도 하나뿐이라 설치된 kiri가 떠 있으면 먼저 종료한다.
- **CLI를 dev 앱에 붙이기:** `./target/release/kiri list`. 소켓 경로는 환경변수 `KIRI_SOCKET`으로 바꿀 수 있다.
- **`.kiripart` 표시:** `yarn tauri dev`는 `.app` 번들이 아니라서 `.kiripart`가 폴더로 보인다. 패키지(파일 하나)로 보이는지 확인하려면 번들을 만들어 macOS에 등록한다.
  ```sh
  yarn tauri build --debug --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}'
  /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f target/debug/bundle/macos/kiri.app
  ```
- 첫 실행이나 의존성을 추가한 직후에는 Vite가 의존성을 다시 최적화하면서 페이지를 한 번 새로 고친다. 이때 콘솔에 "Invalid hook call"이 잠깐 보일 수 있지만, 새로 고침 뒤에는 사라진다.

## 테스트와 검사

```bash
cargo test --workspace   # Rust: kiri-core(엔진·파이프라인·파서·IPC), kiri-cli, 앱(DB·마이그레이션·도메인·에러 직렬화)
yarn test                # 프론트 순수 로직(src/**/*.test.ts) + 스크립트(scripts/*.test.mjs), node:test
yarn typecheck           # TypeScript 타입 검사 (ko 사전 키 누락도 여기서 잡힌다)
yarn lint                # ESLint + Prettier (@bob-park 설정)
```

네 가지 모두 통과해야 변경이 끝난 것이다. `yarn prettier`로 포맷을 일괄 적용한다.

## 빌드

```bash
yarn tauri build --no-bundle         # 서명 키 없이 릴리스 컴파일만 확인
yarn tauri build --bundles app,dmg   # .app/.dmg + 업데이터 아티팩트 (서명 키 필요, 아래 서명 준비 참고)
scripts/release-macos.sh --build-only   # Developer ID 서명 + API 키로 .app/.dmg 공증 (업로드 없음)
```

- 결과물은 워크스페이스의 `target/release/bundle/`에 생긴다.
- `createUpdaterArtifacts`가 켜져 있어서 번들 빌드에는 `TAURI_SIGNING_PRIVATE_KEY`가 필요하다.

## 서명 준비 (한 번)

모두 로그인 키체인과 `~/.config/apple/`에 둔다. 저장소에는 절대 넣지 않는다. `scripts/release-macos.sh`가 여기서 읽어 온다.

### Apple Developer ID와 공증 API 키

- "Developer ID Application" 인증서가 키체인에 있어야 한다. `security find-identity -v -p codesigning`으로 확인한다.
- `~/.config/apple/.env` (앱 공통):

  ```sh
  APPLE_SIGNING_IDENTITY="Developer ID Application: <이름> (<TEAM_ID>)"
  ```

- 공증은 App Store Connect **API 키** 방식이다(Apple ID/앱 전용 암호 방식은 쓰지 않는다).
  1. App Store Connect → 사용자 및 액세스 → 통합 → App Store Connect API에서 키를 만든다(액세스: Developer).
  2. 내려받은 `AuthKey_<키 ID>.p8`을 `~/.config/apple/`에 둔다. 한 번만 내려받을 수 있으니 백업한다.
  3. 키 ID와 발급자 ID(Issuer ID)를 로그인 키체인에 넣는다.

     ```sh
     security add-generic-password -a "$USER" -s tauri-apple-api-key -w '<키 ID>'
     security add-generic-password -a "$USER" -s tauri-apple-api-issuer -w '<Issuer ID>'
     ```

- 환경에 예전 `APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID`가 남아 있으면 지운다. `APPLE_ID`가 있으면 Tauri가 API 키 대신 그쪽으로 공증한다. 예전 `~/.config/kiri/sign.env`는 더 쓰지 않는다.

### 업데이트 서명 키

키는 이미 만들어져 있고, 공개키는 `src-tauri/tauri.conf.json`의 `plugins.updater.pubkey`에 들어 있다. 새 Mac에서는 개인키 **내용**(파일 경로가 아님)과 암호를 키체인에 넣는다.

```sh
security add-generic-password -a "$USER" -s tauri-updater-kiri -w "$(cat <개인키 파일>)"
security add-generic-password -a "$USER" -s tauri-updater-kiri-pw -w '<키 암호>'
```

- 직접 `yarn tauri build --bundles app,dmg`를 할 때는 먼저 `source scripts/load-updater-key-macos.sh`를 실행한다.
- 키 암호가 맞는지 미리 확인하려면 작은 파일에 서명해 본다. `.sig` 파일이 생기면 정상이다.

  ```sh
  echo test > /tmp/t && ( source scripts/load-updater-key-macos.sh && yarn tauri signer sign /tmp/t ) && ls /tmp/t.sig
  ```

- 개인키 파일은 오프라인 백업만 남기고 작업 기계에서 지운다. 개인키와 암호를 잃어버리면 이미 설치된 앱에 더 이상 업데이트를 보낼 수 없다.
- 키를 새로 만들면(`yarn tauri signer generate`) 이미 설치된 앱은 새 키로 서명된 업데이트를 받지 못하고, 사용자가 직접 다시 설치해야 한다. 개인키를 잃어버렸을 때만 바꾼다.
- 공개키가 비어 있으면 릴리스 빌드는 `updater pubkey is empty — see docs/development.md` 오류로 실패하고(`build.rs`), `scripts/latest-json.mjs`도 실행을 거부한다.

## 프로젝트 구조

프론트(`src/domain/<name>`)와 Rust(`src-tauri/src/domain/<name>`)는 **같은 도메인 이름으로 짝을 이룬다.**

```
Cargo.toml                    # 워크스페이스: crates/kiri-core, crates/kiri-cli, src-tauri
crates/kiri-core/             # Tauri 비의존 엔진: 작업 모델, 큐(Engine, QueueStore), 다운로드 파이프라인,
                              # yt-dlp/ffmpeg 인자·파서, 프로세스 실행, yt-dlp·FFmpeg·Deno 설치, CLI 소켓 프로토콜
crates/kiri-cli/              # `kiri` CLI. 실행 중인 앱과 Unix 소켓으로 통신
src/                          # React (Vite SPA). 창 라벨(main/settings)로 화면을 고른다
├─ app/                       # main.tsx(진입점), App.tsx(메인 창), globals.css(색·토큰은 여기에만), settings/(설정 창)
├─ domain/<name>/             # settings, queue, tools, updater, cli — apis · queries · components · store
├─ shared/                    # api(call), i18n(en/ko), providers(theme, i18n), components, hooks, store
└─ utils/                     # 순수 함수만 (+ *.test.ts)
src-tauri/
├─ tauri.conf.json            # 번들(app, dmg), sidecar, 업데이터
├─ capabilities/default.json  # 웹뷰 권한
├─ binaries/                  # yarn sidecars 결과 (git 제외)
└─ src/
   ├─ lib.rs                  # 조립만: 플러그인, setup, command 등록
   ├─ app/                    # 앱 껍데기: state, windows, tray(+앱 메뉴), updater, quit, cli_install, ipc_server, i18n(네이티브 라벨)
   ├─ domain/<name>/          # settings, queue: mod.rs(공개 API) · model.rs · repository.rs(SQL) · commands.rs / tools: mod.rs · commands.rs
   └─ shared/                 # db.rs(열기·마이그레이션), migrations/*.sql, error.rs(AppError)
scripts/                      # build-cli.sh, release-macos.sh, load-updater-key-macos.sh, latest-json.mjs
bump-version-up.sh            # 버전 올림 (master 머지 때 GitHub Actions가 실행)
docs/agents/                  # 에이전트용 규칙 (AGENTS.md에서 시작)
docs/design/                  # 디자인 기준: aurora-glass.md, kraken-design.md
```

새 기능을 추가하는 순서와 규칙은 [AGENTS.md](../AGENTS.md)와 [docs/agents/structure.md](agents/structure.md)를 본다.

## 데이터 위치

| 내용 | 경로 |
|---|---|
| DB(`kiri.db`: 설정, 큐), 구성 요소(`bin/`의 yt-dlp, Deno), CLI 소켓(`kiri.sock`) | `~/Library/Application Support/org.bobpark.kiri/` |
| 앱 로그, 작업별 로그(`jobs/<id>.log`) | `~/Library/Logs/org.bobpark.kiri/` |
| 받은 파일 (기본값) | `~/Movies/kiri/` |

- 이전 버전의 `settings.json`·`queue.json`은 첫 실행 때 DB로 옮겨지고 `*.json.migrated`로 이름이 바뀐다.
- 처음부터 다시 시작하려면 앱을 종료한 뒤 `kiri.db`를 지운다. 설정과 큐가 모두 사라진다.

DB 스키마는 `PRAGMA user_version`으로 관리한다. 스키마를 바꾸는 방법:

1. `src-tauri/src/shared/migrations/000N_<설명>.sql`을 추가한다.
2. `shared/db.rs`의 `MIGRATIONS`에 이어 붙인다.

배포된 마이그레이션 파일은 고치지 않는다.
