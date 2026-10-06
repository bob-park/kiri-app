export type Theme = "system" | "light" | "dark";
export type UiLang = "system" | "ko" | "en" | "ja";
export type Lang = "ko" | "en" | "ja";

export type Preset = "original" | "mp4-h264" | "mp4-hevc" | "mov-prores" | "webm-vp9" | "mp3" | "m4a";
export const PRESETS: Preset[] = ["original", "mp4-h264", "mp4-hevc", "mov-prores", "webm-vp9", "mp3", "m4a"];
export const AUDIO_PRESETS: Preset[] = ["mp3", "m4a"];

export interface Settings {
  version: number;
  general: { ui_language: UiLang; theme: Theme; close_to_tray: boolean };
  download: {
    dir: string;
    quality: string; // "best" | "audio" | "<N>p"
    preset: Preset;
    subtitles: string[];
    skip_sheet: boolean;
    max_concurrent: number;
    hw_accel: boolean;
  };
  update: { auto_check: boolean; last_ytdlp_check: number | null };
}

export type DeepPartial<T> = {
  [K in keyof T]?: T[K] extends unknown[] ? T[K] : T[K] extends object ? DeepPartial<T[K]> : T[K];
};

export type JobState =
  | { kind: "queued" | "downloading" | "encoding" | "completed" | "stopped" }
  | { kind: "failed"; message: string };

export interface JobOptions {
  format_id: string | null;
  preset: Preset;
  subtitles: string[];
  auto_subtitles: boolean;
}

export interface Job {
  id: number;
  url: string;
  title: string;
  thumbnail: string | null;
  duration_secs: number | null;
  quality_label: string;
  options: JobOptions;
  state: JobState;
  progress: number;
  speed: string | null;
  eta: string | null;
  output: string | null;
  work_dir: string | null;
  created_at: number;
}

export type NewJob = Pick<Job, "url" | "title" | "thumbnail" | "duration_secs" | "quality_label" | "options">;

export interface Quality {
  format_id: string;
  height: number;
  fps: number | null;
  vcodec: string;
  filesize: number | null;
  label: string;
}

export interface VideoInfo {
  id: string;
  title: string;
  channel: string | null;
  duration_secs: number | null;
  thumbnail: string | null;
  qualities: Quality[];
  subtitles: string[];
  auto_subtitles: string[];
}

export interface ToolsStatus {
  ready: boolean;
  installing: boolean;
  ytdlp_version: string | null;
  error: string | null;
  last_check: number | null;
}

export interface CliStatus {
  installed: boolean;
  link: string;
  target: string;
  socket_error: string | null;
}

export interface UpdateInfo {
  version: string;
  notes: string;
}

export interface UpdateProgress {
  received: number;
  total: number | null;
}

/** Rust CmdError */
export interface CmdError {
  code: string;
  message: string;
}
