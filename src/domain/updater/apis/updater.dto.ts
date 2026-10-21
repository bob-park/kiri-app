/** Mirrors `src-tauri/src/app/updater.rs`. */
export type UpdateInfo = {
  version: string;
  notes: string;
  /** Downloaded in the background; can be installed right away. */
  ready: boolean;
  downloading: boolean;
};

export type UpdateProgress = { received: number; total: number | null };

/** `update://status` payload: `#[serde(tag = "kind")]`, struct payloads flattened next to the tag. */
export type UpdateStatus =
  | ({ kind: 'available' | 'ready' } & UpdateInfo)
  | ({ kind: 'progress' } & UpdateProgress)
  | { kind: 'download_failed' | 'error'; error: string }
  | { kind: 'scheduled' };
