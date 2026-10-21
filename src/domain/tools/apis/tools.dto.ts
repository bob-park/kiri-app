/** Mirrors `src-tauri/src/domain/tools/mod.rs` `ToolsStatus`. */
export type ToolsStatus = {
  ready: boolean;
  /** The tool being downloaded right now. */
  installing: 'yt-dlp' | 'deno' | null;
  ytdlp_version: string | null;
  error: string | null;
  /** Unix ms. */
  last_check: number | null;
};
