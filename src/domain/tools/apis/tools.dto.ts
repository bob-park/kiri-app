/** Mirrors `src-tauri/src/domain/tools/mod.rs` `ToolsStatus`. */
export type ToolsStatus = {
  ready: boolean;
  installing: boolean;
  ytdlp_version: string | null;
  error: string | null;
  /** Unix ms. */
  last_check: number | null;
};
