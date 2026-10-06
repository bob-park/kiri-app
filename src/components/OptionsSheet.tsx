import { useEffect, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { api } from "../lib/tauri";
import { useSettings } from "../lib/settings";
import { errorText, showError } from "../lib/toast";
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
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!info) return;
    setQuality(pickDefaultQuality(info.qualities, defaults.quality));
    setSubs(defaultSubtitles(info, defaults.subtitles));
  }, [info]);

  const dialogRef = useRef<HTMLDialogElement>(null);
  const downloadRef = useRef<HTMLButtonElement>(null);
  const composing = useRef(false);

  // 모달로 열어 포커스를 가두고, 닫히면(언마운트) 원래 포커스로 돌려준다.
  useEffect(() => {
    const dialog = dialogRef.current!;
    const prev = document.activeElement as HTMLElement | null;
    dialog.showModal();
    (downloadRef.current ?? dialog).focus();
    return () => {
      dialog.close();
      prev?.focus();
    };
  }, []);

  const toggleSub = (l: string) => setSubs((s) => (s.includes(l) ? s.filter((x) => x !== l) : [...s, l]));
  const autoOnly = info ? info.auto_subtitles.filter((l) => !info.subtitles.includes(l)) : [];

  const confirm = async () => {
    if (!info) return;
    setBusy(true);
    setError(null);
    try {
      await api.addJob(buildNewJob(url, info, quality, preset, subs));
    } catch (e) {
      // 모달 밖 토스트는 inert라 읽히지도 눌리지도 않으니 시트 안에 보여 준다.
      setError(errorText(e));
      setBusy(false);
      return;
    }
    // 작업은 이미 큐에 들어갔으니 설정 저장이 실패해도 시트를 닫는다(중복 추가 방지).
    if (remember) await useSettings.getState().update(rememberPatch(quality, preset, subs)).catch(showError);
    onClose();
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
    <dialog
      ref={dialogRef}
      tabIndex={-1}
      aria-label={info?.title ?? t("sheet.loading")}
      className="mx-auto mt-0 h-fit max-h-[88vh] w-[min(520px,94vw)] max-w-none overflow-y-auto rounded-b-2xl bg-base-100 p-0 text-base-content shadow-xl outline-none backdrop:bg-black/30"
      onCompositionStart={() => (composing.current = true)}
      onCompositionEnd={() => (composing.current = false)}
      onCancel={(e) => {
        e.preventDefault();
        if (!composing.current) onClose();
      }}
      onClick={(e) => e.target === dialogRef.current && onClose()}
    >
      <div className="p-4">
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

            {error && <div role="alert" className="alert alert-error mt-3 py-2 text-sm">{error}</div>}

            <div className="mt-4 flex justify-end gap-2">
              <button className="btn btn-outline btn-sm" onClick={onClose}>{t("sheet.cancel")}</button>
              <button ref={downloadRef} autoFocus className="btn btn-primary btn-sm" onClick={confirm} disabled={busy}>{t("sheet.download")}</button>
            </div>
          </>
        )}
      </div>
    </dialog>
  );
}
