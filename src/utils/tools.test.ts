import type { ToolsStatus } from '@/domain/tools/apis/tools.dto';

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { toolsToast } from './tools.ts';

const st = (p: Partial<ToolsStatus>): ToolsStatus => ({
  ready: false,
  installing: null,
  ytdlp_version: null,
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
