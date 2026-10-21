import type { Job, NewJob, VideoInfo } from '@/domain/queue/apis/queue.dto';
import call from '@/shared/api/call';

export async function listJobs() {
  return call<Job[]>('list_jobs');
}

/** Reads the video's title, qualities and subtitles for the options sheet. */
export async function probe(url: string) {
  return call<VideoInfo>('probe', { url });
}

export async function addJob(job: NewJob) {
  return call<Job>('add_job', { job });
}

/** Adds with the settings defaults, skipping the sheet. */
export async function addUrl(url: string) {
  return call<Job>('add_url', { url });
}

export async function stopJob(id: number) {
  return call<void>('stop_job', { id });
}

export async function removeJob(id: number) {
  return call<void>('remove_job', { id });
}

export async function restartJob(id: number) {
  return call<void>('restart_job', { id });
}

/** Returns how many jobs were removed. */
export async function clearCompleted() {
  return call<number>('clear_completed');
}
