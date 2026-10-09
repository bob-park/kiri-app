import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { api } from "../../lib/tauri";
import { useSettings } from "../../lib/settings";
import { useTools } from "../../lib/tools";
import { updatePct, useUpdate } from "../../lib/update";
import { isIdle, useQueue } from "../../lib/queue";
import { showError } from "../../lib/toast";
import { Row } from "./Row";
import { Group } from "./Group";

export function UpdateTab() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const { info, phase, progress, checking, checked, check, install, retry } = useUpdate();
  const idle = useQueue((s) => isIdle(s.jobs));
  const tools = useTools((s) => s.status);
  const [version, setVersion] = useState("");
  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  const v = info?.version;
  const pct = updatePct(progress);
  const appDesc =
    !info || phase === "idle"
      ? checked ? t("settings.update.upToDate") : undefined
      : phase === "downloading"
        ? `${t("update.downloadingBg", { version: v })}${pct !== null ? ` ${pct}%` : ""}`
        : phase === "ready"
          ? t("update.readyTitle", { version: v })
          : phase === "failed"
            ? t("update.downloadFailed")
            : phase === "scheduled"
              ? t("update.scheduled")
              : t("update.installing");
  const last = tools?.last_check ? new Date(tools.last_check).toLocaleString() : t("settings.update.never");

  return (
    <>
      <Group title="kiri">
        <Row label={t("settings.update.appVersion", { version })} desc={appDesc}>
          <div className="flex gap-1.5">
            {phase === "ready" && (
              <button className="btn btn-sm btn-primary" onClick={() => install(!idle)}>
                {idle ? t("update.restartInstall") : t("update.afterQueue")}
              </button>
            )}
            {phase === "failed" && <button className="btn btn-sm btn-secondary" onClick={retry}>{t("update.retry")}</button>}
            <button className="btn btn-sm btn-secondary" disabled={checking} onClick={check}>
              {checking ? t("settings.update.checking") : t("settings.update.check")}
            </button>
          </div>
        </Row>
        <Row label={t("settings.update.autoCheck")}>
          {(id) => <input type="checkbox" aria-labelledby={id} className="toggle toggle-primary toggle-sm" checked={settings!.update.auto_check} onChange={(e) => update({ update: { auto_check: e.target.checked } })} />}
        </Row>
      </Group>
      <Group title="yt-dlp">
        <Row
          label={tools?.ytdlp_version ? t("settings.update.ytdlpVersion", { version: tools.ytdlp_version }) : t("settings.update.ytdlpMissing")}
          desc={t("settings.update.lastCheck", { time: last })}
        >
          <button className="btn btn-sm btn-secondary" aria-label={t("settings.update.ytdlpUpdate")} disabled={tools?.installing} onClick={() => api.updateTools().catch(showError)}>
            {tools?.installing ? <span className="loading loading-spinner loading-xs" /> : t("settings.update.ytdlpUpdate")}
          </button>
        </Row>
      </Group>
      {tools?.error && !tools.installing && <div role="alert" className="alert alert-error mt-2 py-2 text-sm">{tools.error}</div>}
    </>
  );
}
