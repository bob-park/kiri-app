// Type-only import: erased at runtime, so node --test needs no alias resolution.
import type { ShownToast, Toast, ToastInput } from '@/shared/store/toast/toast.state';

/** Adds `input`, or replaces the toast with the same `key` in place (keeping its id). */
export function upsertToast(toasts: Toast[], input: ToastInput, nextId: number): { id: number; toasts: Toast[] } {
  const prev = input.key ? toasts.find((x) => x.key === input.key) : undefined;
  const id = prev?.id ?? nextId;
  const toast = { ...input, id };

  return { id, toasts: prev ? toasts.map((x) => (x.id === id ? toast : x)) : [...toasts, toast] };
}

/** The list to render: toasts gone from the store stay in place as `leaving` during the exit animation. */
export function withLeaving(shown: ShownToast[], live: Toast[]): ShownToast[] {
  const byId = new Map(live.map((x) => [x.id, x]));
  const kept = shown.map((s) => byId.get(s.id) ?? (s.leaving ? s : { ...s, leaving: true }));

  return [...kept, ...live.filter((x) => !shown.some((s) => s.id === x.id))];
}

/** How long a non-sticky toast stays up. */
export const TOAST_MS = 5000;

/** Auto-dismiss bookkeeping for the toast slice. `schedule` replaces any earlier timer for the id. */
export function createToastTimers(onExpire: (id: number) => void, ms = TOAST_MS) {
  const timers = new Map<number, ReturnType<typeof setTimeout>>();

  const clear = (id: number) => {
    clearTimeout(timers.get(id));
    timers.delete(id);
  };

  return {
    clear,
    schedule(id: number, sticky: boolean | undefined) {
      // A replaced toast may have been auto-dismissing; its old timer must not close the new content.
      clear(id);
      if (!sticky) {
        timers.set(
          id,
          setTimeout(() => {
            timers.delete(id);
            onExpire(id);
          }, ms),
        );
      }
    },
  };
}
