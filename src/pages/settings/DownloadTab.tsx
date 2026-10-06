import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { useSettings } from "../../lib/settings";
import { showError } from "../../lib/toast";
import { PRESETS, type Preset } from "../../lib/types";
import { Row } from "./Row";

const QUALITIES = ["best", "2160p", "1440p", "1080p", "720p", "480p", "audio"];

export function DownloadTab() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const d = settings!.download;
  const [subs, setSubs] = useState(d.subtitles.join(","));
  useEffect(() => setSubs(d.subtitles.join(",")), [d.subtitles.join(",")]);

  const pickDir = async () => {
    try {
      const dir = await open({ directory: true, defaultPath: d.dir });
      if (typeof dir === "string") await update({ download: { dir } });
    } catch (e) {
      showError(e);
    }
  };
  const commitSubs = () => update({ download: { subtitles: subs.split(",").map((s) => s.trim()).filter(Boolean) } });
  const qLabel = (q: string) => (q === "best" ? t("quality.best") : q === "audio" ? t("quality.audio") : q);

  return (
    <div>
      <Row label={t("settings.download.dir")} desc={d.dir}>
        <button className="btn btn-sm btn-secondary" onClick={pickDir}>{t("settings.download.change")}</button>
      </Row>
      <Row label={t("settings.download.quality")}>
        <select className="select select-sm w-40" value={d.quality} onChange={(e) => update({ download: { quality: e.target.value } })}>
          {QUALITIES.map((q) => (
            <option key={q} value={q}>{qLabel(q)}</option>
          ))}
        </select>
      </Row>
      <Row label={t("settings.download.preset")}>
        <select className="select select-sm w-40" value={d.preset} onChange={(e) => update({ download: { preset: e.target.value as Preset } })}>
          {PRESETS.map((p) => (
            <option key={p} value={p}>{t(`preset.${p}`)}</option>
          ))}
        </select>
      </Row>
      <Row label={t("settings.download.subtitles")} desc={t("settings.download.subtitlesHint")}>
        <input className="input input-sm w-40" value={subs} onChange={(e) => setSubs(e.target.value)} onBlur={commitSubs} onKeyDown={(e) => e.key === "Enter" && commitSubs()} />
      </Row>
      <Row label={t("settings.download.skipSheet")}>
        <input type="checkbox" className="toggle toggle-primary toggle-sm" checked={d.skip_sheet} onChange={(e) => update({ download: { skip_sheet: e.target.checked } })} />
      </Row>
      <Row label={t("settings.download.maxConcurrent")}>
        <select className="select select-sm w-20" value={d.max_concurrent} onChange={(e) => update({ download: { max_concurrent: Number(e.target.value) } })}>
          {[1, 2, 3, 4].map((n) => (
            <option key={n} value={n}>{n}</option>
          ))}
        </select>
      </Row>
      <Row label={t("settings.download.hwAccel")} desc={t("settings.download.hwAccelDesc")}>
        <input type="checkbox" className="toggle toggle-primary toggle-sm" checked={d.hw_accel} onChange={(e) => update({ download: { hw_accel: e.target.checked } })} />
      </Row>
    </div>
  );
}
