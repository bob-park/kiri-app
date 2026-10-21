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
      <div className="text-muted mb-1.5 text-[10.5px] font-bold">{label}</div>
      {children}
    </div>
  );
}

// Selected chips and tiles grow a little and turn brand-colored.
const choiceClass = (on: boolean) =>
  cx(
    'has-focus-visible:outline-primary cursor-pointer rounded-[10px] border transition-[scale,background-color,color,border-color] duration-250 has-focus-visible:outline-2 has-focus-visible:outline-offset-2 motion-reduce:transition-none',
    {
      'border-primary bg-primary text-primary-content scale-[1.04]': on,
      'border-glass-edge bg-glass hover:border-primary/50': !on,
    },
  );

const chipClass = (on: boolean) =>
  cx(choiceClass(on), 'mr-1.5 mb-1.5 inline-flex items-center px-[11px] py-1.5 text-[11.5px] font-semibold');

const SKELETON =
  'animate-shimmer bg-[linear-gradient(90deg,var(--color-track),var(--color-glass),var(--color-track))] bg-[length:200px_100%]';

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
      <label key={q?.format_id ?? 'audio'} className={cx(choiceClass(on), 'px-2.5 py-1.5')}>
        <input type="radio" name="quality" className="sr-only" checked={on} onChange={() => setQuality(q)} />
        <span className="block text-[12px] font-semibold">{q ? q.label : t.quality.audio}</span>
        <span className={cx('block truncate text-[10px]', on ? 'text-primary-content/80' : 'text-muted')}>
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
      className="glass float-shadow text-base-content animate-sheet-up backdrop:bg-scrim fixed inset-x-2.5 top-auto bottom-2.5 m-0 h-fit max-h-[calc(100%-20px)] w-auto max-w-none overflow-y-auto rounded-[20px] p-4 outline-none backdrop:backdrop-blur-[3px]"
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
        <div role="status" className="flex items-center gap-2.5">
          <div className={cx(SKELETON, 'h-[50px] w-[88px] shrink-0 rounded-[10px]')} />
          <div className="min-w-0 flex-1">
            <div className={cx(SKELETON, 'h-3 w-4/5 rounded-md')} />
            <div className={cx(SKELETON, 'mt-1.5 h-3 w-2/5 rounded-md')} />
          </div>
          <span className="sr-only">{t.sheet.loading}</span>
        </div>
      ) : (
        <>
          <div className="flex items-center gap-2.5">
            {info.thumbnail ? (
              <img src={info.thumbnail} alt="" className="h-[50px] w-[88px] shrink-0 rounded-[10px] object-cover" />
            ) : (
              <div className="h-[50px] w-[88px] shrink-0 rounded-[10px] bg-(image:--gradient-progress)" />
            )}
            <div className="min-w-0 flex-1">
              <div className="truncate text-[13px] font-semibold">{info.title}</div>
              <div className="text-muted mt-0.5 truncate text-[11px] tabular-nums">
                {[info.channel, info.duration_secs != null && formatDuration(info.duration_secs)]
                  .filter(Boolean)
                  .join(' · ')}
              </div>
            </div>
          </div>

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
              <div className="text-muted text-xs">{t.sheet.noSubs}</div>
            )}
            <div>{info.subtitles.map(subChip)}</div>
            {autoOnly.length > 0 && (
              <details className="mt-1">
                <summary className="text-muted cursor-pointer text-[11px]">
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

          <label className="text-base-content/80 mt-3.5 flex items-center gap-2 text-[11.5px]">
            <input
              type="checkbox"
              className="checkbox checkbox-sm checkbox-primary"
              checked={remember}
              onChange={(e) => setRemember(e.target.checked)}
            />
            {t.sheet.remember}
          </label>
          <div className="mt-3 flex items-center gap-2">
            <button className="btn btn-ghost h-10 rounded-xl" onClick={onClose}>
              {t.sheet.cancel}
            </button>
            <button
              ref={downloadRef}
              autoFocus
              className="text-primary-content focus-visible:outline-primary h-10 flex-1 cursor-pointer rounded-xl bg-(image:--gradient-progress) text-[13px] font-bold transition-transform duration-150 focus-visible:outline-2 focus-visible:outline-offset-2 active:scale-96 disabled:cursor-default disabled:opacity-60 motion-reduce:transition-none"
              onClick={handleConfirm}
              disabled={adding}
            >
              {t.sheet.download}
            </button>
          </div>
        </>
      )}
    </dialog>
  );
}
