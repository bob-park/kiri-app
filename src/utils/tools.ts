// Type-only import: erased at runtime, so node --test needs no alias resolution.
import type { ToolInfo, ToolName, ToolsStatus } from '@/domain/tools/apis/tools.dto';

export type ToolsToast = 'hidden' | 'preparing' | 'failed';

export const TOOL_ORDER: ToolName[] = ['yt-dlp', 'ffmpeg', 'deno'];

export function toolInfo(s: ToolsStatus, name: ToolName): ToolInfo {
  return name === 'yt-dlp' ? s.ytdlp : s[name];
}

/** Tools whose latest release differs from the installed one, in `TOOL_ORDER`. */
export function pendingUpdates(s: ToolsStatus | null): ToolName[] {
  if (!s) {
    return [];
  }

  return TOOL_ORDER.filter((name) => {
    const { current, latest } = toolInfo(s, name);

    return latest !== null && latest !== current;
  });
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
