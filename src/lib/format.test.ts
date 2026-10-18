import { describe, expect, it } from 'vitest';
import { age, clock, hours, resetShort, resetWords, room } from './format';

describe('format', () => {
  it('writes time to reset the way the prototype does', () => {
    expect(resetShort(189)).toBe('3:09');
    expect(resetShort(41)).toBe('0:41');
    expect(resetShort(13 * 60)).toBe('13g');
    expect(resetShort(1440 + 18 * 60)).toBe('1n 18g');
    expect(resetShort(5 * 1440)).toBe('5n');
    expect(resetShort(0)).toBe('0:01');
  });

  it('says the reset in words for screen readers', () => {
    expect(resetWords(189)).toBe('đặt lại sau 3 giờ 09 phút');
    expect(resetWords(41)).toBe('đặt lại sau 41 phút');
    expect(resetWords(1440 + 18 * 60)).toBe('đặt lại sau 1 ngày 18 giờ');
  });

  it('rounds room to one decimal with a comma, like the menu bar title', () => {
    expect(room(2.27)).toBe('2,3');
    expect(room(0)).toBe('0,0');
    expect(room(1.25)).toBe('1,3');
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
