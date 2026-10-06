import { useId, type ReactNode } from "react";

/** children 이 함수면 라벨 id 를 받아 컨트롤의 aria-labelledby 로 쓴다. */
export function Row({ label, desc, children }: { label: string; desc?: string; children: ReactNode | ((labelId: string) => ReactNode) }) {
  const id = useId();
  return (
    <div className="flex items-center justify-between gap-4 border-b border-base-300 py-2.5 last:border-none">
      <div className="min-w-0">
        <div id={id} className="text-sm font-medium">{label}</div>
        {desc && <div className="mt-0.5 text-xs text-fg-muted">{desc}</div>}
      </div>
      <div className="shrink-0">{typeof children === "function" ? children(id) : children}</div>
    </div>
  );
}
