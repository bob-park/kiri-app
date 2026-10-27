// Type-only imports: erased at runtime, so node --test needs no alias resolution.
import type { Job } from '@/domain/queue/apis/queue.dto';
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

/**
 * Whether a youtube job seen non-failed this session has failed now. Jobs restored already
 * failed are not in `seen`, so they never count. The returned `seen` records every current job.
 */
export function newlyFailedYoutube(
  seen: ReadonlyMap<number, string>,
  jobs: Job[],
): { failed: boolean; seen: Map<number, string> } {
  const failed = jobs.some(
    (j) => j.source.kind === 'youtube' && j.state.kind === 'failed' && seen.has(j.id) && seen.get(j.id) !== 'failed',
  );

  return { failed, seen: new Map(jobs.map((j) => [j.id, j.state.kind])) };
}

export type UpdateOutcome = { kind: 'updated' } | { kind: 'busy' } | { kind: 'failed'; error: string };

/** The hint toast's update result. `null` status: another install or check held the guard, nothing ran. */
export function updateOutcome(s: ToolsStatus | null): UpdateOutcome {
  if (!s) {
    return { kind: 'busy' };
  }

  return s.error ? { kind: 'failed', error: s.error } : { kind: 'updated' };
}
