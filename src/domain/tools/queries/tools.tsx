import { useEffect } from 'react';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { checkToolUpdates, getToolsStatus, installTools, updateTools } from '@/domain/tools/apis/tools';
import type { ToolName, ToolsStatus } from '@/domain/tools/apis/tools.dto';
import type { AppError } from '@/shared/api/common.dto';

import { listen } from '@tauri-apps/api/event';

export const TOOLS_QUERY_KEY = ['tools', 'status'];

export function useToolsStatus() {
  const queryClient = useQueryClient();

  const { data } = useQuery<ToolsStatus>({
    queryKey: TOOLS_QUERY_KEY,
    queryFn: async () => {
      const before = queryClient.getQueryData<ToolsStatus>(TOOLS_QUERY_KEY);
      const status = await getToolsStatus();
      const after = queryClient.getQueryData<ToolsStatus>(TOOLS_QUERY_KEY);

      // `tools://changed` arrived while the command ran: that value is newer.
      return after !== before && after ? after : status;
    },
    // The listener below keeps the cache current. Mount this hook once per window, for the window's
    // whole life (App, StatusBar, SettingsApp), so no event is missed.
    staleTime: Infinity,
  });

  useEffect(() => {
    const unlisten = listen<ToolsStatus>('tools://changed', (e) =>
      queryClient.setQueryData(TOOLS_QUERY_KEY, e.payload),
    );

    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [queryClient]);

  return { status: data ?? null };
}

export function useToolsInstall() {
  const { mutate, isPending } = useMutation<void, AppError, void>({
    mutationKey: ['tools', 'status', 'install'],
    mutationFn: installTools,
  });

  return { installTools: mutate, isLoading: isPending };
}

/**
 * Checks every tool for updates on mount, and again whenever an install ends: a check made
 * during an install hits the busy guard and is dropped. A failed check is silent.
 */
export function useToolUpdateCheck(installing: ToolName | null | undefined) {
  const busy = !!installing;

  const { mutate, isPending } = useMutation<void, AppError, void>({
    mutationKey: ['tools', 'status', 'check'],
    mutationFn: checkToolUpdates,
    meta: { silent: true },
  });

  useEffect(() => {
    if (!busy) {
      mutate();
    }
  }, [busy, mutate]);

  // The check holds the busy guard, so an update started meanwhile would be dropped.
  return { checking: isPending };
}

export function useToolsUpdate() {
  const { mutate, isPending } = useMutation<void, AppError, ToolName[]>({
    mutationKey: ['tools', 'status', 'update'],
    mutationFn: updateTools,
  });

  return { updateTools: mutate, isLoading: isPending };
}
