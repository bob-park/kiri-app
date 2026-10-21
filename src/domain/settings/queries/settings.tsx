import { useQuery } from '@tanstack/react-query';

import { getSettings } from '@/domain/settings/apis/settings';
import type { Settings } from '@/domain/settings/apis/settings.dto';

export const SETTINGS_QUERY_KEY = ['settings'];

export function useSettings() {
  const { data, isLoading } = useQuery<Settings>({
    queryKey: SETTINGS_QUERY_KEY,
    queryFn: getSettings,
  });

  return { settings: data, isLoading };
}
