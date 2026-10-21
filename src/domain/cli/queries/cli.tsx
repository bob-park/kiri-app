import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { getCliStatus, installCli, uninstallCli } from '@/domain/cli/apis/cli';
import type { CliStatus } from '@/domain/cli/apis/cli.dto';
import type { AppError } from '@/shared/api/common.dto';

export const CLI_QUERY_KEY = ['cli', 'status'];

export function useCliStatus() {
  const { data } = useQuery<CliStatus>({
    queryKey: CLI_QUERY_KEY,
    queryFn: getCliStatus,
    // No event reports changes made outside this tab (e.g. a manual `rm`), so re-read on every visit.
    refetchOnMount: 'always',
  });

  return { status: data ?? null };
}

export function useCliInstall() {
  const queryClient = useQueryClient();

  const { mutate, isPending } = useMutation<void, AppError, void>({
    mutationKey: ['cli', 'status', 'install'],
    mutationFn: installCli,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: CLI_QUERY_KEY }),
  });

  return { installCli: mutate, isLoading: isPending };
}

export function useCliUninstall() {
  const queryClient = useQueryClient();

  const { mutate, isPending } = useMutation<void, AppError, void>({
    mutationKey: ['cli', 'status', 'uninstall'],
    mutationFn: uninstallCli,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: CLI_QUERY_KEY }),
  });

  return { uninstallCli: mutate, isLoading: isPending };
}
