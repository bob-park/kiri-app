import { useEffect } from 'react';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { getToolsStatus, updateTools } from '@/domain/tools/apis/tools';
import type { ToolsStatus } from '@/domain/tools/apis/tools.dto';
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
    // The listener below lives only while a reader is mounted (e.g. Settings → Update), so re-read on mount.
    refetchOnMount: 'always',
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

export function useToolsUpdate() {
  const { mutate, isPending } = useMutation<void, AppError, void>({
    mutationKey: ['tools', 'status', 'update'],
    mutationFn: updateTools,
  });

  return { updateTools: mutate, isLoading: isPending };
}
