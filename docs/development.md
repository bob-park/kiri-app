# 개발 환경 구성과 빌드

## 필요한 도구

| 도구 | 버전 | 비고 |
|---|---|---|
| Rust | stable 1.85 이상 (edition 2024) | [rustup](https://rustup.rs)으로 설치 |
| Node.js | 26 | `.mise.toml`에 고정 |
| Yarn | 4.18.1 | `.mise.toml`에 고정, `nodeLinker: node-modules` |
| [mise](https://mise.jdx.dev) | 최신 | 권장. Node/Yarn 버전을 자동으로 맞춤 |

OS별 추가 준비:

- **macOS**: Xcode Command Line Tools (`xcode-select --install`)
- **Windows**:
  - [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)의 "C++를 사용한 데스크톱 개발" 워크로드
  - WebView2 런타임 (Windows 10/11에는 기본 설치)

## 처음 한 번

```bash
mise install                                  # Node 26, Yarn 4.18.1
export GITHUB_NPM_AUTH_TOKEN=<GitHub PAT>     # read:packages 권한 — @bob-park/* (ESLint/Prettier 설정) 설치용
yarn install
```

- `@bob-park/*` 패키지는 GitHub Packages에서 받는다(`.yarnrc.yml`의 `npmScopes`). 그래서 토큰이 없으면 `yarn install`이 실패한다. 셸 프로필에 `export`해 두면 편하다.
- `.yarnrc.yml`은 `npmMinimalAgeGate: 0`(막 공개된 패키지도 설치)과 `enableScripts: true`(패키지 설치 스크립트 실행)로 설정되어 있다.

## 개발 실행

```bash
yarn tauri dev
```

- Vite 개발 서버(`http://localhost:1420`)와 Rust 앱이 함께 뜬다.
  - 프론트 코드는 즉시 반영된다.
  - Rust 코드나 `tauri.conf.json`을 바꾸면 앱이 다시 빌드되고 재시작된다.
- 창은 메인(`main`)과 설정(`settings`) 두 개다. 설정 창은 다음 방법으로 연다.
  - `Cmd+,` (Windows는 `Ctrl+,`)
  - 앱 메뉴 → 설정…
  - 트레이 → 설정
- 웹 개발자 도구: 창에서 우클릭 → 검사. React Query devtools는 개발 빌드에서만 화면 우하단에 뜬다.
- **트레이 상주 앱이라 창을 닫아도 프로세스가 남는다.**
  - `yarn tauri dev`를 다시 실행하기 전에 트레이 → **종료**를 한다.
  - 남아 있으면 중복 실행 방지 때문에 새 인스턴스는 기존 창만 띄우고 꺼진다.
- 개발 빌드는 자동 업데이트 확인을 하지 않는다. 6시간 주기 확인은 release 빌드에서만 한다.
- 첫 실행이나 의존성을 추가한 직후에는 Vite가 의존성을 다시 최적화하면서 페이지를 한 번 새로 고친다. 이때 콘솔에 "Invalid hook call"이 잠깐 보일 수 있지만, 새로 고침 뒤에는 사라진다.

## 테스트와 검사

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib   # Rust: DB/마이그레이션, 도메인, 에러 직렬화, 업데이트 상태
yarn test                                               # 프론트 순수 로직(src/**/*.test.ts) + 스크립트(scripts/*.test.mjs), node:test
yarn typecheck                                          # TypeScript 타입 검사 (ko 사전 키 누락도 여기서 잡힌다)
yarn lint                                               # ESLint + Prettier (@bob-park 설정)
```

`yarn prettier`로 포맷을 일괄 적용한다.

## 빌드

```bash
yarn tauri build --no-bundle   # 서명 키 없이 컴파일만 확인
yarn tauri build               # .app/.dmg/NSIS 번들 + 업데이터 아티팩트 (서명 키 필요)
```

- 결과물은 `src-tauri/target/release/bundle/`에 생긴다.
- `createUpdaterArtifacts`가 켜져 있어서 번들 빌드에는 `TAURI_SIGNING_PRIVATE_KEY`가 필요하다.
- 서명, 공증, GitHub 릴리스 업로드는 [릴리스 절차](release.md)를 따른다.

## 프로젝트 구조

프론트(`src/domain/<name>`)와 Rust(`src-tauri/src/domain/<name>`)는 **같은 도메인 이름으로 짝을 이룬다.**

```
src/                          # React (Vite SPA). 창 라벨(main/settings)로 화면을 고른다
├─ app/
│  ├─ main.tsx                # 진입점: 테마·언어를 먼저 적용하고 Provider로 감싸 렌더
│  ├─ App.tsx                 # 메인 창
│  ├─ globals.css             # Tailwind + daisyUI 테마 (색은 여기에만)
│  └─ settings/               # 설정 창 (SettingsApp, _components/Tabs 등)
├─ domain/<name>/             # apis(Tauri command 호출 + dto) · queries(React Query) · components · store(Zustand)
├─ shared/                    # api(call), i18n, providers(theme, i18n), components, hooks, queries, store
└─ utils/                     # 순수 함수만 (+ *.test.ts)

src-tauri/
├─ tauri.conf.json            # 창 2개, 번들, 업데이터
├─ capabilities/default.json  # 웹뷰 권한
└─ src/
   ├─ lib.rs                  # 조립만: 플러그인, setup, command 등록
   ├─ app/                    # 앱 껍데기: state, window, menu, tray, updater, autostart, i18n(네이티브 라벨)
   ├─ domain/<name>/          # mod.rs(공개 API) · model.rs · repository.rs(SQL) · commands.rs(Tauri 어댑터)
   └─ shared/                 # db.rs(열기·마이그레이션), migrations/*.sql, error.rs(AppError)

scripts/                      # 로컬 릴리스 스크립트, latest.json 생성, 테스트
bump-version-up.sh            # 버전 올림 (GitHub Actions가 develop 머지 때 실행)
```

새 기능을 추가하는 순서와 규칙은 [AGENTS.md](../AGENTS.md)와 [docs/agents/structure.md](agents/structure.md)를 본다.

## 데이터 초기화

앱을 종료한 뒤 아래 폴더를 지운다. 설정이 모두 사라진다.

- macOS: `~/Library/Application Support/com.bobpark.app/`
- Windows: `%APPDATA%\com.bobpark.app\`

DB 스키마는 `PRAGMA user_version`으로 관리한다. 스키마를 바꾸는 방법:

1. `src-tauri/src/shared/migrations/000N_<설명>.sql`을 추가한다.
2. `shared/db.rs`의 `MIGRATIONS`에 이어 붙인다.

배포된 마이그레이션 파일은 고치지 않는다.
