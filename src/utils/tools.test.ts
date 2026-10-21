import type { ToolsStatus } from '@/domain/tools/apis/tools.dto';

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { toolsToast } from './tools.ts';

const st = (p: Partial<ToolsStatus>): ToolsStatus => ({
  ready: false,
  installing: false,
  ytdlp_version: null,
  error: null,
  last_check: null,
  ...p,
});

void test('toolsToast hides once tools are ready', () => {
  assert.equal(toolsToast(st({ ready: true })), 'hidden');
  assert.equal(toolsToast(st({ ready: true, error: 'old' })), 'hidden');
});

void test('toolsToast shows preparing before status arrives and while installing', () => {
  assert.equal(toolsToast(null), 'preparing');
  assert.equal(toolsToast(st({ installing: true })), 'preparing');
  assert.equal(toolsToast(st({ installing: true, error: 'retrying' })), 'preparing');
});

void test('toolsToast shows failed when an error remains and nothing is installing', () => {
  assert.equal(toolsToast(st({ error: 'offline' })), 'failed');
});
