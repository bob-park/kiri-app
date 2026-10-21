import { useEffect } from 'react';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { addJob, addUrl, clearCompleted, listJobs, removeJob, restartJob, stopJob } from '@/domain/queue/apis/queue';
import type { Job, NewJob } from '@/domain/queue/apis/queue.dto';
import type { AppError } from '@/shared/api/common.dto';

import { listen } from '@tauri-apps/api/event';

export const QUEUE_QUERY_KEY = ['queue', 'jobs'];

/** The whole queue. Mutations need no cache writes: Rust emits `queue://changed` after every change. */
export function useJobs() {
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery<Job[]>({
    queryKey: QUEUE_QUERY_KEY,
    queryFn: listJobs,
    // The listener below lives only while a reader is mounted (e.g. Settings → Update), so re-read on mount.
    refetchOnMount: 'always',
  });

  useEffect(() => {
    const unlisten = listen<Job[]>('queue://changed', (e) => queryClient.setQueryData(QUEUE_QUERY_KEY, e.payload));

    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [queryClient]);

  return { jobs: data ?? [], isLoading };
}

export function useJobAdd() {
  const { mutate, isPending } = useMutation<Job, AppError, NewJob>({
    mutationKey: ['queue', 'jobs', 'add'],
    mutationFn: addJob,
    // The options sheet shows this error inside the modal, where toasts can't be reached.
    meta: { silent: true },
  });

  return { addJob: mutate, isLoading: isPending };
}

export function useUrlAdd() {
  const { mutateAsync, isPending } = useMutation<Job, AppError, string>({
    mutationKey: ['queue', 'jobs', 'addUrl'],
    mutationFn: addUrl,
  });

  return { addUrl: mutateAsync, isLoading: isPending };
}

export function useJobStop() {
  const { mutate, isPending } = useMutation<void, AppError, number>({
    mutationKey: ['queue', 'jobs', 'stop'],
    mutationFn: stopJob,
  });

  return { stopJob: mutate, isLoading: isPending };
}

export function useJobRemove() {
  const { mutate, isPending } = useMutation<void, AppError, number>({
    mutationKey: ['queue', 'jobs', 'remove'],
    mutationFn: removeJob,
  });

  return { removeJob: mutate, isLoading: isPending };
}

export function useJobRestart() {
  const { mutate, isPending } = useMutation<void, AppError, number>({
    mutationKey: ['queue', 'jobs', 'restart'],
    mutationFn: restartJob,
  });

  return { restartJob: mutate, isLoading: isPending };
}

export function useCompletedClear() {
  const { mutate, isPending } = useMutation<number, AppError, void>({
    mutationKey: ['queue', 'jobs', 'clearCompleted'],
    mutationFn: clearCompleted,
  });

  return { clearCompleted: mutate, isLoading: isPending };
}
