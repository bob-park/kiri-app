// Type-only import: erased at runtime, so node --test needs no alias resolution.
import type { ToolInfo, ToolName, ToolsStatus } from '@/domain/tools/apis/tools.dto';

export type ToolsToast = 'hidden' | 'preparing' | 'failed';

export const TOOL_ORDER: ToolName[] = ['yt-dlp', 'ffmpeg', 'deno'];

export function toolInfo(s: ToolsStatus, name: ToolName): ToolInfo {
  return name === 'yt-dlp' ? s.ytdlp : s[name];
}

export type ToolState = 'update' | 'latest' | 'unknown' | 'missing';

/** A tool row: `latest` stays null until a check succeeds, so `unknown` claims nothing about updates. */
export function toolState({ current, latest }: ToolInfo): ToolState {
  if (latest !== null && latest !== current) {
    return 'update';
  }
  if (current === null) {
    return 'missing';
  }

  return latest === null ? 'unknown' : 'latest';
}

/** Tools whose latest release differs from the installed one, in `TOOL_ORDER`. */
export function pendingUpdates(s: ToolsStatus | null): ToolName[] {
  return s ? TOOL_ORDER.filter((name) => toolState(toolInfo(s, name)) === 'update') : [];
}

/** The summary row: "up to date" only when at least one tool's latest version is known. */
export function toolsSummary(s: ToolsStatus | null): 'updates' | 'upToDate' | 'unknown' {
  if (pendingUpdates(s).length > 0) {
    return 'updates';
  }

  return s && TOOL_ORDER.some((name) => toolInfo(s, name).latest !== null) ? 'upToDate' : 'unknown';
}

/** The main window's tools toast: shown only while a tool really downloads, or when tools are missing after a failure. */
export function toolsToast(s: ToolsStatus | null | undefined): ToolsToast {
  if (!s || s.ready) {
    return 'hidden';
  }
  if (s.installing) {
    return 'preparing';
  }

  return s.error ? 'failed' : 'hidden';
}
