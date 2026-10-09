import { useTranslation } from "react-i18next";
import { useSettings } from "../../lib/settings";
import type { Theme, UiLang } from "../../lib/types";
import { Row } from "./Row";
import { Group } from "./Group";

const LANGS: [UiLang, string][] = [
  ["system", "settings.general.langSystem"],
  ["ko", "settings.general.langKo"],
  ["en", "settings.general.langEn"],
  ["ja", "settings.general.langJa"],
];
const THEMES: [Theme, string][] = [
  ["system", "settings.general.themeSystem"],
  ["light", "settings.general.themeLight"],
  ["dark", "settings.general.themeDark"],
];

export function GeneralTab() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const g = settings!.general;
  return (
    <>
      <Group title={t("settings.group.display")}>
        <Row label={t("settings.general.language")}>
          {(id) => (
            <select aria-labelledby={id} className="select select-sm w-40" value={g.ui_language} onChange={(e) => update({ general: { ui_language: e.target.value as UiLang } })}>
              {LANGS.map(([v, k]) => (
                <option key={v} value={v}>{t(k)}</option>
              ))}
            </select>
          )}
        </Row>
        <Row label={t("settings.general.theme")}>
          {(id) => (
            <div role="group" aria-labelledby={id} className="join">
              {THEMES.map(([v, k]) => (
                <button key={v} className={`btn btn-sm join-item ${g.theme === v ? "btn-primary" : ""}`} aria-pressed={g.theme === v} onClick={() => update({ general: { theme: v } })}>
                  {t(k)}
                </button>
              ))}
            </div>
          )}
        </Row>
      </Group>
      <Group title={t("settings.group.window")}>
        <Row label={t("settings.general.closeToTray")} desc={t("settings.general.closeToTrayDesc")}>
          {(id) => <input type="checkbox" aria-labelledby={id} className="toggle toggle-primary toggle-sm" checked={g.close_to_tray} onChange={(e) => update({ general: { close_to_tray: e.target.checked } })} />}
        </Row>
      </Group>
    </>
  );
}
