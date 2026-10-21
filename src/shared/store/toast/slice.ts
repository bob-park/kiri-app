import type { BoundState } from '@/shared/store/rootStore';
import type { ToastState } from '@/shared/store/toast/toast.state';
import { TOAST_MS, upsertToast } from '@/utils/toast';

import type { SlicePattern } from 'zustand';

let seq = 0;
const timers = new Map<number, ReturnType<typeof setTimeout>>();

function clearTimer(id: number) {
  clearTimeout(timers.get(id));
  timers.delete(id);
}

const createToastSlice: SlicePattern<ToastState, BoundState> = (set, get) => ({
  toasts: [],
  pushToast: (input) => {
    const { id, toasts } = upsertToast(get().toasts, input, ++seq);
    // A replaced toast may have been auto-dismissing; its old timer must not close the new content.
    clearTimer(id);
    set({ toasts }, false, 'toast/pushToast');
    if (!input.sticky) {
      timers.set(
        id,
        setTimeout(() => get().dismissToast(id), TOAST_MS),
      );
    }

    return id;
  },
  dismissToast: (id) => {
    clearTimer(id);
    set({ toasts: get().toasts.filter((x) => x.id !== id) }, false, 'toast/dismissToast');
  },
  dismissToastByKey: (key) => {
    const toast = get().toasts.find((x) => x.key === key);
    if (toast) {
      get().dismissToast(toast.id);
    }
  },
});

export default createToastSlice;
