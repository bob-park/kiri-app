// Type-only imports: erased at runtime, so node --test needs no alias resolution.
import type { Job } from '@/domain/queue/apis/queue.dto';
import type { Dict } from '@/shared/i18n/en';

export function formatBytes(n: number | null | undefined): string {
  if (!n) {
    return '';
  }

  const units = ['B', 'KB', 'MB', 'GB'];
  let v = n;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }

  return i === 0 ? `${v} B` : `${v.toFixed(1)} ${units[i]}`;
}

export function formatDuration(secs: number | null | undefined): string {
  if (secs == null) {
    return '';
  }

  const s = Math.round(secs);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = String(s % 60).padStart(2, '0');

  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${r}` : `${m}:${r}`;
}

export const pct = (p: number) => `${Math.round(p * 100)}%`;

/** A message starting with `error.` is a key into `t.errors` (uppercased); anything else is raw tool output. */
export function failureText(message: string, errors: Dict['errors']): string {
  if (!message.startsWith('error.')) {
    return message;
  }

  return (errors as Record<string, string>)[message.slice('error.'.length).toUpperCase()] ?? message;
}

/** Subtitle language code → a readable name (Intl), or the code when unknown. */
export function langName(code: string, locale: string): string {
  try {
    return new Intl.DisplayNames([locale], { type: 'language' }).of(code) ?? code;
  } catch {
    return code;
  }
}

/** A job row's second line: quality · format · subtitles · per-state info. */
export function jobDetail(job: Job, t: Dict): string {
  const label = job.quality_label;
  const parts = [label === 'audio' || label === 'original' ? t.quality[label] : label, t.preset[job.options.preset]];
  if (job.source.kind === 'file') {
    parts.unshift(t.job.transcode);
  }
  if (job.options.subtitles.length) {
    parts.push(t.job.subs(job.options.subtitles.join(', ')));
  }

  const s = job.state;
  if (s.kind === 'downloading') {
    parts.push([pct(job.progress), job.speed, job.eta].filter(Boolean).join(' · '));
  } else if (s.kind === 'encoding') {
    parts.push(`${t.state.encoding} ${pct(job.progress)}`);
    if (job.original) {
      parts.push(t.job.encodingSaved);
    }
  } else if (s.kind === 'failed') {
    parts.push(failureText(s.message, t.errors));
  }

  return parts.join(' · ');
}
