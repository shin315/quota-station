// What the Lịch sử tab shows (F08): the app's own switches, the switcher's
// log and the changes the app saw, a day at a time, newest first. Pure,
// like `view.ts`. Copy follows the prototype's `tabHistory`: accounts by
// the window's names, a short reason, and the switcher's own line on hover.

import { agyNames, claudeNames } from './accounts';
import { clock, pad } from './format';
import type { AgyHow, HistoryEntry, HistorySource, Model, Why } from './model';
import { dayClock, type NameOf } from './switching';
import type { ChipView } from './view';

export type HistoryFilter = 'all' | 'claude' | 'antigravity';

export const HISTORY_FILTERS: { v: HistoryFilter; t: string }[] = [
  { v: 'all', t: 'Tất cả' },
  { v: 'claude', t: 'Claude' },
  { v: 'antigravity', t: 'Antigravity' },
];

export interface HistoryRowView {
  key: string;
  time: string;
  /** The tag before the accounts; `k` is the `.pill` color. */
  pill: { t: string; k: '' | 'sw' | 'g' | 'w' };
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

/** `Hôm nay`, `Hôm qua`, else `Thứ Hai 12/01`. */
export function dayLabel(iso: string, now: Date): string {
  const d = new Date(iso);
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (d.toDateString() === now.toDateString()) return 'Hôm nay';
  if (d.toDateString() === yesterday.toDateString()) return 'Hôm qua';
  return WEEKDAYS[d.getDay()] + ' ' + pad(d.getDate()) + '/' + pad(d.getMonth() + 1);
}

const SOURCES: Record<HistorySource, string> = { app: 'app', switcher: 'switcher', observed: 'quan sát' };

/** The switcher's line without the stamp it starts with. */
function logText(raw: string): string {
  return raw.replace(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} /, '');
}

const AGY_HOW: Record<AgyHow, string> = {
  plain: '',
  quitFirst: ' · thoát app rồi chuyển',
  keepRunning: ' · chuyển khi app vẫn mở',
};

/** Why the switcher switched, short: `5 giờ đã dùng 0% · đặt lại 23:40`. Every percent is the share used. */
function switcherWhy(w: Why, nameOf: NameOf): string {
  switch (w.kind) {
    case 'expiring':
      return 'Rời vòng lúc ' + w.at;
    case 'early':
      return '5 giờ đã dùng ' + Math.round(100 - w.left) + '% · đặt lại ' + w.at + ', sớm hơn ' + nameOf(w.before);
    case 'room':
      return '5 giờ đã dùng ' + Math.round(w.used) + '%' + (w.ends ? ' · đặt lại ' + w.ends : '');
    case 'personal':
      return 'Hết tài khoản Team còn hạn mức · 5 giờ đã dùng ' + Math.round(w.used) + '%';
    case 'park':
      return 'Hết tài khoản còn hạn mức · đặt lại sớm nhất ' + w.at;
    default:
      return 'Switcher chuyển';
  }
}

function why(e: HistoryEntry, nameOf: NameOf, now: Date): string {
  const c = e.cause;
  switch (c.kind) {
    case 'manual': {
      let s = 'Bạn chuyển tay';
      if (c.holdUntil) s += ' · giữ đến ' + dayClock(c.holdUntil, now);
      if (c.result === 'raced') s += ' · một lần chuyển khác ghi ngay sau đó';
      if (c.result === 'unconfirmed') s += ' · chưa đọc lại được để kiểm';
      return s;
    }
    case 'switcher':
      return switcherWhy(c.why, nameOf);
    case 'primed':
      return 'Bắt đầu tính 5 giờ sớm';
    case 'observed':
      return e.app === 'claude' ? 'Đổi giữa hai lần đọc, không do app hay switcher' : 'Đổi giữa hai lần đọc, không do app';
    case 'agySwitch': {
      let s = 'Bạn chuyển trong app' + (AGY_HOW[c.how as AgyHow] ?? '');
      if (c.reopened === false) s += ' · chưa mở lại được app';
      if (c.result === 'unconfirmed') s += ' · agy chưa cho thấy tài khoản mới';
      return s;
    }
    case 'agyOverwrite':
      return 'Antigravity.app đổi về tài khoản cũ (lần ' + c.overwrites + (c.overwrites <= 2 ? '/2' : '') + ')';
  }
}

function pill(e: HistoryEntry): HistoryRowView['pill'] {
  const c = e.cause;
  if (c.kind === 'agyOverwrite') return { t: 'Cảnh báo', k: 'w' };
  if (e.app === 'antigravity') return { t: 'Antigravity', k: 'g' };
  if (c.kind === 'primed') return { t: 'Mồi', k: '' };
  return { t: 'Chuyển', k: 'sw' };
}

/**
 * The window's names for each app's accounts, built from the snapshot now:
 * the email's part before `@` while names show, else Team A, Ultra 1. A slot
 * the snapshot no longer lists goes by its place.
 */
export function historyNames(model: Model): { claude: NameOf; agy: NameOf } {
  const c = claudeNames(model.snapshot.claude.accounts, model.snapshot.claude.personalHours);
  const g = agyNames(model.snapshot.agy.accounts);
  return {
    claude: (n) => c.get(n)?.name ?? 'Tài khoản ' + n,
    agy: (n) => g.get(n)?.name ?? 'Ultra ' + n,
  };
}

export function historyRow(e: HistoryEntry, i: number, model: Model, now: Date): HistoryRowView {
  const names = historyNames(model);
  const claude = names.claude;
  const nameOf: NameOf = e.app === 'claude' ? claude : names.agy;
  const c = e.cause;
  return {
    key: e.at + ':' + i,
    time: clock(e.at),
    pill: pill(e),
    from: e.from != null && c.kind !== 'primed' ? nameOf(e.from) : null,
    to: e.to != null ? nameOf(e.to) : 'tài khoản đã đăng xuất',
    why: why(e, claude, now),
    source: { t: SOURCES[e.source], k: '' },
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
