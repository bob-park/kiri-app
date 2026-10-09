import { useEffect, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { api } from "../lib/tauri";
import { useSettings } from "../lib/settings";
import { errorText, showError } from "../lib/toast";
import { langName } from "../lib/i18n";
import { formatBytes, formatDuration } from "../lib/format";
import { buildNewJob, defaultSubtitles, pickDefaultQuality, rememberPatch, visiblePresets } from "../lib/sheet";
import { PRESETS, type Preset, type Quality, type VideoInfo } from "../lib/types";

interface Props {
  url: string;
  info: VideoInfo | null; // null = probe 중
  onClose: () => void;
}

function Section({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="mt-3.5">
      <div className="mb-1.5 text-[10.5px] font-bold text-fg-muted">{label}</div>
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
  const [allPresets, setAllPresets] = useState(false);

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

  const qualityTile = (q: Quality | null) => {
    const on = (q?.format_id ?? null) === (quality?.format_id ?? null);
    return (
      <label
        key={q?.format_id ?? "audio"}
        className={`cursor-pointer rounded-[10px] border px-2.5 py-1.5 has-focus-visible:outline-2 has-focus-visible:outline-primary ${
          on ? "border-[1.5px] border-primary bg-primary/8" : "border-base-300 hover:border-primary/50"
        }`}
      >
        <input type="radio" name="quality" className="sr-only" checked={on} onChange={() => setQuality(q)} />
        <span className={`block text-[12px] font-semibold ${on ? "text-secondary-content" : ""}`}>{q ? q.label : t("quality.audio")}</span>
        <span className="block truncate text-[10px] text-fg-muted">{q ? [q.vcodec, formatBytes(q.filesize)].filter(Boolean).join(" · ") : "m4a"}</span>
      </label>
    );
  };

  const chipCls = (on: boolean) =>
    `mr-1 mb-1 inline-flex cursor-pointer items-center rounded-lg border px-2.5 py-1 text-[11px] has-focus-visible:outline-2 has-focus-visible:outline-primary ${
      on ? "border-primary bg-primary font-semibold text-primary-content" : "border-base-300 text-base-content/80 hover:border-primary/50"
    }`;

  const presetChip = (p: Preset) => (
    <label key={p} className={chipCls(preset === p)}>
      <input type="radio" name="preset" className="sr-only" checked={preset === p} onChange={() => setPreset(p)} />
      {t(`preset.${p}`)}
    </label>
  );

  const subChip = (l: string) => (
    <label key={l} className={chipCls(subs.includes(l))}>
      <input type="checkbox" className="sr-only" checked={subs.includes(l)} onChange={() => toggleSub(l)} />
      {langName(l, i18n.language)}
    </label>
  );

  return (
    <dialog
      ref={dialogRef}
      tabIndex={-1}
      aria-label={info?.title ?? t("sheet.loading")}
      className="float-shadow m-auto h-fit max-h-[88vh] w-[min(380px,92vw)] max-w-none overflow-y-auto rounded-[18px] bg-base-100 p-0 text-base-content outline-none backdrop:bg-black/30"
      onCompositionStart={() => (composing.current = true)}
      onCompositionEnd={() => (composing.current = false)}
      onCancel={(e) => {
        e.preventDefault();
        if (!composing.current) onClose();
      }}
      onClick={(e) => e.target === dialogRef.current && onClose()}
    >
      {!info ? (
        <div>
          <div className="h-[110px] animate-pulse bg-base-200" />
          <div className="flex items-center gap-3 p-4 text-sm">
            <span className="loading loading-spinner loading-sm" />
            {t("sheet.loading")}
          </div>
        </div>
      ) : (
        <>
          <div className="relative h-[110px] bg-gradient-to-br from-primary to-secondary">
            {info.thumbnail && <img src={info.thumbnail} alt="" className="absolute inset-0 h-full w-full object-cover" />}
            <div className="absolute inset-0 bg-gradient-to-b from-transparent from-30% to-black/75" />
            {info.duration_secs != null && (
              <span className="absolute top-2.5 right-2.5 rounded-md bg-black/60 px-1.5 py-0.5 text-[10px] text-white">{formatDuration(info.duration_secs)}</span>
            )}
            <div className="absolute right-3.5 bottom-2.5 left-3.5 text-white">
              <div className="truncate text-[13.5px] font-semibold">{info.title}</div>
              {info.channel && <div className="truncate text-[11px] opacity-80">{info.channel}</div>}
            </div>
          </div>

          <div className="px-4 pt-0.5 pb-4">
            <Section label={t("sheet.quality")}>
              <div role="radiogroup" aria-label={t("sheet.quality")} className="grid grid-cols-3 gap-1.5">
                {info.qualities.map(qualityTile)}
                {qualityTile(null)}
              </div>
            </Section>

            <Section label={t("sheet.format")}>
              <div role="radiogroup" aria-label={t("sheet.format")}>
                {(allPresets ? PRESETS : visiblePresets(defaults.preset)).map(presetChip)}
                {!allPresets && (
                  <button type="button" className={chipCls(false)} onClick={() => setAllPresets(true)}>{t("sheet.more")} ▾</button>
                )}
              </div>
            </Section>

            <Section label={t("sheet.subtitles")}>
              {info.subtitles.length === 0 && autoOnly.length === 0 && <div className="text-xs text-fg-muted">{t("sheet.noSubs")}</div>}
              <div>{info.subtitles.map(subChip)}</div>
              {autoOnly.length > 0 && (
                <details className="mt-1">
                  <summary className="cursor-pointer text-[11px] text-fg-muted">{t("sheet.autoSubs")} ({autoOnly.length})</summary>
                  <div className="mt-1.5 max-h-32 overflow-y-auto">{autoOnly.map(subChip)}</div>
                </details>
              )}
            </Section>

            {error && <div role="alert" className="alert alert-error mt-3 py-2 text-sm">{error}</div>}

            <div className="mt-4 flex items-center gap-2">
              <label className="flex flex-1 items-center gap-2 text-[11.5px] text-base-content/80">
                <input type="checkbox" className="checkbox checkbox-sm checkbox-primary" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
                {t("sheet.remember")}
              </label>
              <button className="btn btn-sm" onClick={onClose}>{t("sheet.cancel")}</button>
              <button ref={downloadRef} autoFocus className="btn btn-primary btn-sm" onClick={confirm} disabled={busy}>{t("sheet.download")}</button>
            </div>
          </div>
        </>
      )}
    </dialog>
  );
}
