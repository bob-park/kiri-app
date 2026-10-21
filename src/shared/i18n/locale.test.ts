import type { LocalePref } from '@/domain/settings/apis/settings.dto';

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { resolveLocale } from './locale.ts';

void test('resolveLocale follows the OS language for system', () => {
  assert.equal(resolveLocale('system', 'ko-KR'), 'ko');
  assert.equal(resolveLocale('system', 'ja-JP'), 'en');
});

void test('resolveLocale keeps an explicit language', () => {
  assert.equal(resolveLocale('en', 'ko-KR'), 'en');
  assert.equal(resolveLocale('ko', 'en-US'), 'ko');
});

void test('resolveLocale reads a dropped language (ja) as system', () => {
  assert.equal(resolveLocale('ja' as unknown as LocalePref, 'ko-KR'), 'ko');
});
