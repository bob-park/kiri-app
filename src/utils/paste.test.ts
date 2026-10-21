import assert from 'node:assert/strict';
import { test } from 'node:test';

import { extractUrl } from './paste.ts';

void test('extractUrl finds the first http(s) url', () => {
  assert.equal(extractUrl('https://youtu.be/x'), 'https://youtu.be/x');
  assert.equal(extractUrl('  watch this https://www.youtube.com/watch?v=a then'), 'https://www.youtube.com/watch?v=a');
  assert.equal(extractUrl('# comment\nhttps://youtu.be/y\n'), 'https://youtu.be/y'); // text/uri-list
});

void test('extractUrl returns null without a url', () => {
  assert.equal(extractUrl('hello'), null);
  assert.equal(extractUrl(''), null);
  assert.equal(extractUrl(null), null);
});
