import type { Toast } from '@/shared/store/toast/toast.state';

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { upsertToast, withLeaving } from './toast.ts';

const toast = (id: number, text = `t${id}`): Toast => ({ id, kind: 'info', text });

void test('upsertToast appends a toast without a matching key', () => {
  assert.deepEqual(upsertToast([toast(1)], { kind: 'info', text: 'new' }, 2), {
    id: 2,
    toasts: [toast(1), { id: 2, kind: 'info', text: 'new' }],
  });
});

void test('upsertToast replaces a toast with the same key in place and keeps its id', () => {
  const first = upsertToast([], { key: 'tools', kind: 'progress', text: 'prep', sticky: true }, 1);
  const second = upsertToast(first.toasts, { kind: 'info', text: 'other' }, 2);
  const third = upsertToast(second.toasts, { key: 'tools', kind: 'error', text: 'failed', sticky: true }, 3);
  assert.equal(third.id, first.id);
  assert.deepEqual(
    third.toasts.map((x) => x.text),
    ['failed', 'other'],
  );
});

void test('withLeaving keeps removed toasts in place, marked leaving, and appends new ones', () => {
  const shown = withLeaving([], [toast(1), toast(2), toast(3)]);
  assert.deepEqual(withLeaving(shown, [toast(1), toast(3), toast(4)]), [
    toast(1),
    { ...toast(2), leaving: true },
    toast(3),
    toast(4),
  ]);
});

void test('withLeaving updates live toasts in place and revives nothing by accident', () => {
  assert.deepEqual(withLeaving([toast(1, 'old')], [toast(1, 'new')]), [toast(1, 'new')]);
  assert.deepEqual(withLeaving([{ ...toast(1), leaving: true }], []), [{ ...toast(1), leaving: true }]);
});
