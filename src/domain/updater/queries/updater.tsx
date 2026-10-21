import { useEffect } from 'react';

import { type QueryClient, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { checkUpdate, getUpdateStatus, installUpdate, retryUpdateDownload } from '@/domain/updater/apis/updater';
import type { UpdateInfo, UpdateStatus } from '@/domain/updater/apis/updater.dto';
import type { AppError } from '@/shared/api/common.dto';
import { useStore } from '@/shared/store/rootStore';
import {
  INITIAL_UPDATE_VIEW,
  type UpdateEvent,
  type UpdateView,
  applyFetchedUpdate,
  applyUpdateStatus,
  foundUpdate,
  isBusy,
  stepUpdate,
} from '@/utils/update';

import { listen } from '@tauri-apps/api/event';

export const UPDATER_QUERY_KEY = ['updater', 'status'];

function updateView(queryClient: QueryClient, change: (v: UpdateView) => UpdateView) {
  queryClient.setQueryData<UpdateView>(UPDATER_QUERY_KEY, (v) => change(v ?? INITIAL_UPDATE_VIEW));
}

const step = (queryClient: QueryClient, e: UpdateEvent) => updateView(queryClient, (v) => stepUpdate(v, e));

/** The app update as this window sees it: `update://status` events plus the user's own install choices. */
export function useUpdateStatus() {
  const queryClient = useQueryClient();

  const { data } = useQuery<UpdateView>({
    queryKey: UPDATER_QUERY_KEY,
    queryFn: async () => {
      const info = await getUpdateStatus();

      return applyFetchedUpdate(queryClient.getQueryData<UpdateView>(UPDATER_QUERY_KEY) ?? INITIAL_UPDATE_VIEW, info);
    },
    // The listener below lives only while a reader is mounted (e.g. Settings → Update), so re-read on mount.
    refetchOnMount: 'always',
  });

  useEffect(() => {
    const unlisten = listen<UpdateStatus>('update://status', ({ payload }) => {
      updateView(queryClient, (v) => applyUpdateStatus(v, payload));
      if (payload.kind === 'error' && !isBusy(payload.error)) {
        useStore.getState().showError({ code: 'UNKNOWN', message: payload.error });
      }
    });

    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [queryClient]);

  return data ?? INITIAL_UPDATE_VIEW;
}

export function useUpdateCheck() {
  const queryClient = useQueryClient();

  const { mutate, isPending } = useMutation<UpdateInfo | null, AppError, void>({
    mutationKey: ['updater', 'status', 'check'],
    mutationFn: checkUpdate,
    onSuccess: (info) => updateView(queryClient, (v) => ({ ...(info ? foundUpdate(v, info) : v), checked: true })),
  });

  return { checkUpdate: mutate, isLoading: isPending };
}

export function useUpdateInstall() {
  const queryClient = useQueryClient();

  const { mutate, isPending } = useMutation<void, AppError, boolean>({
    mutationKey: ['updater', 'status', 'install'],
    mutationFn: installUpdate,
    // Shown below, except for a second request while one runs ("busy").
    meta: { silent: true },
    onMutate: (afterQueue) => {
      if (!afterQueue) {
        step(queryClient, { type: 'install' });
      }
    },
    onError: (err) => {
      if (isBusy(err.message)) {
        return;
      }
      updateView(queryClient, (v) => stepUpdate(v, { type: 'installFailed', ready: v.info?.ready ?? false }));
      useStore.getState().showError(err);
    },
  });

  return { installUpdate: mutate, isLoading: isPending };
}

export function useUpdateDownloadRetry() {
  const queryClient = useQueryClient();

  const { mutate, isPending } = useMutation<void, AppError, void>({
    mutationKey: ['updater', 'status', 'retry'],
    mutationFn: retryUpdateDownload,
    onMutate: () => step(queryClient, { type: 'retry' }),
  });

  return { retryUpdateDownload: mutate, isLoading: isPending };
}
