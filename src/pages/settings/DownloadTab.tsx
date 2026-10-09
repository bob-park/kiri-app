import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { useSettings } from "../../lib/settings";
import { showError } from "../../lib/toast";
import { PRESETS, type Preset } from "../../lib/types";
import { Row } from "./Row";
import { Group } from "./Group";

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
    <>
      <Group title={t("settings.group.save")}>
        <Row label={t("settings.download.dir")} desc={d.dir}>
          <button className="btn btn-sm btn-secondary" onClick={pickDir}>{t("settings.download.change")}</button>
        </Row>
      </Group>
      <Group title={t("settings.group.defaults")}>
        <Row label={t("settings.download.quality")}>
          {(id) => (
            <select aria-labelledby={id} className="select select-sm w-40" value={d.quality} onChange={(e) => update({ download: { quality: e.target.value } })}>
              {QUALITIES.map((q) => (
                <option key={q} value={q}>{qLabel(q)}</option>
              ))}
            </select>
          )}
        </Row>
        <Row label={t("settings.download.preset")}>
          {(id) => (
            <select aria-labelledby={id} className="select select-sm w-40" value={d.preset} onChange={(e) => update({ download: { preset: e.target.value as Preset } })}>
              {PRESETS.map((p) => (
                <option key={p} value={p}>{t(`preset.${p}`)}</option>
              ))}
            </select>
          )}
        </Row>
        <Row label={t("settings.download.subtitles")} desc={t("settings.download.subtitlesHint")}>
          {(id) => <input aria-labelledby={id} className="input input-sm w-40" value={subs} onChange={(e) => setSubs(e.target.value)} onBlur={commitSubs} onKeyDown={(e) => e.key === "Enter" && commitSubs()} />}
        </Row>
        <Row label={t("settings.download.skipSheet")}>
          {(id) => <input type="checkbox" aria-labelledby={id} className="toggle toggle-primary toggle-sm" checked={d.skip_sheet} onChange={(e) => update({ download: { skip_sheet: e.target.checked } })} />}
        </Row>
      </Group>
      <Group title={t("settings.group.performance")}>
        <Row label={t("settings.download.maxConcurrent")}>
          {(id) => (
            <div role="group" aria-labelledby={id} className="join">
              {[1, 2, 3, 4].map((n) => (
                <button
                  key={n}
                  className={`btn btn-sm join-item w-9 ${d.max_concurrent === n ? "btn-primary" : ""}`}
                  aria-pressed={d.max_concurrent === n}
                  onClick={() => update({ download: { max_concurrent: n } })}
                >
                  {n}
                </button>
              ))}
            </div>
          )}
        </Row>
        <Row label={t("settings.download.hwAccel")} desc={t("settings.download.hwAccelDesc")}>
          {(id) => <input type="checkbox" aria-labelledby={id} className="toggle toggle-primary toggle-sm" checked={d.hw_accel} onChange={(e) => update({ download: { hw_accel: e.target.checked } })} />}
        </Row>
      </Group>
    </>
  );
}
