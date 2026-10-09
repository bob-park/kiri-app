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
