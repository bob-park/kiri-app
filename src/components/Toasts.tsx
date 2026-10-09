import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useToasts, withLeaving, type ShownToast } from "../lib/toast";

const EXIT_MS = 150;

function ToastCard({ x, dismiss }: { x: ShownToast; dismiss: (id: number) => void }) {
  const { t } = useTranslation();
  // 액션이 없는 오류·정보 토스트만 눌러서 닫는다. 진행 토스트는 상태가 바뀔 때까지 남는다.
  const clickToClose = !x.action && x.kind !== "progress" && !x.leaving;
  return (
    <div
      role={x.leaving ? undefined : x.kind === "progress" ? "status" : "alert"}
      aria-hidden={x.leaving || undefined}
      className={`${x.leaving ? "animate-toast-out pointer-events-none" : "animate-toast-in"} float-shadow rounded-xl bg-toast px-3 py-2.5 text-[12px] text-toast-content ${clickToClose ? "cursor-pointer" : ""}`}
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
  const [shown, setShown] = useState<ShownToast[]>([]);
  const ref = useRef<HTMLDivElement>(null);

  // 닫힌 토스트는 퇴장 애니메이션이 끝날 때까지 남겼다가 지운다.
  useEffect(() => {
    setShown((cur) => withLeaving(cur, toasts));
    const timer = setTimeout(() => setShown((cur) => cur.filter((s) => !s.leaving)), EXIT_MS);
    return () => clearTimeout(timer);
  }, [toasts]);

  // 모달 <dialog>(top layer) 위에 보이도록 popover로 띄운다.
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const open = el.matches(":popover-open");
    if (shown.length === 0 && open) el.hidePopover();
    else if (shown.length > 0 && !open) el.showPopover();
  }, [shown]);

  // 새 토스트가 생길 때만 다시 열어 top layer 맨 위(열린 모달 위)로 올린다.
  // 내용만 바뀌거나 닫힐 때 다시 열면 남은 토스트의 등장 애니메이션이 다시 재생된다.
  const liveCount = useRef(0);
  useEffect(() => {
    const el = ref.current;
    if (el && toasts.length > liveCount.current && el.matches(":popover-open")) {
      el.hidePopover();
      el.showPopover();
    }
    liveCount.current = toasts.length;
  }, [toasts]);

  return (
    <div
      ref={ref}
      popover="manual"
      style={{ inset: "auto", right: 16, bottom }}
      className="fixed m-0 flex w-[260px] flex-col gap-2 overflow-visible border-0 bg-transparent p-0"
    >
      {shown.map((x) => (
        <ToastCard key={x.id} x={x} dismiss={dismiss} />
      ))}
    </div>
  );
}
