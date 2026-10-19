// What every window shows of one account: its name, and each of its quota
// windows as the share USED (100 − what quota-axi calls percent left).
// Shared by the popover, the main window's tables and the rules editor.

import { hours, minutesUntil, resetShort, resetWords } from './format';
import type { Account, Line, LineKind } from './model';

export type UsedTone = 'ok' | 'warn' | 'bad' | 'muted';

/** Warning colours rise with use: amber from 70 %, red from 90 %. */
export const WARN_USED = 70;
export const BAD_USED = 90;

/** The share used, 0–100; null when the window has no number. */
export function usedOf(line: Pick<Line, 'percent' | 'pending'>): number | null {
  if (line.pending || line.percent == null) return null;
  return Math.min(100, Math.max(0, Math.round(100 - line.percent)));
}

export function usedTone(used: number | null): UsedTone {
  return used == null ? 'muted' : used >= BAD_USED ? 'bad' : used >= WARN_USED ? 'warn' : 'ok';
}

// ===== Windows =====

export interface CellView {
  /** The window it is: the menu bar's choice picks a cell by it. */
  kind: LineKind;
  /** `5 giờ`, `Tuần`, `Fable`, `Gemini 5 giờ`… */
  label: string;
  /** Lower case, after a number: `5 giờ`, `tuần`. */
  unit: string;
  /** Share used; null draws an empty meter and `—`. */
  used: number | null;
  /** `38%`, `≤4%` (only a bound is known), `—`. */
  text: string;
  tone: UsedTone;
  /** `↻ 3:09`, `chưa dùng`, `chờ số`, ''. */
  reset: string;
  /** `↻ 3 ngày 21 giờ`, for the tables. */
  long: string;
  /** No such window for this account (a Team seat has no week). */
  none: boolean;
  /** Hover text. */
  title: string;
  /** For screen readers: `5 giờ đã dùng 38%, đặt lại sau 3 giờ 09 phút`. */
  words: string;
  /** Longer windows win ties for the tightest. */
  rank: number;
}

const LABEL: Record<LineKind, [string, string, number]> = {
  session: ['5 giờ', '5 giờ', 0],
  week: ['Tuần', 'tuần', 1],
  fableWeek: ['Fable', 'Fable', 1],
  gemini: ['Gemini', 'Gemini', 0],
  claudeGpt: ['Claude/GPT', 'Claude/GPT', 0],
  geminiSession: ['Gemini 5 giờ', '5 giờ', 0],
  geminiWeek: ['Gemini tuần', 'tuần', 1],
  claudeGptSession: ['Claude/GPT 5 giờ', '5 giờ', 0],
  claudeGptWeek: ['Claude/GPT tuần', 'tuần', 1],
};

/** `↻ 3 giờ 09 phút` within a day, `↻ 4 ngày 23 giờ` beyond: minutes stop mattering. */
function longReset(minutes: number): string {
  const m = minutes >= 1440 ? minutes - (minutes % 60) : minutes;
  return '↻ ' + resetWords(m).replace('đặt lại sau ', '');
}

export function cellOf(line: Line | undefined, kind: LineKind, now: Date, missing = 'không có'): CellView {
  const [label, unit, rank] = LABEL[kind];
  if (!line) {
    return { kind, label, unit, used: null, text: '—', tone: 'muted', reset: missing, long: missing, none: true, title: missing, words: label + ' ' + missing, rank };
  }
  const used = usedOf(line);
  const mins = minutesUntil(line.resetsAt, now);
  let reset = '';
  let long = '';
  let words = '';
  if (line.pending || (used != null && mins != null && mins <= 0)) {
    reset = long = 'chờ số';
    words = 'đang chờ số mới';
  } else if (mins != null) {
    reset = '↻ ' + resetShort(mins);
    long = longReset(mins);
    words = resetWords(mins);
  } else if (used === 0 && !line.atLeast) {
    reset = long = words = 'chưa dùng';
  }
  // quota-axi knows only a bound for a window another one limits first.
  const bound = used != null && line.atLeast;
  const text = used == null ? '—' : (bound ? '≤' : '') + used + '%';
  return {
    kind,
    label,
    unit,
    used,
    text,
    tone: usedTone(used),
    reset,
    long,
    none: false,
    title: used == null ? label + ': chưa có số' : label + ' đã dùng ' + (bound ? 'nhiều nhất ' : '') + used + '%',
    words: label + ' ' + (used == null ? 'chưa có số' : 'đã dùng ' + (bound ? 'nhiều nhất ' : '') + used + '%') + (words ? ', ' + words : ''),
    rank,
  };
}

/** The window quota-axi's one number for the account stands for, when it has no list of windows. */
function fromLines(a: Account, kind: LineKind): Line | undefined {
  return a.lines.find((l) => l.kind === kind);
}

/** Claude's three columns: 5 giờ · Tuần · Fable tuần. */
export function claudeCells(a: Account, now: Date): CellView[] {
  const has = a.windows.length > 0;
  const pick = (kind: LineKind) => (has ? a.windows.find((l) => l.kind === kind) : fromLines(a, kind));
  return [
    cellOf(pick('session'), 'session', now),
    cellOf(pick('week'), 'week', now, has ? 'không có' : '—'),
    cellOf(pick('fableWeek'), 'fableWeek', now),
  ].map((c) => (c.none && c.label === 'Tuần' && has ? { ...c, title: 'claude-swap không báo hạn mức tuần cho tài khoản này' } : c));
}

export const AGY_KINDS: LineKind[] = ['geminiSession', 'geminiWeek', 'claudeGptSession', 'claudeGptWeek'];

/**
 * Antigravity's four windows. Without them (an account only Cockpit Tools
 * reads) quota-axi has one number per model group: those two instead.
 */
export function agyCells(a: Account, now: Date): CellView[] {
  if (a.windows.length) return AGY_KINDS.map((k) => cellOf(a.windows.find((l) => l.kind === k), k, now));
  return (['gemini', 'claudeGpt'] as LineKind[]).map((k) => cellOf(fromLines(a, k), k, now, '—'));
}

/** The window used the most; a tie names the longer window, which recovers later. */
export function tightest(cells: CellView[]): CellView | null {
  let best: CellView | null = null;
  for (const c of cells) {
    if (c.used == null) continue;
    if (!best || c.used > best.used! || (c.used === best.used && c.rank > best.rank)) best = c;
  }
  return best;
}

/** Antigravity's Gemini and Claude/GPT numbers, each its tighter window. */
export function agyGroups(cells: CellView[]): { label: string; short: string; cell: CellView | null }[] {
  const gemini = cells.filter((c) => c.label.startsWith('Gemini'));
  const other = cells.filter((c) => c.label.startsWith('Claude/GPT'));
  return [
    { label: 'Gemini', short: 'Gemini', cell: tightest(gemini) },
    { label: 'Claude/GPT', short: 'C/GPT', cell: tightest(other) },
  ];
}

// ===== Names =====

export interface AccountName {
  /** `lan.acme1`, or `Team A` while names are hidden. */
  name: string;
  /** The avatar letters of the older layout, still used by the sheets. */
  short: string;
  /** `Acme Studio`, `Cá nhân · 08–23`, ''. */
  sub: string;
  /** The whole email, for hover; '' while names are hidden. */
  full: string;
}

const LETTERS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';

/**
 * Each email's part before `@` when every account here shares one domain
 * (nothing is lost by dropping it), else the whole email.
 */
export function emailNames(emails: (string | undefined)[]): Map<string, string> {
  const known = emails.filter((e): e is string => !!e && e.includes('@'));
  const domains = new Set(known.map((e) => e.slice(e.indexOf('@') + 1).toLowerCase()));
  const out = new Map<string, string>();
  for (const e of known) out.set(e, domains.size === 1 ? e.slice(0, e.indexOf('@')) : e);
  return out;
}

/**
 * Claude accounts: their email when the owner shows names, else Cá nhân,
 * Team A, Team B… by slot order; `Tài khoản N` without a rotation file.
 * The sub-line is the organization of a Team seat, and the hours of the
 * personal account.
 */
export function claudeNames(accounts: Account[], personalHours: string | null = null): Map<number, AccountName> {
  const out = new Map<number, AccountName>();
  const team = accounts.filter((a) => a.role === 'team').map((a) => a.number).sort((x, y) => x - y);
  const emails = emailNames(accounts.map((a) => a.email));
  const h = hours(personalHours);
  for (const a of accounts) {
    let name: string;
    let short: string;
    if (a.number === 0) [name, short] = ['Tài khoản đang dùng', 'C'];
    else if (a.role === 'personal') [name, short] = ['Cá nhân', 'CN'];
    else if (a.role === 'team') {
      const l = LETTERS[team.indexOf(a.number)] ?? String(a.number);
      [name, short] = ['Team ' + l, l];
    } else [name, short] = ['Tài khoản ' + a.number, String(a.number)];
    const email = a.email ? emails.get(a.email) : undefined;
    let sub = '';
    if (email) {
      sub = a.role === 'personal' ? 'Cá nhân' + (h ? ' · ' + h : '') : a.role === 'team' ? (a.org ?? '') : '';
      name = email;
    } else if (a.role === 'personal' && h) sub = h;
    out.set(a.number, { name, short, sub, full: a.email ?? '' });
  }
  return out;
}

/** Antigravity accounts: their email when shown, else Ultra 1, Ultra 2. */
export function agyNames(accounts: Pick<Account, 'number' | 'email'>[]): Map<number, AccountName> {
  const emails = emailNames(accounts.map((a) => a.email));
  const out = new Map<number, AccountName>();
  for (const a of accounts) {
    const email = a.email ? emails.get(a.email) : undefined;
    const base = a.number === 0 ? { name: 'Tài khoản đang dùng', short: 'G' } : { name: 'Ultra ' + a.number, short: 'U' + a.number };
    out.set(a.number, { ...base, name: email ?? base.name, sub: '', full: a.email ?? '' });
  }
  return out;
}
