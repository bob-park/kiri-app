import { useEffect } from 'react';

import { useQueryClient } from '@tanstack/react-query';

import type { Settings, ThemePref } from '@/domain/settings/apis/settings.dto';
import { SETTINGS_QUERY_KEY, settingsFromEcho, useSettings } from '@/domain/settings/queries/settings';
import { resolveTheme } from '@/utils/theme';

import { listen } from '@tauri-apps/api/event';

const DARK_QUERY = '(prefers-color-scheme: dark)';

export function applyTheme(pref: ThemePref) {
  document.documentElement.dataset.theme = resolveTheme(pref, window.matchMedia(DARK_QUERY).matches);
}

export default function ThemeProvider({ children }: Readonly<{ children: React.ReactNode }>) {
  // hooks
  const queryClient = useQueryClient();

  // queries
  const { settings } = useSettings();

  const theme = settings.general.theme;

  // useEffect
  useEffect(() => {
    applyTheme(theme);

    if (theme !== 'system') {
      return;
    }

    const media = window.matchMedia(DARK_QUERY);
    const handleChange = () => applyTheme(theme);
    media.addEventListener('change', handleChange);

    return () => media.removeEventListener('change', handleChange);
  }, [theme]);

  useEffect(() => {
    // Settings changed (here or in another window). This one listener keeps the window's whole
    // `['settings']` cache current, so I18nProvider and every `useSettings()` reader follow too.
    // Patches still in flight from this window are laid over the echo (see useSettingsPatch).
    const unlisten = listen<Settings>('settings://changed', (e) =>
      queryClient.setQueryData(SETTINGS_QUERY_KEY, settingsFromEcho(e.payload)),
    );

    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [queryClient]);

  return children;
}
