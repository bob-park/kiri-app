import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { api } from "../../lib/tauri";
import { useSettings } from "../../lib/settings";
import { useTools } from "../../lib/tools";
import { useUpdate } from "../../lib/update";
import { showError } from "../../lib/toast";
import { Row } from "./Row";

export function UpdateTab() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const { info, checking, checked, check } = useUpdate();
  const tools = useTools((s) => s.status);
  const [version, setVersion] = useState("");
  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  const appDesc = info ? t("update.available", { version: info.version }) : checked ? t("settings.update.upToDate") : undefined;
  const last = tools?.last_check ? new Date(tools.last_check).toLocaleString() : t("settings.update.never");

  return (
    <div>
      <Row label={t("settings.update.appVersion", { version })} desc={appDesc}>
        <button className="btn btn-sm btn-secondary" disabled={checking} onClick={check}>
          {checking ? t("settings.update.checking") : t("settings.update.check")}
        </button>
      </Row>
      <Row label={t("settings.update.autoCheck")}>
        {(id) => <input type="checkbox" aria-labelledby={id} className="toggle toggle-primary toggle-sm" checked={settings!.update.auto_check} onChange={(e) => update({ update: { auto_check: e.target.checked } })} />}
      </Row>
      <Row
        label={tools?.ytdlp_version ? t("settings.update.ytdlpVersion", { version: tools.ytdlp_version }) : t("settings.update.ytdlpMissing")}
        desc={t("settings.update.lastCheck", { time: last })}
      >
        <button className="btn btn-sm btn-secondary" aria-label={t("settings.update.ytdlpUpdate")} disabled={tools?.installing} onClick={() => api.updateTools().catch(showError)}>
          {tools?.installing ? <span className="loading loading-spinner loading-xs" /> : t("settings.update.ytdlpUpdate")}
        </button>
      </Row>
      {tools?.error && !tools.installing && <div role="alert" className="alert alert-error mt-2 py-2 text-sm">{tools.error}</div>}
    </div>
  );
}
