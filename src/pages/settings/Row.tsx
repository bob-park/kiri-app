import type { ReactNode } from "react";

export function Row({ label, desc, children }: { label: string; desc?: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-base-300 py-2.5 last:border-none">
      <div className="min-w-0">
        <div className="text-sm font-medium">{label}</div>
        {desc && <div className="mt-0.5 text-xs text-fg-muted">{desc}</div>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}
