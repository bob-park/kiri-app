import assert from 'node:assert/strict';
import { test } from 'node:test';

import { resolveTheme } from './theme.ts';

void test('resolveTheme follows the system when pref is system', () => {
  assert.equal(resolveTheme('system', true), 'dark');
  assert.equal(resolveTheme('system', false), 'light');
});

void test('resolveTheme: an explicit pref wins', () => {
  assert.equal(resolveTheme('dark', false), 'dark');
  assert.equal(resolveTheme('light', true), 'light');
});
