// Type-only import: erased at runtime, so node --test needs no alias resolution.
import type { Job, JobState } from '@/domain/queue/apis/queue.dto';

export const hasActive = (jobs: Job[]) =>
  jobs.some((j) => j.state.kind === 'downloading' || j.state.kind === 'encoding');

export const isIdle = (jobs: Job[]) => !jobs.some((j) => ['queued', 'downloading', 'encoding'].includes(j.state.kind));

export type SummaryKey = 'running' | 'queued' | 'completed' | 'failed';

/** The status bar's left side. Zero counts are dropped; stopped jobs are not counted. */
export function queueSummary(jobs: Job[]): [SummaryKey, number][] {
  const count = (...kinds: JobState['kind'][]) => jobs.filter((j) => kinds.includes(j.state.kind)).length;
  const all: [SummaryKey, number][] = [
    ['running', count('downloading', 'encoding')],
    ['queued', count('queued')],
    ['completed', count('completed')],
    ['failed', count('failed')],
  ];

  return all.filter(([, n]) => n > 0);
}
