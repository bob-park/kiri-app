import type { ErrorState } from '@/shared/store/error/error.state';
import createErrorSlice from '@/shared/store/error/slice';
import createToastSlice from '@/shared/store/toast/slice';
import type { ToastState } from '@/shared/store/toast/toast.state';

import { create } from 'zustand';
import { devtools } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';

// Register slices here: spread `createXxxSlice(...a)` and add `XxxState` to BoundState.
export const useStore = create<BoundState>()(
  devtools(
    immer((...a) => ({
      ...createErrorSlice(...a),
      ...createToastSlice(...a),
    })),
    { name: 'kiri', enabled: import.meta.env.DEV },
  ),
);

export type BoundState = ErrorState & ToastState;
