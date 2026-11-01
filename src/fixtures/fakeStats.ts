// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Thống kê tab's fake Rust side (M16), for the tests and the browser
// preview: made-up counts for made-up projects and the fixture's Claude
// accounts, shaped like `stats::report::Report`, on the fixtures' clock
// (16/01/2026, UTC+7). No log is read.

import type { StatsPeriod, StatsPoint, StatsReport, StatsSkipped, StatsView } from '../lib/model';

const M = 1_000_000;

/** 10/01 to 16/01, millions of tokens. */
const WEEK: [number, number][] = [
  [4.1, 1.8],
  [5.6, 2.4],
  [3.2, 2.9],
  [6.8, 1.1],
  [2.4, 0.6],
  [4.9, 3.7],
  [3.9, 2.1],
];
/** 18/12 to 16/01. */
const MONTH = [2.1, 3.4, 4.0, 1.2, 0.8, 5.5, 6.1, 4.4, 3.9, 2.7, 0.4, 1.9, 5.2, 6.6, 4.8, 3.1, 5.9, 7.2, 2.2, 1.0, 6.4, 5.9, 8.0, 6.1, 7.9, 3.0, 8.6, 6.0, 5.9, 6.0];
/** Today by hour, up to 14:00. */
const HOURS = [0, 0, 0, 0, 0, 0, 0, 0.1, 0.4, 0.9, 1.1, 0.7, 0.3, 0.6, 1.2, 0, 0, 0, 0, 0, 0, 0, 0, 0];

const pad = (n: number) => String(n).padStart(2, '0');

function points(period: StatsPeriod): StatsPoint[] {
  if (period === 'today') return HOURS.map((v, h) => ({ label: pad(h) + ':00', claude: Math.round(v * 0.65 * M), codex: Math.round(v * 0.35 * M) }));
  if (period === 'month')
    return MONTH.map((v, i) => {
      const d = new Date(2025, 11, 18 + i);
      return { label: pad(d.getDate()) + '/' + pad(d.getMonth() + 1), claude: Math.round(v * 0.68 * M), codex: Math.round(v * 0.32 * M) };
    });
  return WEEK.map(([c, x], i) => ({ label: pad(10 + i) + '/01', claude: Math.round(c * M), codex: Math.round(x * M) }));
}

export const SKIPPED_NONE: StatsSkipped[] = [
  { agent: 'claude', sessions: 0, lines: 0, versions: [], since: null, acknowledged: 0 },
  { agent: 'codex', sessions: 0, lines: 0, versions: [], since: null, acknowledged: 0 },
];

/** 38 Codex sessions from a version whose log the app cannot read, since 14/01. */
export const SKIPPED_CODEX: StatsSkipped[] = [
  SKIPPED_NONE[0],
  { agent: 'codex', sessions: 38, lines: 412, versions: ['0.170.0'], since: Date.UTC(2026, 0, 14, 2) / 1000, acknowledged: 0 },
];

/** The report for `period`, every number scaled from the week's. */
export function statsReport(period: StatsPeriod, skipped: StatsSkipped[] = SKIPPED_NONE): StatsReport {
  const pts = points(period);
  const claude = pts.reduce((a, p) => a + p.claude, 0);
  const codex = pts.reduce((a, p) => a + p.codex, 0);
  const f = (claude + codex) / (45.5 * M);
  const n = (x: number) => Math.round(x * f * M);
  return {
    period,
    points: pts,
    claude,
    codex,
    total: claude + codex,
    usd: 412 * f,
    unpriced: [],
    models: [
      { model: 'claude-opus-5-5', agent: 'claude', input: n(4.2), cacheRead: n(17.9), output: n(1.1), total: n(23.2), usd: 268 * f },
      { model: 'gpt-5-codex', agent: 'codex', input: n(5.1), cacheRead: n(8.2), output: n(1.3), total: n(14.6), usd: 61 * f },
      { model: 'claude-sonnet-5-5', agent: 'claude', input: n(2.9), cacheRead: n(3.6), output: n(0.8), total: n(7.3), usd: 61 * f },
      { model: 'claude-fable-5-1', agent: 'claude', input: n(0.3), cacheRead: 0, output: n(0.1), total: n(0.4), usd: 22 * f },
    ],
    projects: [
      { name: 'web-shop', path: '~/Projects/web-shop', tokens: n(14.2) },
      { name: 'notes-app', path: '~/Projects/notes-app', tokens: n(9.6) },
      { name: 'infra', path: '~/Projects/infra', tokens: n(6.1) },
      { name: 'docs-site', path: '~/Projects/docs-site', tokens: n(4.4) },
      { name: 'api-server', path: '~/Projects/api-server', tokens: n(3.9) },
    ],
    accounts: [
      { slot: 2, tokens: Math.round(claude * 0.54) },
      { slot: 3, tokens: Math.round(claude * 0.29) },
      { slot: 1, tokens: Math.round(claude * 0.17) },
    ],
    pricesAsOf: '2026-10-19',
    pricesSource: 'anthropic.com/pricing · openai.com/api/pricing',
    skipped,
    readAt: Date.UTC(2026, 0, 16, 7, 30) / 1000,
  };
}

/** Read at 14:30, nothing running. */
export function statsFixture(over: Partial<StatsView> = {}): StatsView {
  return { reading: false, first: false, progress: [], generation: 1, failed: false, readAt: '2026-01-16T07:30:00Z', ...over };
}

/** The first read halfway: Claude Code done, Codex at 640 of 960 logs. */
export function firstReadFixture(): StatsView {
  return statsFixture({
    reading: true,
    first: true,
    readAt: null,
    progress: [
      { agent: 'claude', done: 2140, total: 2140 },
      { agent: 'codex', done: 640, total: 960 },
    ],
  });
}
