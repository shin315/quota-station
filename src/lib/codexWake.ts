// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// "Tự đánh thức ChatGPT" in words (27/10/2026, direction C): its wake
// times, kept as Rust keeps them (earliest first, five hours apart at
// least, over midnight too), the one line it rests on, and each account's
// last run in a few words. The long explanations are the tab's Chi tiết.

import { t } from './i18n';
import { dayClock } from './switching';
import type { CodexTrigger, CodexTriggerRun } from './model';

/** Why a list of wake times is refused, as Rust names it. */
export type WakeError = 'at' | 'same' | 'empty' | 'gap';

/** The least time between two wake times, in minutes. */
const GAP = 5 * 60;
const DAY = 24 * 60;

/** `07:00` → 420; null for anything but a clock. */
function minutes(at: string): number | null {
  const m = /^(\d{2}):(\d{2})$/.exec(at);
  if (!m) return null;
  const [h, mm] = [Number(m[1]), Number(m[2])];
  return h < 24 && mm < 60 ? h * 60 + mm : null;
}

const clockOf = (m: number) => String(Math.floor(m / 60)).padStart(2, '0') + ':' + String(m % 60).padStart(2, '0');

/** The wake times as Rust keeps them, or why they are refused. */
export function wakeTimes(list: string[]): { times: string[] } | { error: WakeError } {
  const all: number[] = [];
  for (const at of list) {
    const m = minutes(at);
    if (m == null) return { error: 'at' };
    all.push(m);
  }
  all.sort((a, b) => a - b);
  if (!all.length) return { error: 'empty' };
  if (all.some((m, i) => i > 0 && m === all[i - 1])) return { error: 'same' };
  const close = all.some((m, i) => i > 0 && m - all[i - 1] < GAP) || (all.length > 1 && all[0] + DAY - all[all.length - 1] < GAP);
  if (close) return { error: 'gap' };
  return { times: all.map(clockOf) };
}

/** What a refused list says under "Giờ chạy". */
export function wakeErrorText(e: string): string {
  switch (e) {
    case 'same':
      return t('Giờ này đã có.', 'This time is already there.');
    case 'empty':
      return t('Cần ít nhất một giờ chạy.', 'Keep at least one wake time.');
    case 'gap':
      return t('Các mốc phải cách nhau ít nhất 5 giờ.', 'Wake times must be at least 5 hours apart.');
    default:
      return t('Giờ chưa hợp lệ.', 'That time is not valid.');
  }
}

/** A time to offer for "Thêm giờ": five hours after the last. */
export function suggestWake(times: string[]): string {
  const last = minutes(times[times.length - 1] ?? '') ?? 0;
  return clockOf((last + GAP) % DAY);
}

/** `07:00` five hours on: when a window opened then resets. */
export function plusFive(at: string): string {
  return clockOf(((minutes(at) ?? 0) + GAP) % DAY);
}

/** The next of `times` after `now`, the Mac's clock, as RFC 3339. */
export function nextWake(times: string[], now: Date): string | null {
  const due = times
    .map((at) => {
      const m = minutes(at);
      if (m == null) return null;
      const d = new Date(now);
      d.setHours(Math.floor(m / 60), m % 60, 0, 0);
      if (d <= now) d.setDate(d.getDate() + 1);
      return d;
    })
    .filter((d): d is Date => d != null)
    .sort((a, b) => a.getTime() - b.getTime())[0];
  return due ? due.toISOString() : null;
}

/** The line the card rests on: off or on, when next, how many accounts; or the account it wakes now. */
export function triggerLine(tr: CodexTrigger, chosen: number, total: number, now: Date, running: string | null): string {
  if (running) return t('Đang đánh thức ' + running + '…', 'Waking ' + running + '…');
  const count = t(chosen + '/' + total + ' tài khoản', chosen + '/' + total + (total === 1 ? ' account' : ' accounts'));
  const when = tr.mode === 'daily' ? tr.times.join(', ') : t('Khi đặt lại', 'On reset');
  if (!tr.on) return [t('Đang tắt', 'Off'), when, count].join(' · ');
  if (tr.mode === 'reset') return [t('Chờ đặt lại', 'Waiting for a reset'), count].join(' · ');
  return [tr.next ? t('Lần tới: ', 'Next: ') + dayClock(tr.next, now) : when, count].join(' · ');
}

/** Why a run sent nothing, in a few words. */
function skipText(r: CodexTriggerRun['result']): string | null {
  switch (r) {
    case 'open':
      return t('Khung còn mở', 'Window still open');
    case 'soon':
      return t('Chưa đủ 5 giờ', 'Under 5 hours');
    case 'capped':
      return t('Đủ 5 lượt trong 24 giờ', '5 runs in 24 hours');
    default:
      return null;
  }
}

/** An account's last run under its name: when, then what it did, in a few words. */
export function runLine(r: CodexTriggerRun | undefined, now: Date): string {
  if (!r) return t('Chưa chạy', 'Not run yet');
  const when = dayClock(r.at, now) + ' · ';
  const skip = skipText(r.result);
  if (skip) return when + t('Bỏ qua · ', 'Skipped · ') + skip;
  switch (r.result) {
    case 'started':
      return when + t('Đã mở khung', 'Window opened') + (r.resetsAt ? t(', đặt lại ', ', resets ') + dayClock(r.resetsAt, now) : '');
    case 'relogin':
      return when + t('Cần đăng nhập lại', 'Must sign in again');
    case 'limited':
      return when + t('Hết hạn mức tuần', 'Week used up');
    default:
      return when + t('Lỗi', 'Error') + (r.detail ? ' · ' + r.detail : '');
  }
}

/** The newest run of a chosen account, when it ended in an error: the card says so while it rests. */
export function triggerError(tr: CodexTrigger): CodexTriggerRun | null {
  const r = tr.runs.find((x) => tr.ids.includes(x.id));
  return r && (r.result === 'failed' || r.result === 'relogin') ? r : null;
}
