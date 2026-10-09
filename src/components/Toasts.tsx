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
