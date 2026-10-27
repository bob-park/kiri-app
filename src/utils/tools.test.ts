import type { ToolsStatus } from '@/domain/tools/apis/tools.dto';

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { pendingUpdates, toolState, toolsSummary, toolsToast } from './tools.ts';

const st = (p: Partial<ToolsStatus>): ToolsStatus => ({
  ready: false,
  installing: null,
  ytdlp: { current: null, latest: null },
  ffmpeg: { current: null, latest: null },
  deno: { current: null, latest: null },
  error: null,
  last_check: null,
  ...p,
});

void test('toolsToast hides once tools are ready, even with a check error', () => {
  assert.equal(toolsToast(st({ ready: true })), 'hidden');
  assert.equal(toolsToast(st({ ready: true, error: 'x' })), 'hidden');
  assert.equal(toolsToast(st({ ready: true, installing: 'yt-dlp' })), 'hidden');
});

void test('toolsToast hides before the first status and while nothing is downloading', () => {
  assert.equal(toolsToast(null), 'hidden');
  assert.equal(toolsToast(st({})), 'hidden');
});

void test('toolsToast shows preparing only during a real download', () => {
  assert.equal(toolsToast(st({ installing: 'yt-dlp' })), 'preparing');
  assert.equal(toolsToast(st({ installing: 'deno', error: 'retrying' })), 'preparing');
});

void test('toolsToast shows failed when an error remains and nothing is downloading', () => {
  assert.equal(toolsToast(st({ installing: null, error: 'x' })), 'failed');
});

void test('pendingUpdates lists tools with a different latest, in order', () => {
  const s = st({
    ytdlp: { current: 'a', latest: 'b' },
    ffmpeg: { current: 'x', latest: 'x' },
    deno: { current: 'd', latest: 'e' },
  });
  assert.deepEqual(pendingUpdates(s), ['yt-dlp', 'deno']);
  assert.deepEqual(pendingUpdates(null), []);
  assert.deepEqual(pendingUpdates(st({ ytdlp: { current: 'a', latest: null } })), []);
});

void test('toolState: update, latest, unknown (latest null) or missing', () => {
  assert.equal(toolState({ current: 'a', latest: 'b' }), 'update');
  assert.equal(toolState({ current: null, latest: 'b' }), 'update');
  assert.equal(toolState({ current: 'a', latest: 'a' }), 'latest');
  assert.equal(toolState({ current: 'a', latest: null }), 'unknown');
  assert.equal(toolState({ current: null, latest: null }), 'missing');
});

void test('toolsSummary claims up to date only when some latest is known', () => {
  assert.equal(toolsSummary(null), 'unknown');
  assert.equal(toolsSummary(st({ ytdlp: { current: 'a', latest: null } })), 'unknown');
  assert.equal(toolsSummary(st({ ytdlp: { current: 'a', latest: 'a' } })), 'upToDate');
  assert.equal(toolsSummary(st({ deno: { current: 'a', latest: 'b' } })), 'updates');
});
