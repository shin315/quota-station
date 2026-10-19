// What the Thống kê tab shows (M16, J), from the report the Rust side
// counts: numbers in Vietnamese (a comma before the decimals) and in one
// unit per chart or table, the notices of skipped logs, the first read's
// progress lines, and the Claude accounts by the window's names. Pure,
// like `view.ts`. Copy follows the prototype's `tabStats`.

import { clock, pad } from './format';
import type { Agent, Model, StatsPeriod, StatsPoint, StatsReport, StatsView } from './model';
import { namesOf } from './switching';

export const PERIODS: { v: StatsPeriod; t: string; per: string }[] = [
  { v: 'today', t: 'Hôm nay', per: 'hôm nay' },
  { v: 'week', t: '7 ngày', per: '7 ngày' },
  { v: 'month', t: '30 ngày', per: '30 ngày' },
];

export const AGENT_NAME: Record<Agent, string> = { claude: 'Claude Code', codex: 'Codex', agy: 'Antigravity' };

/** Antigravity keeps no token log on the Mac that the app can read. */
export const AGY_NONE = 'chưa có log token trên máy';

/** The unit numbers are shown in: `long` after a big number, `short` on an axis. */
export interface Unit {
  div: number;
  long: string;
  short: string;
}

const UNITS: Unit[] = [
  { div: 1e9, long: 'tỷ', short: 'tỷ' },
  { div: 1e6, long: 'triệu', short: 'tr' },
  { div: 1e3, long: 'nghìn', short: 'k' },
];

/** The unit for numbers up to `max`. */
export function unitOf(max: number): Unit {
  return UNITS.find((u) => max >= u.div) ?? { div: 1, long: '', short: '' };
}

const FORMATS = new Map<number, Intl.NumberFormat>();

/** `1234.56` → `1.234,6` with one decimal. */
export function num(x: number, digits = 1): string {
  let f = FORMATS.get(digits);
  if (!f) {
    f = new Intl.NumberFormat('vi-VN', { minimumFractionDigits: digits, maximumFractionDigits: digits });
    FORMATS.set(digits, f);
  }
  return f.format(x);
}

/** A count in `unit`: one decimal, none for whole tokens. */
export function inUnit(n: number, unit: Unit): string {
  return num(n / unit.div, unit.div === 1 ? 0 : 1);
}

/** A count alone in its own unit, a decimal below 100: `45,5 triệu`, `780 nghìn`, `820 token`. */
function alone(n: number, u: Unit): string {
  const v = n / u.div;
  return num(v, u.div === 1 || v >= 100 ? 0 : 1);
}

/** A count in its own unit: `45,5 triệu token`, `820 token`. */
export function tokensText(n: number): string {
  const u = unitOf(n);
  return alone(n, u) + ' ' + (u.long ? u.long + ' token' : 'token');
}

/** A big number and its unit, for the tiles: `{ value: '45,5', unit: 'triệu token' }`. */
export function bigTokens(n: number): { value: string; unit: string } {
  const u = unitOf(n);
  return { value: alone(n, u), unit: u.long ? u.long + ' token' : 'token' };
}

/** `$412`, `$61`, `$3,45`, `$0,08`. */
export function usd(x: number): string {
  return '$' + (x >= 10 ? num(Math.round(x), 0) : num(x, 2));
}

/** A share of a total, rounded: `65%`. */
export function share(part: number, total: number): string {
  return total > 0 ? Math.round((part / total) * 100) + '%' : '0%';
}

/** `2026-10-19` → `19/10/2026`. */
export function day(iso: string): string {
  const [y, m, d] = iso.split('-');
  return d && m && y ? d + '/' + m + '/' + y : iso;
}

/** Unix seconds → `17/10` on this Mac's clock. */
export function dayMonth(secs: number): string {
  const d = new Date(secs * 1000);
  return pad(d.getDate()) + '/' + pad(d.getMonth() + 1);
}

/** The line next to the title: `đọc log lúc 14:30`. */
export function readLine(stats: StatsView): string {
  if (stats.reading) return 'đang đọc log…';
  return stats.readAt ? 'đọc log lúc ' + clock(stats.readAt) : 'chưa đọc log';
}

export interface SkippedNotice {
  agent: Agent;
  title: string;
  body: string;
}

/** The logs a format the app does not know, for each agent with more than "Đã biết" saw. */
export function skippedNotices(report: StatsReport | null): SkippedNotice[] {
  if (!report) return [];
  return report.skipped
    .filter((s) => s.sessions > s.acknowledged)
    .map((s) => {
      const name = AGENT_NAME[s.agent];
      const versions = s.versions.length ? ' ' + s.versions.join(', ') : '';
      const since = s.since != null ? ' từ ' + dayMonth(s.since) : '';
      return {
        agent: s.agent,
        title: 'Bỏ qua ' + num(s.sessions, 0) + ' phiên ' + name,
        body: 'Log của ' + name + versions + ' có dạng app chưa đọc được. Số ' + name + since + ' thiếu các phiên này.',
      };
    });
}

/** The first read's lines: `[tone, words]`; ✓ done, running, or waiting. */
export function progressLines(stats: StatsView): { lines: [string, string][]; percent: number } {
  const lines: [string, string][] = (['claude', 'codex'] as const).map((agent) => {
    const p = stats.progress.find((x) => x.agent === agent);
    const name = AGENT_NAME[agent];
    if (!p) return ['wait', name];
    if (p.done >= p.total && stats.progress[stats.progress.length - 1]?.agent !== agent)
      return ['', name + ' · ' + num(p.total, 0) + ' file log'];
    return ['run', name + ' · ' + num(p.done, 0) + '/' + num(p.total, 0) + ' file'];
  });
  lines.push(['wait', AGENT_NAME.agy + ' · ' + AGY_NONE]);
  const done = stats.progress.reduce((a, p) => a + p.done, 0);
  const total = stats.progress.reduce((a, p) => a + p.total, 0);
  return { lines, percent: total ? Math.round((done / total) * 100) : 0 };
}

/** A column's total. */
export function pointTotal(p: StatsPoint): number {
  return p.claude + p.codex;
}

/** What a column says, for its label and the hover card. */
export function pointWords(p: StatsPoint, period: StatsPeriod): string {
  return (period === 'today' ? 'Giờ ' : 'Ngày ') + p.label + ': ' + tokensText(pointTotal(p));
}

/** The chart's title: `Token mỗi giờ` for Hôm nay. */
export function chartTitle(period: StatsPeriod): string {
  return period === 'today' ? 'Token mỗi giờ' : 'Token mỗi ngày';
}

/** Axis labels every `n` columns: each hour of 24 would crowd the axis. */
export function labelEvery(count: number): number {
  return count > 20 ? 5 : count > 12 ? 3 : 1;
}

/** The Claude accounts by the window's names; one the history cannot place is "Chưa rõ". */
export function accountRows(model: Model, report: StatsReport): { name: string; tokens: number }[] {
  const nameOf = namesOf(model);
  return report.accounts.map((a) => ({ name: a.slot == null ? 'Chưa rõ tài khoản' : nameOf(a.slot), tokens: a.tokens }));
}

/** The projects shown: the biggest seven, the rest in one row. */
export function projectRows(report: StatsReport): { name: string; path: string; tokens: number }[] {
  const rows = report.projects.map((p) => ({ name: p.name || 'Không rõ thư mục', path: p.path, tokens: p.tokens }));
  if (rows.length <= 8) return rows;
  const rest = rows.slice(7).reduce((a, r) => a + r.tokens, 0);
  return [...rows.slice(0, 7), { name: 'Các dự án khác', path: '', tokens: rest }];
}

/** A model row's name: the id, or a word when the log does not say. */
export function modelName(model: string): string {
  return model || 'Không rõ model';
}
