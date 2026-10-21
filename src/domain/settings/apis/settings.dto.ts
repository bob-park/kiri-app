/** Mirrors `src-tauri/src/domain/settings/model.rs` (snake_case keys). */
export type ThemePref = 'system' | 'light' | 'dark';

export type LocalePref = 'system' | 'en' | 'ko';

export type Preset = 'original' | 'mp4-h264' | 'mp4-hevc' | 'mov-prores' | 'webm-vp9' | 'mp3' | 'm4a';

export type Settings = {
  version: number;
  general: { ui_language: LocalePref; theme: ThemePref; close_to_tray: boolean };
  download: {
    dir: string;
    /** `best` | `audio` | `<N>p` */
    quality: string;
    preset: Preset;
    subtitles: string[];
    skip_sheet: boolean;
    max_concurrent: number;
    hw_accel: boolean;
  };
  /** `last_ytdlp_check`: Unix ms. */
  update: { auto_check: boolean; last_ytdlp_check: number | null };
};
