import assert from 'node:assert/strict';
import { test } from 'node:test';

import { arrowStep } from './radio.ts';

void test('arrowStep moves forward and back with wrap-around', () => {
  assert.equal(arrowStep('ArrowRight', 0, 3), 1);
  assert.equal(arrowStep('ArrowDown', 2, 3), 0);
  assert.equal(arrowStep('ArrowLeft', 0, 3), 2);
  assert.equal(arrowStep('ArrowUp', 2, 3), 1);
});

void test('arrowStep jumps with Home/End and ignores other keys', () => {
  assert.equal(arrowStep('Home', 2, 3), 0);
  assert.equal(arrowStep('End', 0, 3), 2);
  assert.equal(arrowStep('Tab', 1, 3), null);
  assert.equal(arrowStep(' ', 1, 3), null);
});
