// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Numbers, times and durations as the prototype writes them.

import { t } from './i18n';

export function pad(n: number): string {
  return (n < 10 ? '0' : '') + n;
}

/** Local clock time, `14:20`. */
export function clock(iso: string | null): string {
  if (!iso) return '';
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? '' : pad(d.getHours()) + ':' + pad(d.getMinutes());
}

export function percent(value: number): string {
  return Math.round(value) + '%';
}

/** Whole minutes from `now` until `iso`; null when unknown. */
export function minutesUntil(iso: string | null, now: Date): number | null {
  if (!iso) return null;
  const t = new Date(iso).getTime();
  return Number.isNaN(t) ? null : Math.floor((t - now.getTime()) / 60000);
}

/**
 * Time to reset, in international units with the unit on every number so
 * it never reads as a clock: hours and minutes within a day (`4h 37m`,
 * `0h 17m`), days and hours beyond (`4d 10h`, `5d 0h`), where minutes stop
 * mattering.
 */
export function resetShort(minutes: number): string {
  const m = Math.max(minutes, 1);
  if (m < 1440) return Math.floor(m / 60) + 'h ' + (m % 60) + 'm';
  return Math.floor(m / 1440) + 'd ' + Math.floor((m % 1440) / 60) + 'h';
}

/** `1 hour`, `2 hours`. */
const plural = (n: number, one: string) => n + ' ' + one + (n === 1 ? '' : 's');

/** The same in words, for screen readers: `đặt lại sau 3 giờ 09 phút`, `resets in 3 hours 9 minutes`. */
export function resetWords(minutes: number): string {
  const m = Math.max(minutes, 1);
  const d = Math.floor(m / 1440);
  const h = Math.floor((m % 1440) / 60);
  const mi = m % 60;
  const parts: string[] = [];
  if (d) parts.push(t(d + ' ngày', plural(d, 'day')));
  if (h) parts.push(t(h + ' giờ', plural(h, 'hour')));
  if (mi) parts.push(t((h ? pad(mi) : String(mi)) + ' phút', plural(mi, 'minute')));
  return t('đặt lại sau ', 'resets in ') + parts.join(' ');
}

/** Age as words: `36 giây`, `41 phút`, `2 giờ`, `3 ngày`; `36 sec`, `41 min`, `2 hours`, `3 days`. */
export function age(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  if (s < 60) return t(s + ' giây', s + ' sec');
  if (s < 3600) return t(Math.floor(s / 60) + ' phút', Math.floor(s / 60) + ' min');
  if (s < 86400) return t(Math.floor(s / 3600) + ' giờ', plural(Math.floor(s / 3600), 'hour'));
  return t(Math.floor(s / 86400) + ' ngày', plural(Math.floor(s / 86400), 'day'));
}

export function secondsSince(iso: string | null, now: Date): number | null {
  if (!iso) return null;
  const t = new Date(iso).getTime();
  return Number.isNaN(t) ? null : (now.getTime() - t) / 1000;
}

/** `8-23` → `08–23`, the personal hours chip. */
export function hours(range: string | null): string | null {
  const m = /^(\d{1,2})-(\d{1,2})$/.exec(range ?? '');
  return m ? pad(Number(m[1])) + '–' + pad(Number(m[2])) : null;
}
