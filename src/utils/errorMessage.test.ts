import type { Dict } from '@/shared/i18n/en';

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { errorDetail, errorMessage } from './errorMessage.ts';

const errors = { INVALID_URL: 'Not YouTube', UNKNOWN: 'Oops' } as unknown as Dict['errors'];

void test('errorMessage maps a known code', () => {
  assert.equal(errorMessage({ code: 'INVALID_URL', message: 'not a YouTube URL' }, errors), 'Not YouTube');
});

void test('errorMessage falls back to errors.UNKNOWN for an unknown code', () => {
  assert.equal(errorMessage({ code: 'NEW_CODE', message: 'x' }, errors), 'Oops');
});

void test('errorDetail keeps the raw message only for codes whose text is generic', () => {
  assert.equal(errorDetail({ code: 'FAILED', message: 'disk full' }), 'disk full');
  assert.equal(errorDetail({ code: 'PROBE_FAILED', message: 'ERROR: private video' }), 'ERROR: private video');
  assert.equal(errorDetail({ code: 'UNKNOWN', message: 'boom' }), 'boom');
  assert.equal(errorDetail({ code: 'UNKNOWN', message: '' }), undefined);
  assert.equal(errorDetail({ code: 'INVALID_URL', message: 'not a YouTube URL' }), undefined);
});
