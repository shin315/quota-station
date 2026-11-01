// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import { describe, expect, it } from 'vitest';
import { age, clock, hours, resetShort, resetWords } from './format';

describe('format', () => {
  it('writes time to reset in d, h and m, never as a clock', () => {
    expect(resetShort(4 * 60 + 37)).toBe('4h 37m');
    expect(resetShort(17)).toBe('0h 17m');
    expect(resetShort(189)).toBe('3h 9m');
    expect(resetShort(13 * 60)).toBe('13h 0m');
    expect(resetShort(1439)).toBe('23h 59m');
    expect(resetShort(1440)).toBe('1d 0h');
    expect(resetShort(4 * 1440 + 10 * 60 + 59)).toBe('4d 10h');
    expect(resetShort(3 * 1440 + 5 * 60)).toBe('3d 5h');
    expect(resetShort(0)).toBe('0h 1m');
    expect(resetShort(-5)).toBe('0h 1m');
  });

  it('says the reset in words for screen readers', () => {
    expect(resetWords(189)).toBe('đặt lại sau 3 giờ 09 phút');
    expect(resetWords(41)).toBe('đặt lại sau 41 phút');
    expect(resetWords(1440 + 18 * 60)).toBe('đặt lại sau 1 ngày 18 giờ');
  });

  it('formats ages, local clock and personal hours', () => {
    expect(age(36)).toBe('36 giây');
    expect(age(2460)).toBe('41 phút');
    expect(age(7200)).toBe('2 giờ');
    expect(clock('2026-01-16T10:45:00.000Z')).toBe('17:45');
    expect(hours('8-23')).toBe('08–23');
    expect(hours('nope')).toBeNull();
  });
});
