import { useEffect, useRef, useState } from 'react';

import type { Quality, VideoInfo } from '@/domain/queue/apis/queue.dto';
import { useJobAdd } from '@/domain/queue/queries/queue';
import type { Preset, Settings } from '@/domain/settings/apis/settings.dto';
import { useSettingsPatch } from '@/domain/settings/queries/settings';
import type { AppError } from '@/shared/api/common.dto';
import useI18n from '@/shared/hooks/useI18n';
import { errorDetail, errorMessage } from '@/utils/errorMessage';
import { formatBytes, formatDuration, langName } from '@/utils/format';
import {
  PRESETS,
  buildNewJob,
  defaultSubtitles,
  pickDefaultQuality,
  rememberPatch,
  visiblePresets,
} from '@/utils/sheet';

import cx from 'classnames';
import { flushSync } from 'react-dom';

interface SectionProps {
  label: string;
  children: React.ReactNode;
}

function Section({ label, children }: Readonly<SectionProps>) {
  return (
    <div className="mt-3.5">
      <div className="text-fg-muted mb-1.5 text-[10.5px] font-bold">{label}</div>
      {children}
    </div>
  );
}

const chipClass = (on: boolean) =>
  cx(
    'has-focus-visible:outline-primary mr-1 mb-1 inline-flex cursor-pointer items-center rounded-lg border px-2.5 py-1 text-[11px] has-focus-visible:outline-2',
    {
      'border-primary bg-primary text-primary-content font-semibold': on,
      'border-base-300 text-base-content/80 hover:border-primary/50': !on,
    },
  );

interface OptionsSheetProps {
  url: string;
  /** `null` while probing. */
  info: VideoInfo | null;
  defaults: Settings['download'];
  onClose: () => void;
}

/** Modal sheet to pick quality, format and subtitles before adding a YouTube job. */
export default function OptionsSheet({ url, info, defaults, onClose }: Readonly<OptionsSheetProps>) {
  // ref
  const dialogRef = useRef<HTMLDialogElement>(null);
  const downloadRef = useRef<HTMLButtonElement>(null);
  const presetGroupRef = useRef<HTMLDivElement>(null);
  const composing = useRef(false);

  // state
  const [quality, setQuality] = useState<Quality | null>(null);
  const [preset, setPreset] = useState<Preset>(defaults.preset);
  const [subs, setSubs] = useState<string[]>([]);
  const [remember, setRemember] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const [allPresets, setAllPresets] = useState(false);

  // hooks
  const { t, locale } = useI18n();

  // queries
  const { addJob, isLoading: adding } = useJobAdd();
  const { patchSettings } = useSettingsPatch();

  // useEffect
  useEffect(() => {
    if (!info) {
      return;
    }
    setQuality(pickDefaultQuality(info.qualities, defaults.quality));
    setSubs(defaultSubtitles(info, defaults.subtitles));
    // Only when the probe result arrives; later settings changes must not reset the user's picks.
  }, [info]);

  // Open as a modal to trap focus; on close (unmount) give focus back.
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) {
      return;
    }
    const prev = document.activeElement as HTMLElement | null;
    dialog.showModal();
    (downloadRef.current ?? dialog).focus();

    return () => {
      dialog.close();
      prev?.focus();
    };
  }, []);

  // handle
  // "More" disappears once expanded, so move keyboard focus to the selected format chip.
  const handleShowAllPresets = () => {
    flushSync(() => setAllPresets(true));
    presetGroupRef.current?.querySelector<HTMLInputElement>('input:checked')?.focus();
  };

  const handleToggleSub = (l: string) => setSubs((s) => (s.includes(l) ? s.filter((x) => x !== l) : [...s, l]));

  const handleConfirm = () => {
    if (!info) {
      return;
    }
    setError(null);
    addJob(buildNewJob(url, info, quality, preset, subs), {
      onSuccess: () => {
        // The job is queued already, so close even if saving the defaults fails (no duplicate adds).
        if (remember) {
          patchSettings(rememberPatch(quality, preset, subs));
        }
        onClose();
      },
      // Toasts outside the modal are inert, so the error shows inside the sheet.
      onError: setError,
    });
  };

  const autoOnly = info ? info.auto_subtitles.filter((l) => !info.subtitles.includes(l)) : [];

  const qualityTile = (q: Quality | null) => {
    const on = (q?.format_id ?? null) === (quality?.format_id ?? null);

    return (
      <label
        key={q?.format_id ?? 'audio'}
        className={cx(
          'has-focus-visible:outline-primary cursor-pointer rounded-[10px] border px-2.5 py-1.5 has-focus-visible:outline-2',
          {
            'border-primary bg-primary/8 border-[1.5px]': on,
            'border-base-300 hover:border-primary/50': !on,
          },
        )}
      >
        <input type="radio" name="quality" className="sr-only" checked={on} onChange={() => setQuality(q)} />
        <span className={cx('block text-[12px] font-semibold', { 'text-secondary-content': on })}>
          {q ? q.label : t.quality.audio}
        </span>
        <span className="text-fg-muted block truncate text-[10px]">
          {q ? [q.vcodec, formatBytes(q.filesize)].filter(Boolean).join(' · ') : 'm4a'}
        </span>
      </label>
    );
  };

  const presetChip = (p: Preset) => (
    <label key={p} className={chipClass(preset === p)}>
      <input type="radio" name="preset" className="sr-only" checked={preset === p} onChange={() => setPreset(p)} />
      {t.preset[p]}
    </label>
  );

  const subChip = (l: string) => (
    <label key={l} className={chipClass(subs.includes(l))}>
      <input type="checkbox" className="sr-only" checked={subs.includes(l)} onChange={() => handleToggleSub(l)} />
      {langName(l, locale)}
    </label>
  );

  const detail = error && errorDetail(error);

  return (
    <dialog
      ref={dialogRef}
      tabIndex={-1}
      aria-label={info?.title ?? t.sheet.loading}
      className="float-shadow bg-base-100 text-base-content m-auto h-fit max-h-[88vh] w-[min(380px,92vw)] max-w-none overflow-y-auto rounded-[18px] p-0 outline-none backdrop:bg-black/30"
      onCompositionStart={() => (composing.current = true)}
      onCompositionEnd={() => (composing.current = false)}
      onCancel={(e) => {
        e.preventDefault();
        if (!composing.current) {
          onClose();
        }
      }}
      onClick={(e) => e.target === dialogRef.current && onClose()}
    >
      {!info ? (
        <div>
          <div className="bg-base-200 h-[110px] animate-pulse" />
          <div className="flex items-center gap-3 p-4 text-sm">
            <span className="loading loading-spinner loading-sm" />
            {t.sheet.loading}
          </div>
        </div>
      ) : (
        <>
          <div className="from-primary to-secondary relative h-[110px] bg-gradient-to-br">
            {info.thumbnail && (
              <img src={info.thumbnail} alt="" className="absolute inset-0 h-full w-full object-cover" />
            )}
            <div className="absolute inset-0 bg-gradient-to-b from-transparent from-30% to-black/75" />
            {info.duration_secs != null && (
              <span className="absolute top-2.5 right-2.5 rounded-md bg-black/60 px-1.5 py-0.5 text-[10px] text-white">
                {formatDuration(info.duration_secs)}
              </span>
            )}
            <div className="absolute right-3.5 bottom-2.5 left-3.5 text-white">
              <div className="truncate text-[13.5px] font-semibold">{info.title}</div>
              {info.channel && <div className="truncate text-[11px] opacity-80">{info.channel}</div>}
            </div>
          </div>

          <div className="px-4 pt-0.5 pb-4">
            <Section label={t.sheet.quality}>
              <div role="radiogroup" aria-label={t.sheet.quality} className="grid grid-cols-3 gap-1.5">
                {info.qualities.map(qualityTile)}
                {qualityTile(null)}
              </div>
            </Section>

            <Section label={t.sheet.format}>
              <div ref={presetGroupRef} role="radiogroup" aria-label={t.sheet.format}>
                {(allPresets ? PRESETS : visiblePresets(defaults.preset)).map(presetChip)}
                {!allPresets && (
                  <button type="button" className={chipClass(false)} onClick={handleShowAllPresets}>
                    {t.sheet.more} ▾
                  </button>
                )}
              </div>
            </Section>

            <Section label={t.sheet.subtitles}>
              {info.subtitles.length === 0 && autoOnly.length === 0 && (
                <div className="text-fg-muted text-xs">{t.sheet.noSubs}</div>
              )}
              <div>{info.subtitles.map(subChip)}</div>
              {autoOnly.length > 0 && (
                <details className="mt-1">
                  <summary className="text-fg-muted cursor-pointer text-[11px]">
                    {t.sheet.autoSubs} ({autoOnly.length})
                  </summary>
                  <div className="mt-1.5 max-h-32 overflow-y-auto">{autoOnly.map(subChip)}</div>
                </details>
              )}
            </Section>

            {error && (
              <div role="alert" className="alert alert-error mt-3 py-2 text-sm">
                <div>
                  <div>{errorMessage(error, t.errors)}</div>
                  {detail && <div className="text-xs opacity-80">{detail}</div>}
                </div>
              </div>
            )}

            <div className="mt-4 flex items-center gap-2">
              <label className="text-base-content/80 flex flex-1 items-center gap-2 text-[11.5px]">
                <input
                  type="checkbox"
                  className="checkbox checkbox-sm checkbox-primary"
                  checked={remember}
                  onChange={(e) => setRemember(e.target.checked)}
                />
                {t.sheet.remember}
              </label>
              <button className="btn btn-sm" onClick={onClose}>
                {t.sheet.cancel}
              </button>
              <button
                ref={downloadRef}
                autoFocus
                className="btn btn-primary btn-sm"
                onClick={handleConfirm}
                disabled={adding}
              >
                {t.sheet.download}
              </button>
            </div>
          </div>
        </>
      )}
    </dialog>
  );
}
