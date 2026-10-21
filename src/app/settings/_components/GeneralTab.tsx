import ThemePicker from '@/app/settings/_components/ThemePicker';
import type { LocalePref } from '@/domain/settings/apis/settings.dto';
import { useSettings, useSettingsPatch } from '@/domain/settings/queries/settings';
import SettingRow from '@/shared/components/settings/SettingRow';
import SettingSection from '@/shared/components/settings/SettingSection';
import Switch from '@/shared/components/settings/Switch';
import useI18n from '@/shared/hooks/useI18n';

const LOCALES: LocalePref[] = ['system', 'ko', 'en'];

export default function GeneralTab() {
  // hooks
  const { t } = useI18n();

  // queries
  const { settings } = useSettings();
  const { patchSettings } = useSettingsPatch();

  const general = settings.general;
  const localeLabels = {
    system: t.settings.general.langSystem,
    ko: t.settings.general.langKo,
    en: t.settings.general.langEn,
  };
  const themeLabels = {
    system: t.settings.general.themeSystem,
    light: t.settings.general.themeLight,
    dark: t.settings.general.themeDark,
  };

  return (
    <>
      <SettingSection title={t.settings.group.display}>
        <SettingRow label={t.settings.general.language}>
          {(labelId) => (
            <select
              aria-labelledby={labelId}
              className="select select-sm w-40"
              value={general.ui_language}
              onChange={(e) => patchSettings({ general: { ui_language: e.target.value as LocalePref } })}
            >
              {LOCALES.map((locale) => (
                <option key={locale} value={locale}>
                  {localeLabels[locale]}
                </option>
              ))}
            </select>
          )}
        </SettingRow>
        <SettingRow label={t.settings.general.theme} stacked>
          {(labelId) => (
            <ThemePicker
              labelledBy={labelId}
              value={general.theme}
              labels={themeLabels}
              onChange={(theme) => patchSettings({ general: { theme } })}
            />
          )}
        </SettingRow>
      </SettingSection>
      <SettingSection title={t.settings.group.window}>
        <SettingRow label={t.settings.general.closeToTray} desc={t.settings.general.closeToTrayDesc}>
          {(labelId) => (
            <Switch
              labelledBy={labelId}
              checked={general.close_to_tray}
              onChange={(close_to_tray) => patchSettings({ general: { close_to_tray } })}
            />
          )}
        </SettingRow>
      </SettingSection>
    </>
  );
}
