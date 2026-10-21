# kiri 리디자인 · 시작 준비 · 다운로드 흐름 · 문서 규칙 리팩토링 설계

- 날짜: 2026-10-21
- 대상 버전: 0.3.1 이후

## 1. 배경과 의도

이 브랜치에서 네 가지 일을 한다.

1. **UI/UX 전체 리디자인:** 기존 화면에 큰 불만이 있는 건 아니다. 새로운 느낌을 주고 싶고, 부드럽고 신선한 애니메이션이 많기를 원한다. 사용자는 링크를 넣을 때 ⌘V만 쓴다.
2. **앱 시작 시 "다운로드 준비 중":** 사용자는 이게 무엇을 준비하는지 모른다. yt-dlp가 이미 있는데도 기다리게 하면 안 된다.
3. **다운로드 흐름:** 사용자는 다운로드가 끝나면 바로 재생하는 일이 많다. 그런데 지금은 변환이 필요한 작업이면 변환까지 끝나야 원본이 저장 폴더에 나타난다.
4. **문서 규칙 준수:** `AGENTS.md`와 `docs/agents/`에 정의한 구조와 규칙대로 코드를 리팩토링한다. 사용자는 문서 쪽에 **완전히** 맞추기를 원했다. 단 일본어는 필요 없다.

## 2. 목표 / 비목표 / 안티골

**목표**
- 메인 창과 설정 창을 "오로라 글래스" 스타일로 새로 디자인하고, 라이트와 다크 테마를 모두 지원한다.
- yt-dlp와 Deno가 이미 설치되어 있으면 앱을 켜자마자 바로 쓸 수 있다.
- 원본 다운로드가 끝나는 즉시 원본이 저장 폴더에 들어가 재생할 수 있다. 변환은 그 뒤에 그 원본 파일로 한다.
- 프론트엔드와 Rust 코드가 `docs/agents/` 규칙을 따르고, 문서에서 kiri와 맞지 않는 부분은 kiri에 맞게 고친다.

**비목표**
- Windows 지원은 하지 않는다. yt-dlp·Deno·ffmpeg 바이너리, `ditto`, VideoToolbox가 모두 macOS 전용이다. *(내 판단)*
- 작업 목록의 재생 버튼은 만들지 않는다. 사용자가 명시적으로 원하지 않았다.
- 새 기능(재생, 플레이리스트 등)은 추가하지 않는다.

**안티골 (기술적으로는 돌아가도 실패로 보는 것)**
- 리팩토링 후 기존 사용자의 설정이나 작업 목록이 사라지는 것.
- 애니메이션이 화려하지만 버벅이거나, `prefers-reduced-motion`을 무시하는 것.
- 변환이 실패하거나 취소됐을 때 원본까지 사라지는 것.
- "준비 중"이 이유 없이 몇 초씩 뜨는 것.

## 3. 제약

- 브랜치: `master`에서 `feature/aurora-redesign`을 따서 작업한다(사용자 결정). `main`/`develop`은 만들지 않는다. 대신 `workflow/branching.md`, `pr.md`, `versioning.md`, `.github/workflows/bump-version.yml`을 `master` 기준으로 고친다. *(문서·Actions 조정은 내 판단)*
- 의존성을 추가하거나 바꾸기 전에 사용자에게 확인받는다(CLAUDE.md). 목록은 §7에 있다.
- 이미 배포된 마이그레이션은 수정하지 않는다. 이번에 처음 도입하는 SQLite 마이그레이션은 `0001`부터 시작한다.
- 버전 파일은 직접 고치지 않는다(`bump-version-up.sh`가 담당).
- 사용자에게 보이는 문구는 전부 `src/shared/i18n/{en,ko}.ts`에 둔다.
- 완료 전에 다음을 모두 통과해야 한다: Rust 테스트(§6.5의 workspace 명령), `yarn test`, `yarn typecheck`, `yarn lint`. 또 라이트/다크 × 한국어/English로 앱을 직접 띄워서 확인한다.

## 4. 시작 시 준비 (yt-dlp · Deno)

**현재 동작** (`src-tauri/src/bootstrap.rs`, `crates/kiri-core/src/tools.rs`)
- `ToolsStatus.ready`는 `false`로 시작한다.
- `ensure()`가 끝나야 `ready`가 된다. `ensure()`는 다음을 순서대로 한다.
  1. Deno가 없으면 설치한다.
  2. 하루에 한 번 GitHub의 `SHA2-256SUMS`를 받아 yt-dlp와 해시를 비교하고, 다르면 새로 받는다.
  3. `yt-dlp --version`을 실행한다. PyInstaller 바이너리라 이 실행이 느리다.
- 그래서 파일이 이미 있어도 매번 몇 초 동안 "준비 중" 토스트가 보인다.
- 이미 있으면 다시 받지 않는 로직(해시 비교)은 이미 있다. 문제는 화면이 그 확인 과정이 끝나기를 기다린다는 것이다.

**변경**
- `ready`는 "두 실행 파일이 존재하는가"로만 정한다. 앱 상태를 처음 만들 때 바로 계산해서 넣는다.
- 업데이트 확인(해시 비교, 하루 1회)과 버전 조회는 백그라운드에서 한다. 이 과정에서는 `ready`를 `false`로 되돌리지 않는다.
- "준비 중" 토스트는 실제로 내려받을 때만 띄운다. 즉 파일이 없어서 설치해야 하는 경우다.
- 토스트 문구는 무엇을 받는지 밝힌다. 예: "yt-dlp 내려받는 중…", "Deno 내려받는 중…". 이를 위해 상태에 현재 받고 있는 도구 이름을 담는다.
- 업데이트 확인이 실패해도 파일이 있으면 앱은 정상으로 동작한다. 오류는 설정의 업데이트 탭에만 표시한다. *(내 판단)*
- "지금 업데이트"(force)를 눌렀을 때의 동작은 지금과 같다.

## 5. 다운로드 흐름

**현재 동작** (`crates/kiri-core/src/pipeline.rs`)
1. `<저장 폴더>/<제목>.kiripart/` 작업 폴더에 원본(과 자막)을 받는다.
2. 변환이 필요하면 작업 폴더 안의 원본으로 `out/`에 인코딩한다.
3. **인코딩이 끝난 뒤에야** 원본과 자막을 저장 폴더로 옮기고, 변환본은 `<원본 stem>-<n>.<ext>`로 옮긴다.

**새 동작**
1. **받는 중:** 지금처럼 `<제목>.kiripart/` 작업 폴더(표식 `.kiri` 포함)에 받는다. 작업 폴더 방식은 유지한다. 자막과 yt-dlp 임시 파일을 함께 담아야 하므로 파일 하나로 바꾸지 않는다. *(내 판단: 사용자가 말한 "xx.kiripart"를 지금의 작업 폴더로 해석함)*
2. **다운로드 완료 즉시:** 원본과 자막을 저장 폴더로 옮긴다(`finalize`). 이름 충돌 규칙(" (n)")과 자막 이름 맞추기 규칙은 지금과 같다. 이 시점부터 원본을 재생할 수 있다.
3. **변환이 필요하면:** **저장 폴더로 옮긴 원본 파일**을 입력으로 인코딩한다. 출력은 작업 폴더의 `out/`에 만들고, 끝나면 `<원본 최종 stem>-<n>.<ext>`로 옮긴다.
4. **변환 실패, 취소, 작업 삭제:** 원본은 이미 저장 폴더에 있는 사용자 파일이므로 그대로 남긴다. 변환 중에 작업을 삭제해도 마찬가지다. 작업 폴더만 정리한다. *(삭제 시 원본 유지는 내 판단)*
5. **재시작과 복구:** 변환 도중 앱이 꺼졌거나 사용자가 다시 시작하면, 원본이 이미 저장된 작업은 다시 받지 않고 변환만 다시 한다.
   - 이를 위해 작업(`Job`)에 "저장된 원본 경로"를 기록한다(새 필드).
   - 그 파일이 사라졌으면 처음부터 다시 받는다.
6. **작업 목록 UI:** 지금과 같은 단계 표시(다운로드 중 → 변환 중 → 완료)를 유지하고, 재생 버튼은 추가하지 않는다. 변환 중 보조 문구에 "원본 저장됨"을 표시할지는 i18n 문구로 정한다. *(내 판단: 표시함)*
7. **로컬 파일 변환(`JobSource::File`):** 원본을 읽기만 하므로 동작에 변화가 없다.

## 6. 문서 규칙 리팩토링

사용자 결정: **문서 쪽으로 완전히 맞춘다. 일본어는 제거한다.** 문서 중 kiri에 해당하지 않는 내용은 문서를 고친다.

### 6.1 프론트엔드

**구조**
- `docs/agents/structure.md`의 트리를 따른다: `src/app/{main.tsx, App.tsx, globals.css, settings/SettingsApp.tsx, _components}`, `src/domain/<d>/{apis,queries,components,store}`, `src/shared/{api,components,hooks,i18n,providers,queries,store}`, `src/utils`.
- **도메인 후보(내 판단):** `queue`(작업), `tools`(yt-dlp/Deno 상태), `settings`, `update`(앱 업데이트), `cli`(CLI 설치).

**데이터와 IPC**
- `invoke`는 `shared/api`의 `call<T>()`만 호출한다. 도메인마다 `apis/<d>.ts`와 DTO(`*.dto.ts`)를 둔다.
- 서버 데이터(설정, 작업 목록, 도구 상태, 업데이트 상태)는 React Query로 관리한다. 창이 여러 개일 때는 이벤트를 받아 `setQueryData`로 동기화한다(`libs/react-query.md`).
- 화면에만 필요한 상태(토스트, 시트 열림 등)는 zustand slice와 `rootStore`로 관리한다(immer, devtools 포함).

**i18n**
- i18next 기반 JSON을 `src/shared/i18n/{en,ko}.ts`의 타입 있는 사전과 `useI18n()`으로 바꾼다.
- `ja`는 프론트엔드와 Rust(`Lang::Ja`, 트레이/메뉴 라벨) 모두에서 삭제한다.
- 저장된 설정의 `ui_language`가 `ja`이면 `system`으로 바꿔 읽는다.

**테마와 스타일**
- `ThemeProvider`와 `resolveTheme`(`src/utils/theme.ts`)로 바꾼다.
- daisyUI 테마 이름은 `light`/`dark`로 하고, 색은 `globals.css`에만 둔다.
- 클래스 조합에는 `classnames`(`cx`)를 쓴다.

**코드 규칙**
- `@/` 별칭(tsconfig, vite)을 쓰고, `../` 상대 경로는 ESLint로 금지한다.
- `import type`, 섹션 주석(`react-sections.md`), default export, `Readonly` props를 따른다.

**도구**
- ESLint와 Prettier(`@bob-park/*` 설정)를 도입하고 `typecheck`, `lint`, `prettier` 스크립트를 추가한다.
- 테스트는 vitest에서 `node:test`로 옮기고, 테스트 파일은 대상 파일 옆에 둔다(`testing.md`). 기존 `src/test/*.test.ts`의 검증 내용은 유지한다.

**설정 화면 부품**
- `src/shared/components/settings/{SettingSection, SettingRow, Segmented, Switch, Button}`을 만들어 쓴다.

### 6.2 Rust (`src-tauri`)

**구조**
- `lib.rs`는 조립만 한다. `app/`(창, 트레이, 메뉴, 업데이터, 종료, i18n 등 Tauri에 묶인 것)과 `domain/<d>/{mod, model, repository, commands}.rs`, `shared/{db, error, migrations}`로 나눈다.
- 도메인 코어와 `shared`는 `tauri`를 쓰지 않는다.

**명령과 이벤트**
- 명령은 얇게 두고 `AppResult<T>`를 반환한다.
- `AppError`는 `{ code, message }`로 직렬화하고, 코드는 SCREAMING_SNAKE를 쓴다. 프론트엔드는 `t.errors[code]`로 문구를 보여준다.
- 이벤트 이름은 `<domain>://<kebab>` 형식으로 하고 상수로 둔다. 예: `queue://changed`, `tools://changed`, `settings://changed`, `update://status`.

**로깅과 주석**
- 로그는 `log::` 매크로와 `tauri-plugin-log`를 쓴다(`eprintln!` 대체).
- I/O에서 `unwrap`을 쓰지 않는다.
- 모듈마다 `//!` 문서 줄을 둔다.
- 주석은 영어로 바꾼다(CLAUDE.md).

### 6.3 저장소: JSON → SQLite

- 설정과 작업 목록을 SQLite(`rusqlite`, bundled)로 옮긴다. 마이그레이션은 `shared/migrations/0001_init.sql`, 버전은 `user_version`으로 관리한다(`sqlite-migrations.md`). *(작업 목록 이전은 내 판단)*
- **기존 데이터 이전**
  - 첫 실행 때 `settings.json`과 `queue.json`이 있고 DB가 비어 있으면, 그 내용을 DB로 옮긴다.
  - 옮기고 나면 원본 JSON은 `*.json.migrated`로 이름만 바꿔 남긴다. 문제가 생겼을 때 되돌릴 수 있게 하기 위해서다. *(내 판단)*
  - `queue.json`의 v0.1 레거시 형식을 읽는 로직(`migrate_legacy`)은 이전 단계에서 그대로 쓴다.
- **작업 목록 위치**
  - 작업 목록(`QueueState`)은 지금 `kiri-core`가 JSON으로 저장한다.
  - **선택지 1:** 저장 경계를 `kiri-core`의 trait 같은 추상화 하나로 두고, 구현은 `src-tauri/domain/queue/repository.rs`에 둔다.
  - **선택지 2:** `kiri-core`에 rusqlite를 두는 것이 단순하면 그렇게 해도 된다.
  - 어느 쪽이든 `kiri-core`는 `tauri`를 쓰지 않는다. 둘 중 무엇을 고를지는 빌더에게 맡긴다.

### 6.4 Workspace (`crates/kiri-core`, `crates/kiri-cli`)

- 두 crate는 그대로 둔다. CLI가 `kiri-core`를 쓰고 앱과는 IPC로 통신하므로 `src-tauri`로 합칠 수 없다. *(내 판단)*
- 문서(`structure.md`, `architecture.md`, `rust-modules.md`)에 workspace 구조, CLI 사이드카, ffmpeg 사이드카, IPC 서버를 추가한다.

### 6.5 문서 쪽에서 고칠 것

- **AGENTS.md와 overview:** 템플릿 이름(`template-rust-app`, `com.bobpark.app` 등)을 실제 값(`kiri`, `org.bobpark.kiri`)으로 바꾸고, "템플릿 이름 바꾸기" 안내는 삭제한다.
- **플랫폼:** Windows/NSIS/`release.ps1` 관련 내용은 삭제한다. 대상은 macOS arm64 전용이다.
- **테스트 명령:** `cargo test --manifest-path src-tauri/Cargo.toml --lib`를 `cargo test --workspace`로 바꾼다. CLAUDE.md의 같은 줄도 바꾼다.
- **도구 버전:** 문서대로 올린다(사용자 결정). `.mise.toml`은 Node 26 / Yarn 4.18.1로, Vite는 8로 올린다. 그 뒤 `yarn.lock`을 다시 만든다.
- **디자인 문서:** `docs/agents/design.md`와 `docs/design/`을 새 "오로라 글래스" 스타일로 갱신한다. 지금 `design.md`는 "제3의 스타일을 만들지 말라"고 하는데, 이번 리디자인은 사용자가 승인한 새 기준이다.
- **i18n 문서:** 언어 목록은 en/ko만 둔다. 이미 문서와 같다.

## 7. 추가될 의존성 (사용자 승인 완료, 2026-10-21)

- **npm:** `@tanstack/react-query`, `@tanstack/react-query-devtools`, `immer`, `classnames`, `eslint`, `@bob-park/eslint-config-bobpark`, `prettier`, `@bob-park/prettier-config-bobpark`
- **npm 제거:** `i18next`, `react-i18next`, `vitest`, (사용하지 않게 되면) `pretendard`
- **Cargo:** `rusqlite`(bundled), `thiserror`(src-tauri), `log`, `tauri-plugin-log`, `tauri-plugin-single-instance`
  - **단일 인스턴스는 넣는다.** 두 번째 실행은 기존 창을 앞으로 가져온다. 이렇게 하면 CLI용 IPC 소켓을 두 프로세스가 동시에 잡는 일도 막을 수 있다. *(내 판단)*
  - **`tauri-plugin-autostart`는 넣지 않는다.** 새 기능이므로 비목표에 해당한다. 문서의 플러그인 목록에서도 뺀다. *(내 판단)*
- **애니메이션 라이브러리:** 추가하지 않는다. CSS 트랜지션/키프레임과 Web Animations API로 구현한다. *(내 판단)*

## 8. 리디자인: "오로라 글래스"

사용자가 시안 A를 골랐고, 드롭존을 뺀 v2 시안(`.superpowers/brainstorm/.../aurora-v2.html`)을 승인했다.

### 8.1 메인 창

**배경**
- 브랜드 보라, 하늘, 분홍 세 개의 흐릿한 빛 덩어리가 아주 천천히(약 12초 주기) 떠다닌다.
- 라이트 배경은 `#f6f5fb` 계열, 다크는 `#121318` 계열이다.

**유리 표면**
- 카드, 시트, 키캡, 아이콘 받침은 반투명 배경에 `backdrop-filter: blur + saturate`를 주고, 얇은 밝은 테두리를 두른다.

**헤더**
- 로고 `kiri.`(점은 브랜드색)와 설정 버튼만 둔다. 설정 버튼은 hover 시 톱니가 돈다.
- "완료 항목 지우기"는 유지한다. 위치는 빌더에게 맡긴다.

**드롭존 제거**
- 상단 드롭존 UI와 창에 끌어다 놓는 기능(drag & drop)을 없앤다. 입력은 ⌘V만 받는다. *(드래그 기능 제거는 내 판단)*

**빈 화면**
- 가운데에 유리 받침 위 다운로드 아이콘이 둥실 떠 있다.
- 그 아래 `⌘` `V` 키캡과 안내 문구("YouTube 링크를 복사한 뒤 붙여넣으세요")가 있다.
- ⌘V를 누르면 키캡이 눌리는 애니메이션이 나온다.

**옵션 시트**
- 배경이 흐려지는 scrim 위로, 가장자리에서 10px 떨어진 떠 있는 유리 시트가 아래에서 스프링으로 올라온다.
- 영상 정보를 불러오는 동안에는 반짝이는 skeleton을 보여준다.
- 화질과 형식 칩은 선택하면 살짝 커지며 브랜드색으로 바뀐다.
- 다운로드 버튼은 그라데이션이고, 누르면 눌림 효과가 있다.
- 시트의 항목과 기능(자막, 기억하기, 더보기 등)은 지금과 같다.

**작업 카드**
- 유리 카드 하나에 썸네일, 제목, 상태 문구, 퍼센트를 보여준다. 진행 바는 흐르는 그라데이션이다.
- 새 카드는 흐릿한 상태에서 선명해지며 위에서 스프링으로 내려앉는다.
- 변환 단계에서는 진행 바 그라데이션이 다른 톤으로 바뀐다.
- 완료되면 진행 바가 초록으로 바뀌고, ✓ 배지가 퍼지는 링과 함께 튀어나온다.
- 실패, 정지, 대기 상태도 같은 언어로 디자인한다(빌더 재량).
- 버튼(정지, 재시작, 폴더에서 보기, 삭제)과 동작은 지금과 같다. 재생 버튼은 추가하지 않는다.
- 카드를 삭제할 때는 접히며 사라지는 퇴장 애니메이션을 둔다(빌더 재량).

**토스트와 상태 바**
- 같은 유리 스타일로 바꾸고, 등장과 퇴장 애니메이션을 유지한다.

### 8.2 설정 창

- 같은 배경과 유리 표면 언어를 쓴다.
- 탭 구성(일반 / 다운로드 / CLI / 업데이트)과 항목은 그대로 두고, 새 설정 부품(`SettingSection`, `SettingRow`, `Segmented`, `Switch`, `Button`)으로 다시 만든다.
- 탭을 바꿀 때 선택 표시가 미끄러지듯 옮겨간다. 세부는 빌더 재량.
- 설정 창은 시안을 만들지 않았다. 메인 창의 언어를 그대로 적용한다. *(내 판단)*

### 8.3 모션 원칙

- 등장은 스프링 계열(`cubic-bezier(.2,.9,.3,1.2)` 정도, 450~650ms)로 하고, 퇴장은 더 짧게(150~250ms) 한다.
- `transform`과 `opacity`만 애니메이션한다. 진행 바 너비 변화는 예외로 허용한다.
- `prefers-reduced-motion: reduce`이면 배경 흐름, 둥실 효과, 스프링을 모두 끄고 최종 상태만 보여준다.
- 블러가 많으므로 빛 덩어리는 3개 이하로 두고, 성능이 떨어지면 줄인다.

### 8.4 디자인 토큰

- 색과 그라데이션은 `globals.css`의 `light`/`dark` 블록에만 둔다.
- 브랜드색은 kraken 기반 그대로다: 라이트 `#7132f5`, 다크 `#8b5cff`, 성공색 `#149e61` / `#4cd393`.
- 새 토큰(`glass`, `glass-edge`, `aurora-1..3`, `progress-gradient`, `progress-gradient-encoding` 등)은 두 테마 모두에 정의한다.
- 글꼴은 빌더가 정한다. 지금은 Pretendard 14px이고 `design.md`는 Helvetica Neue 13px이다. 한국어 표시 품질을 위해 Pretendard 유지를 권하고, 문서를 그에 맞게 고친다. *(내 판단)*

## 9. 빌더에게 맡기는 것

- 프론트엔드 도메인을 정확히 어떻게 나눌지, 파일 이름.
- 작업 목록 저장 경계를 어디에 둘지(§6.3).
- 각 상태(실패, 정지)의 세부 시각 표현, 설정 창의 세부 레이아웃.
- 작업 순서. 권장 순서는 리팩토링 → 시작 준비 → 다운로드 흐름 → 리디자인이고, 각 단계마다 네 가지 검사를 통과해야 한다.

## 10. 열린 질문

없음. 빌더 검토 질문은 사용자 답변(의존성 추가, 버전 올림, master 기준 브랜치)과 위의 '내 판단' 항목으로 정리했다.
