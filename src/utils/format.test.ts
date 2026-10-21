import type { Job, JobState } from '@/domain/queue/apis/queue.dto';
import type { Dict } from '@/shared/i18n/en';

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { failureText, formatBytes, formatDuration, jobDetail } from './format.ts';

// A fake dictionary whose leaves read as their key, so expectations stay readable.
const t = {
  quality: { audio: 'quality.audio', original: 'quality.original', best: 'quality.best' },
  preset: { 'mp4-h264': 'preset.mp4-h264', mp3: 'preset.mp3' },
  job: { transcode: 'job.transcode', subs: (langs: string) => `Subs ${langs}` },
  state: { encoding: 'state.encoding' },
  errors: { NO_OUTPUT: 'No file' },
} as unknown as Dict;

const job = (state: JobState, extra: Partial<Job> = {}): Job => ({
  id: 1,
  source: { kind: 'youtube', url: 'u' },
  title: 't',
  thumbnail: null,
  duration_secs: null,
  quality_label: '1080p60',
  options: { format_id: '299', preset: 'mp4-h264', subtitles: ['ko'], auto_subtitles: false, max_height: null },
  state,
  progress: 0.48,
  speed: '2.1 MB/s',
  eta: '00:12',
  output: null,
  work_dir: null,
  created_at: 0,
  ...extra,
});

void test('formats bytes and durations', () => {
  assert.equal(formatBytes(null), '');
  assert.equal(formatBytes(500), '500 B');
  assert.equal(formatBytes(88_000_000), '83.9 MB');
  assert.equal(formatBytes(3 * 1024 ** 3), '3.0 GB');
  assert.equal(formatDuration(144), '2:24');
  assert.equal(formatDuration(3725), '1:02:05');
  assert.equal(formatDuration(null), '');
});

void test('describes a downloading job', () => {
  assert.equal(
    jobDetail(job({ kind: 'downloading' }), t),
    '1080p60 · preset.mp4-h264 · Subs ko · 48% · 2.1 MB/s · 00:12',
  );
});

void test('describes audio, encoding and failure', () => {
  const audio = job(
    { kind: 'encoding' },
    {
      quality_label: 'audio',
      options: { format_id: null, preset: 'mp3', subtitles: [], auto_subtitles: false, max_height: null },
    },
  );
  assert.equal(jobDetail(audio, t), 'quality.audio · preset.mp3 · state.encoding 48%');
  assert.equal(
    jobDetail(job({ kind: 'failed', message: 'ERROR: private' }), t),
    '1080p60 · preset.mp4-h264 · Subs ko · ERROR: private',
  );
});

void test('failureText translates error.* keys and passes raw tool output through', () => {
  assert.equal(failureText('error.no_output', t.errors), 'No file');
  assert.equal(failureText('error.brand_new', t.errors), 'error.brand_new');
  assert.equal(failureText('ERROR: private', t.errors), 'ERROR: private');
});

void test('labels file transcode jobs', () => {
  const f = job(
    { kind: 'encoding' },
    { source: { kind: 'file', path: '/a/clip.mkv', output_dir: null }, quality_label: '720p' },
  );
  assert.equal(jobDetail(f, t), 'job.transcode · 720p · preset.mp4-h264 · Subs ko · state.encoding 48%');
  assert.equal(
    jobDetail({ ...f, quality_label: 'original' }, t),
    'job.transcode · quality.original · preset.mp4-h264 · Subs ko · state.encoding 48%',
  );
});
