// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// What the Thống kê tab shows (M16, J), from the report the Rust side
// counts: numbers in Vietnamese (a comma before the decimals) and in one
// unit per chart or table, the notices of skipped logs, the first read's
// progress lines, and the Claude accounts by the window's names. Pure,
// like `view.ts`. Copy follows the prototype's `tabStats`.

import { clock, pad } from './format';
import { lang, t } from './i18n';
import type { Agent, Model, StatsPeriod, StatsPoint, StatsReport, StatsView } from './model';
import { namesOf } from './switching';

/** `per` after a label (`Tổng · 7 ngày`), `within` at the end of a sentence (`… in 7 days`). */
export function periods(): { v: StatsPeriod; t: string; per: string; within: string }[] {
  return [
    { v: 'today', t: t('Hôm nay', 'Today'), per: t('hôm nay', 'today'), within: t('hôm nay', 'today') },
    { v: 'week', t: t('7 ngày', '7 days'), per: t('7 ngày', '7 days'), within: t('7 ngày', 'in 7 days') },
    { v: 'month', t: t('30 ngày', '30 days'), per: t('30 ngày', '30 days'), within: t('30 ngày', 'in 30 days') },
  ];
}

export const AGENT_NAME: Record<Agent, string> = { claude: 'Claude Code', codex: 'Codex', agy: 'Antigravity' };

/** Antigravity keeps no token log on the Mac that the app can read. */
export function agyNone(): string {
  return t('chưa có log token trên máy', 'no token log on this Mac');
}

/** The unit numbers are shown in: `long` after a big number, `short` on an axis. */
export interface Unit {
  div: number;
  long: string;
  short: string;
}

function units(): Unit[] {
  return [
    { div: 1e9, long: t('tỷ', 'billion'), short: t('tỷ', 'B') },
    { div: 1e6, long: t('triệu', 'million'), short: t('tr', 'M') },
    { div: 1e3, long: t('nghìn', 'thousand'), short: 'k' },
  ];
}

/** The unit for numbers up to `max`. */
export function unitOf(max: number): Unit {
  return units().find((u) => max >= u.div) ?? { div: 1, long: '', short: '' };
}

const FORMATS = new Map<string, Intl.NumberFormat>();

/** `1234.56` → `1.234,6` with one decimal (`1,234.6` in English). */
export function num(x: number, digits = 1): string {
  const locale = lang() === 'en' ? 'en-US' : 'vi-VN';
  let f = FORMATS.get(locale + digits);
  if (!f) {
    f = new Intl.NumberFormat(locale, { minimumFractionDigits: digits, maximumFractionDigits: digits });
    FORMATS.set(locale + digits, f);
  }
  return f.format(x);
}

/** `token` after a count: English says `tokens` unless it is one. */
function tokenWord(n: number, u: Unit): string {
  return t(u.long ? u.long + ' token' : 'token', (u.long ? u.long + ' ' : '') + (n === 1 ? 'token' : 'tokens'));
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
  return alone(n, u) + ' ' + tokenWord(n, u);
}

/** A big number and its unit, for the tiles: `{ value: '45,5', unit: 'triệu token' }`. */
export function bigTokens(n: number): { value: string; unit: string } {
  const u = unitOf(n);
  return { value: alone(n, u), unit: tokenWord(n, u) };
}

/** `$412`, `$61`, `$3,45`, `$0,08`. */
export function usd(x: number): string {
  return '$' + (x >= 10 ? num(Math.round(x), 0) : num(x, 2));
}

/** A share of a total, rounded: `65%`. */
export function share(part: number, total: number): string {
  return total > 0 ? Math.round((part / total) * 100) + '%' : '0%';
}

const MONTHS_EN = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** `2026-10-19` → `19/10/2026` (`19 Oct 2026`). */
export function day(iso: string): string {
  const [y, m, d] = iso.split('-');
  if (!(d && m && y)) return iso;
  const month = MONTHS_EN[Number(m) - 1];
  return t(d + '/' + m + '/' + y, month ? Number(d) + ' ' + month + ' ' + y : d + '/' + m + '/' + y);
}

/** Unix seconds → `17/10` (`17 Oct`) on this Mac's clock. */
export function dayMonth(secs: number): string {
  const d = new Date(secs * 1000);
  return t(pad(d.getDate()) + '/' + pad(d.getMonth() + 1), d.getDate() + ' ' + MONTHS_EN[d.getMonth()]);
}

/** The line next to the title: `đọc log lúc 14:30`. */
export function readLine(stats: StatsView): string {
  if (stats.reading) return t('đang đọc log…', 'reading logs…');
  return stats.readAt ? t('đọc log lúc ', 'logs read at ') + clock(stats.readAt) : t('chưa đọc log', 'logs not read yet');
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
      const since = s.since != null ? t(' từ ', ' since ') + dayMonth(s.since) : '';
      return {
        agent: s.agent,
        title: t(
          'Bỏ qua ' + num(s.sessions, 0) + ' phiên ' + name,
          'Skipped ' + num(s.sessions, 0) + ' ' + name + (s.sessions === 1 ? ' session' : ' sessions'),
        ),
        body: t(
          'Log của ' + name + versions + ' có dạng app chưa đọc được. Số ' + name + since + ' thiếu các phiên này.',
          name + versions + ' logs are in a format the app cannot read yet. The ' + name + ' numbers' + since + ' leave these sessions out.',
        ),
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
      return ['', name + ' · ' + num(p.total, 0) + t(' file log', p.total === 1 ? ' log file' : ' log files')];
    return ['run', name + ' · ' + num(p.done, 0) + '/' + num(p.total, 0) + t(' file', p.total === 1 ? ' file' : ' files')];
  });
  lines.push(['wait', AGENT_NAME.agy + ' · ' + agyNone()]);
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
  return (period === 'today' ? t('Giờ ', 'Hour ') : t('Ngày ', 'Day ')) + p.label + ': ' + tokensText(pointTotal(p));
}

/** The chart's title: `Token mỗi giờ` for Hôm nay. */
export function chartTitle(period: StatsPeriod): string {
  return period === 'today' ? t('Token mỗi giờ', 'Tokens per hour') : t('Token mỗi ngày', 'Tokens per day');
}

/** Axis labels every `n` columns: each hour of 24 would crowd the axis. */
export function labelEvery(count: number): number {
  return count > 20 ? 5 : count > 12 ? 3 : 1;
}

/** The Claude accounts by the window's names; one the history cannot place is "Chưa rõ". */
export function accountRows(model: Model, report: StatsReport): { name: string; tokens: number }[] {
  const nameOf = namesOf(model);
  return report.accounts.map((a) => ({ name: a.slot == null ? t('Chưa rõ tài khoản', 'Unknown account') : nameOf(a.slot), tokens: a.tokens }));
}

/** The projects shown: the biggest seven, the rest in one row. */
export function projectRows(report: StatsReport): { name: string; path: string; tokens: number }[] {
  const rows = report.projects.map((p) => ({ name: p.name || t('Không rõ thư mục', 'Unknown folder'), path: p.path, tokens: p.tokens }));
  if (rows.length <= 8) return rows;
  const rest = rows.slice(7).reduce((a, r) => a + r.tokens, 0);
  return [...rows.slice(0, 7), { name: t('Các dự án khác', 'Other projects'), path: '', tokens: rest }];
}

/** A model row's name: the id, or a word when the log does not say. */
export function modelName(model: string): string {
  return model || t('Không rõ model', 'Unknown model');
}
