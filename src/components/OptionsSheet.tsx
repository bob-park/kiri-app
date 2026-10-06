import { useEffect, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { api } from "../lib/tauri";
import { useSettings } from "../lib/settings";
import { showError } from "../lib/toast";
import { langName } from "../lib/i18n";
import { formatBytes, formatDuration } from "../lib/format";
import { buildNewJob, defaultSubtitles, pickDefaultQuality, rememberPatch } from "../lib/sheet";
import { PRESETS, type Preset, type Quality, type VideoInfo } from "../lib/types";

interface Props {
  url: string;
  info: VideoInfo | null; // null = probe 중
  onClose: () => void;
}

function Section({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="mt-3">
      <div className="mb-1 text-[10px] font-semibold uppercase tracking-wide text-fg-muted">{label}</div>
      {children}
    </div>
  );
}

export function OptionsSheet({ url, info, onClose }: Props) {
  const { t, i18n } = useTranslation();
  const defaults = useSettings((s) => s.settings!.download);
  const [quality, setQuality] = useState<Quality | null>(null);
  const [preset, setPreset] = useState<Preset>(defaults.preset);
  const [subs, setSubs] = useState<string[]>([]);
  const [remember, setRemember] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!info) return;
    setQuality(pickDefaultQuality(info.qualities, defaults.quality));
    setSubs(defaultSubtitles(info, defaults.subtitles));
  }, [info]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const toggleSub = (l: string) => setSubs((s) => (s.includes(l) ? s.filter((x) => x !== l) : [...s, l]));
  const autoOnly = info ? info.auto_subtitles.filter((l) => !info.subtitles.includes(l)) : [];

  const confirm = async () => {
    if (!info) return;
    setBusy(true);
    try {
      await api.addJob(buildNewJob(url, info, quality, preset, subs));
      if (remember) await useSettings.getState().update(rememberPatch(quality, preset, subs));
      onClose();
    } catch (e) {
      showError(e);
    } finally {
      setBusy(false);
    }
  };

  const qualityRow = (q: Quality | null) => {
    const on = (q?.format_id ?? null) === (quality?.format_id ?? null);
    return (
      <label
        key={q?.format_id ?? "audio"}
        className={`mb-1 flex cursor-pointer items-center justify-between rounded-lg border px-3 py-1.5 text-sm ${
          on ? "border-primary bg-secondary font-semibold text-secondary-content" : "border-base-300"
        }`}
      >
        <span>
          <input type="radio" name="quality" className="sr-only" checked={on} onChange={() => setQuality(q)} />
          {q ? `${q.label} · ${q.vcodec}` : t("quality.audio")}
        </span>
        <span className="text-xs text-fg-muted">{q ? formatBytes(q.filesize) : ""}</span>
      </label>
    );
  };

  const subChip = (l: string) => (
    <label key={l} className="mr-1 mb-1 inline-flex cursor-pointer items-center gap-1 rounded-lg border border-base-300 px-2 py-0.5 text-xs">
      <input type="checkbox" className="checkbox checkbox-xs checkbox-primary" checked={subs.includes(l)} onChange={() => toggleSub(l)} />
      {langName(l, i18n.language)}
    </label>
  );

  return (
    <div className="fixed inset-0 z-40 flex justify-center bg-black/30" onClick={onClose}>
      <div
        role="dialog"
        aria-modal="true"
        aria-label={info?.title ?? t("sheet.loading")}
        className="h-fit max-h-[88vh] w-[min(520px,94vw)] overflow-y-auto rounded-b-2xl bg-base-100 p-4 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        {!info ? (
          <div className="flex items-center gap-3 py-6 text-sm">
            <span className="loading loading-spinner loading-sm" />
            {t("sheet.loading")}
          </div>
        ) : (
          <>
            <div className="flex items-center gap-3">
              {info.thumbnail && <img src={info.thumbnail} alt="" className="h-12 w-20 rounded-md object-cover" />}
              <div className="min-w-0">
                <div className="truncate font-semibold">{info.title}</div>
                <div className="text-xs text-fg-muted">{[info.channel, formatDuration(info.duration_secs)].filter(Boolean).join(" · ")}</div>
              </div>
            </div>

            <Section label={t("sheet.quality")}>
              {info.qualities.map(qualityRow)}
              {qualityRow(null)}
            </Section>

            <Section label={t("sheet.format")}>
              <select className="select select-sm w-full" value={preset} onChange={(e) => setPreset(e.target.value as Preset)}>
                {PRESETS.map((p) => (
                  <option key={p} value={p}>{t(`preset.${p}`)}</option>
                ))}
              </select>
            </Section>

            <Section label={t("sheet.subtitles")}>
              {info.subtitles.length === 0 && autoOnly.length === 0 && <div className="text-xs text-fg-muted">{t("sheet.noSubs")}</div>}
              <div>{info.subtitles.map(subChip)}</div>
              {autoOnly.length > 0 && (
                <details className="mt-1">
                  <summary className="cursor-pointer text-xs text-fg-muted">{t("sheet.autoSubs")} ({autoOnly.length})</summary>
                  <div className="mt-1 max-h-32 overflow-y-auto">{autoOnly.map(subChip)}</div>
                </details>
              )}
            </Section>

            <label className="mt-3 flex items-center gap-2 text-sm">
              <input type="checkbox" className="checkbox checkbox-sm checkbox-primary" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
              {t("sheet.remember")}
            </label>

            <div className="mt-4 flex justify-end gap-2">
              <button className="btn btn-outline btn-sm" onClick={onClose}>{t("sheet.cancel")}</button>
              <button className="btn btn-primary btn-sm" onClick={confirm} disabled={busy}>{t("sheet.download")}</button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
