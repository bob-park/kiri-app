// Type-only imports: erased at runtime, so node --test needs no alias resolution.
import type { NewJob, Quality, VideoInfo } from '@/domain/queue/apis/queue.dto';
import type { Preset, SettingsPatch } from '@/domain/settings/apis/settings.dto';

export const PRESETS: Preset[] = ['original', 'mp4-h264', 'mp4-hevc', 'mov-prores', 'webm-vp9', 'mp3', 'm4a'];

export const AUDIO_PRESETS: Preset[] = ['mp3', 'm4a'];

/** Same rule as Rust `ytdlp::resolve_quality`. `null` means audio only. */
export function pickDefaultQuality(qualities: Quality[], want: string): Quality | null {
  if (want === 'audio' || qualities.length === 0) {
    return null;
  }

  const h = parseInt(want, 10);
  if (want === 'best' || Number.isNaN(h)) {
    return qualities[0];
  }

  return qualities.find((q) => q.height <= h) ?? qualities[qualities.length - 1];
}

export function defaultSubtitles(info: VideoInfo, langs: string[]): string[] {
  return langs.filter((l) => info.subtitles.includes(l) || info.auto_subtitles.includes(l));
}

export function buildNewJob(
  url: string,
  info: VideoInfo,
  quality: Quality | null,
  preset: Preset,
  subs: string[],
): NewJob {
  const video = quality !== null && !AUDIO_PRESETS.includes(preset) ? quality : null;

  return {
    source: { kind: 'youtube', url },
    title: info.title,
    thumbnail: info.thumbnail,
    duration_secs: info.duration_secs,
    quality_label: video ? video.label : 'audio',
    options: {
      format_id: video ? video.format_id : null,
      preset,
      subtitles: subs,
      auto_subtitles: subs.some((s) => !info.subtitles.includes(s)),
      max_height: null,
    },
  };
}

/** "Start right away with these settings next time". */
export function rememberPatch(quality: Quality | null, preset: Preset, subs: string[]): SettingsPatch {
  return { download: { quality: quality ? `${quality.height}p` : 'audio', preset, subtitles: subs, skip_sheet: true } };
}

const COMMON: Preset[] = ['original', 'mp4-h264', 'mp3'];

/**
 * The format chips the sheet shows first: at most three, always including the default,
 * in `PRESETS` order (so a chip never moves when selected).
 */
export function visiblePresets(defaultPreset: Preset): Preset[] {
  const pick = COMMON.includes(defaultPreset) ? COMMON : [...COMMON.slice(0, 2), defaultPreset];

  return PRESETS.filter((p) => pick.includes(p));
}
