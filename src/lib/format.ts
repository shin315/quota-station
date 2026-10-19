// Numbers, times and durations as the prototype writes them.

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
 * Short time to reset (I12): `3:09` under ten hours, `13g` within a day,
 * `1n 18g` or `5n` beyond. n = ngày (day), g = giờ (hour).
 */
export function resetShort(minutes: number): string {
  const m = Math.max(minutes, 1);
  if (m < 600) return Math.floor(m / 60) + ':' + pad(m % 60);
  if (m < 1440) return Math.floor(m / 60) + 'g';
  const d = Math.floor(m / 1440);
  const h = Math.floor((m % 1440) / 60);
  return h ? d + 'n ' + h + 'g' : d + 'n';
}

/** The same in words, for screen readers: `đặt lại sau 3 giờ 09 phút`. */
export function resetWords(minutes: number): string {
  const m = Math.max(minutes, 1);
  const d = Math.floor(m / 1440);
  const h = Math.floor((m % 1440) / 60);
  const mi = m % 60;
  const parts: string[] = [];
  if (d) parts.push(d + ' ngày');
  if (h) parts.push(h + ' giờ');
  if (mi) parts.push((h ? pad(mi) : String(mi)) + ' phút');
  return 'đặt lại sau ' + parts.join(' ');
}

/** Age as words: `36 giây`, `41 phút`, `2 giờ`, `3 ngày`. */
export function age(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  if (s < 60) return s + ' giây';
  if (s < 3600) return Math.floor(s / 60) + ' phút';
  if (s < 86400) return Math.floor(s / 3600) + ' giờ';
  return Math.floor(s / 86400) + ' ngày';
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
