# kiri: UI 리디자인 · 토스트 준비 알림 · 백그라운드 업데이트 설계

- 날짜: 2026-10-09
- 브랜치: `feature/ui-redesign`
- 디자인 기준: `docs/design/kraken-design.md`
- 목업: `2026-10-09-kiri-ui-redesign-mockups/` (확정안: 메인 A, 옵션 시트 1, 설정 1, 상태 막대 ①②③)

## 0. 배경과 범위

요구사항은 세 가지입니다.

1. **전체 UI 리디자인:** 사용자 친화적이고 모던한 UI.
2. **'다운로드 준비 중' 개선:** 메인 창 상단 배너 대신 토스트로 표시.
3. **업데이트:** 새 버전이 있으면 먼저 백그라운드로 받고, 메인 창 하단 상태 막대에 새 버전이 있다고 표시.

### 범위 밖

- 받은 업데이트를 디스크에 보관해 다음 실행에서 재사용. 메모리에만 두고, 설치 전에 앱을 끄면 다음 실행에서 다시 받습니다.
- 메인 창의 사이드바·상태 필터 (목업 B안, 채택 안 함).
- 앱 UI에서 로컬 파일 변환 시작.
- 새 npm·cargo 의존성. Tailwind 4 + daisyUI 5 + Pretendard를 그대로 씁니다.

## 1. 디자인 토큰 (`src/index.css`)

기존 daisyUI 테마 두 개(`kiri-light` 기본, `kiri` 다크)를 유지하고 값만 다듬습니다.

- 브랜드: `--color-primary` 라이트 `#7132f5`, 다크 `#8b5cff`. 보조(subtle) 배경은 보라 16%(다크 22%).
- 표면: 창 배경 `base-100`, 설정 창·상태 막대 배경 `base-200`, 구분선 `base-300`.
- 모서리: 버튼·입력 12px(`--radius-field`), 카드·그룹 14px(`--radius-box`), 칩 6px.
- 그림자: 카드 `rgba(16,24,40,0.04) 0 1px 4px`, 떠 있는 요소(모달·토스트·팝오버) `rgba(16,17,20,0.16~0.25) 0 12px 32px`.
- 상태 칩 색은 기존 semantic 색을 씁니다: 완료 `success`, 진행 `primary`(subtle 배경), 대기·중지 중립 회색, 실패 `error`.
- 타이포: Pretendard 유지. 로고 `kiri.`는 800 굵기, 음수 자간, 마침표만 primary.

공통 클래스(예: `.chip`, `.card-surface`)는 `index.css`의 `@layer components`에 둡니다. 두 번 이상 쓰는 것만 만듭니다.

## 2. 메인 창 (목업 A)

720×520 기준. 위에서 아래로:

1. **헤더**: 왼쪽 `kiri.` 로고, 오른쪽 아이콘 버튼 두 개(완료 항목 정리: 완료 작업이 있을 때만 / 설정). 기존 가운데 안내 문구(`app.dropHint`)는 드롭존으로 옮깁니다.
2. **드롭존**: 점선 보라 테두리 카드. 아이콘 + "YouTube 링크를 붙여넣거나 끌어다 놓으세요" + `⌘ V` 키캡. 클릭하면 클립보드를 읽어 ⌘V와 같은 `submit`을 탑니다. `<button>`이라 키보드로도 누를 수 있습니다. 드래그가 창 위에 있을 때 테두리를 진하게 강조합니다(`dragenter`/`dragleave`).
3. **작업 리스트**: 각 작업은 둥근 카드(`JobRow`). 썸네일 72×42, 제목, 세부 줄(`jobDetail`), 실행 중이면 진행률 바. 오른쪽에 상태 칩(실행 중이면 퍼센트), 그 옆 아이콘 버튼은 기존 조건·동작 그대로(중지, 다시 시작, 설정, Finder에서 보기, 삭제). 아이콘 버튼은 카드 hover·focus-within 때만 진하게 보입니다(항상 접근 가능).
4. **빈 상태**: 작업이 없으면 리스트 자리에 아이콘 + `app.empty` + 짧은 보조 문구.
5. **상태 막대** (`StatusBar`, 높이 32px): 왼쪽은 큐 요약("진행 중 n · 대기 n · 완료 n · 실패 n", 0인 항목은 생략, 작업이 없으면 비움), 오른쪽은 `UpdateStatus`(6절).

`ToolsNotice`와 `UpdateBanner`는 메인 창에서 사라집니다.

## 3. 옵션 시트 (목업 1, 중앙 모달)

`OptionsSheet`의 동작(모달 `<dialog>`, 포커스 가두기·복원, IME 조합 중 Esc 무시, 바깥 클릭 닫기, 시트 안 오류 표시, 저장 실패해도 닫기)은 그대로 두고 배치만 바꿉니다.

- 위치: 화면 중앙, 폭 `min(380px, 92vw)`, 18px 모서리, `max-h-[88vh]` 내부 스크롤.
- **히어로**: 썸네일을 폭 전체 높이 ~110px로 깔고 아래쪽 어두운 그라데이션 위에 제목(굵게)과 채널. 오른쪽 위에 길이 배지. 썸네일이 없으면 보라 그라데이션.
- **화질**: 3열 타일 그리드. 타일 = 라벨(1080p 등) + 코덱·용량. 마지막 타일은 "오디오만". 라디오 의미 유지(`role=radio` 대신 기존처럼 sr-only `<input type=radio>`).
- **출력 포맷**: 칩. 기본으로 보이는 칩은 `original`, 현재 선택값, `mp4-h264`, `mp3` 중 중복을 뺀 최대 3개 + "더보기". 더보기를 누르면 나머지 칩이 펼쳐집니다.
- **자막**: 칩(체크 의미, sr-only checkbox). 자동 생성 자막은 지금처럼 접힌 "자동 생성 (n)" 아래.
- **아래쪽**: 왼쪽 "다음부터 이 설정으로 바로 시작" 체크박스, 오른쪽 취소(회색)·다운로드(primary).
- 로딩(probe 중): 히어로 자리에 스켈레톤 + "영상 정보를 읽는 중…".

## 4. 설정 창 (목업 1, 상단 아이콘 탭)

560×460. 창 배경 `base-200`.

- 탭 바: 기존 4개 아이콘 탭 유지, 선택 탭은 보라 subtle 배경.
- 본문: 섹션 제목(작은 굵은 회색) + 흰 카드 그룹. 새 컴포넌트 `Group({ title, children })`가 카드 테두리·모서리를 맡고, 기존 `Row`는 그룹 안 한 줄로 그대로 씁니다(구분선은 그룹 안에서만).
- 그룹 구성:
  - 일반: [화면] 언어·테마, [창] 메뉴 막대로 숨기기
  - 다운로드: [저장] 저장 위치, [기본 옵션] 화질·포맷·자막·바로 시작, [성능] 동시 다운로드·하드웨어 가속
  - CLI: 기존 항목을 한 그룹으로
  - 업데이트: [kiri] 버전·지금 확인·자동 확인, [yt-dlp] 버전·업데이트
- 동시 다운로드: `<select>` → 1~4 세그먼트(`join` 버튼, `aria-pressed`). 테마 선택과 같은 패턴.
- 업데이트 탭의 kiri 행 설명은 6절 상태를 따릅니다("받는 중… n%", "v0.3.0 설치 준비 완료" + 설치 버튼, "받기 실패" + 다시 시도).

## 5. 토스트 (`src/lib/toast.ts`, `src/components/Toasts.tsx`)

### 5.1 스토어 확장

```ts
interface Toast {
  id: number;
  key?: string;            // 같은 key는 새로 쌓지 않고 덮어쓴다
  kind: "error" | "info" | "progress";
  text: string;
  desc?: string;
  progress?: number | null; // progress 종류: 0~1, null이면 indeterminate
  action?: { label: string; run: () => void };
  sticky?: boolean;         // true면 자동으로 닫지 않는다
}
push(t: Omit<Toast, "id">): number
dismissKey(key: string): void
```

- `showError`·`showInfo`는 지금처럼 5초 뒤 닫히는 토스트를 띄웁니다(시그니처 유지).
- key가 같은 토스트가 있으면 그 자리에서 내용만 바꿉니다. 덮어쓸 때 타이머는 새 값의 `sticky` 기준으로 다시 정합니다.

### 5.2 표시

- 위치: 오른쪽 아래, 메인 창에서는 상태 막대 위(bottom 44px), 설정 창에서는 bottom 16px.
- 모양: 어두운 카드(`#101114`, 다크 테마에서는 `base-300`), 흰 글씨, 12px 모서리, 폭 260px. 오류는 왼쪽에 빨강 점, progress는 스피너 + 얇은 진행 바.
- 액션 버튼은 토스트 안 오른쪽. 액션이 없는 토스트는 클릭하면 닫힙니다(기존 동작). 액션이 있으면 닫기 × 버튼을 따로 둡니다.
- 진입·퇴장 애니메이션: 아래에서 8px 떠오르며 페이드(150ms). `prefers-reduced-motion`이면 생략.
- popover로 띄워 모달 위에 보이게 하는 기존 방식 유지. 오류·정보는 `role="alert"`, progress는 `role="status"`.

### 5.3 '다운로드 준비 중'

`MainWindow`의 `ToolsNotice`를 화면 요소 대신 효과(effect)로 바꿉니다. `useTools` 상태를 보고:

| 도구 상태 | 토스트(key `"tools"`) |
|---|---|
| `ready` | 닫기 |
| 설치 중, 또는 아직 준비 안 됨·오류 없음 | progress, sticky, indeterminate. 제목 `app.toolsPreparing`, 설명 `app.toolsPreparingDesc` |
| 오류 있고 설치 중 아님 | error, sticky. `app.toolsFailed`, 액션 `app.retry` → `api.updateTools()` |

메인 창에서만 띄웁니다(설정 창의 업데이트 탭에는 기존 인라인 표시가 있음).

## 6. 백그라운드 업데이트

### 6.1 Rust (`src-tauri/src/updater.rs`)

상태:

```rust
struct Pending { update: Update, bytes: Option<Vec<u8>> }
pub struct UpdateState(Mutex<Option<Pending>>);
static DOWNLOADING: AtomicBool;
```

`UpdateInfo`에 `ready: bool`(받기 완료 여부)을 더합니다.

흐름:

1. `check()`가 새 버전을 찾으면 보관합니다. 이미 같은 버전을 받아 두었으면 `bytes`를 유지합니다(다시 받지 않음). 그 뒤 `update-available`(ready 반영)을 보내고 `start_download()`를 부릅니다.
2. `start_download()`: `bytes`가 이미 있거나 `DOWNLOADING`이 켜져 있으면 아무것도 하지 않습니다. 아니면 백그라운드 task에서 `update.download()`로 받으며 `update-progress`를 보냅니다.
   - 성공: 보관 중인 버전이 받은 버전과 같을 때만 `bytes`를 채우고 `update-ready`(UpdateInfo, ready=true)를 보냅니다.
   - 실패: `update-download-failed`(메시지)를 보냅니다. 로그만 남기고 오류 토스트는 띄우지 않습니다.
3. `install()`: 보관된 `bytes`가 있으면 그대로 `update.install(bytes)`. 없으면 지금처럼 받고 설치합니다. 백그라운드 받기와 드물게 겹치면 한 번 더 받습니다. 받는 시간만 늘고 결과는 같으므로 허용합니다. 이 판단을 순수 함수 `fn needs_download(ready: bool) -> bool` 같은 형태로 분리해 테스트합니다.
4. 새 명령 `retry_update_download`: 실패 뒤 다시 시도 버튼이 부릅니다. 내용은 `start_download()`.
5. 자동 확인(`spawn_periodic`)은 `auto_check`가 켜져 있을 때만 돌므로, 백그라운드 받기도 그때와 수동 "지금 확인" 때만 일어납니다.
6. 트레이 항목, "큐가 끝나면 재시작" 예약(`request_install`, `on_queue_change`)은 그대로입니다.

`update_status` 명령은 `UpdateInfo`(ready 포함)를 돌려주므로, 늦게 열린 창도 현재 단계를 복구합니다. 받는 중인지 여부는 `DOWNLOADING`을 같이 실어 `downloading: bool`로 돌려줍니다.

### 6.2 프론트 (`src/lib/update.ts`)

스토어에 `phase`를 둡니다.

```ts
type UpdatePhase = "idle" | "downloading" | "ready" | "failed" | "scheduled" | "installing";
```

| 이벤트·동작 | phase |
|---|---|
| `update-available` (ready=false) | `downloading` |
| `update-progress` | `downloading`, 또는 설치를 누른 뒤면 `installing` |
| `update-ready` | `ready` |
| `update-download-failed` | `failed` |
| `install(true)` 뒤 `update-scheduled` | `scheduled` |
| `install(false)` | `installing` |
| `update-error` | 직전이 ready였으면 `ready`, 아니면 `failed`. 오류 토스트도 띄웁니다(사용자가 누른 동작의 실패) |

상태 전이를 순수 함수 `nextPhase(phase, event)`로 분리해 vitest로 테스트합니다.

### 6.3 `UpdateStatus` (상태 막대 오른쪽)

| phase | 표시 |
|---|---|
| `idle` | 없음 |
| `downloading` | 회색 작은 링 + "v{ver} 받는 중… {pct}%" (total 모르면 퍼센트 생략) |
| `ready` | 보라 알약 "새 버전 v{ver}" + 버튼. 큐가 비어 있으면 "재시작하여 설치" → `install(false)`. 큐가 있으면 "설치…" → 팝오버(아래) |
| `failed` | "업데이트 받기 실패" + "다시 시도" → `retry_update_download` |
| `scheduled` | "큐가 끝나면 재시작됩니다" |
| `installing` | 링 + "설치 중…" |

팝오버: 제목 "kiri v{ver} 설치 준비 완료", 설명 "다운로드 n개가 진행 중이에요. 재시작하면 대기열로 돌아가 다시 이어집니다.", 버튼 "지금 재시작"(회색) / "큐가 끝나면"(primary). 네이티브 `popover` 속성 + 바깥 클릭·Esc로 닫힘.

## 7. 문구 (ko · en · ja)

추가하거나 바꾸는 키 (ko 기준, en·ja도 함께 추가):

- `app.dropTitle`: "YouTube 링크를 붙여넣거나 끌어다 놓으세요" / `app.dropShortcut`: "{{key}} 로 바로 추가"
- `app.emptyDesc`: "링크를 붙여넣으면 여기에 나타나요"
- `status.running` "진행 중 {{n}}", `status.queued` "대기 {{n}}", `status.completed` "완료 {{n}}", `status.failed` "실패 {{n}}"
- `update.downloadingBg` "v{{version}} 받는 중… {{pct}}%", `update.ready` "새 버전 v{{version}}", `update.restartInstall` "재시작하여 설치", `update.installMenu` "설치…", `update.readyTitle` "kiri v{{version}} 설치 준비 완료", `update.busyDesc` "다운로드 {{n}}개가 진행 중이에요. 재시작하면 대기열로 돌아가 다시 이어집니다.", `update.downloadFailed` "업데이트 받기 실패", `update.retry` "다시 시도", `update.installing` "설치 중…"
- `sheet.more` "더보기"
- `settings.group.*`: 4절의 그룹 제목들

`app.dropHint`, `update.available`, `update.downloading`은 쓰는 곳이 없어지면 지웁니다. `locales.test.ts`가 세 언어의 키 일치를 검사합니다.

## 8. 테스트

- vitest
  - `toast.test.ts`: key 덮어쓰기, sticky는 자동으로 닫히지 않음, `dismissKey`.
  - `update.test.ts`(신규): `nextPhase` 전이표.
  - `queue.test.ts`: 상태 막대 요약 함수(`queueSummary(jobs)`)가 0인 항목을 뺌.
  - `sheet.test.ts`: 기본으로 보이는 포맷 칩 계산(중복 제거, 최대 3개, 선택값 포함).
  - `locales.test.ts`: 기존 테스트로 새 키 일치 확인.
- cargo test: `needs_download`, `UpdateInfo` 직렬화에 `ready` 반영.
- 수동 확인: `yarn tauri dev`로 라이트·다크 두 테마에서 메인·시트·설정 창, 첫 실행 토스트(도구 폴더 삭제 후 실행), 업데이트 상태 막대(로컬 `latest.json`으로 높은 버전 제공).

## 9. 바뀌는 파일

- 수정: `src/index.css`, `src/pages/MainWindow.tsx`, `src/components/JobRow.tsx`, `src/components/OptionsSheet.tsx`, `src/components/Toasts.tsx`, `src/lib/toast.ts`, `src/lib/update.ts`, `src/lib/queue.ts`, `src/lib/sheet.ts`, `src/lib/tauri.ts`, `src/lib/types.ts`, `src/pages/SettingsWindow.tsx`, `src/pages/settings/*.tsx`, `src/locales/*.json`, `src-tauri/src/updater.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`(명령 등록)
- 신규: `src/components/StatusBar.tsx`(큐 요약 + UpdateStatus), `src/pages/settings/Group.tsx`, `src/test/update.test.ts`
- 삭제: `src/components/UpdateBanner.tsx`
