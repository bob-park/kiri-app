import type { Quality, VideoInfo } from '@/domain/queue/apis/queue.dto';

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { buildNewJob, defaultSubtitles, pickDefaultQuality, rememberPatch, visiblePresets } from './sheet.ts';

const q = (format_id: string, height: number, label: string): Quality => ({
  format_id,
  height,
  fps: null,
  vcodec: 'AVC',
  filesize: null,
  label,
});
const qualities = [q('299', 1080, '1080p60'), q('136', 720, '720p'), q('18', 360, '360p')];
const info: VideoInfo = {
  id: 'abc',
  title: 'Rust',
  channel: 'Fireship',
  duration_secs: 144,
  thumbnail: 'th',
  qualities,
  subtitles: ['en', 'ko'],
  auto_subtitles: ['en', 'ja', 'ko'],
};

void test('pickDefaultQuality mirrors the backend resolution rules', () => {
  assert.equal(pickDefaultQuality(qualities, 'best')?.format_id, '299');
  assert.equal(pickDefaultQuality(qualities, '720p')?.format_id, '136');
  assert.equal(pickDefaultQuality(qualities, '480p')?.format_id, '18');
  assert.equal(pickDefaultQuality(qualities, '144p')?.format_id, '18');
  assert.equal(pickDefaultQuality(qualities, 'audio'), null);
  assert.equal(pickDefaultQuality([], 'best'), null);
});

void test('defaultSubtitles keeps only languages the video has', () => {
  assert.deepEqual(defaultSubtitles(info, ['ko', 'ja', 'fr']), ['ko', 'ja']);
});

void test('buildNewJob: a video job marks auto subs', () => {
  assert.deepEqual(buildNewJob('u', info, qualities[1], 'mp4-h264', ['ko', 'ja']), {
    source: { kind: 'youtube', url: 'u' },
    title: 'Rust',
    thumbnail: 'th',
    duration_secs: 144,
    quality_label: '720p',
    options: { format_id: '136', preset: 'mp4-h264', subtitles: ['ko', 'ja'], auto_subtitles: true, max_height: null },
  });
});

void test('buildNewJob: an audio preset or audio quality drops the video format', () => {
  assert.equal(buildNewJob('u', info, qualities[0], 'mp3', []).options.format_id, null);
  assert.equal(buildNewJob('u', info, null, 'original', []).quality_label, 'audio');
});

void test('rememberPatch stores height-based quality and enables skip_sheet', () => {
  assert.deepEqual(rememberPatch(qualities[0], 'mp4-hevc', ['ko']), {
    download: { quality: '1080p', preset: 'mp4-hevc', subtitles: ['ko'], skip_sheet: true },
  });
  assert.equal(rememberPatch(null, 'mp3', []).download?.quality, 'audio');
});

void test('visiblePresets shows up to three common formats, always including the default, in PRESETS order', () => {
  assert.deepEqual(visiblePresets('original'), ['original', 'mp4-h264', 'mp3']);
  assert.deepEqual(visiblePresets('mp4-h264'), ['original', 'mp4-h264', 'mp3']);
  assert.deepEqual(visiblePresets('webm-vp9'), ['original', 'mp4-h264', 'webm-vp9']);
  assert.deepEqual(visiblePresets('m4a'), ['original', 'mp4-h264', 'm4a']);
});
