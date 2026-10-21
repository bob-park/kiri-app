/** Mirrors `crates/kiri-core/src/model.rs` and `ytdlp.rs` (snake_case keys). */
import type { Preset } from '@/domain/settings/apis/settings.dto';

/** `#[serde(tag = "kind", content = "message")]` */
export type JobState =
  { kind: 'queued' | 'downloading' | 'encoding' | 'completed' | 'stopped' } | { kind: 'failed'; message: string };

export type JobSource = { kind: 'youtube'; url: string } | { kind: 'file'; path: string; output_dir: string | null };

export type JobOptions = {
  format_id: string | null;
  preset: Preset;
  subtitles: string[];
  auto_subtitles: boolean;
  /** Height cap for file transcodes; `null` keeps the source size. */
  max_height: number | null;
};

export type Job = {
  id: number;
  source: JobSource;
  title: string;
  thumbnail: string | null;
  duration_secs: number | null;
  quality_label: string;
  options: JobOptions;
  state: JobState;
  /** 0..1 within the current stage. */
  progress: number;
  speed: string | null;
  eta: string | null;
  output: string | null;
  work_dir: string | null;
  /** Unix ms. */
  created_at: number;
};

export type NewJob = Pick<Job, 'source' | 'title' | 'thumbnail' | 'duration_secs' | 'quality_label' | 'options'>;

export type Quality = {
  format_id: string;
  height: number;
  fps: number | null;
  vcodec: string;
  filesize: number | null;
  label: string;
};

export type VideoInfo = {
  id: string;
  title: string;
  channel: string | null;
  duration_secs: number | null;
  thumbnail: string | null;
  qualities: Quality[];
  subtitles: string[];
  auto_subtitles: string[];
};
