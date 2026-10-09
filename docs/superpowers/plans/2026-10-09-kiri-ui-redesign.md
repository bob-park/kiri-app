# kiri UI 리디자인 · 토스트 준비 알림 · 백그라운드 업데이트 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** kiri의 메인 창·옵션 시트·설정 창을 Kraken 기반의 모던한 UI로 바꾸고, '다운로드 준비 중'을 토스트로, 앱 업데이트를 "백그라운드로 받고 하단 상태 막대에 표시"로 바꾼다.

**Architecture:** 기존 Tailwind 4 + daisyUI 5 테마 토큰을 다듬고 컴포넌트 마크업만 다시 짠다. 토스트 스토어에 key·sticky·progress·action을 더해 지속 토스트를 지원한다. Rust `updater.rs`가 새 버전을 찾으면 바로 받아 메모리에 보관하고, 프론트는 `nextPhase` 상태 기계로 상태 막대를 그린다.

**Tech Stack:** React 19, zustand 5, react-i18next, Tailwind 4, daisyUI 5, vitest 4(node 환경), Tauri 2, tauri-plugin-updater 2.

**Spec:** `docs/superpowers/specs/2026-10-09-kiri-ui-redesign-design.md` (목업: `docs/superpowers/specs/2026-10-09-kiri-ui-redesign-mockups/`)

## Global Constraints

- 새 npm·cargo 의존성 금지. Tailwind 4 + daisyUI 5 + Pretendard 그대로.
- 브랜드 primary: 라이트 `#7132f5`, 다크 `#8b5cff`. 버튼·입력 모서리 12px, 카드·그룹 14px, 칩 6px.
- 테마 두 개(`kiri-light` 기본, `kiri` 다크) 모두에서 동작해야 한다.
- 모든 사용자 문구는 `src/locales/{ko,en,ja}.json` 세 파일에 같은 키로 추가한다(`locales.test.ts`가 검사). 빈 문자열 금지.
- 받은 업데이트는 메모리에만 보관한다(디스크 저장 안 함).
- 백그라운드 받기 실패는 상태 막대에만 표시하고 오류 토스트를 띄우지 않는다. 사용자가 누른 설치의 실패는 오류 토스트를 띄운다.
- 기존 접근성 동작 유지: 옵션 시트 모달의 포커스 가두기·복원, IME 조합 중 Esc 무시, aria-label, `aria-pressed`.
- macOS 표준 타이틀바는 그대로 둔다(목업의 신호등 버튼은 표준 타이틀바를 뜻함). 앱 헤더는 그 아래 콘텐츠 영역에 있다.
- 커밋 메시지는 기존 관례(한국어, `feat(ui):` / `feat(core):` / `refactor:` 등 접두사)를 따르고 끝에 다음 두 줄을 붙인다:
  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01EFAf1zkoP9v7SZ5KB1tKkD
  ```

## Review Focus

1. **설치 전에 같은 버전을 다시 확인**(24시간 주기 또는 "지금 확인"): 이미 받은 파일을 버리고 다시 받으면 안 된다. Task 4의 `check()`는 같은 버전일 때 `bytes`를 유지하며, 수동 확인 단계에서 검사한다.
2. **'다운로드 준비 중' 토스트가 옵션 시트 모달 위에서도 보인다**: 기존 popover 재표시 로직을 Task 2에서 그대로 유지한다. 수동 확인 단계에서 검사한다.
3. **같은 key로 토스트를 연달아 띄우기**(도구 상태가 설치 중 → 실패 → 재시도로 바뀜): 토스트가 쌓이지 않고 하나가 바뀌어야 한다. 앞선 비-sticky 타이머가 새 sticky 토스트를 닫으면 안 된다. Task 2 테스트로 검사한다.
4. **다운로드가 진행 중일 때 "설치…"**: 바로 재시작하지 않고 팝오버에서 고르게 한다. Task 6 수동 확인과 `pending` 계산 테스트로 검사한다.
5. **설치 실패 뒤 상태 복구**: 받아 둔 파일이 있으면 `ready`로 돌아가 다시 누를 수 있어야 한다. Task 5 `nextPhase` 테스트로 검사한다.

---

## File Structure

| 파일 | 책임 |
|---|---|
| `src/index.css` | 테마 토큰, 공통 클래스(`surface-card`, `chip-*`, `float-shadow`), 토스트 애니메이션 |
| `src/lib/toast.ts` | 토스트 스토어(key·sticky·progress·action), `showError`/`showInfo` |
| `src/components/Toasts.tsx` | 토스트 렌더링(popover, 위치 prop) |
| `src/lib/tools.ts` | `toolsToast(status)` 순수 함수 추가 |
| `src-tauri/src/updater.rs` | 백그라운드 받기, 받은 파일 보관, 설치 |
| `src-tauri/src/commands.rs`, `lib.rs` | `retry_update_download` 명령 |
| `src/lib/update.ts` | `UpdatePhase`, `nextPhase`, `updatePct`, 스토어 |
| `src/lib/queue.ts` | `queueSummary(jobs)` 추가 |
| `src/components/StatusBar.tsx` (신규) | 상태 막대: 큐 요약 + `UpdateStatus` + 설치 팝오버 |
| `src/components/DropZone.tsx` (신규) | 붙여넣기/드롭 영역 |
| `src/components/JobRow.tsx` | 작업 카드 |
| `src/pages/MainWindow.tsx` | 레이아웃, 드래그 상태, 도구 토스트 effect |
| `src/lib/sheet.ts` | `visiblePresets(defaultPreset)` 추가 |
| `src/components/OptionsSheet.tsx` | 중앙 모달 시트 |
| `src/pages/settings/Group.tsx` (신규) | 설정 카드 그룹 |
| `src/pages/SettingsWindow.tsx`, `src/pages/settings/*Tab.tsx` | 설정 창 |
| `src/components/UpdateBanner.tsx` | 삭제 |

---

### Task 1: 디자인 토큰과 공통 클래스

**Files:**
- Modify: `src/index.css`

**Interfaces:**
- Produces: CSS 클래스 `surface-card`, `chip`, `chip-run`, `chip-ok`, `chip-err`, `chip-idle`, `float-shadow`, `animate-toast-in`. 색 토큰 `bg-toast`/`text-toast-content`(Tailwind 유틸리티로 생성됨). 이후 모든 UI 태스크가 이 이름을 쓴다.

- [ ] **Step 1: 테마 반경 토큰 변경**

`src/index.css`의 두 `@plugin "daisyui/theme"` 블록에서 `--radius-box: 12px;`를 `--radius-box: 14px;`로 바꾼다(필드 12px, 셀렉터 6px은 그대로).

- [ ] **Step 2: 토스트 색 토큰 추가**

`@theme` 블록과 그 아래 라이트 오버라이드를 다음으로 바꾼다:

```css
@theme {
  --color-fg-muted: #8b8ea3;
  --color-toast: #2c2e38;
  --color-toast-content: #ffffff;
  --font-sans: "Pretendard Variable", Pretendard, -apple-system, "Helvetica Neue", Arial, "Hiragino Sans", sans-serif;
}
[data-theme="kiri-light"] { --color-fg-muted: #9497a9; --color-toast: #101114; }
```

- [ ] **Step 3: 공통 클래스와 애니메이션 추가**

파일 끝(`html, body, #root` 규칙 뒤)에 추가:

```css
@layer components {
  .surface-card { @apply rounded-box border border-base-300/70 bg-base-100 shadow-[0_1px_4px_rgba(16,24,40,0.04)]; }
  .float-shadow { box-shadow: 0 12px 32px rgba(16, 17, 20, 0.18); }
  .chip { @apply inline-flex items-center whitespace-nowrap rounded-[6px] px-[7px] py-[2px] text-[10px] font-semibold; }
  .chip-run { @apply bg-secondary text-secondary-content; }
  .chip-ok { @apply bg-success/15 text-success; }
  .chip-err { @apply bg-error/15 text-error; }
  .chip-idle { @apply bg-base-content/10 text-base-content/70; }
}

@keyframes toast-in { from { opacity: 0; transform: translateY(8px); } }
.animate-toast-in { animation: toast-in 150ms ease-out; }
@media (prefers-reduced-motion: reduce) { .animate-toast-in { animation: none; } }
```

- [ ] **Step 4: 빌드 확인**

Run: `yarn build`
Expected: 성공(tsc 오류 없음, vite 번들 생성). `@apply` 관련 오류가 나면 클래스 이름 오타를 고친다.

- [ ] **Step 5: Commit**

```bash
git add src/index.css
git commit -m "feat(ui): 디자인 토큰 다듬기 — 카드 14px, 토스트 색, 공통 칩·카드 클래스"
```

---

### Task 2: 토스트 스토어 확장과 새 토스트 모양

**Files:**
- Modify: `src/lib/toast.ts`, `src/components/Toasts.tsx`, `src/main.tsx`, `src/locales/{ko,en,ja}.json`
- Test: `src/test/toast.test.ts`

**Interfaces:**
- Produces:
  ```ts
  export interface Toast {
    id: number; key?: string; kind: "error" | "info" | "progress";
    text: string; desc?: string; progress?: number | null;
    action?: { label: string; run: () => void }; sticky?: boolean;
  }
  export type ToastInput = Omit<Toast, "id">;
  export const TOAST_MS = 5000;
  useToasts.getState().push(t: ToastInput): number
  useToasts.getState().dismiss(id: number): void
  useToasts.getState().dismissKey(key: string): void
  export function Toasts({ bottom }: { bottom?: number }): JSX.Element
  ```
  `showError(e)`, `showInfo(text)` 시그니처는 그대로.

- [ ] **Step 1: 실패하는 테스트 작성**

`src/test/toast.test.ts` 끝에 추가(맨 위 import를 `import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";`, `import { errorText, useToasts, TOAST_MS } from "../lib/toast";`로 바꾼다):

```ts
describe("toast store", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useToasts.setState({ toasts: [] });
  });
  afterEach(() => vi.useRealTimers());

  const push = useToasts.getState().push;

  it("auto-dismisses plain toasts", () => {
    push({ kind: "info", text: "hi" });
    expect(useToasts.getState().toasts).toHaveLength(1);
    vi.advanceTimersByTime(TOAST_MS);
    expect(useToasts.getState().toasts).toHaveLength(0);
  });

  it("keeps sticky toasts", () => {
    push({ kind: "progress", text: "prep", sticky: true });
    vi.advanceTimersByTime(TOAST_MS * 3);
    expect(useToasts.getState().toasts).toHaveLength(1);
  });

  it("replaces a toast with the same key in place", () => {
    const a = push({ key: "tools", kind: "progress", text: "prep", sticky: true });
    push({ kind: "info", text: "other" });
    const b = push({ key: "tools", kind: "error", text: "failed", sticky: true });
    const { toasts } = useToasts.getState();
    expect(b).toBe(a);
    expect(toasts.map((x) => x.text)).toEqual(["failed", "other"]);
  });

  it("an earlier timer does not close a toast that became sticky", () => {
    push({ key: "k", kind: "info", text: "short" });
    push({ key: "k", kind: "error", text: "long", sticky: true });
    vi.advanceTimersByTime(TOAST_MS * 2);
    expect(useToasts.getState().toasts.map((x) => x.text)).toEqual(["long"]);
  });

  it("dismissKey removes by key and ignores unknown keys", () => {
    push({ key: "tools", kind: "progress", text: "prep", sticky: true });
    useToasts.getState().dismissKey("nope");
    expect(useToasts.getState().toasts).toHaveLength(1);
    useToasts.getState().dismissKey("tools");
    expect(useToasts.getState().toasts).toHaveLength(0);
  });
});
```

- [ ] **Step 2: 테스트가 실패하는지 확인**

Run: `yarn vitest run src/test/toast.test.ts`
Expected: FAIL — `TOAST_MS`가 없고 `push`가 객체 인자를 받지 않음.

- [ ] **Step 3: 스토어 구현**

`src/lib/toast.ts`의 `Toast` 인터페이스부터 `useToasts` 정의까지를 다음으로 바꾸고, 파일 끝의 두 export를 바꾼다(`errorText`는 그대로):

```ts
export interface Toast {
  id: number;
  /** 같은 key는 새로 쌓지 않고 그 자리에서 바꾼다. */
  key?: string;
  kind: "error" | "info" | "progress";
  text: string;
  desc?: string;
  /** progress 종류: 0~1, null이면 끝을 모르는 진행 */
  progress?: number | null;
  action?: { label: string; run: () => void };
  /** true면 자동으로 닫지 않는다. */
  sticky?: boolean;
}
export type ToastInput = Omit<Toast, "id">;

interface ToastStore {
  toasts: Toast[];
  push: (t: ToastInput) => number;
  dismiss: (id: number) => void;
  dismissKey: (key: string) => void;
}

export const TOAST_MS = 5000;
let seq = 0;
const timers = new Map<number, ReturnType<typeof setTimeout>>();
const clearTimer = (id: number) => {
  clearTimeout(timers.get(id));
  timers.delete(id);
};

export const useToasts = create<ToastStore>((set, get) => ({
  toasts: [],
  push: (input) => {
    const prev = input.key ? get().toasts.find((x) => x.key === input.key) : undefined;
    const id = prev?.id ?? ++seq;
    const toast = { ...input, id };
    clearTimer(id);
    set({ toasts: prev ? get().toasts.map((x) => (x.id === id ? toast : x)) : [...get().toasts, toast] });
    if (!input.sticky) timers.set(id, setTimeout(() => get().dismiss(id), TOAST_MS));
    return id;
  },
  dismiss: (id) => {
    clearTimer(id);
    set({ toasts: get().toasts.filter((x) => x.id !== id) });
  },
  dismissKey: (key) => {
    const x = get().toasts.find((t) => t.key === key);
    if (x) get().dismiss(x.id);
  },
}));
```

```ts
export const showError = (e: unknown) => useToasts.getState().push({ kind: "error", text: errorText(e) });
export const showInfo = (text: string) => useToasts.getState().push({ kind: "info", text });
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `yarn vitest run src/test/toast.test.ts`
Expected: PASS (기존 `errorText` 4개 + 새 5개).

- [ ] **Step 5: 닫기 문구 추가**

세 locale 파일 최상위에 `"toast"` 객체를 추가한다(`"app"` 바로 뒤):
- ko: `"toast": { "close": "닫기" },`
- en: `"toast": { "close": "Close" },`
- ja: `"toast": { "close": "閉じる" },`

- [ ] **Step 6: Toasts 컴포넌트 교체**

`src/components/Toasts.tsx` 전체:

```tsx
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useToasts, type Toast } from "../lib/toast";

function ToastCard({ x, dismiss }: { x: Toast; dismiss: (id: number) => void }) {
  const { t } = useTranslation();
  // 액션이 없는 오류·정보 토스트만 눌러서 닫는다. 진행 토스트는 상태가 바뀔 때까지 남는다.
  const clickToClose = !x.action && x.kind !== "progress";
  return (
    <div
      role={x.kind === "progress" ? "status" : "alert"}
      className={`animate-toast-in float-shadow rounded-xl bg-toast px-3 py-2.5 text-[12px] text-toast-content ${clickToClose ? "cursor-pointer" : ""}`}
      onClick={clickToClose ? () => dismiss(x.id) : undefined}
    >
      <div className="flex items-start gap-2">
        {x.kind === "progress" ? (
          <span className="loading loading-spinner loading-xs mt-0.5 shrink-0 text-primary" />
        ) : (
          <span className={`mt-[5px] h-2 w-2 shrink-0 rounded-full ${x.kind === "error" ? "bg-error" : "bg-primary"}`} />
        )}
        <div className="min-w-0 flex-1">
          <div className="font-semibold">{x.text}</div>
          {x.desc && <div className="mt-0.5 text-[11px] leading-snug opacity-70">{x.desc}</div>}
        </div>
        {x.action && (
          <>
            <button className="btn btn-primary btn-xs" onClick={x.action.run}>{x.action.label}</button>
            <button className="px-1 opacity-60 hover:opacity-100" aria-label={t("toast.close")} onClick={() => dismiss(x.id)}>×</button>
          </>
        )}
      </div>
      {x.kind === "progress" && (
        <progress
          className="progress progress-primary mt-2 h-1 w-full"
          aria-label={x.text}
          value={x.progress == null ? undefined : x.progress * 100}
          max={100}
        />
      )}
    </div>
  );
}

/** bottom: 창 아래에서 띄울 거리(px). 메인 창은 상태 막대 위로 올린다. */
export function Toasts({ bottom = 16 }: { bottom?: number }) {
  const { toasts, dismiss } = useToasts();
  const ref = useRef<HTMLDivElement>(null);

  // 모달 <dialog>(top layer) 위에 보이도록 popover로 띄운다. 다시 열어야 top layer 맨 위로 올라간다.
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (el.matches(":popover-open")) el.hidePopover();
    if (toasts.length > 0) el.showPopover();
  }, [toasts]);

  return (
    <div
      ref={ref}
      popover="manual"
      style={{ inset: "auto", right: 16, bottom }}
      className="fixed m-0 flex w-[260px] flex-col gap-2 overflow-visible border-0 bg-transparent p-0"
    >
      {toasts.map((x) => (
        <ToastCard key={x.id} x={x} dismiss={dismiss} />
      ))}
    </div>
  );
}
```

- [ ] **Step 7: 창별 위치 전달**

`src/main.tsx`에서 `<Toasts />`를 `<Toasts bottom={label === "main" ? 44 : 16} />`로 바꾼다(상태 막대 32px + 여백 12px).

- [ ] **Step 8: 전체 테스트·빌드**

Run: `yarn test && yarn build`
Expected: 모두 PASS, 빌드 성공.

- [ ] **Step 9: Commit**

```bash
git add src/lib/toast.ts src/components/Toasts.tsx src/main.tsx src/locales src/test/toast.test.ts
git commit -m "feat(ui): 토스트에 key·sticky·진행·액션 추가, 어두운 카드 모양"
```

---

### Task 3: '다운로드 준비 중'을 토스트로

**Files:**
- Modify: `src/lib/tools.ts`, `src/pages/MainWindow.tsx`
- Test: `src/test/tools.test.ts` (신규)

**Interfaces:**
- Consumes: `useToasts.getState().push/dismissKey` (Task 2)
- Produces: `export type ToolsToast = "hidden" | "preparing" | "failed"; export function toolsToast(s: ToolsStatus | null): ToolsToast` (`src/lib/tools.ts`)

- [ ] **Step 1: 실패하는 테스트 작성**

`src/test/tools.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { toolsToast } from "../lib/tools";
import type { ToolsStatus } from "../lib/types";

const st = (p: Partial<ToolsStatus>): ToolsStatus => ({ ready: false, installing: false, ytdlp_version: null, error: null, last_check: null, ...p });

describe("toolsToast", () => {
  it("hides once tools are ready", () => {
    expect(toolsToast(st({ ready: true }))).toBe("hidden");
    expect(toolsToast(st({ ready: true, error: "old" }))).toBe("hidden");
  });
  it("shows preparing before status arrives and while installing", () => {
    expect(toolsToast(null)).toBe("preparing");
    expect(toolsToast(st({ installing: true }))).toBe("preparing");
    expect(toolsToast(st({ installing: true, error: "retrying" }))).toBe("preparing");
  });
  it("shows failed when an error remains and nothing is installing", () => {
    expect(toolsToast(st({ error: "offline" }))).toBe("failed");
  });
});
```

- [ ] **Step 2: 실패 확인**

Run: `yarn vitest run src/test/tools.test.ts`
Expected: FAIL — `toolsToast` is not exported.

- [ ] **Step 3: 구현**

`src/lib/tools.ts` 끝에 추가(맨 위 import에 `ToolsStatus`는 이미 있음):

```ts
export type ToolsToast = "hidden" | "preparing" | "failed";

/** 메인 창의 '다운로드 준비 중' 토스트 상태. 예전 상단 배너와 같은 규칙. */
export function toolsToast(s: ToolsStatus | null): ToolsToast {
  if (s?.ready) return "hidden";
  if (s?.error && !s.installing) return "failed";
  return "preparing";
}
```

- [ ] **Step 4: 통과 확인**

Run: `yarn vitest run src/test/tools.test.ts`
Expected: PASS

- [ ] **Step 5: MainWindow의 ToolsNotice를 토스트 effect로 교체**

`src/pages/MainWindow.tsx`:
- import 변경: `import { useTools } from "../lib/tools";` → `import { toolsToast, useTools } from "../lib/tools";`, `import { showError } from "../lib/toast";` → `import { showError, useToasts } from "../lib/toast";`
- `function ToolsNotice() { ... }` 전체를 다음으로 바꾼다:

```tsx
/** 첫 실행 등 도구가 준비되지 않았을 때 상단 배너 대신 지속 토스트를 띄운다. */
function useToolsToast() {
  const { t } = useTranslation();
  const status = useTools((s) => s.status);
  const kind = toolsToast(status);
  const error = status?.error;
  useEffect(() => {
    const { push, dismissKey } = useToasts.getState();
    if (kind === "hidden") dismissKey("tools");
    else if (kind === "failed")
      push({
        key: "tools", kind: "error", sticky: true,
        text: t("app.toolsFailed", { error }),
        action: { label: t("app.retry"), run: () => void api.updateTools().catch(showError) },
      });
    else push({ key: "tools", kind: "progress", sticky: true, progress: null, text: t("app.toolsPreparing"), desc: t("app.toolsPreparingDesc") });
  }, [kind, error, t]);
}
```

- `MainWindow` 본문 맨 위(`const { t } = useTranslation();` 다음 줄)에 `useToolsToast();`를 추가하고, JSX에서 `<ToolsNotice />` 줄을 지운다.

- [ ] **Step 6: 테스트·빌드**

Run: `yarn test && yarn build`
Expected: PASS, 빌드 성공.

- [ ] **Step 7: Commit**

```bash
git add src/lib/tools.ts src/pages/MainWindow.tsx src/test/tools.test.ts
git commit -m "feat(ui): '다운로드 준비 중'을 상단 배너 대신 토스트로"
```

---

### Task 4: Rust — 새 버전을 백그라운드로 받아 보관

**Files:**
- Modify: `src-tauri/src/updater.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/tray.rs:285-288`(테스트의 `UpdateInfo` 리터럴)

**Interfaces:**
- Produces:
  - `UpdateInfo { version: String, notes: String, ready: bool, downloading: bool }`(JSON 키 동일)
  - 이벤트: `update-available`(UpdateInfo), `update-progress`(UpdateProgress, 기존), `update-ready`(UpdateInfo), `update-download-failed`(String), `update-scheduled`(기존), `update-error`(기존)
  - 명령: `retry_update_download()` → `()`
  - `pub fn start_download(app: &AppHandle)`

- [ ] **Step 1: 직렬화 테스트를 먼저 고친다(실패하게)**

`updater.rs` 테스트 `info_serializes_with_frontend_keys`의 `UpdateInfo` 부분을 다음으로 바꾼다:

```rust
        let v = serde_json::to_value(UpdateInfo {
            version: "0.2.0".into(),
            notes: "fix".into(),
            ready: true,
            downloading: false,
        })
        .unwrap();
        assert_eq!(
            v,
            serde_json::json!({"version": "0.2.0", "notes": "fix", "ready": true, "downloading": false})
        );
```

`tray.rs` 테스트 `update_label_follows_pending_state`의 리터럴에도 `ready: false, downloading: false,`를 더한다.

- [ ] **Step 2: 실패 확인**

Run: `cargo test -p kiri-app --lib updater`
Expected: 컴파일 FAIL — `UpdateInfo`에 `ready` 필드 없음.

- [ ] **Step 3: 상태·정보 타입 변경**

`updater.rs`:
- `use std::{ sync::{ Mutex, atomic::{...} }, time::Duration };`에 `Arc`를 더한다: `sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}`.
- `UpdateState`와 `UpdateInfo`·`impl UpdateInfo`를 다음으로 바꾼다:

```rust
/// 찾은 업데이트와, 받아 두었으면 그 파일. 메모리에만 둔다(앱을 끄면 다음에 다시 받는다).
struct Pending {
    update: Update,
    bytes: Option<Arc<Vec<u8>>>,
}

#[derive(Default)]
pub struct UpdateState(Mutex<Option<Pending>>);

/// 백그라운드 받기가 도는 중.
static DOWNLOADING: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
    /// 받기를 마쳐 바로 설치할 수 있다.
    pub ready: bool,
    pub downloading: bool,
}

impl UpdateInfo {
    fn from_pending(p: &Pending) -> Self {
        Self {
            version: p.update.version.clone(),
            notes: p.update.body.clone().unwrap_or_default(),
            ready: p.bytes.is_some(),
            downloading: DOWNLOADING.load(Ordering::SeqCst),
        }
    }
}
```

- `status()`의 `.map(UpdateInfo::from_update)`를 `.map(UpdateInfo::from_pending)`로 바꾼다.

- [ ] **Step 4: check — 같은 버전이면 받은 파일 유지, 그 뒤 받기 시작**

`check()` 전체를 바꾼다:

```rust
pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let found = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    {
        let mut slot = app.state::<UpdateState>().0.lock().unwrap();
        let old = slot.take();
        *slot = found.map(|update| {
            // 같은 버전을 다시 찾았으면 이미 받은 파일을 버리지 않는다.
            let bytes = old
                .filter(|o| o.update.version == update.version)
                .and_then(|o| o.bytes);
            Pending { update, bytes }
        });
    }
    start_download(app); // DOWNLOADING 을 먼저 켜서 아래 info 가 downloading=true 를 싣는다
    let info = status(app);
    if let Some(i) = &info {
        let _ = app.emit("update-available", i);
    }
    crate::tray::relabel_update(app);
    Ok(info)
}
```

- [ ] **Step 5: 받기 함수와 start_download**

`check()` 아래에 추가:

```rust
/// 진행률을 update-progress 로 알리며 받는다. 백그라운드 받기와 설치가 같이 쓴다.
async fn download(app: &AppHandle, update: &Update) -> Result<Vec<u8>, String> {
    let mut received: u64 = 0;
    let mut last_pct = u64::MAX;
    update
        .download(
            |chunk, total| {
                received += chunk as u64;
                let pct = total.map_or(0, |t| received * 100 / t.max(1));
                if pct != last_pct {
                    last_pct = pct;
                    let _ = app.emit("update-progress", UpdateProgress { received, total });
                }
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())
}

/// 보관한 업데이트를 아직 받지 않았으면 백그라운드에서 받는다. 이미 받는 중이면 아무것도 하지 않는다.
pub fn start_download(app: &AppHandle) {
    let update = {
        let slot = app.state::<UpdateState>().0.lock().unwrap();
        match slot.as_ref() {
            Some(p) if p.bytes.is_none() => p.update.clone(),
            _ => return,
        }
    };
    if DOWNLOADING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = download(&app, &update).await;
        DOWNLOADING.store(false, Ordering::SeqCst);
        match result {
            Ok(bytes) => {
                let info = {
                    let mut slot = app.state::<UpdateState>().0.lock().unwrap();
                    match slot.as_mut() {
                        // 받는 사이 다른 버전으로 바뀌었으면 버린다.
                        Some(p) if p.update.version == update.version => {
                            p.bytes = Some(Arc::new(bytes));
                            Some(UpdateInfo::from_pending(p))
                        }
                        _ => None,
                    }
                };
                if let Some(i) = info {
                    let _ = app.emit("update-ready", i);
                }
            }
            Err(e) => {
                eprintln!("kiri update: background download failed: {e}");
                let _ = app.emit("update-download-failed", e);
            }
        }
    });
}
```

- [ ] **Step 6: install — 받아 둔 파일을 쓴다**

`install()`에서 `let update = ... .ok_or_else(...)?;`부터 `update.install(bytes).map_err(|e| fail(e.to_string()))?;`까지를 다음으로 바꾼다:

```rust
    let (update, ready) = app
        .state::<UpdateState>()
        .0
        .lock()
        .unwrap()
        .as_ref()
        .map(|p| (p.update.clone(), p.bytes.clone()))
        .ok_or_else(|| fail("error.no_update".to_string()))?;
    // 받아 두었으면 그대로, 아니면 지금 받는다. 백그라운드 받기와 드물게 겹치면 한 번 더 받는다(결과는 같음).
    let bytes = match ready {
        Some(b) => b,
        None => Arc::new(download(app, &update).await.map_err(fail)?),
    };
    update.install(bytes.as_slice()).map_err(|e| fail(e.to_string()))?;
```

- [ ] **Step 7: 다시 시도 명령**

`commands.rs`의 `install_update` 아래에 추가:

```rust
#[tauri::command]
pub fn retry_update_download(app: AppHandle) {
    crate::updater::start_download(&app);
}
```

`lib.rs`의 `generate_handler!` 목록에서 `commands::install_update,` 다음 줄에 `commands::retry_update_download,`를 추가한다.

- [ ] **Step 8: 테스트·빌드**

Run: `cargo test --workspace && cargo clippy --workspace -- -D warnings`
Expected: 모두 PASS, 경고 없음. (clippy가 이 저장소에서 원래 경고를 내면 새로 생긴 경고만 없애면 된다.)

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src
git commit -m "feat(core): 새 버전을 찾으면 백그라운드로 받아 두고, 설치 때 받은 파일을 쓴다"
```

---

### Task 5: 프론트 업데이트 상태 기계

**Files:**
- Modify: `src/lib/update.ts`, `src/lib/types.ts`, `src/lib/tauri.ts`
- Test: `src/test/update.test.ts` (신규)

**Interfaces:**
- Consumes: Task 4의 이벤트·명령·`UpdateInfo` 필드
- Produces (`src/lib/update.ts`):
  ```ts
  export type UpdatePhase = "idle" | "downloading" | "ready" | "failed" | "scheduled" | "installing";
  export type UpdateEvent =
    | { type: "found"; info: UpdateInfo } | { type: "progress" } | { type: "ready" }
    | { type: "downloadFailed" } | { type: "retry" } | { type: "scheduled" }
    | { type: "install" } | { type: "installFailed"; ready: boolean };
  export function nextPhase(p: UpdatePhase, e: UpdateEvent): UpdatePhase
  export function updatePct(p: UpdateProgress | null): number | null
  useUpdate: { info, phase, progress, checking, checked, check(), install(afterQueue), retry(), subscribe() }
  ```
  (`scheduled: boolean` 필드는 없어지고 `phase === "scheduled"`로 대신한다.)

- [ ] **Step 1: 타입과 API**

`src/lib/types.ts`의 `UpdateInfo`:

```ts
export interface UpdateInfo {
  version: string;
  notes: string;
  /** 백그라운드 받기를 마쳐 바로 설치할 수 있다 */
  ready: boolean;
  downloading: boolean;
}
```

`src/lib/tauri.ts`의 `installUpdate` 다음 줄에: `retryUpdateDownload: () => invoke<void>("retry_update_download"),`

- [ ] **Step 2: 실패하는 테스트 작성**

`src/test/update.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { nextPhase, updatePct, type UpdatePhase } from "../lib/update";
import type { UpdateInfo } from "../lib/types";

const info = (p: Partial<UpdateInfo>): UpdateInfo => ({ version: "0.3.0", notes: "", ready: false, downloading: false, ...p });

describe("nextPhase", () => {
  it("found maps backend state", () => {
    expect(nextPhase("idle", { type: "found", info: info({ downloading: true }) })).toBe("downloading");
    expect(nextPhase("idle", { type: "found", info: info({ ready: true }) })).toBe("ready");
    expect(nextPhase("idle", { type: "found", info: info({}) })).toBe("failed");
  });
  it("background events advance download phases", () => {
    expect(nextPhase("idle", { type: "progress" })).toBe("downloading");
    expect(nextPhase("downloading", { type: "ready" })).toBe("ready");
    expect(nextPhase("downloading", { type: "downloadFailed" })).toBe("failed");
    expect(nextPhase("failed", { type: "retry" })).toBe("downloading");
  });
  it("scheduled and installing are not overridden by background events", () => {
    for (const p of ["scheduled", "installing"] as UpdatePhase[]) {
      expect(nextPhase(p, { type: "progress" })).toBe(p);
      expect(nextPhase(p, { type: "ready" })).toBe(p);
      expect(nextPhase(p, { type: "downloadFailed" })).toBe(p);
      expect(nextPhase(p, { type: "found", info: info({ ready: true }) })).toBe(p);
    }
  });
  it("user actions", () => {
    expect(nextPhase("ready", { type: "install" })).toBe("installing");
    expect(nextPhase("ready", { type: "scheduled" })).toBe("scheduled");
  });
  it("a failed install returns to ready only when the file is already downloaded", () => {
    expect(nextPhase("installing", { type: "installFailed", ready: true })).toBe("ready");
    expect(nextPhase("installing", { type: "installFailed", ready: false })).toBe("failed");
  });
});

describe("updatePct", () => {
  it("is null without a known total", () => {
    expect(updatePct(null)).toBeNull();
    expect(updatePct({ received: 5, total: null })).toBeNull();
    expect(updatePct({ received: 48, total: 100 })).toBe(48);
  });
});
```

- [ ] **Step 3: 실패 확인**

Run: `yarn vitest run src/test/update.test.ts`
Expected: FAIL — `nextPhase` is not exported.

- [ ] **Step 4: 구현**

`src/lib/update.ts` 전체:

```ts
import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { showError } from "./toast";
import type { UpdateInfo, UpdateProgress } from "./types";

export type UpdatePhase = "idle" | "downloading" | "ready" | "failed" | "scheduled" | "installing";
export type UpdateEvent =
  | { type: "found"; info: UpdateInfo }
  | { type: "progress" }
  | { type: "ready" }
  | { type: "downloadFailed" }
  | { type: "retry" }
  | { type: "scheduled" }
  | { type: "install" }
  | { type: "installFailed"; ready: boolean };

/** 사용자가 고른 설치 흐름은 백그라운드 이벤트로 덮지 않는다. */
const userDriven = (p: UpdatePhase) => p === "scheduled" || p === "installing";

export function nextPhase(p: UpdatePhase, e: UpdateEvent): UpdatePhase {
  switch (e.type) {
    case "found":
      if (userDriven(p)) return p;
      return e.info.ready ? "ready" : e.info.downloading ? "downloading" : "failed";
    case "progress":
      return userDriven(p) ? p : "downloading";
    case "ready":
      return userDriven(p) ? p : "ready";
    case "downloadFailed":
      return userDriven(p) ? p : "failed";
    case "retry":
      return "downloading";
    case "scheduled":
      return "scheduled";
    case "install":
      return "installing";
    case "installFailed":
      return e.ready ? "ready" : "failed";
  }
}

export const updatePct = (p: UpdateProgress | null) => (p?.total ? Math.round((p.received / p.total) * 100) : null);

interface UpdateStore {
  info: UpdateInfo | null;
  phase: UpdatePhase;
  progress: UpdateProgress | null;
  checking: boolean;
  checked: boolean; // 수동 확인을 끝낸 뒤에만 "최신 버전" 을 말한다
  check: () => Promise<void>;
  install: (afterQueue: boolean) => Promise<void>;
  retry: () => void;
  subscribe: () => () => void;
}

// 설치 중 두 번째 요청. 사용자에게 알릴 일이 아니다.
const isBusy = (e: unknown) => (e instanceof Error ? e.message : String(e)) === "busy";

export const useUpdate = create<UpdateStore>((set, get) => {
  const step = (e: UpdateEvent) => set((s) => ({ phase: nextPhase(s.phase, e) }));
  const found = (info: UpdateInfo) => {
    set({ info, progress: null });
    step({ type: "found", info });
  };
  const installFailed = (e: unknown) => {
    if (isBusy(e)) return;
    step({ type: "installFailed", ready: get().info?.ready ?? false });
    showError(e);
  };
  return {
    info: null,
    phase: "idle",
    progress: null,
    checking: false,
    checked: false,
    check: async () => {
      set({ checking: true });
      try {
        const info = await api.checkUpdate();
        set({ checked: true });
        if (info) found(info);
      } catch (e) {
        showError(e);
      } finally {
        set({ checking: false });
      }
    },
    install: async (afterQueue) => {
      if (!afterQueue) step({ type: "install" });
      try {
        await api.installUpdate(afterQueue);
      } catch (e) {
        installFailed(e);
      }
    },
    retry: () => {
      step({ type: "retry" });
      api.retryUpdateDownload().catch(showError);
    },
    subscribe: () => {
      api.updateStatus().then((info) => info && !get().info && found(info)).catch(() => {});
      const subs = [
        listen<UpdateInfo>("update-available", (e) => found(e.payload)),
        listen<UpdateProgress>("update-progress", (e) => {
          set({ progress: e.payload });
          step({ type: "progress" });
        }),
        listen<UpdateInfo>("update-ready", (e) => {
          set({ info: e.payload, progress: null });
          step({ type: "ready" });
        }),
        listen<string>("update-download-failed", () => step({ type: "downloadFailed" })),
        listen<boolean>("update-scheduled", () => step({ type: "scheduled" })),
        listen<string>("update-error", (e) => installFailed(e.payload)),
      ];
      return () => {
        for (const p of subs) p.then((un) => un()).catch(() => {});
      };
    },
  };
});
```

- [ ] **Step 5: 통과 확인**

Run: `yarn vitest run src/test/update.test.ts`
Expected: PASS

- [ ] **Step 6: 타입 오류 정리**

Run: `yarn build`
Expected: `src/components/UpdateBanner.tsx`에서 `scheduled`가 없다는 tsc 오류. 이 컴포넌트는 Task 6에서 지운다. 지금은 `src/components/UpdateBanner.tsx`의 `const { info, progress, scheduled, install } = useUpdate();`를 다음 두 줄로 바꿔 빌드를 통과시킨다:

```tsx
  const { info, progress, phase, install } = useUpdate();
  const scheduled = phase === "scheduled";
```

`src/pages/settings/UpdateTab.tsx`는 그대로 컴파일된다. 다시 `yarn build` → 성공.

- [ ] **Step 7: Commit**

```bash
git add src/lib/update.ts src/lib/types.ts src/lib/tauri.ts src/test/update.test.ts src/components/UpdateBanner.tsx
git commit -m "feat(ui): 업데이트 상태 기계(nextPhase) — 백그라운드 받기·준비·실패·예약·설치"
```

---

### Task 6: 메인 창 — 드롭존, 작업 카드, 빈 상태, 상태 막대

**Files:**
- Create: `src/components/DropZone.tsx`, `src/components/StatusBar.tsx`
- Modify: `src/lib/queue.ts`, `src/components/JobRow.tsx`, `src/pages/MainWindow.tsx`, `src/locales/{ko,en,ja}.json`
- Delete: `src/components/UpdateBanner.tsx`
- Test: `src/test/queue.test.ts`

**Interfaces:**
- Consumes: `useUpdate`, `updatePct`, `UpdatePhase` (Task 5); `.surface-card`, `.chip-*`, `.float-shadow` (Task 1)
- Produces:
  - `export type SummaryKey = "running" | "queued" | "completed" | "failed"; export function queueSummary(jobs: Job[]): [SummaryKey, number][]` (`src/lib/queue.ts`)
  - `export function DropZone({ dragging, onClick }: { dragging: boolean; onClick: () => void })`
  - `export function StatusBar()`

- [ ] **Step 1: 실패하는 테스트 작성**

`src/test/queue.test.ts`: import를 `import { hasActive, isIdle, queueSummary } from "../lib/queue";`로 바꾸고 `describe` 블록 안 끝에 추가:

```ts
  it("queueSummary groups running states and drops zero counts", () => {
    expect(queueSummary([])).toEqual([]);
    expect(
      queueSummary([
        job({ kind: "downloading" }), job({ kind: "encoding" }), job({ kind: "queued" }),
        job({ kind: "completed" }), job({ kind: "stopped" }),
      ]),
    ).toEqual([["running", 2], ["queued", 1], ["completed", 1]]);
    expect(queueSummary([job({ kind: "failed", message: "x" })])).toEqual([["failed", 1]]);
  });
```

- [ ] **Step 2: 실패 확인**

Run: `yarn vitest run src/test/queue.test.ts`
Expected: FAIL — `queueSummary` is not exported.

- [ ] **Step 3: 구현**

`src/lib/queue.ts`의 `isIdle` 다음 줄에 추가(맨 위 import를 `import type { Job, JobState } from "./types";`로):

```ts
export type SummaryKey = "running" | "queued" | "completed" | "failed";

/** 상태 막대 왼쪽 요약. 0인 항목은 뺀다. 중지된 작업은 세지 않는다. */
export function queueSummary(jobs: Job[]): [SummaryKey, number][] {
  const count = (...kinds: JobState["kind"][]) => jobs.filter((j) => kinds.includes(j.state.kind)).length;
  const all: [SummaryKey, number][] = [
    ["running", count("downloading", "encoding")],
    ["queued", count("queued")],
    ["completed", count("completed")],
    ["failed", count("failed")],
  ];
  return all.filter(([, n]) => n > 0);
}
```

- [ ] **Step 4: 통과 확인**

Run: `yarn vitest run src/test/queue.test.ts`
Expected: PASS

- [ ] **Step 5: 문구 추가·정리**

세 locale 파일에서:
- `app.dropHint` 삭제. `app`에 `dropTitle`, `dropShortcut`, `emptyDesc` 추가.
- 최상위에 `status` 객체 추가.
- `update`에서 `available`, `install`, `downloading` 삭제. `downloadingBg`, `ready`, `restartInstall`, `installMenu`, `readyTitle`, `busyDesc`, `downloadFailed`, `retry`, `installing` 추가(`installNow`, `afterQueue`, `scheduled`는 유지).

ko:
```json
"dropTitle": "YouTube 링크를 붙여넣거나 끌어다 놓으세요",
"dropShortcut": "로 바로 추가",
"emptyDesc": "링크를 붙여넣으면 여기에 나타나요",
```
```json
"status": { "running": "진행 중 {{n}}", "queued": "대기 {{n}}", "completed": "완료 {{n}}", "failed": "실패 {{n}}" },
```
```json
"downloadingBg": "v{{version}} 받는 중…",
"ready": "새 버전 v{{version}}",
"restartInstall": "재시작하여 설치",
"installMenu": "설치…",
"readyTitle": "kiri v{{version}} 설치 준비 완료",
"busyDesc": "다운로드 {{n}}개가 진행 중이에요. 재시작하면 대기열로 돌아가 다시 이어집니다.",
"downloadFailed": "업데이트 받기 실패",
"retry": "다시 시도",
"installing": "설치 중…"
```

en:
```json
"dropTitle": "Paste or drop a YouTube link",
"dropShortcut": "to add it right away",
"emptyDesc": "Links you paste will show up here",
```
```json
"status": { "running": "{{n}} running", "queued": "{{n}} queued", "completed": "{{n}} done", "failed": "{{n}} failed" },
```
```json
"downloadingBg": "Downloading v{{version}}…",
"ready": "New version v{{version}}",
"restartInstall": "Restart to install",
"installMenu": "Install…",
"readyTitle": "kiri v{{version}} is ready to install",
"busyDesc": "{{n}} downloads are in progress. Restarting puts them back in the queue and they continue afterwards.",
"downloadFailed": "Update download failed",
"retry": "Try again",
"installing": "Installing…"
```

ja:
```json
"dropTitle": "YouTube のリンクをペーストまたはドロップ",
"dropShortcut": "ですぐに追加",
"emptyDesc": "ペーストしたリンクがここに表示されます",
```
```json
"status": { "running": "進行中 {{n}}", "queued": "待機 {{n}}", "completed": "完了 {{n}}", "failed": "失敗 {{n}}" },
```
```json
"downloadingBg": "v{{version}} をダウンロード中…",
"ready": "新しいバージョン v{{version}}",
"restartInstall": "再起動してインストール",
"installMenu": "インストール…",
"readyTitle": "kiri v{{version}} のインストール準備ができました",
"busyDesc": "{{n}} 件のダウンロードが進行中です。再起動するとキューに戻り、その後再開します。",
"downloadFailed": "アップデートのダウンロードに失敗",
"retry": "再試行",
"installing": "インストール中…"
```

Run: `yarn vitest run src/test/locales.test.ts`
Expected: PASS

- [ ] **Step 6: DropZone**

`src/components/DropZone.tsx`:

```tsx
import { useTranslation } from "react-i18next";
import { Icon } from "./Icon";

/** 붙여넣기·드롭 안내. 누르면 클립보드에서 붙여넣는다(⌘V와 같음). */
export function DropZone({ dragging, onClick }: { dragging: boolean; onClick: () => void }) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      onClick={onClick}
      className={`mx-3.5 mt-1.5 mb-2.5 flex items-center gap-3 rounded-[14px] border-[1.5px] border-dashed px-3 py-2.5 text-left transition-colors ${
        dragging ? "border-primary bg-secondary" : "border-primary/40 bg-primary/5 hover:bg-primary/10"
      }`}
    >
      <span className="grid h-[30px] w-[30px] shrink-0 place-items-center rounded-[10px] bg-secondary text-primary">
        <Icon name="download" className="h-4 w-4" />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[12.5px] font-semibold">{t("app.dropTitle")}</span>
        <span className="mt-0.5 block text-[11px] text-fg-muted">
          <kbd className="kbd kbd-xs">⌘</kbd> <kbd className="kbd kbd-xs">V</kbd> {t("app.dropShortcut")}
        </span>
      </span>
    </button>
  );
}
```

- [ ] **Step 7: StatusBar (큐 요약 + 업데이트 상태 + 설치 팝오버)**

`src/components/StatusBar.tsx`:

```tsx
import { Fragment } from "react";
import { useTranslation } from "react-i18next";
import { queueSummary, useQueue } from "../lib/queue";
import { updatePct, useUpdate } from "../lib/update";

const POPOVER_ID = "update-install";

function UpdateStatus({ pending }: { pending: number }) {
  const { t } = useTranslation();
  const { info, phase, progress, install, retry } = useUpdate();
  if (!info || phase === "idle") return null;
  const version = info.version;
  const ring = <span className="loading loading-spinner loading-xs text-primary" />;

  if (phase === "downloading") {
    const pct = updatePct(progress);
    return (
      <span className="flex items-center gap-1.5">
        {ring}
        {t("update.downloadingBg", { version })}
        {pct !== null && ` ${pct}%`}
      </span>
    );
  }
  if (phase === "installing") return <span className="flex items-center gap-1.5">{ring}{t("update.installing")}</span>;
  if (phase === "scheduled") return <span>{t("update.scheduled")}</span>;
  if (phase === "failed") {
    return (
      <span className="flex items-center gap-2">
        <span className="text-error">{t("update.downloadFailed")}</span>
        <button className="btn btn-ghost btn-xs" onClick={retry}>{t("update.retry")}</button>
      </span>
    );
  }
  // ready
  const hide = () => document.getElementById(POPOVER_ID)?.hidePopover();
  return (
    <span className="flex items-center gap-1.5 rounded-lg bg-secondary py-0.5 pr-0.5 pl-2 font-semibold text-secondary-content">
      <span className="h-[7px] w-[7px] rounded-full bg-primary" />
      {t("update.ready", { version })}
      {pending === 0 ? (
        <button className="btn btn-primary btn-xs" onClick={() => install(false)}>{t("update.restartInstall")}</button>
      ) : (
        <>
          <button className="btn btn-primary btn-xs" popoverTarget={POPOVER_ID}>{t("update.installMenu")}</button>
          <div
            id={POPOVER_ID}
            popover="auto"
            style={{ inset: "auto", right: 12, bottom: 40 }}
            className="float-shadow fixed m-0 w-[260px] rounded-[14px] border border-base-300 bg-base-100 p-3 font-normal text-base-content"
          >
            <div className="text-[12.5px] font-bold">{t("update.readyTitle", { version })}</div>
            <p className="mt-1 text-[11px] leading-relaxed text-fg-muted">{t("update.busyDesc", { n: pending })}</p>
            <div className="mt-2.5 flex gap-1.5">
              <button className="btn btn-sm flex-1" onClick={() => { hide(); install(false); }}>{t("update.installNow")}</button>
              <button className="btn btn-primary btn-sm flex-1" onClick={() => { hide(); install(true); }}>{t("update.afterQueue")}</button>
            </div>
          </div>
        </>
      )}
    </span>
  );
}

export function StatusBar() {
  const { t } = useTranslation();
  const jobs = useQueue((s) => s.jobs);
  const summary = queueSummary(jobs);
  const pending = summary.filter(([k]) => k === "running" || k === "queued").reduce((a, [, n]) => a + n, 0);
  return (
    <footer className="flex h-8 shrink-0 items-center gap-2.5 border-t border-base-300 bg-base-200 px-3.5 text-[11px] text-fg-muted">
      {summary.map(([k, n], i) => (
        <Fragment key={k}>
          {i > 0 && <span className="h-3 w-px bg-base-300" />}
          <span>{t(`status.${k}`, { n })}</span>
        </Fragment>
      ))}
      <span className="flex-1" />
      <UpdateStatus pending={pending} />
    </footer>
  );
}
```

- [ ] **Step 8: JobRow를 카드로**

`src/components/JobRow.tsx`에서 `const badge = ...;` 줄을 다음으로 바꾼다:

```tsx
  const chip = kind === "completed" ? "chip-ok" : kind === "failed" ? "chip-err" : running ? "chip-run" : "chip-idle";
```

`return (...)` 전체를 다음으로 바꾼다(버튼 조건·동작은 기존과 같다):

```tsx
  const btn = "btn btn-ghost btn-sm btn-square";
  return (
    <div className="group surface-card mx-3.5 mb-2 flex items-center gap-3 px-2.5 py-2.5">
      {job.thumbnail ? (
        <img src={job.thumbnail} alt="" className="h-[42px] w-[72px] shrink-0 rounded-lg object-cover" />
      ) : (
        <div className="flex h-[42px] w-[72px] shrink-0 items-center justify-center rounded-lg bg-base-200 text-fg-muted">
          {job.source.kind === "file" && <Icon name="file" />}
        </div>
      )}
      <div className="min-w-0 flex-1">
        <div className="truncate text-[12.5px] font-semibold">{job.title}</div>
        <div className={`truncate text-[11px] ${kind === "failed" ? "text-error" : "text-fg-muted"}`}>{jobDetail(job, t)}</div>
        {running && (
          <progress
            className={`progress mt-1.5 h-[5px] w-full ${kind === "encoding" ? "progress-info" : "progress-primary"}`}
            value={job.progress * 100}
            max={100}
          />
        )}
      </div>
      <span className={`chip ${chip}`}>{running ? pct(job.progress) : t(`state.${kind}`)}</span>
      <div className="flex opacity-50 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
        {(running || kind === "queued") && (
          <button className={btn} aria-label={t("job.stop")} title={t("job.stop")} onClick={act(() => api.stopJob(job.id))}><Icon name="pause" className="h-4 w-4" /></button>
        )}
        {(kind === "stopped" || kind === "failed") && (
          <button className={btn} aria-label={t("job.restart")} title={t("job.restart")} onClick={act(() => api.restartJob(job.id))}><Icon name="restart" className="h-4 w-4" /></button>
        )}
        {job.state.kind === "failed" && job.state.message === "error.download_dir_unwritable" && (
          <button className={btn} aria-label={t("app.settings")} title={t("app.settings")} onClick={act(() => api.openSettings())}><Icon name="settings" className="h-4 w-4" /></button>
        )}
        {kind === "completed" && job.output && (
          <button className={btn} aria-label={t("job.reveal")} title={t("job.reveal")} onClick={act(() => revealItemInDir(job.output!))}><Icon name="reveal" className="h-4 w-4" /></button>
        )}
        <button className={btn} aria-label={t("job.remove")} title={t("job.remove")} onClick={act(() => api.removeJob(job.id))}><Icon name="remove" className="h-4 w-4" /></button>
      </div>
    </div>
  );
```

import 변경: `import { jobDetail } from "../lib/format";` → `import { jobDetail, pct } from "../lib/format";`

- [ ] **Step 9: MainWindow 레이아웃**

`src/pages/MainWindow.tsx`:
- import: `UpdateBanner` import 삭제. 추가: `import { DropZone } from "../components/DropZone";`, `import { StatusBar } from "../components/StatusBar";`
- `MainWindow` 안 `const [sheet, setSheet] = ...` 다음 줄에 `const [dragging, setDragging] = useState(false);`
- `useEffect(() => { const onKey = ...` 위에 붙여넣기 함수를 추가하고, keydown 핸들러의 `submit(readText().catch(() => null));`를 `paste();`로 바꾼다:

```tsx
  // ⌘V와 드롭존 클릭이 같이 쓴다. 시트가 열려 있으면 무시한다.
  const paste = useCallback(() => {
    if (sheetUrl.current !== null) return;
    submit(readText().catch(() => null));
  }, [submit]);
```

  keydown 핸들러의 v 분기는 다음이 된다(시트 검사는 `paste`가 하지만 `preventDefault` 전에 남겨 기존 동작을 유지):

```tsx
      } else if (e.key.toLowerCase() === "v" && !isEditableTarget(e.target)) {
        if (sheetUrl.current !== null) return;
        e.preventDefault();
        if (e.repeat) return;
        paste();
      }
```

  effect 의존성 배열을 `[paste]`로 바꾼다.
- `onDrop` 첫 줄 `e.preventDefault();` 다음에 `setDragging(false);`를 추가한다.
- `return (...)` 전체를 다음으로 바꾼다:

```tsx
  return (
    <div
      className="flex h-full flex-col"
      onDragOver={(e) => {
        e.preventDefault();
        if (!dragging) setDragging(true);
      }}
      onDragLeave={(e) => {
        // 자식 요소 사이를 오갈 때가 아니라 창 밖으로 나갈 때만 끈다.
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setDragging(false);
      }}
      onDrop={onDrop}
    >
      <header className="flex h-[42px] shrink-0 items-center gap-1 px-4">
        <span className="text-[15px] font-extrabold tracking-[-0.5px]">kiri<span className="text-primary">.</span></span>
        <span className="flex-1" />
        {jobs.some((j) => j.state.kind === "completed") && (
          <button className="btn btn-ghost btn-sm btn-square" aria-label={t("app.clearCompleted")} title={t("app.clearCompleted")} onClick={() => api.clearCompleted().catch(showError)}><Icon name="clear" className="h-4 w-4" /></button>
        )}
        <button className="btn btn-ghost btn-sm btn-square" aria-label={t("app.settings")} title={t("app.settings")} onClick={() => api.openSettings().catch(showError)}><Icon name="settings" className="h-4 w-4" /></button>
      </header>
      <DropZone dragging={dragging} onClick={paste} />
      <main className="flex-1 overflow-y-auto pb-1.5">
        {jobs.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-1.5 text-center">
            <span className="mb-1 grid h-10 w-10 place-items-center rounded-xl bg-base-200 text-fg-muted"><Icon name="download" /></span>
            <span className="text-sm font-semibold">{t("app.empty")}</span>
            <span className="text-xs text-fg-muted">{t("app.emptyDesc")}</span>
          </div>
        ) : (
          [...jobs].reverse().map((j) => <JobRow key={j.id} job={j} />)
        )}
      </main>
      <StatusBar />
      {sheet && <OptionsSheet url={sheet.url} info={sheet.info} onClose={() => setSheet(null)} />}
    </div>
  );
```

- [ ] **Step 10: UpdateBanner 삭제**

Run: `git rm src/components/UpdateBanner.tsx`

- [ ] **Step 11: 테스트·빌드**

Run: `yarn test && yarn build`
Expected: PASS, 빌드 성공. `src/pages/settings/UpdateTab.tsx`가 지운 `update.available`을 쓰고 있지만 i18n 키는 런타임 문자열이라 tsc는 통과한다. Task 8에서 바꾼다. 그 전에 실행하면 키 이름이 그대로 보일 수 있다.

- [ ] **Step 12: Commit**

```bash
git add -A src
git commit -m "feat(ui): 메인 창 리디자인 — 드롭존, 작업 카드, 빈 상태, 하단 상태 막대와 업데이트 표시"
```

---

### Task 7: 옵션 시트 — 중앙 모달

**Files:**
- Modify: `src/lib/sheet.ts`, `src/components/OptionsSheet.tsx`, `src/locales/{ko,en,ja}.json`
- Test: `src/test/sheet.test.ts`

**Interfaces:**
- Produces: `export function visiblePresets(defaultPreset: Preset): Preset[]` (`src/lib/sheet.ts`)

- [ ] **Step 1: 실패하는 테스트 작성**

`src/test/sheet.test.ts`의 import에 `visiblePresets`를 더하고 끝에 추가:

```ts
describe("visiblePresets", () => {
  it("shows up to three common formats, always including the default, in PRESETS order", () => {
    expect(visiblePresets("original")).toEqual(["original", "mp4-h264", "mp3"]);
    expect(visiblePresets("mp4-h264")).toEqual(["original", "mp4-h264", "mp3"]);
    expect(visiblePresets("webm-vp9")).toEqual(["original", "mp4-h264", "webm-vp9"]);
    expect(visiblePresets("m4a")).toEqual(["original", "mp4-h264", "m4a"]);
  });
});
```

- [ ] **Step 2: 실패 확인**

Run: `yarn vitest run src/test/sheet.test.ts`
Expected: FAIL — `visiblePresets` is not exported.

- [ ] **Step 3: 구현**

`src/lib/sheet.ts` 맨 위 import를 `import { AUDIO_PRESETS, PRESETS } from "./types";`로 바꾸고 끝에 추가:

```ts
const COMMON: Preset[] = ["original", "mp4-h264", "mp3"];

/** 시트에 처음 보이는 포맷 칩. 기본 포맷은 늘 포함하고 최대 3개, 순서는 PRESETS를 따른다(선택해도 자리가 바뀌지 않게). */
export function visiblePresets(defaultPreset: Preset): Preset[] {
  const pick = COMMON.includes(defaultPreset) ? COMMON : [...COMMON.slice(0, 2), defaultPreset];
  return PRESETS.filter((p) => pick.includes(p));
}
```

- [ ] **Step 4: 통과 확인**

Run: `yarn vitest run src/test/sheet.test.ts`
Expected: PASS

- [ ] **Step 5: 문구**

세 locale의 `sheet` 객체에 추가: ko `"more": "더보기"`, en `"more": "More"`, ja `"more": "もっと見る"`.

- [ ] **Step 6: 시트 마크업 교체**

`src/components/OptionsSheet.tsx`:
- import: `import { buildNewJob, defaultSubtitles, pickDefaultQuality, rememberPatch, visiblePresets } from "../lib/sheet";`
- `const [error, setError] = ...` 다음 줄에 `const [allPresets, setAllPresets] = useState(false);`
- `Section` 컴포넌트를 다음으로 바꾼다:

```tsx
function Section({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="mt-3.5">
      <div className="mb-1.5 text-[10.5px] font-bold text-fg-muted">{label}</div>
      {children}
    </div>
  );
}
```

- `qualityRow`와 `subChip`을 다음으로 바꾼다:

```tsx
  const qualityTile = (q: Quality | null) => {
    const on = (q?.format_id ?? null) === (quality?.format_id ?? null);
    return (
      <label
        key={q?.format_id ?? "audio"}
        className={`cursor-pointer rounded-[10px] border px-2.5 py-1.5 has-focus-visible:outline-2 has-focus-visible:outline-primary ${
          on ? "border-[1.5px] border-primary bg-primary/8" : "border-base-300 hover:border-primary/50"
        }`}
      >
        <input type="radio" name="quality" className="sr-only" checked={on} onChange={() => setQuality(q)} />
        <span className={`block text-[12px] font-semibold ${on ? "text-secondary-content" : ""}`}>{q ? q.label : t("quality.audio")}</span>
        <span className="block truncate text-[10px] text-fg-muted">{q ? [q.vcodec, formatBytes(q.filesize)].filter(Boolean).join(" · ") : "m4a"}</span>
      </label>
    );
  };

  const chipCls = (on: boolean) =>
    `mr-1 mb-1 inline-flex cursor-pointer items-center rounded-lg border px-2.5 py-1 text-[11px] has-focus-visible:outline-2 has-focus-visible:outline-primary ${
      on ? "border-primary bg-primary font-semibold text-primary-content" : "border-base-300 text-base-content/80 hover:border-primary/50"
    }`;

  const presetChip = (p: Preset) => (
    <label key={p} className={chipCls(preset === p)}>
      <input type="radio" name="preset" className="sr-only" checked={preset === p} onChange={() => setPreset(p)} />
      {t(`preset.${p}`)}
    </label>
  );

  const subChip = (l: string) => (
    <label key={l} className={chipCls(subs.includes(l))}>
      <input type="checkbox" className="sr-only" checked={subs.includes(l)} onChange={() => toggleSub(l)} />
      {langName(l, i18n.language)}
    </label>
  );
```

- `<dialog ...>`의 `className`을 다음으로 바꾼다(나머지 속성은 그대로):

```tsx
      className="float-shadow m-auto h-fit max-h-[88vh] w-[min(380px,92vw)] max-w-none overflow-y-auto rounded-[18px] bg-base-100 p-0 text-base-content outline-none backdrop:bg-black/30"
```

- `<div className="p-4">` 블록 전체(로딩 분기 포함)를 다음으로 바꾼다:

```tsx
      {!info ? (
        <div>
          <div className="h-[110px] animate-pulse bg-base-200" />
          <div className="flex items-center gap-3 p-4 text-sm">
            <span className="loading loading-spinner loading-sm" />
            {t("sheet.loading")}
          </div>
        </div>
      ) : (
        <>
          <div className="relative h-[110px] bg-gradient-to-br from-primary to-secondary">
            {info.thumbnail && <img src={info.thumbnail} alt="" className="absolute inset-0 h-full w-full object-cover" />}
            <div className="absolute inset-0 bg-gradient-to-b from-transparent from-30% to-black/75" />
            {info.duration_secs != null && (
              <span className="absolute top-2.5 right-2.5 rounded-md bg-black/60 px-1.5 py-0.5 text-[10px] text-white">{formatDuration(info.duration_secs)}</span>
            )}
            <div className="absolute right-3.5 bottom-2.5 left-3.5 text-white">
              <div className="truncate text-[13.5px] font-semibold">{info.title}</div>
              {info.channel && <div className="truncate text-[11px] opacity-80">{info.channel}</div>}
            </div>
          </div>

          <div className="px-4 pt-0.5 pb-4">
            <Section label={t("sheet.quality")}>
              <div role="radiogroup" aria-label={t("sheet.quality")} className="grid grid-cols-3 gap-1.5">
                {info.qualities.map(qualityTile)}
                {qualityTile(null)}
              </div>
            </Section>

            <Section label={t("sheet.format")}>
              <div role="radiogroup" aria-label={t("sheet.format")}>
                {(allPresets ? PRESETS : visiblePresets(defaults.preset)).map(presetChip)}
                {!allPresets && (
                  <button type="button" className={chipCls(false)} onClick={() => setAllPresets(true)}>{t("sheet.more")} ▾</button>
                )}
              </div>
            </Section>

            <Section label={t("sheet.subtitles")}>
              {info.subtitles.length === 0 && autoOnly.length === 0 && <div className="text-xs text-fg-muted">{t("sheet.noSubs")}</div>}
              <div>{info.subtitles.map(subChip)}</div>
              {autoOnly.length > 0 && (
                <details className="mt-1">
                  <summary className="cursor-pointer text-[11px] text-fg-muted">{t("sheet.autoSubs")} ({autoOnly.length})</summary>
                  <div className="mt-1.5 max-h-32 overflow-y-auto">{autoOnly.map(subChip)}</div>
                </details>
              )}
            </Section>

            {error && <div role="alert" className="alert alert-error mt-3 py-2 text-sm">{error}</div>}

            <div className="mt-4 flex items-center gap-2">
              <label className="flex flex-1 items-center gap-2 text-[11.5px] text-base-content/80">
                <input type="checkbox" className="checkbox checkbox-sm checkbox-primary" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
                {t("sheet.remember")}
              </label>
              <button className="btn btn-sm" onClick={onClose}>{t("sheet.cancel")}</button>
              <button ref={downloadRef} autoFocus className="btn btn-primary btn-sm" onClick={confirm} disabled={busy}>{t("sheet.download")}</button>
            </div>
          </div>
        </>
      )}
```

  (기본 포맷이 `visiblePresets`에 늘 들어가므로, 처음 선택값은 항상 보인다. "더보기"로 다른 포맷을 고른 뒤에는 `allPresets`가 true라 계속 보인다.)

- [ ] **Step 7: 테스트·빌드**

Run: `yarn test && yarn build`
Expected: PASS, 빌드 성공. `has-focus-visible:` 변형이 Tailwind 4에서 인식되지 않으면 `has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-primary`로 바꾼다.

- [ ] **Step 8: Commit**

```bash
git add src/lib/sheet.ts src/components/OptionsSheet.tsx src/locales src/test/sheet.test.ts
git commit -m "feat(ui): 옵션 시트를 썸네일 히어로 중앙 모달로 — 화질 타일, 포맷·자막 칩"
```

---

### Task 8: 설정 창 — 카드 그룹, 세그먼트, 업데이트 상태

**Files:**
- Create: `src/pages/settings/Group.tsx`
- Modify: `src/pages/SettingsWindow.tsx`, `src/pages/settings/{GeneralTab,DownloadTab,CliTab,UpdateTab}.tsx`, `src/locales/{ko,en,ja}.json`

**Interfaces:**
- Consumes: `useUpdate().phase/info/progress/install/retry`, `updatePct` (Task 5); `isIdle` (`src/lib/queue.ts`); `.surface-card` (Task 1)
- Produces: `export function Group({ title, children }: { title?: string; children: ReactNode })`

- [ ] **Step 1: 문구**

세 locale의 `settings` 객체에 `group` 추가:
- ko: `"group": { "display": "화면", "window": "창", "save": "저장", "defaults": "기본 옵션", "performance": "성능" }`
- en: `"group": { "display": "Appearance", "window": "Window", "save": "Saving", "defaults": "Defaults", "performance": "Performance" }`
- ja: `"group": { "display": "表示", "window": "ウインドウ", "save": "保存", "defaults": "デフォルト", "performance": "パフォーマンス" }`

Run: `yarn vitest run src/test/locales.test.ts` → PASS

- [ ] **Step 2: Group 컴포넌트**

`src/pages/settings/Group.tsx`:

```tsx
import type { ReactNode } from "react";

/** 설정 항목(Row)을 묶는 흰 카드. 제목은 카드 위 작은 회색 글씨. */
export function Group({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <section className="mb-3">
      {title && <h3 className="mx-1 mb-1.5 text-[10.5px] font-bold text-fg-muted">{title}</h3>}
      <div className="surface-card px-3.5">{children}</div>
    </section>
  );
}
```

- [ ] **Step 3: SettingsWindow 배경과 탭**

`src/pages/SettingsWindow.tsx`의 `return (...)`를 다음으로 바꾼다:

```tsx
  return (
    <div className="flex h-full flex-col bg-base-200">
      <nav role="tablist" className="flex justify-center gap-1 px-2 pt-2 pb-2.5">
        {TABS.map((x) => (
          <button
            key={x.key}
            role="tab"
            aria-selected={tab === x.key}
            className={`flex w-[72px] flex-col items-center gap-0.5 rounded-xl px-2 py-1.5 text-[11px] transition-colors ${tab === x.key ? "bg-secondary font-semibold text-secondary-content" : "text-fg-muted hover:bg-base-300/60"}`}
            onClick={() => setTab(x.key)}
          >
            <Icon name={x.icon} className="h-[18px] w-[18px]" />
            {t(`settings.tab.${x.key}`)}
          </button>
        ))}
      </nav>
      <div className="flex-1 overflow-y-auto px-5 pt-1 pb-4">
        <Body />
      </div>
    </div>
  );
```

- [ ] **Step 4: GeneralTab 그룹화**

`src/pages/settings/GeneralTab.tsx`: `import { Group } from "./Group";` 추가. `return` 안의 최상위 `<div>`를 `<>`로 바꾸고, 언어·테마 두 `Row`를 `<Group title={t("settings.group.display")}>…</Group>`로, 닫기 동작 `Row`를 `<Group title={t("settings.group.window")}>…</Group>`로 감싼다. `Row` 내용은 바꾸지 않는다.

- [ ] **Step 5: DownloadTab 그룹화와 동시 다운로드 세그먼트**

`src/pages/settings/DownloadTab.tsx`: `import { Group } from "./Group";` 추가. `return`을 다음 구조로 바꾼다(각 `Row`의 기존 내용은 그대로 옮기고, 동시 다운로드만 교체):

```tsx
  return (
    <>
      <Group title={t("settings.group.save")}>
        {/* 저장 위치 Row (기존 그대로, 버튼 클래스만 btn-secondary 유지) */}
      </Group>
      <Group title={t("settings.group.defaults")}>
        {/* 기본 화질 Row, 기본 포맷 Row, 기본 자막 Row, 바로 시작 Row (기존 그대로) */}
      </Group>
      <Group title={t("settings.group.performance")}>
        <Row label={t("settings.download.maxConcurrent")}>
          {(id) => (
            <div role="group" aria-labelledby={id} className="join">
              {[1, 2, 3, 4].map((n) => (
                <button
                  key={n}
                  className={`btn btn-sm join-item w-9 ${d.max_concurrent === n ? "btn-primary" : ""}`}
                  aria-pressed={d.max_concurrent === n}
                  onClick={() => update({ download: { max_concurrent: n } })}
                >
                  {n}
                </button>
              ))}
            </div>
          )}
        </Row>
        {/* 하드웨어 가속 Row (기존 그대로) */}
      </Group>
    </>
  );
```

주석 자리에 기존 `Row` JSX를 그대로 옮겨 넣고, 주석은 남기지 않는다.

- [ ] **Step 6: CliTab 그룹화**

`src/pages/settings/CliTab.tsx`: `import { Group } from "./Group";` 추가. 상태 `Row`를 `<Group>…</Group>`(제목 없음)로 감싸고, 최상위 `<div>`를 `<>`로 바꾼다. `<pre>`의 `bg-base-200`을 `surface-card`로 바꾼다(창 배경이 base-200이 되었으므로): `className="surface-card overflow-x-auto p-3 text-xs select-text"`.

- [ ] **Step 7: UpdateTab — 그룹과 업데이트 단계 표시**

`src/pages/settings/UpdateTab.tsx` 전체:

```tsx
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { api } from "../../lib/tauri";
import { useSettings } from "../../lib/settings";
import { useTools } from "../../lib/tools";
import { updatePct, useUpdate } from "../../lib/update";
import { isIdle, useQueue } from "../../lib/queue";
import { showError } from "../../lib/toast";
import { Row } from "./Row";
import { Group } from "./Group";

export function UpdateTab() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const { info, phase, progress, checking, checked, check, install, retry } = useUpdate();
  const idle = useQueue((s) => isIdle(s.jobs));
  const tools = useTools((s) => s.status);
  const [version, setVersion] = useState("");
  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  const v = info?.version;
  const pct = updatePct(progress);
  const appDesc =
    !info || phase === "idle"
      ? checked ? t("settings.update.upToDate") : undefined
      : phase === "downloading"
        ? `${t("update.downloadingBg", { version: v })}${pct !== null ? ` ${pct}%` : ""}`
        : phase === "ready"
          ? t("update.readyTitle", { version: v })
          : phase === "failed"
            ? t("update.downloadFailed")
            : phase === "scheduled"
              ? t("update.scheduled")
              : t("update.installing");
  const last = tools?.last_check ? new Date(tools.last_check).toLocaleString() : t("settings.update.never");

  return (
    <>
      <Group title="kiri">
        <Row label={t("settings.update.appVersion", { version })} desc={appDesc}>
          <div className="flex gap-1.5">
            {phase === "ready" && (
              <button className="btn btn-sm btn-primary" onClick={() => install(!idle)}>
                {idle ? t("update.restartInstall") : t("update.afterQueue")}
              </button>
            )}
            {phase === "failed" && <button className="btn btn-sm btn-secondary" onClick={retry}>{t("update.retry")}</button>}
            <button className="btn btn-sm btn-secondary" disabled={checking} onClick={check}>
              {checking ? t("settings.update.checking") : t("settings.update.check")}
            </button>
          </div>
        </Row>
        <Row label={t("settings.update.autoCheck")}>
          {(id) => <input type="checkbox" aria-labelledby={id} className="toggle toggle-primary toggle-sm" checked={settings!.update.auto_check} onChange={(e) => update({ update: { auto_check: e.target.checked } })} />}
        </Row>
      </Group>
      <Group title="yt-dlp">
        <Row
          label={tools?.ytdlp_version ? t("settings.update.ytdlpVersion", { version: tools.ytdlp_version }) : t("settings.update.ytdlpMissing")}
          desc={t("settings.update.lastCheck", { time: last })}
        >
          <button className="btn btn-sm btn-secondary" aria-label={t("settings.update.ytdlpUpdate")} disabled={tools?.installing} onClick={() => api.updateTools().catch(showError)}>
            {tools?.installing ? <span className="loading loading-spinner loading-xs" /> : t("settings.update.ytdlpUpdate")}
          </button>
        </Row>
      </Group>
      {tools?.error && !tools.installing && <div role="alert" className="alert alert-error mt-2 py-2 text-sm">{tools.error}</div>}
    </>
  );
}
```

- [ ] **Step 8: 남은 옛 키 사용 확인**

Run: `grep -rnE "update\.(available|downloading|install)\"|app\.dropHint" src`
Expected: 출력 없음(`update.installNow`, `update.installMenu`, `update.installing`은 패턴에 걸리지 않는다).

- [ ] **Step 9: 테스트·빌드**

Run: `yarn test && yarn build`
Expected: PASS, 빌드 성공.

- [ ] **Step 10: Commit**

```bash
git add src/pages src/locales
git commit -m "feat(ui): 설정 창 카드 그룹화, 동시 다운로드 세그먼트, 업데이트 단계 표시"
```

---

### Task 9: 전체 확인과 수동 점검

**Files:** 없음(문제가 있으면 해당 태스크 파일 수정)

- [ ] **Step 1: 자동 검사 전체**

Run: `yarn test && yarn build && cargo test --workspace`
Expected: 모두 PASS.

- [ ] **Step 2: 앱 실행 점검**

Run: `yarn tauri dev`

설정에서 테마를 라이트·다크로 바꿔 가며 아래를 확인한다:
1. 메인 창: 로고, 드롭존(클릭 시 클립보드 링크로 시트가 열림, 링크를 끌어 오면 테두리 강조), 작업 카드·칩·진행 바, 빈 상태, 하단 상태 막대 요약.
2. 옵션 시트: 중앙 모달, 썸네일 히어로, 화질 타일, 포맷 칩 3개 + 더보기, 자막 칩, Tab·Space로 선택 가능, Esc로 닫힘.
3. 설정 창: 회색 배경 위 카드 그룹, 동시 다운로드 1~4 세그먼트, 4개 탭 모두.
4. 도구 토스트: 앱을 끄고 `~/Library/Application Support/<번들 id>/` 안의 yt-dlp 바이너리를 지운 뒤(경로는 `src-tauri/src/bootstrap.rs`의 `paths.ytdlp` 참고) 다시 실행 → 오른쪽 아래 '다운로드 준비 중…' 진행 토스트, 준비 끝나면 사라짐. 네트워크를 끊고 반복 → 오류 토스트 + 재시도 버튼. 준비 중에 링크를 붙여넣어 오류 토스트가 진행 토스트와 겹치지 않고 위아래로 쌓이는지 확인.
5. 업데이트: 원격에 새 릴리스가 없으면 `src-tauri/tauri.conf.json`의 `version`을 잠시 낮춰(예: `0.0.1`) 실행 → 10초 뒤 상태 막대 "v… 받는 중… n%" → "새 버전 v… [재시작하여 설치]". 작업을 하나 진행시킨 채 "설치…" → 팝오버 두 버튼. "큐가 끝나면" → 상태 막대 "큐가 끝나면 재시작됩니다". 설정 → 업데이트 탭의 "지금 확인"을 눌러도 다시 받지 않는지(상태가 바로 준비 완료로 유지) 확인. **확인 후 `version`을 원래 값으로 되돌린다.**

- [ ] **Step 3: 마무리 커밋(수정한 게 있을 때만)**

```bash
git add -A src src-tauri
git commit -m "fix(ui): 수동 점검 반영"
```
