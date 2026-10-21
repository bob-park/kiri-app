/** The first http(s) URL in pasted text. The backend decides whether it is YouTube. */
export function extractUrl(text: string | null | undefined): string | null {
  const m = text?.match(/https?:\/\/\S+/);

  return m ? m[0] : null;
}

export function isEditableTarget(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;

  return (
    !!el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT' || el.isContentEditable)
  );
}
