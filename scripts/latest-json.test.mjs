import assert from 'node:assert/strict';
import { test } from 'node:test';

import { buildManifest } from './latest-json.mjs';

void test('buildManifest maps the mac .sig to darwin-aarch64', () => {
  const m = buildManifest('v0.2.0', [{ name: 'kiri.app.tar.gz.sig', body: 'SIG\n' }], new Date('2026-10-06T00:00:00Z'));
  assert.deepEqual(m, {
    version: '0.2.0',
    pub_date: '2026-10-06T00:00:00.000Z',
    platforms: {
      'darwin-aarch64': {
        signature: 'SIG',
        url: 'https://github.com/bob-park/kiri-app/releases/download/v0.2.0/kiri.app.tar.gz',
      },
    },
  });
});

void test('buildManifest is empty without signatures', () => {
  assert.deepEqual(buildManifest('v0.2.0', []).platforms, {});
});
