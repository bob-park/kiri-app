import type { DeepPartial, NewJob, Preset, Quality, Settings, VideoInfo } from "./types";
import { AUDIO_PRESETS, PRESETS } from "./types";

/** Rust ytdlp::resolve_quality 와 같은 규칙. null 은 오디오만. */
export function pickDefaultQuality(qualities: Quality[], want: string): Quality | null {
  if (want === "audio") return null;
  if (qualities.length === 0) return null;
  const h = parseInt(want, 10);
  if (want === "best" || Number.isNaN(h)) return qualities[0];
  return qualities.find((q) => q.height <= h) ?? qualities[qualities.length - 1];
}

export function defaultSubtitles(info: VideoInfo, langs: string[]): string[] {
  return langs.filter((l) => info.subtitles.includes(l) || info.auto_subtitles.includes(l));
}

export function buildNewJob(url: string, info: VideoInfo, quality: Quality | null, preset: Preset, subs: string[]): NewJob {
  const video = quality !== null && !AUDIO_PRESETS.includes(preset) ? quality : null;
  return {
    source: { kind: "youtube", url },
    title: info.title,
    thumbnail: info.thumbnail,
    duration_secs: info.duration_secs,
    quality_label: video ? video.label : "audio",
    options: {
      format_id: video ? video.format_id : null,
      preset,
      subtitles: subs,
      auto_subtitles: subs.some((s) => !info.subtitles.includes(s)),
      max_height: null,
    },
  };
}

/** "다음부터 이 설정으로 바로 시작" */
export function rememberPatch(quality: Quality | null, preset: Preset, subs: string[]): DeepPartial<Settings> {
  return { download: { quality: quality ? `${quality.height}p` : "audio", preset, subtitles: subs, skip_sheet: true } };
}

const COMMON: Preset[] = ["original", "mp4-h264", "mp3"];

/** 시트에 처음 보이는 포맷 칩. 기본 포맷은 늘 포함하고 최대 3개, 순서는 PRESETS를 따른다(선택해도 자리가 바뀌지 않게). */
export function visiblePresets(defaultPreset: Preset): Preset[] {
  const pick = COMMON.includes(defaultPreset) ? COMMON : [...COMMON.slice(0, 2), defaultPreset];
  return PRESETS.filter((p) => pick.includes(p));
}
