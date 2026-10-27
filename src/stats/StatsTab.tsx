// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Thống kê tab of the main window (M16, J), as the approved prototype's
// `st` screens: the tokens counted from this Mac's own session logs for
// Hôm nay (by hour), 7 ngày or 30 ngày; the totals, a column per hour or
// day, and the counts by agent, Claude account, model and project, with
// "Giá API tương đương" only to compare with the subscriptions.
//
// The chart stacks each column by agent (redraw of 25/10/2026): the total
// and who made it in one mark, the agents in their fixed series colours with
// a 2px gap between them and a legend that names each with its total, so a
// colour never stands alone. 4px rounded tops, bars at most 24px wide, a
// number only on the tallest, and each value on hover and from the keyboard
// (one stop in the Tab order, the arrows move along it). "Xem bảng" has
// every number of the chart; the bar lists are tables already. Drawn as SVG
// on the tokens: no chart library, nothing the app's CSP would have to allow.
//
// The first read shows each agent's progress; a log in a format the app
// does not know is a notice with the sessions skipped, never a guess.

import { useCallback, useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';
import { Card as Panel, CardTitle } from '@/components/ui/card';
import { Segment, Segmented } from '@/components/ui/segmented';
import { cn } from '@/lib/utils';
import { IdMark } from '../components/accounts';
import { Progress } from '../components/blocks';
import { CARD_END, Empty, GRID2_EVEN, TabHead, TabNote, TabSub, TabTitle } from '../components/page';
import { BannerBox, Button, Dot, LinkButton, Meter, More } from '../components/parts';
import type { Api } from '../lib/api';
import type { Agent, Model, StatsPeriod, StatsPoint, StatsReport } from '../lib/model';
import {
  accountRows,
  AGENT_NAME,
  agyNone,
  bigTokens,
  chartTitle,
  day,
  inUnit,
  labelEvery,
  modelName,
  periods,
  pointTotal,
  pointWords,
  progressLines,
  projectRows,
  readLine,
  share,
  skippedNotices,
  tokensText,
  unitOf,
  usd,
  type Unit,
} from '../lib/stats';
import { mac, t } from '../lib/i18n';
import type { PopoverView } from '../lib/view';

const MARK: Record<Agent, string> = { claude: 'c', codex: 'x', agy: 'g' };

/** The agent's mark beside its name: the only place an agent has a colour here. */
function AgentMark({ agent }: { agent: Agent }) {
  return <IdMark id={MARK[agent]} className="shrink-0" />;
}

export interface Stats {
  period: StatsPeriod;
  table: boolean;
  report: StatsReport | null | 'failed';
  setPeriod(p: StatsPeriod): void;
  toggleTable(): void;
  reread(): void;
  ack(): void;
}

/** The tab's state: the period, the chart or its table, and the report asked again after each read while the tab is open. */
export function useStats(api: Api, model: Model | null, open: boolean): Stats {
  const [period, setPeriod] = useState<StatsPeriod>('week');
  const [table, setTable] = useState(false);
  const [report, setReport] = useState<StatsReport | null | 'failed'>(null);
  const generation = model?.stats.generation ?? 0;
  useEffect(() => {
    if (!open) return;
    let live = true;
    api.statsReport(period).then(
      (r) => live && setReport(r),
      () => live && setReport((cur) => (cur && cur !== 'failed' ? cur : 'failed')),
    );
    return () => {
      live = false;
    };
  }, [api, period, generation, open]);
  const reread = useCallback(() => void api.statsRead().catch(() => {}), [api]);
  const ack = useCallback(() => void api.statsAck().catch(() => {}), [api]);
  return { period, table, report, setPeriod, toggleTable: () => setTable((on) => !on), reread, ack };
}

/** A step for the axis: 1, 2, 2,5 or 5 times a power of ten, four lines at most. */
function niceStep(max: number): number {
  if (max <= 0) return 1;
  const pow = Math.pow(10, Math.floor(Math.log10(max)));
  for (const m of [0.2, 0.25, 0.5, 1, 2, 2.5, 5, 10]) {
    if (max / (m * pow) <= 4) return m * pow;
  }
  return 10 * pow;
}

/** The chart's width until its box is measured; then it is drawn at the box's own width, so its words keep their size. */
const W_FIRST = 640;
const H = 210;
/** The least room a day's or an hour's label needs under its column. */
const LABEL_ROOM = 36;
const PAD_L = 40;
const PAD_B = 22;
const PAD_T = 18;

/** A bar from the baseline up to `top`, its two top corners rounded by 4px (`round`) or square. */
function barPath(x: number, w: number, top: number, base: number, round = true): string {
  if (!round) return `M${x},${base}V${top}H${x + w}V${base}Z`;
  const r = Math.min(4, base - top, w / 2);
  return `M${x},${base}V${top + r}Q${x},${top} ${x + r},${top}H${x + w - r}Q${x + w},${top} ${x + w},${top + r}V${base}Z`;
}

/** The agents a column stacks, bottom up, each in its fixed series colour. */
const STACK_AGENTS = [
  { agent: 'claude', fill: 'fill-series-1', swatch: 'bg-series-1' },
  { agent: 'codex', fill: 'fill-series-2', swatch: 'bg-series-2' },
] as const;

/** The space between two agents' parts of a column: the card shows through. */
const SEG_GAP = 2;

/** An agent's colour where it is named: a small square, the same as its part of the columns. */
function Swatch({ agent }: { agent: 'claude' | 'codex' }) {
  return <i className={cn('inline-block size-2.5 shrink-0 rounded-[3px]', STACK_AGENTS.find((a) => a.agent === agent)!.swatch)} aria-hidden="true" />;
}

/** The column chart: each hour's or day's total, stacked by agent. */
export function ColumnChart({ points, period }: { points: StatsPoint[]; period: StatsPeriod }) {
  const box = useRef<HTMLDivElement | null>(null);
  const [W, setW] = useState(W_FIRST);
  useEffect(() => {
    const el = box.current;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(() => setW(Math.max(280, Math.round(el.clientWidth))));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const n = points.length;
  const totals = points.map(pointTotal);
  const max = Math.max(0, ...totals);
  const unit = unitOf(max);
  const step = niceStep(max / unit.div);
  const top = Math.ceil(max / unit.div / step) * step || step;
  const ih = H - PAD_B - PAD_T;
  const band = (W - PAD_L) / n;
  const bw = Math.min(24, band * 0.62);
  const y = (v: number) => PAD_T + ih - (v / unit.div / top) * ih;
  const tallest = max > 0 ? totals.indexOf(max) : -1;
  // Fewer labels where the columns are narrow, so they never run into each other.
  const every = Math.max(labelEvery(n), Math.ceil(LABEL_ROOM / band));
  const ticks: number[] = [];
  for (let v = 0; v <= top + 1e-9; v += step) ticks.push(v);

  const [shown, setShown] = useState<number | null>(null);
  // One stop in the Tab order: the newest column with tokens, then wherever the arrows went.
  const [stop, setStop] = useState(() => {
    for (let i = n - 1; i >= 0; i--) if (totals[i] > 0) return i;
    return n - 1;
  });
  const hits = useRef<(SVGRectElement | null)[]>([]);
  const move = (i: number) => {
    const next = Math.max(0, Math.min(n - 1, i));
    setStop(next);
    hits.current[next]?.focus();
  };
  const onKey = (e: KeyboardEvent<SVGRectElement>, i: number) => {
    const to = { ArrowLeft: i - 1, ArrowRight: i + 1, Home: 0, End: n - 1 }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    move(to);
  };

  const label = chartTitle(period);
  const tip = shown != null ? points[shown] : null;
  const tipX = shown != null ? ((PAD_L + band * shown + band / 2) / W) * 100 : 0;
  return (
    <div className="relative" ref={box}>
      <svg viewBox={`0 0 ${W} ${H}`} className="block h-auto w-full overflow-visible" role="group" aria-label={label}>
        {ticks.map((v) => (
          <g key={v} aria-hidden="true">
            <line x1={PAD_L} x2={W} y1={y(v * unit.div)} y2={y(v * unit.div)} className="stroke-grid" strokeWidth={1} />
            <text x={PAD_L - 6} y={y(v * unit.div) + 3.5} textAnchor="end" className="fill-ink2 text-[10.5px] tabular-nums">
              {v === 0 ? '0' : inUnit(v * unit.div, unit) + (unit.short ? ' ' + unit.short : '')}
            </text>
          </g>
        ))}
        {points.map((p, i) => {
          const cx = PAD_L + band * i + band / 2;
          const total = totals[i];
          const by = y(total);
          return (
            <g key={p.label}>
              {total > 0 ? <Stack point={p} x={cx - bw / 2} w={bw} y={y} base={PAD_T + ih} lit={shown === i} /> : null}
              {i === tallest ? (
                <text x={cx} y={by - 5} textAnchor="middle" className="fill-ink text-[10.5px] font-semi tabular-nums" aria-hidden="true">
                  {inUnit(total, unit) + (unit.short ? ' ' + unit.short : '')}
                </text>
              ) : null}
              {i % every === 0 ? (
                <text x={cx} y={H - 6} textAnchor="middle" className="fill-ink2 text-[10.5px] tabular-nums" aria-hidden="true">
                  {p.label}
                </text>
              ) : null}
              <rect
                ref={(el) => {
                  hits.current[i] = el;
                }}
                x={PAD_L + band * i}
                y={PAD_T}
                width={band}
                height={ih}
                className="cursor-default fill-transparent outline-none focus-visible:stroke-accent focus-visible:[stroke-width:1.5]"
                rx={4}
                role="img"
                tabIndex={i === stop ? 0 : -1}
                aria-label={pointWords(p, period) + ', Claude Code ' + tokensText(p.claude) + ', Codex ' + tokensText(p.codex)}
                onPointerEnter={() => setShown(i)}
                onPointerLeave={() => setShown((s) => (s === i ? null : s))}
                onFocus={() => {
                  setStop(i);
                  setShown(i);
                }}
                onBlur={() => setShown((s) => (s === i ? null : s))}
                onKeyDown={(e) => onKey(e, i)}
              />
            </g>
          );
        })}
      </svg>
      {tip ? (
        <div
          className="pointer-events-none absolute -top-1.5 z-[5] min-w-[150px] rounded-lg bg-card px-[9px] py-[7px] text-xs shadow-[var(--sh-sheet),inset_0_0_0_.5px_var(--card-line)]"
          style={{ left: `clamp(0px, calc(${tipX}% - 75px), calc(100% - 150px))` }}
          aria-hidden="true"
          data-testid="chart-tip"
        >
          <div className="text-ink2">{(period === 'today' ? t('Giờ ', 'Hour ') : t('Ngày ', 'Day ')) + tip.label}</div>
          <b className="block text-body tabular-nums">{tokensText(pointTotal(tip))}</b>
          {(['claude', 'codex'] as const).map((a) => (
            <span key={a} className="flex items-center justify-between gap-3 text-ink2 tabular-nums">
              <span className="flex items-center gap-1.5">
                <Swatch agent={a} />
                {AGENT_NAME[a]}
              </span>
              {tokensText(tip[a])}
            </span>
          ))}
        </div>
      ) : null}
    </div>
  );
}

/**
 * One column's parts, Claude Code at the bottom: each part its agent's
 * height, a 2px gap between parts, only the top part rounded.
 */
function Stack({ point, x, w, y, base, lit }: { point: StatsPoint; x: number; w: number; y(v: number): number; base: number; lit: boolean }) {
  const parts = STACK_AGENTS.filter((a) => point[a.agent] > 0);
  let sum = 0;
  return (
    <g className={cn(lit && 'brightness-110')} aria-hidden="true">
      {parts.map((a, i) => {
        const from = y(sum);
        sum += point[a.agent];
        const last = i === parts.length - 1;
        // The part below gives up the gap; a part too thin for it is drawn without.
        const bottom = i > 0 && from - y(sum) > SEG_GAP + 1 ? from - SEG_GAP : from;
        return <path key={a.agent} d={barPath(x, w, Math.min(y(sum), bottom - 1), i === 0 ? base : bottom, last)} className={a.fill} />;
      })}
    </g>
  );
}

/** Under the chart: each agent with its colour, its total and its share; Antigravity, which has no token log. */
function Legend({ report }: { report: StatsReport }) {
  return (
    <ul className="m-0 mt-1.5 flex list-none flex-wrap items-center gap-x-4 gap-y-1 p-0 text-xs text-ink2 tabular-nums" aria-label={t('Chú thích', 'Legend')}>
      {(['claude', 'codex'] as const).map((a) => (
        <li key={a} className="flex items-center gap-1.5">
          <Swatch agent={a} />
          <span className="text-ink">{AGENT_NAME[a]}</span>
          {' · ' + tokensText(report[a]) + ' (' + share(report[a], report.total) + ')'}
        </li>
      ))}
      <li className="flex items-center gap-1.5">
        <AgentMark agent="agy" />
        {AGENT_NAME.agy + ': ' + agyNone()}
      </li>
    </ul>
  );
}

const TABLE = 'w-full border-collapse text-sm [&_td]:border-t-[.5px] [&_td]:border-card-line [&_td]:px-1.5 [&_td]:py-1.5 [&_td]:text-right [&_td]:tabular-nums [&_th]:px-1.5 [&_th]:pb-1 [&_th]:text-right [&_th]:text-xs [&_th]:font-reg [&_th]:text-ink3 [&_td:first-child]:text-left [&_th:first-child]:text-left';

/** "Xem bảng": every number of the chart, in its unit. */
export function PointTable({ report }: { report: StatsReport }) {
  const unit = unitOf(Math.max(0, ...report.points.map(pointTotal)));
  const rows = report.points.filter((p) => pointTotal(p) > 0);
  return (
    <>
      {/* Thirty days or twenty-four hours scroll inside the card, under their head row. */}
      <div className="max-h-[360px] overflow-y-auto overscroll-contain [&_thead_th]:sticky [&_thead_th]:top-0 [&_thead_th]:bg-card">
        <table className={TABLE}>
          <caption className="sr-only">{chartTitle(report.period)}</caption>
          <thead>
            <tr>
              <th scope="col">{report.period === 'today' ? t('Giờ', 'Hour') : t('Ngày', 'Day')}</th>
              <th scope="col">Claude Code</th>
              <th scope="col">Codex</th>
              <th scope="col">{t('Tổng', 'Total')}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((p) => (
              <tr key={p.label}>
                <td>{p.label}</td>
                <td>{inUnit(p.claude, unit)}</td>
                <td>{inUnit(p.codex, unit)}</td>
                <td>{inUnit(pointTotal(p), unit)}</td>
              </tr>
            ))}
            <tr className="font-semi">
              <td>{t('Cộng', 'Sum')}</td>
              <td>{inUnit(report.claude, unit)}</td>
              <td>{inUnit(report.codex, unit)}</td>
              <td>{inUnit(report.total, unit)}</td>
            </tr>
          </tbody>
        </table>
      </div>
      <p className="mt-1.5 mb-0 text-xs text-ink2">
        {unitWords(unit)}
        {rows.length < report.points.length ? t(' · giờ, ngày không có token không ghi', ' · hours and days with no tokens are left out') : ''}
      </p>
    </>
  );
}

function unitWords(unit: Unit): string {
  return t(unit.long ? unit.long + ' token' : 'token', unit.long ? unit.long + ' tokens' : 'tokens');
}

interface BarRow {
  key: string;
  name: string;
  title?: string;
  mark?: ReactNode;
  tokens: number | null;
  /** Instead of a number: why there is none. */
  none?: string;
}

/** A list of bars that is a table: the name, a bar, the number in its own unit. */
export function BarTable({ caption, rows }: { caption: string; rows: BarRow[] }) {
  const max = Math.max(0, ...rows.map((r) => r.tokens ?? 0));
  return (
    <table className="w-full border-collapse text-sm">
      <caption className="sr-only">{caption}</caption>
      <tbody>
        {rows.map((r) => (
          <tr key={r.key}>
            <th scope="row" className="w-[124px] max-w-[124px] py-1 pr-2.5 text-left font-reg" title={r.title}>
              <span className="flex min-w-0 items-center gap-1.5">
                {r.mark}
                <span className="truncate">{r.name}</span>
              </span>
            </th>
            <td className="py-1">
              {r.tokens == null ? (
                <span className="text-xs text-ink2">{r.none}</span>
              ) : (
                <span className="flex min-w-0 items-center gap-1.5">
                  <i
                    className="block h-3 min-w-0.5 rounded-r-[4px] bg-series-1"
                    style={{ width: (max > 0 ? Math.max(1, (r.tokens / max) * 78) : 1) + '%' }}
                    aria-hidden="true"
                  />
                  <span className="whitespace-nowrap text-xs tabular-nums">{tokensText(r.tokens)}</span>
                </span>
              )}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function ModelTable({ report }: { report: StatsReport }) {
  const unit = unitOf(Math.max(0, ...report.models.map((m) => Math.max(m.input, m.output, m.cacheRead))));
  return (
    <>
      <table className={cn(TABLE, '[&_td:nth-child(2)]:text-left [&_td:nth-child(2)]:text-ink2 [&_th:nth-child(2)]:text-left')}>
        <caption className="sr-only">{t('Theo model', 'By model')}</caption>
        <thead>
          <tr>
            <th scope="col">Model</th>
            <th scope="col">Agent</th>
            <th scope="col">
              {t('Vào', 'In')}
              <sup aria-hidden="true">1</sup>
              <span className="sr-only">{t(', gồm ghi cache', ', cache writes included')}</span>
            </th>
            <th scope="col">{t('Ra', 'Out')}</th>
            <th scope="col">{t('Đọc cache', 'Cache read')}</th>
            <th scope="col">{t('Giá API tương đương', 'Equivalent API price')}</th>
          </tr>
        </thead>
        <tbody>
          {report.models.map((m) => (
            <tr key={m.agent + m.model}>
              <td className="[overflow-wrap:anywhere]">{modelName(m.model)}</td>
              <td>{AGENT_NAME[m.agent]}</td>
              <td>{inUnit(m.input, unit)}</td>
              <td>{inUnit(m.output, unit)}</td>
              <td>{inUnit(m.cacheRead, unit)}</td>
              <td>{m.usd == null ? <span className="text-ink2">{t('chưa có giá', 'no price yet')}</span> : usd(m.usd)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="mt-1.5 text-xs text-ink2">
        {capital(unitWords(unit)) + t(' · ¹ gồm ghi cache · giá API ' + day(report.pricesAsOf), ' · ¹ cache writes included · API prices of ' + day(report.pricesAsOf))}
        {report.unpriced.length ? t(' · model chưa có giá không tính vào giá tương đương', ' · models with no price are left out of the equivalent price') : ''}
        {' · '}
        <More small about={t('Theo model', 'By model')}>
          <p>
            {t(
              'Vào gồm token vào và token ghi cache; Ra gồm cả token suy luận; Đọc cache là token đọc lại từ cache. Tổng của trang cộng cả bốn loại.',
              'In counts input and cache-write tokens; Out includes reasoning tokens; Cache read is tokens read back from the cache. The page’s total adds all four.',
            )}
          </p>
          <p>{priceWords(report)}</p>
        </More>
      </div>
    </>
  );
}

/** `triệu token` → `Triệu token`, at the start of a line. */
function capital(words: string): string {
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** How "Giá API tương đương" is worked out, behind its "Chi tiết". */
function priceWords(report: StatsReport): string {
  return t(
    'Giá nhân số token từng loại (vào, ghi cache, đọc cache, ra) với bảng giá API ngày ' +
      day(report.pricesAsOf) +
      ' có trong bản app. Đây không phải số tiền đã trả và không quy đổi phí thuê bao; model chưa có giá không tính vào.',
    'Each kind of token (input, cache write, cache read, output) is multiplied by the API prices of ' +
      day(report.pricesAsOf) +
      ' that come with the app. It is not what was paid and does not convert a subscription fee; models with no price are left out.',
  );
}

/** One of the totals over the chart: its name, the number large, and its words. */
function Total({ label, value, unit, children }: { label: string; value: string; unit?: string; children?: ReactNode }) {
  return (
    <div className="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5">
      <span className="text-sm text-ink2">{label}</span>
      <b className="text-big leading-[1.1] font-bold tracking-[-.02em] tabular-nums">{value}</b>
      {unit ? <span className="text-sm font-med text-ink2">{unit}</span> : null}
      {children ? <span className="text-sm text-ink2">{children}</span> : null}
    </div>
  );
}

export interface StatsTabProps {
  model: Model;
  fresh: PopoverView['fresh'];
  stats: Stats;
}

export function StatsTab({ model, fresh, stats }: StatsTabProps) {
  const v = model.stats;
  const report = stats.report === 'failed' ? null : stats.report;
  const all = periods();
  const per = all.find((p) => p.v === stats.period)!;
  const notices = skippedNotices(report);
  const head = (
    <TabHead>
      <TabTitle>{t('Thống kê', 'Stats')}</TabTitle>
      <TabSub>
        <Dot tone={v.reading ? 'muted' : fresh.tone} />
        {model.machine} · {readLine(v)}
      </TabSub>
      <Segmented variant="inline" aria-label={t('Khoảng thời gian', 'Time range')}>
        {all.map((p) => (
          <Segment key={p.v} pressed={stats.period === p.v} onClick={() => stats.setPeriod(p.v)}>
            {p.t}
          </Segment>
        ))}
      </Segmented>
      <Button disabled={v.reading} onClick={stats.reread}>
        {t('Đọc lại', 'Read again')}
      </Button>
    </TabHead>
  );

  const note = (
    <TabNote>
      {mac(t('Token từ log trên máy · không lưu nội dung hội thoại', 'Tokens from this Mac’s logs · no conversation is kept'))}
      {' · '}
      <More about={t('Thống kê', 'Stats')}>
        <p>
          {mac(
            t(
              'App chỉ lấy số token, model, thời gian và thư mục dự án từ log phiên của Claude Code và Codex trên máy này. Nội dung hội thoại không được lưu, ghi lại hay hiện ra, và không có gì được gửi đi.',
              'The app takes only the token counts, the model, the time and the project folder from Claude Code’s and Codex’s session logs on this Mac. No conversation is kept, logged or shown, and nothing is sent anywhere.',
            ),
          )}
        </p>
        <p>
          {t(
            'Lần đọc sau chỉ đọc phần log mới. Dòng log ở dạng app chưa biết thì bỏ qua và báo, không đoán. Antigravity chưa có log token mà app đọc được, nên không có số, không phải 0.',
            'Later reads take only what is new. A log line in a format the app does not know is skipped and reported, never guessed. Antigravity keeps no token log the app can read, so it has no number, not 0.',
          )}
        </p>
        <p>{t('Số đã làm tròn nên cộng lại có thể lệch 0,1.', 'Numbers are rounded, so their sum may be off by 0.1.')}</p>
      </More>
    </TabNote>
  );

  if (v.reading && v.first) {
    const prog = progressLines(v);
    return (
      <>
        {head}
        <Panel aria-label={t('Đọc log lần đầu', 'Reading the logs for the first time')}>
          <Progress lines={prog.lines.map(([tone, words]) => [tone, words, null])} busy />
          <Meter percent={prog.percent} tone="" className="[&>i]:bg-accent" />
          <p className="mt-1.5 mb-0 text-xs text-ink2">{t('Đọc lần đầu mất vài phút. Lần sau chỉ đọc phần mới.', 'The first read takes a few minutes. Later reads only take what is new.')}</p>
        </Panel>
        <div className="pointer-events-none opacity-50" aria-hidden="true">
          <Panel>
            <CardTitle>{chartTitle(stats.period)}</CardTitle>
            <ColumnChart points={report?.points ?? []} period={stats.period} />
          </Panel>
        </div>
      </>
    );
  }

  const failedRead = v.failed ? (
    <BannerBox
      tone="warn"
      title={t('Chưa lưu được chỉ mục thống kê', 'Could not save the stats index')}
      body={t('Số dưới đây đúng cho lần đọc này; lần sau app đọc lại log từ đầu.', 'The numbers below are right for this read; next time the app reads the logs from the start.')}
      style={{ margin: 0 }}
    />
  ) : null;
  const noticeBoxes = notices.map((n, i) => (
    <BannerBox key={n.agent} tone="warn" title={n.title} body={n.body} style={{ margin: 0 }}>
      {i === notices.length - 1 ? (
        <div className="flex gap-1.5">
          <Button onClick={stats.ack}>{t('Đã biết', 'Got it')}</Button>
        </div>
      ) : null}
    </BannerBox>
  ));

  if (stats.report === 'failed' || !report) {
    return (
      <>
        {head}
        <Panel>
          {stats.report === 'failed' ? (
            <Empty>
              <b>{t('Chưa đọc được số', 'Could not read the numbers')}</b>
              {t('Bấm Đọc lại để thử lần nữa.', 'Press Read again to try once more.')}
            </Empty>
          ) : null}
        </Panel>
      </>
    );
  }

  const accounts = accountRows(model, report);
  const projects = projectRows(report);
  const big = bigTokens(report.total);
  return (
    <>
      {head}
      {note}
      {failedRead}
      {noticeBoxes}
      {/* The two totals in one line: the tokens, and what they would cost at API prices. */}
      <div className="flex flex-wrap items-center gap-x-5 gap-y-1.5 px-0.5" role="group" aria-label={t('Tổng', 'Totals')}>
        <Total label={t('Tổng · ', 'Total · ') + per.per} value={big.value} unit={big.unit} />
        <span className="h-6 w-px bg-card-line @max-[640px]/main:hidden" aria-hidden="true" />
        <Total label={t('Giá API tương đương', 'Equivalent API price')} value={usd(report.usd)}>
          {t('không phải số tiền đã trả', 'not what was paid')}
          {' · '}
          <More about={t('Giá API tương đương', 'Equivalent API price')}>
            <p>{priceWords(report)}</p>
          </More>
        </Total>
      </div>
      <Panel>
        <CardTitle>
          {chartTitle(stats.period) + t(' · theo agent', ' · by agent')}
          <span className={CARD_END}>
            <LinkButton small onClick={stats.toggleTable}>
              {stats.table ? t('Xem biểu đồ', 'Show chart') : t('Xem bảng', 'Show table')}
            </LinkButton>
          </span>
        </CardTitle>
        {report.total === 0 ? (
          <Empty>
            <b>{t('Chưa có token nào ' + per.within, 'No tokens ' + per.within)}</b>
            {mac(
              t(
                'Log phiên của Claude Code và Codex trên máy này chưa có số trong khoảng này.',
                'The Claude Code and Codex session logs on this Mac have no numbers for this range.',
              ),
            )}
          </Empty>
        ) : stats.table ? (
          <PointTable report={report} />
        ) : (
          <>
            <ColumnChart key={stats.period} points={report.points} period={stats.period} />
            <Legend report={report} />
          </>
        )}
      </Panel>
      {/* The two rankings side by side, the same size, ending on one line. */}
      <div className={GRID2_EVEN}>
        <Panel className="flex flex-col">
          <CardTitle>{t('Theo tài khoản Claude', 'By Claude account')}</CardTitle>
          {accounts.length ? (
            <BarTable caption={t('Theo tài khoản Claude', 'By Claude account')} rows={accounts.map((a) => ({ key: a.name, name: a.name, tokens: a.tokens }))} />
          ) : (
            <p className="m-0 text-sm text-ink2">{t('Chưa có token Claude Code ' + per.within + '.', 'No Claude Code tokens ' + per.within + '.')}</p>
          )}
          <div className="mt-auto pt-1.5 text-xs text-ink2">
            {t('Ước tính theo lịch sử chuyển', 'Estimated from the switch history')}
            {' · '}
            <More small about={t('Theo tài khoản Claude', 'By Claude account')}>
              <p>
                {t(
                  'Log được gom theo lát 15 phút. Token của mỗi lát tính cho tài khoản đang dùng ở giữa lát theo lịch sử chuyển; thiếu mốc thì vào “Chưa rõ tài khoản”.',
                  'The logs are counted in quarter hours. Each quarter hour’s tokens go to the account in use at its middle by the switch history; with no switch to go by, they go to “Unknown account”.',
                )}
              </p>
              <p>
                {t(
                  'Nhiều phiên chạy cùng lúc trên các tài khoản khác nhau, hoặc đăng nhập đổi ngoài app, có thể bị tính nhầm tài khoản.',
                  'Sessions running at once on different accounts, or a sign-in changed outside the app, may be counted to the wrong account.',
                )}
              </p>
            </More>
          </div>
        </Panel>
        <Panel className="flex flex-col">
          <CardTitle>{t('Theo dự án', 'By project')}</CardTitle>
          {projects.length ? (
            <BarTable caption={t('Theo dự án', 'By project')} rows={projects.map((p) => ({ key: p.path || p.name, name: p.name, title: p.path || undefined, tokens: p.tokens }))} />
          ) : (
            <p className="m-0 text-sm text-ink2">{t('Chưa có dự án nào ' + per.within + '.', 'No projects ' + per.within + '.')}</p>
          )}
        </Panel>
      </div>
      <Panel>
        <CardTitle>{t('Theo model', 'By model')}</CardTitle>
        {report.models.length ? <ModelTable report={report} /> : <p className="m-0 text-sm text-ink2">{t('Chưa có model nào ' + per.within + '.', 'No models ' + per.within + '.')}</p>}
      </Panel>
    </>
  );
}
