// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// "Tự đánh thức ChatGPT"'s wake times, kept as Rust keeps them, and its
// lines in Vietnamese and English.

import { afterEach, describe, expect, it } from 'vitest';
import { codexTriggerFixture, FIXTURE_NOW } from '../fixtures/models';
import { nextWake, plusFive, runLine, suggestWake, triggerError, triggerLine, wakeErrorText, wakeTimes } from './codexWake';
import { setLang } from './i18n';
import type { CodexTrigger, CodexTriggerRun } from './model';

afterEach(() => setLang('vi'));

const run = (over: Partial<CodexTriggerRun>): CodexTriggerRun => ({ at: '2026-01-16T05:00:00Z', id: 'chatgpt:1', why: 'daily', result: 'started', resetsAt: null, detail: null, ...over });

describe('the wake times', () => {
  it('are kept earliest first, once each, five hours apart over midnight too, as Rust refuses them', () => {
    expect(wakeTimes(['12:00', '07:00'])).toEqual({ times: ['07:00', '12:00'] });
    expect(wakeTimes(['07:00', '12:00', '17:00', '22:00'])).toEqual({ times: ['07:00', '12:00', '17:00', '22:00'] });
    expect(wakeTimes(['07:00', '09:00'])).toEqual({ error: 'gap' });
    expect(wakeTimes(['02:00', '12:00', '22:00'])).toEqual({ error: 'gap' });
    expect(wakeTimes(['07:00', '07:00'])).toEqual({ error: 'same' });
    expect(wakeTimes([])).toEqual({ error: 'empty' });
    expect(wakeTimes(['7:00'])).toEqual({ error: 'at' });
    expect(wakeTimes(['24:00'])).toEqual({ error: 'at' });
  });

  it('offer five hours after the last, and say why one is refused', () => {
    expect(suggestWake(['07:00', '12:00'])).toBe('17:00');
    expect(suggestWake(['21:30'])).toBe('02:30');
    expect(plusFive('07:00')).toBe('12:00');
    expect(wakeErrorText('gap')).toBe('Các mốc phải cách nhau ít nhất 5 giờ.');
    expect(wakeErrorText('same')).toBe('Giờ này đã có.');
    expect(wakeErrorText('empty')).toBe('Cần ít nhất một giờ chạy.');
    expect(wakeErrorText('ids')).toBe('Giờ chưa hợp lệ.');
    setLang('en');
    expect(wakeErrorText('gap')).toBe('Wake times must be at least 5 hours apart.');
    expect(wakeErrorText('empty')).toBe('Keep at least one wake time.');
  });

  it('the next is the first still ahead, tomorrow once all have passed', () => {
    // FIXTURE_NOW is 14:20 at UTC+7.
    expect(nextWake(['06:00', '18:00'], FIXTURE_NOW)).toBe('2026-01-16T11:00:00.000Z');
    expect(nextWake(['06:00', '12:00'], FIXTURE_NOW)).toBe('2026-01-16T23:00:00.000Z');
  });
});

describe('the line the card rests on', () => {
  const tr: CodexTrigger = codexTriggerFixture(false);

  it('off, on, on reset, or waking someone', () => {
    expect(triggerLine(tr, 0, 2, FIXTURE_NOW, null)).toBe('Đang tắt · 06:00, 18:00 · 0/2 tài khoản');
    expect(triggerLine(codexTriggerFixture(true), 3, 4, FIXTURE_NOW, null)).toBe('Lần tới: 18:00 · 3/4 tài khoản');
    expect(triggerLine({ ...tr, on: true, mode: 'reset' }, 2, 2, FIXTURE_NOW, null)).toBe('Chờ đặt lại · 2/2 tài khoản');
    expect(triggerLine(tr, 2, 2, FIXTURE_NOW, 'mây.sáng')).toBe('Đang đánh thức mây.sáng…');
    setLang('en');
    expect(triggerLine(tr, 0, 2, FIXTURE_NOW, null)).toBe('Off · 06:00, 18:00 · 0/2 accounts');
    expect(triggerLine({ ...tr, mode: 'reset' }, 1, 1, FIXTURE_NOW, null)).toBe('Off · On reset · 1/1 account');
    expect(triggerLine({ ...tr, on: true, mode: 'reset' }, 2, 2, FIXTURE_NOW, null)).toBe('Waiting for a reset · 2/2 accounts');
  });

  it('an account\'s last run in a few words, and the newest error of a chosen account', () => {
    expect(runLine(undefined, FIXTURE_NOW)).toBe('Chưa chạy');
    expect(runLine(run({ resetsAt: '2026-01-16T10:00:00Z' }), FIXTURE_NOW)).toBe('12:00 · Đã mở khung, đặt lại 17:00');
    expect(runLine(run({ result: 'soon' }), FIXTURE_NOW)).toBe('12:00 · Bỏ qua · Chưa đủ 5 giờ');
    expect(runLine(run({ result: 'capped' }), FIXTURE_NOW)).toBe('12:00 · Bỏ qua · Đủ 5 lượt trong 24 giờ');
    expect(runLine(run({ result: 'open' }), FIXTURE_NOW)).toBe('12:00 · Bỏ qua · Khung còn mở');
    expect(runLine(run({ result: 'limited' }), FIXTURE_NOW)).toBe('12:00 · Hết hạn mức tuần');
    expect(runLine(run({ result: 'failed', detail: 'timeout' }), FIXTURE_NOW)).toBe('12:00 · Lỗi · timeout');
    expect(triggerError({ ...tr, ids: ['chatgpt:1'], runs: [run({ result: 'failed' })] })?.result).toBe('failed');
    expect(triggerError({ ...tr, ids: ['chatgpt:2'], runs: [run({ result: 'failed' })] })).toBeNull();
    setLang('en');
    expect(runLine(undefined, FIXTURE_NOW)).toBe('Not run yet');
    expect(runLine(run({ result: 'soon' }), FIXTURE_NOW)).toBe('12:00 · Skipped · Under 5 hours');
  });
});
