// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// What the Lịch sử tab shows (F08): the app's own switches, the switcher's
// log and the changes the app saw, a day at a time, newest first. Pure,
// like `view.ts`. Copy follows the prototype's `tabHistory`: accounts by
// the window's names, a short reason, and the switcher's own line on hover.

import { agyNames, claudeAliases, claudeNames } from './accounts';
import { shortName } from './codex';
import { clock, pad } from './format';
import { appName, t } from './i18n';
import type { AgyHow, HistoryEntry, HistorySource, Model, Why } from './model';
import { dayClock, type NameOf } from './switching';
import type { ChipView } from './view';

export type HistoryFilter = 'all' | 'claude' | 'antigravity' | 'codex';

export function historyFilters(): { v: HistoryFilter; t: string }[] {
  return [
    { v: 'all', t: t('Tất cả', 'All') },
    { v: 'claude', t: 'Claude' },
    { v: 'antigravity', t: 'Antigravity' },
    { v: 'codex', t: 'Codex' },
  ];
}

export interface HistoryRowView {
  key: string;
  time: string;
  /** The tag before the accounts; `k` is the `.pill` color. */
  pill: { t: string; k: '' | 'sw' | 'g' | 'x' | 'w' };
  from: string | null;
  to: string;
  why: string;
  source: ChipView;
  /** The switcher's line as it wrote it, on hover. */
  raw: string | null;
}

export interface HistoryDayView {
  label: string;
  rows: HistoryRowView[];
}

const WEEKDAYS = ['Chủ nhật', 'Thứ Hai', 'Thứ Ba', 'Thứ Tư', 'Thứ Năm', 'Thứ Sáu', 'Thứ Bảy'];
const WEEKDAYS_EN = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
const MONTHS_EN = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** `Hôm nay`, `Hôm qua`, else `Thứ Hai 12/01` (`Monday 12 Jan`). */
export function dayLabel(iso: string, now: Date): string {
  const d = new Date(iso);
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (d.toDateString() === now.toDateString()) return t('Hôm nay', 'Today');
  if (d.toDateString() === yesterday.toDateString()) return t('Hôm qua', 'Yesterday');
  return t(
    WEEKDAYS[d.getDay()] + ' ' + pad(d.getDate()) + '/' + pad(d.getMonth() + 1),
    WEEKDAYS_EN[d.getDay()] + ' ' + d.getDate() + ' ' + MONTHS_EN[d.getMonth()],
  );
}

function sourceText(s: HistorySource): string {
  return s === 'observed' ? t('quan sát', 'observed') : s;
}

/** The switcher's line without the stamp it starts with. */
function logText(raw: string): string {
  return raw.replace(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} /, '');
}

function agyHow(how: AgyHow): string {
  if (how === 'quitFirst') return t(' · thoát app rồi chuyển', ' · quit the app, then switched');
  if (how === 'keepRunning') return t(' · chuyển khi app vẫn mở', ' · switched with the app open');
  return '';
}

/** Why the switcher switched, short: `5 giờ đã dùng 0% · đặt lại 23:40`. Every percent is the share used. */
function switcherWhy(w: Why, nameOf: NameOf): string {
  switch (w.kind) {
    case 'expiring':
      return t('Rời vòng lúc ', 'Left rotation at ') + w.at;
    case 'early':
      return t(
        '5 giờ đã dùng ' + Math.round(100 - w.left) + '% · đặt lại ' + w.at + ', sớm hơn ' + nameOf(w.before),
        '5-hour ' + Math.round(100 - w.left) + '% used · resets ' + w.at + ', before ' + nameOf(w.before),
      );
    case 'room':
      return t(
        '5 giờ đã dùng ' + Math.round(w.used) + '%' + (w.ends ? ' · đặt lại ' + w.ends : ''),
        '5-hour ' + Math.round(w.used) + '% used' + (w.ends ? ' · resets ' + w.ends : ''),
      );
    case 'personal':
      return t('Hết tài khoản Team còn hạn mức · 5 giờ đã dùng ', 'No Team account has quota left · 5-hour used ') + Math.round(w.used) + '%';
    case 'park':
      return t('Hết tài khoản còn hạn mức · đặt lại sớm nhất ', 'No account has quota left · earliest reset ') + w.at;
    default:
      return t('Switcher chuyển', 'The switcher switched');
  }
}

function why(e: HistoryEntry, nameOf: NameOf, now: Date): string {
  const c = e.cause;
  switch (c.kind) {
    case 'manual': {
      let s = t('Bạn chuyển tay', 'You switched by hand');
      if (c.holdUntil) s += t(' · giữ đến ', ' · held until ') + dayClock(c.holdUntil, now);
      if (c.result === 'raced') s += t(' · một lần chuyển khác ghi ngay sau đó', ' · another switch wrote right after');
      if (c.result === 'unconfirmed') s += t(' · chưa đọc lại được để kiểm tra', ' · could not read it back to check');
      return s;
    }
    case 'switcher':
      return switcherWhy(c.why, nameOf);
    case 'primed':
      return t('Bắt đầu tính 5 giờ sớm', 'Started the 5-hour window early');
    case 'observed':
      return e.app === 'claude'
        ? t('Đổi giữa hai lần đọc, không do app hay switcher', 'Changed between two reads, not by the app or the switcher')
        : t('Đổi giữa hai lần đọc, không do app', 'Changed between two reads, not by the app');
    case 'agySwitch': {
      let s = t('Bạn chuyển trong app', 'You switched in the app') + agyHow(c.how as AgyHow);
      if (c.reopened === false) s += t(' · chưa mở lại được app', ' · could not reopen the app');
      if (c.result === 'unconfirmed') s += t(' · agy chưa cho thấy tài khoản mới', ' · agy does not show the new account yet');
      return s;
    }
    case 'agyOverwrite':
      return t(
        appName('Antigravity') + ' đổi về tài khoản cũ (lần ' + c.overwrites + (c.overwrites <= 2 ? '/2' : '') + ')',
        appName('Antigravity') + ' went back to the old account (' + (c.overwrites <= 2 ? c.overwrites + ' of 2' : c.overwrites + ' times') + ')',
      );
    case 'codexSwitch':
      return (
        t('Bạn chuyển trong app · giữ đăng nhập ChatGPT', 'You switched in the app · keeps the ChatGPT sign-in') +
        (c.restarted ? t(' · đã khởi động lại tiến trình nền', ' · restarted the background process') : '')
      );
    case 'codexObserved':
      return t('config.toml đổi ngoài app (có thể do CC Switch hoặc sửa tay)', 'config.toml changed outside the app (possibly by CC Switch or by hand)');
  }
}

function pill(e: HistoryEntry): HistoryRowView['pill'] {
  const c = e.cause;
  if (c.kind === 'agyOverwrite') return { t: t('Cảnh báo', 'Alert'), k: 'w' };
  if (e.app === 'antigravity') return { t: 'Antigravity', k: 'g' };
  if (e.app === 'codex') return { t: 'Codex', k: 'x' };
  if (c.kind === 'primed') return { t: t('Khởi tạo sớm', 'Prime'), k: '' };
  return { t: t('Chuyển', 'Switch'), k: 'sw' };
}

/**
 * The window's names for each app's accounts, built from the snapshot now:
 * the email's part before `@` while names show, else Team A, Ultra 1 (with
 * Nâng cao) or Tài khoản 1, Antigravity 1. A slot
 * the snapshot no longer lists goes by its place.
 */
export function historyNames(model: Model): { claude: NameOf; agy: NameOf } {
  const roles = model.advanced.on;
  const c = claudeNames(model.snapshot.claude.accounts, model.snapshot.claude.personalHours, roles, claudeAliases(model));
  const g = agyNames(model.snapshot.agy.accounts, roles);
  return {
    claude: (n) => c.get(n)?.name ?? t('Tài khoản ', 'Account ') + n,
    agy: (n) => g.get(n)?.name ?? (roles ? 'Ultra ' : 'Antigravity ') + n,
  };
}

export function historyRow(e: HistoryEntry, i: number, model: Model, now: Date): HistoryRowView {
  const names = historyNames(model);
  const claude = names.claude;
  const nameOf: NameOf = e.app === 'claude' ? claude : names.agy;
  const c = e.cause;
  if (c.kind === 'codexSwitch' || c.kind === 'codexObserved') {
    return {
      key: e.at + ':' + i,
      time: clock(e.at),
      pill: pill(e),
      from: shortName(model.codex, c.from),
      to: shortName(model.codex, c.to),
      why: why(e, claude, now),
      source: { t: sourceText(e.source), k: '' },
      raw: null,
    };
  }
  return {
    key: e.at + ':' + i,
    time: clock(e.at),
    pill: pill(e),
    from: e.from != null && c.kind !== 'primed' ? nameOf(e.from) : null,
    to: e.to != null ? nameOf(e.to) : t('tài khoản đã đăng xuất', 'signed-out account'),
    why: why(e, claude, now),
    source: { t: sourceText(e.source), k: '' },
    raw: c.kind === 'switcher' || c.kind === 'primed' ? logText(c.raw) : null,
  };
}

/** The entries of `filter`, a day at a time. */
export function historyDays(entries: HistoryEntry[], filter: HistoryFilter, model: Model, now: Date): HistoryDayView[] {
  const days: HistoryDayView[] = [];
  entries
    .filter((e) => filter === 'all' || e.app === filter)
    .forEach((e, i) => {
      const label = dayLabel(e.at, now);
      const last = days[days.length - 1];
      const row = historyRow(e, i, model, now);
      if (last && last.label === label) last.rows.push(row);
      else days.push({ label, rows: [row] });
    });
  return days;
}
