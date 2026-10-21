import type { UpdateInfo } from '@/domain/updater/apis/updater.dto';

import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  INITIAL_UPDATE_VIEW,
  type UpdatePhase,
  applyFetchedUpdate,
  applyUpdateStatus,
  nextPhase,
  updatePct,
} from './update.ts';

const info = (p: Partial<UpdateInfo>): UpdateInfo => ({
  version: '0.3.0',
  notes: '',
  ready: false,
  downloading: false,
  ...p,
});

void test('nextPhase: found maps backend state', () => {
  assert.equal(nextPhase('idle', { type: 'found', info: info({ downloading: true }) }), 'downloading');
  assert.equal(nextPhase('idle', { type: 'found', info: info({ ready: true }) }), 'ready');
  assert.equal(nextPhase('idle', { type: 'found', info: info({}) }), 'failed');
});

void test('nextPhase: background events advance download phases', () => {
  assert.equal(nextPhase('idle', { type: 'progress' }), 'downloading');
  assert.equal(nextPhase('downloading', { type: 'ready' }), 'ready');
  assert.equal(nextPhase('downloading', { type: 'downloadFailed' }), 'failed');
  assert.equal(nextPhase('failed', { type: 'retry' }), 'downloading');
});

void test('nextPhase: scheduled and installing are not overridden by background events', () => {
  for (const p of ['scheduled', 'installing'] as UpdatePhase[]) {
    assert.equal(nextPhase(p, { type: 'progress' }), p);
    assert.equal(nextPhase(p, { type: 'ready' }), p);
    assert.equal(nextPhase(p, { type: 'downloadFailed' }), p);
    assert.equal(nextPhase(p, { type: 'found', info: info({ ready: true }) }), p);
  }
});

void test('nextPhase: user actions', () => {
  assert.equal(nextPhase('ready', { type: 'install' }), 'installing');
  assert.equal(nextPhase('ready', { type: 'scheduled' }), 'scheduled');
});

void test('nextPhase: a failed install returns to ready only when the file is already downloaded', () => {
  assert.equal(nextPhase('installing', { type: 'installFailed', ready: true }), 'ready');
  assert.equal(nextPhase('installing', { type: 'installFailed', ready: false }), 'failed');
  assert.equal(nextPhase('idle', { type: 'installFailed', ready: false }), 'idle');
});

void test('updatePct is null without a known total', () => {
  assert.equal(updatePct(null), null);
  assert.equal(updatePct({ received: 5, total: null }), null);
  assert.equal(updatePct({ received: 48, total: 100 }), 48);
});

void test('applyUpdateStatus follows update://status events', () => {
  let v = applyUpdateStatus(INITIAL_UPDATE_VIEW, { kind: 'available', ...info({ downloading: true }) });
  assert.equal(v.phase, 'downloading');
  assert.deepEqual(v.info, info({ downloading: true }));

  v = applyUpdateStatus(v, { kind: 'progress', received: 48, total: 100 });
  assert.deepEqual(v.progress, { received: 48, total: 100 });

  v = applyUpdateStatus(v, { kind: 'ready', ...info({ ready: true }) });
  assert.equal(v.phase, 'ready');
  assert.equal(v.progress, null);
  assert.deepEqual(v.info, info({ ready: true }));

  assert.equal(applyUpdateStatus(v, { kind: 'scheduled' }).phase, 'scheduled');
  assert.equal(
    applyUpdateStatus({ ...v, phase: 'downloading' }, { kind: 'download_failed', error: 'x' }).phase,
    'failed',
  );
});

void test('applyUpdateStatus: an install error returns to ready, but a second request (busy) changes nothing', () => {
  const installing = { ...INITIAL_UPDATE_VIEW, info: info({ ready: true }), phase: 'installing' as const };
  assert.equal(applyUpdateStatus(installing, { kind: 'error', error: 'disk full' }).phase, 'ready');
  assert.equal(applyUpdateStatus(installing, { kind: 'error', error: 'busy' }), installing);
});

void test('applyFetchedUpdate fills in a missing update and moves a download forward, never back', () => {
  assert.equal(applyFetchedUpdate(INITIAL_UPDATE_VIEW, null), INITIAL_UPDATE_VIEW);

  const downloading = applyFetchedUpdate(INITIAL_UPDATE_VIEW, info({ downloading: true }));
  assert.equal(downloading.phase, 'downloading');

  // The download finished while no listener was mounted.
  assert.equal(applyFetchedUpdate(downloading, info({ ready: true })).phase, 'ready');

  // A snapshot taken before a newer `ready` event must not roll it back.
  const ready = { ...downloading, info: info({ ready: true }), phase: 'ready' as const };
  assert.equal(applyFetchedUpdate(ready, info({ downloading: true })), ready);
});
