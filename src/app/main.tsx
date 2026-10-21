import { StrictMode } from 'react';

import App from '@/app/App';
import '@/app/globals.css';
import SettingsApp from '@/app/settings/SettingsApp';
import { getSettings } from '@/domain/settings/apis/settings';
import { SETTINGS_QUERY_KEY } from '@/domain/settings/queries/settings';
import type { AppError } from '@/shared/api/common.dto';
import Toasts from '@/shared/components/Toasts';
import RQProvider, { queryClient } from '@/shared/components/queries/RQProvider';
import I18nProvider from '@/shared/providers/I18nProvider';
import ThemeProvider, { applyTheme } from '@/shared/providers/ThemeProvider';
import { useStore } from '@/shared/store/rootStore';
import { DEFAULT_SETTINGS } from '@/utils/settings';

import { getCurrentWindow } from '@tauri-apps/api/window';
import { createRoot } from 'react-dom/client';

const label = getCurrentWindow().label;

// Seed settings (theme + language) before the first paint, so neither flashes.
async function seedSettings() {
  try {
    return await getSettings();
  } catch (e) {
    useStore.getState().showError(e as AppError);

    return DEFAULT_SETTINGS;
  }
}

function render() {
  const root = document.getElementById('root');
  if (!root) {
    return;
  }

  createRoot(root).render(
    <StrictMode>
      <RQProvider>
        <ThemeProvider>
          <I18nProvider>
            {label === 'settings' ? <SettingsApp /> : <App />}
            <Toasts bottom={label === 'main' ? 44 : 16} />
          </I18nProvider>
        </ThemeProvider>
      </RQProvider>
    </StrictMode>,
  );
}

void seedSettings().then((settings) => {
  queryClient.setQueryData(SETTINGS_QUERY_KEY, settings);
  applyTheme(settings.general.theme);
  render();
});
