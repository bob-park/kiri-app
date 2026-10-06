/** 붙여넣거나 끌어다 놓은 텍스트에서 첫 http(s) URL. YouTube 여부는 백엔드가 판정한다. */
export function extractUrl(text: string | null | undefined): string | null {
  const m = text?.match(/https?:\/\/\S+/);
  return m ? m[0] : null;
}

export function isEditableTarget(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  return !!el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT" || el.isContentEditable);
}
