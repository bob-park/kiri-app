import { useEffect, useRef } from "react";
import { useToasts } from "../lib/toast";

export function Toasts() {
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
      className="toast toast-end toast-bottom z-50 m-0 overflow-visible border-0 bg-transparent p-0"
    >
      {toasts.map((x) => (
        <div
          key={x.id}
          role="alert"
          className={`alert ${x.kind === "error" ? "alert-error" : "alert-info"} cursor-pointer py-2 text-sm`}
          onClick={() => dismiss(x.id)}
        >
          {x.text}
        </div>
      ))}
    </div>
  );
}
