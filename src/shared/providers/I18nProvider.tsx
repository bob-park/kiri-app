import { createContext, useEffect } from 'react';

import { useSettings } from '@/domain/settings/queries/settings';
import { type Dict, type Locale, dicts, resolveLocale } from '@/shared/i18n';

export const I18nContext = createContext<{ t: Dict; locale: Locale }>({ t: dicts.en, locale: 'en' });

/** Resolves `settings.general.ui_language` and provides the dictionary. Read it with `useI18n()`. */
export default function I18nProvider({ children }: Readonly<{ children: React.ReactNode }>) {
  // queries
  const { settings } = useSettings();

  const locale = resolveLocale(settings?.general.ui_language ?? 'system', navigator.language);

  // useEffect
  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  return <I18nContext.Provider value={{ t: dicts[locale], locale }}>{children}</I18nContext.Provider>;
}
