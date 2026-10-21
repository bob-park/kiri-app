import type { Settings } from '@/domain/settings/apis/settings.dto';

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { DEFAULT_SETTINGS, createPendingPatches, mergeSettings } from './settings.ts';

void test('mergeSettings merges nested objects and replaces arrays', () => {
  const base = { ...DEFAULT_SETTINGS, download: { ...DEFAULT_SETTINGS.download, subtitles: ['ko'] } };
  const next = mergeSettings(base, { general: { theme: 'dark' }, download: { subtitles: [] } });
  assert.equal(next.general.theme, 'dark');
  assert.equal(next.general.ui_language, 'system');
  assert.deepEqual(next.download.subtitles, []);
  assert.equal(next.download.max_concurrent, 2);
});

const echo = (theme: Settings['general']['theme'], max = 2): Settings =>
  mergeSettings(DEFAULT_SETTINGS, { general: { theme }, download: { max_concurrent: max } });

void test('an echo of an earlier patch does not overwrite a newer patch still in flight', () => {
  const pending = createPendingPatches();
  pending.begin({ general: { theme: 'dark' } });
  pending.begin({ download: { max_concurrent: 4 } });
  // The backend echoes the first patch only.
  const shown = pending.overlay(echo('dark'));
  assert.equal(shown.general.theme, 'dark');
  assert.equal(shown.download.max_concurrent, 4);
});

void test('pending patches are kept until every patch has settled, then echoes pass through', () => {
  const pending = createPendingPatches();
  pending.begin({ general: { theme: 'dark' } });
  pending.begin({ general: { theme: 'light' } });
  pending.end();
  assert.equal(pending.overlay(echo('dark')).general.theme, 'light');
  pending.end();
  assert.equal(pending.overlay(echo('dark')).general.theme, 'dark');
});
