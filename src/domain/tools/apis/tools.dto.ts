/** Mirrors `kiri_core::tools::ToolName` (serde names). */
export type ToolName = 'yt-dlp' | 'ffmpeg' | 'deno';

/** Mirrors `src-tauri/src/domain/tools/mod.rs` `ToolInfo`. */
export interface ToolInfo {
  /** The installed version; `null` while missing. */
  current: string | null;
  /** The newest release seen this session; `null` until a check succeeds. */
  latest: string | null;
}

/** Mirrors `kiri_core::tools::Progress`: bytes received for the tool being installed. */
export interface ToolProgress {
  done: number;
  /** `null` when a server did not report a size. */
  total: number | null;
}

/** Mirrors `src-tauri/src/domain/tools/mod.rs` `ToolsStatus`. */
export type ToolsStatus = {
  ready: boolean;
  /** The tool being downloaded right now. */
  installing: ToolName | null;
  /** Download progress of `installing`; `null` when nothing downloads. */
  progress: ToolProgress | null;
  ytdlp: ToolInfo;
  ffmpeg: ToolInfo;
  deno: ToolInfo;
  /** The last failed install. */
  error: string | null;
  /** Unix ms of the last check of all tools. */
  last_check: number | null;
};
