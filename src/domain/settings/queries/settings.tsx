import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { getSettings, patchSettings } from '@/domain/settings/apis/settings';
import type { Settings, SettingsPatch } from '@/domain/settings/apis/settings.dto';
import type { AppError } from '@/shared/api/common.dto';
import { DEFAULT_SETTINGS, createPendingPatches, mergeSettings } from '@/utils/settings';

export const SETTINGS_QUERY_KEY = ['settings'];

// One per window (each window is its own JS context), shared by the mutation and the event listener.
const pendingPatches = createPendingPatches();

/** For the `settings://changed` listener: the echo with any still-pending optimistic patches laid over it. */
export function settingsFromEcho(echo: Settings) {
  return pendingPatches.overlay(echo);
}

export function useSettings() {
  const { data, isLoading } = useQuery<Settings>({
    queryKey: SETTINGS_QUERY_KEY,
    queryFn: getSettings,
    // `settings://changed` keeps the cache current (see ThemeProvider).
    staleTime: Infinity,
  });

  // main.tsx seeds the cache before the first render, so the fallback only covers a failed startup read.
  return { settings: data ?? DEFAULT_SETTINGS, isLoading };
}

export function useSettingsPatch() {
  const queryClient = useQueryClient();

  const { mutate, isPending } = useMutation<Settings, AppError, SettingsPatch, { prev?: Settings }>({
    mutationKey: ['settings', 'patch'],
    mutationFn: patchSettings,
    onMutate: async (patch) => {
      await queryClient.cancelQueries({ queryKey: SETTINGS_QUERY_KEY });
      const prev = queryClient.getQueryData<Settings>(SETTINGS_QUERY_KEY);
      queryClient.setQueryData(SETTINGS_QUERY_KEY, mergeSettings(prev ?? DEFAULT_SETTINGS, patch));
      pendingPatches.begin(patch);

      return { prev };
    },
    onError: (_err, _patch, context) => {
      queryClient.setQueryData(SETTINGS_QUERY_KEY, context?.prev);
    },
    onSettled: () => pendingPatches.end(),
  });

  return { patchSettings: mutate, isLoading: isPending };
}
