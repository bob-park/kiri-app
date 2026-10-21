import type { Job, JobState } from '@/domain/queue/apis/queue.dto';

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { hasActive, isIdle, queueSummary } from './queue.ts';

const job = (state: JobState): Job => ({
  id: 1,
  source: { kind: 'youtube', url: 'u' },
  title: 't',
  thumbnail: null,
  duration_secs: null,
  quality_label: '720p',
  options: { format_id: null, preset: 'original', subtitles: [], auto_subtitles: false, max_height: null },
  state,
  progress: 0,
  speed: null,
  eta: null,
  output: null,
  work_dir: null,
  created_at: 0,
});

void test('hasActive only counts running jobs', () => {
  assert.equal(hasActive([job({ kind: 'queued' })]), false);
  assert.equal(hasActive([job({ kind: 'encoding' })]), true);
});

void test('isIdle is false while anything is queued or running', () => {
  assert.equal(isIdle([job({ kind: 'completed' }), job({ kind: 'failed', message: 'x' })]), true);
  assert.equal(isIdle([job({ kind: 'queued' })]), false);
  assert.equal(isIdle([]), true);
});

void test('queueSummary groups running states and drops zero counts', () => {
  assert.deepEqual(queueSummary([]), []);
  assert.deepEqual(
    queueSummary([
      job({ kind: 'downloading' }),
      job({ kind: 'encoding' }),
      job({ kind: 'queued' }),
      job({ kind: 'completed' }),
      job({ kind: 'stopped' }),
    ]),
    [
      ['running', 2],
      ['queued', 1],
      ['completed', 1],
    ],
  );
  assert.deepEqual(queueSummary([job({ kind: 'failed', message: 'x' })]), [['failed', 1]]);
});
