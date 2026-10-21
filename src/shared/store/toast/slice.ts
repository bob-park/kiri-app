import type { BoundState } from '@/shared/store/rootStore';
import type { ToastState } from '@/shared/store/toast/toast.state';
import { createToastTimers, upsertToast } from '@/utils/toast';

import type { SlicePattern } from 'zustand';

let seq = 0;

const createToastSlice: SlicePattern<ToastState, BoundState> = (set, get) => {
  const timers = createToastTimers((id) => get().dismissToast(id));

  return {
    toasts: [],
    pushToast: (input) => {
      const { id, toasts } = upsertToast(get().toasts, input, ++seq);
      set({ toasts }, false, 'toast/pushToast');
      timers.schedule(id, input.sticky);

      return id;
    },
    dismissToast: (id) => {
      timers.clear(id);
      set({ toasts: get().toasts.filter((x) => x.id !== id) }, false, 'toast/dismissToast');
    },
    dismissToastByKey: (key) => {
      const toast = get().toasts.find((x) => x.key === key);
      if (toast) {
        get().dismissToast(toast.id);
      }
    },
  };
};

export default createToastSlice;
