// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// The Thống kê tab of the main window (M16, J), as the approved prototype's
// `st` screens: the tokens counted from this Mac's own session logs for
// Hôm nay (by hour), 7 ngày or 30 ngày; the totals, a column per hour or
// day, and the counts by agent, Claude account, model and project, with
// "Giá API tương đương" only to compare with the subscriptions.
//
// The chart follows the design's rules: one series colour for every bar,
// the agents told apart by a mark beside their name, 4px rounded tops,
// bars at most 24px wide, a number only on the tallest, and each value on
// hover and from the keyboard (one stop in the Tab order, the arrows move
// along it). "Xem bảng" has every number of the chart; the bar lists are
// tables already. Drawn as SVG on the tokens: no chart library, nothing the
// app's CSP would have to allow.
//
// The first read shows each agent's progress; a log in a format the app
// does not know is a notice with the sessions skipped, never a guess.

import { useCallback, useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';
import { Card as Panel, CardTitle } from '@/components/ui/card';
import { Segment, Segmented } from '@/components/ui/segmented';
import { cn } from '@/lib/utils';
import { IdMark } from '../components/accounts';
import { Progress } from '../components/blocks';
import { Empty, GRID, GRID2, TabHead, TabSub, TabTitle } from '../components/page';
import { BannerBox, Button, Dot, LinkButton, Meter } from '../components/parts';
import type { Api } from '../lib/api';
import type { Agent, Model, StatsPeriod, StatsPoint, StatsReport } from '../lib/model';
import {
  accountRows,
  AGENT_NAME,
  AGY_NONE,
  bigTokens,
  chartTitle,
  day,
  inUnit,
  labelEvery,
  modelName,
  PERIODS,
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
  return { period, table, report, setPeriod, toggleTable: () => setTable((t) => !t), reread, ack };
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

const W = 640;
const H = 210;
const PAD_L = 40;
const PAD_B = 22;
const PAD_T = 18;

/** A bar from the baseline up to `top`, its two top corners rounded by 4px. */
function barPath(x: number, w: number, top: number, base: number): string {
  const r = Math.min(4, base - top, w / 2);
  return `M${x},${base}V${top + r}Q${x},${top} ${x + r},${top}H${x + w - r}Q${x + w},${top} ${x + w},${top + r}V${base}Z`;
}

/** The column chart: the total of each hour or day, one colour. */
export function ColumnChart({ points, period }: { points: StatsPoint[]; period: StatsPeriod }) {
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
  const every = labelEvery(n);
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
    <div className="relative">
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
          const t = totals[i];
          const by = y(t);
          return (
            <g key={p.label}>
              {t > 0 ? <path d={barPath(cx - bw / 2, bw, by, PAD_T + ih)} className={cn('fill-series-1', shown === i && 'brightness-110')} aria-hidden="true" /> : null}
              {i === tallest ? (
                <text x={cx} y={by - 5} textAnchor="middle" className="fill-ink text-[10.5px] font-semi tabular-nums" aria-hidden="true">
                  {inUnit(t, unit) + (unit.short ? ' ' + unit.short : '')}
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
          <div className="text-ink2">{(period === 'today' ? 'Giờ ' : 'Ngày ') + tip.label}</div>
          <b className="block text-body tabular-nums">{tokensText(pointTotal(tip))}</b>
          {(['claude', 'codex'] as const).map((a) => (
            <span key={a} className="flex items-center justify-between gap-3 text-ink2 tabular-nums">
              <span className="flex items-center gap-1.5">
                <AgentMark agent={a} />
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

const TABLE = 'w-full border-collapse text-sm [&_td]:border-t-[.5px] [&_td]:border-card-line [&_td]:px-1.5 [&_td]:py-1.5 [&_td]:text-right [&_td]:tabular-nums [&_th]:px-1.5 [&_th]:pb-1 [&_th]:text-right [&_th]:text-xs [&_th]:font-reg [&_th]:text-ink3 [&_td:first-child]:text-left [&_th:first-child]:text-left';

/** "Xem bảng": every number of the chart, in its unit. */
export function PointTable({ report }: { report: StatsReport }) {
  const unit = unitOf(Math.max(0, ...report.points.map(pointTotal)));
  const rows = report.points.filter((p) => pointTotal(p) > 0);
  return (
    <>
      <table className={TABLE}>
        <caption className="sr-only">{chartTitle(report.period)}</caption>
        <thead>
          <tr>
            <th scope="col">{report.period === 'today' ? 'Giờ' : 'Ngày'}</th>
            <th scope="col">Claude Code</th>
            <th scope="col">Codex</th>
            <th scope="col">Tổng</th>
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
            <td>Cộng</td>
            <td>{inUnit(report.claude, unit)}</td>
            <td>{inUnit(report.codex, unit)}</td>
            <td>{inUnit(report.total, unit)}</td>
          </tr>
        </tbody>
      </table>
      <p className="mt-1.5 mb-0 text-xs text-ink2">{unitWords(unit)}{rows.length < report.points.length ? ' · giờ, ngày không có token không ghi' : ''}</p>
    </>
  );
}

function unitWords(unit: Unit): string {
  return unit.long ? unit.long + ' token' : 'token';
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
        <caption className="sr-only">Theo model</caption>
        <thead>
          <tr>
            <th scope="col">Model</th>
            <th scope="col">Agent</th>
            <th scope="col">Vào</th>
            <th scope="col">Ra</th>
            <th scope="col">Đọc cache</th>
            <th scope="col">Giá tương đương</th>
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
              <td>{m.usd == null ? <span className="text-ink2">chưa có giá</span> : usd(m.usd)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="mt-1.5 mb-0 text-xs text-ink2">
        {unitWords(unit)} · bảng giá API ngày {day(report.pricesAsOf)}, cập nhật theo bản app
        {report.unpriced.length ? ' · model chưa có giá không tính vào giá tương đương' : ''}
      </p>
    </>
  );
}

function Tile({ label, children, sub }: { label: ReactNode; children: ReactNode; sub?: ReactNode }) {
  return (
    <Panel className="px-3 py-2.5">
      <small className="flex items-center gap-1.5 text-xs text-ink2">{label}</small>
      <b className="mt-0.5 block text-big font-bold tracking-[-.02em] tabular-nums">{children}</b>
      {sub ? <em className="block text-xs text-ink2 not-italic">{sub}</em> : null}
    </Panel>
  );
}

function Big({ n }: { n: number }) {
  const b = bigTokens(n);
  return (
    <>
      {b.value}
      <i className="ml-[3px] text-sm font-med text-ink2 not-italic">{b.unit}</i>
    </>
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
  const per = PERIODS.find((p) => p.v === stats.period)!;
  const notices = skippedNotices(report);
  const head = (
    <TabHead>
      <TabTitle>Thống kê</TabTitle>
      <TabSub>
        <Dot tone={v.reading ? 'muted' : fresh.tone} />
        {model.machine} · {readLine(v)}
      </TabSub>
      <Segmented variant="inline" aria-label="Khoảng thời gian">
        {PERIODS.map((p) => (
          <Segment key={p.v} pressed={stats.period === p.v} onClick={() => stats.setPeriod(p.v)}>
            {p.t}
          </Segment>
        ))}
      </Segmented>
      <Button disabled={v.reading} onClick={stats.reread}>
        Đọc lại
      </Button>
    </TabHead>
  );

  if (v.reading && v.first) {
    const prog = progressLines(v);
    return (
      <>
        {head}
        <Panel aria-label="Đọc log lần đầu">
          <Progress lines={prog.lines.map(([tone, words]) => [tone, words, null])} busy />
          <Meter percent={prog.percent} tone="" className="[&>i]:bg-accent" />
          <p className="mt-1.5 mb-0 text-xs text-ink2">Đọc lần đầu mất vài phút. Lần sau chỉ đọc phần mới.</p>
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
    <BannerBox tone="warn" title="Chưa lưu được chỉ mục thống kê" body="Số dưới đây đúng cho lần đọc này; lần sau app đọc lại log từ đầu." style={{ margin: 0 }} />
  ) : null;
  const noticeBoxes = notices.map((n, i) => (
    <BannerBox key={n.agent} tone="warn" title={n.title} body={n.body} style={{ margin: 0 }}>
      {i === notices.length - 1 ? (
        <div className="flex gap-1.5">
          <Button onClick={stats.ack}>Đã biết</Button>
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
              <b>Chưa đọc được số</b>Bấm Đọc lại để thử lần nữa.
            </Empty>
          ) : null}
        </Panel>
      </>
    );
  }

  const agentRows: BarRow[] = [
    { key: 'claude', name: AGENT_NAME.claude, mark: <AgentMark agent="claude" />, tokens: report.claude },
    { key: 'codex', name: AGENT_NAME.codex, mark: <AgentMark agent="codex" />, tokens: report.codex },
    { key: 'agy', name: AGENT_NAME.agy, mark: <AgentMark agent="agy" />, tokens: null, none: AGY_NONE },
  ];
  const accounts = accountRows(model, report);
  const projects = projectRows(report);
  return (
    <>
      {head}
      {failedRead}
      {noticeBoxes}
      <p className="m-0 text-xs text-ink2">Đếm token từ log phiên trên máy này. Không đọc nội dung, không gửi đi.</p>
      <div className="grid grid-cols-4 gap-2.5">
        <Tile label={'Tổng · ' + per.per}>
          <Big n={report.total} />
        </Tile>
        <Tile
          label={
            <>
              <AgentMark agent="claude" />
              Claude Code
            </>
          }
          sub={share(report.claude, report.total)}
        >
          <Big n={report.claude} />
        </Tile>
        <Tile
          label={
            <>
              <AgentMark agent="codex" />
              Codex
            </>
          }
          sub={share(report.codex, report.total)}
        >
          <Big n={report.codex} />
        </Tile>
        <Tile label="Giá API tương đương" sub="gói thuê bao: chỉ để so sánh">
          {usd(report.usd)}
        </Tile>
      </div>
      <Panel>
        <CardTitle>
          {chartTitle(stats.period)}
          <span className="ml-auto font-reg">
            <LinkButton small onClick={stats.toggleTable}>
              {stats.table ? 'Xem biểu đồ' : 'Xem bảng'}
            </LinkButton>
          </span>
        </CardTitle>
        {report.total === 0 ? (
          <Empty>
            <b>Chưa có token nào {per.per}</b>Log phiên của Claude Code và Codex trên máy này chưa có số trong khoảng này.
          </Empty>
        ) : stats.table ? (
          <PointTable report={report} />
        ) : (
          <ColumnChart key={stats.period} points={report.points} period={stats.period} />
        )}
      </Panel>
      <div className={GRID2}>
        <Panel>
          <CardTitle>Theo agent</CardTitle>
          <BarTable caption="Theo agent" rows={agentRows} />
        </Panel>
        <Panel>
          <CardTitle>Theo tài khoản Claude</CardTitle>
          {accounts.length ? (
            <BarTable caption="Theo tài khoản Claude" rows={accounts.map((a) => ({ key: a.name, name: a.name, tokens: a.tokens }))} />
          ) : (
            <p className="m-0 text-sm text-ink2">Chưa có token Claude Code {per.per}.</p>
          )}
          <p className="mt-1.5 mb-0 text-xs text-ink2">Gần đúng: ghép giờ của phiên với lịch sử chuyển tài khoản.</p>
        </Panel>
      </div>
      <div className={GRID}>
        <Panel>
          <CardTitle>Theo model</CardTitle>
          {report.models.length ? <ModelTable report={report} /> : <p className="m-0 text-sm text-ink2">Chưa có model nào {per.per}.</p>}
        </Panel>
        <Panel>
          <CardTitle>Theo dự án</CardTitle>
          {projects.length ? (
            <BarTable caption="Theo dự án" rows={projects.map((p) => ({ key: p.path || p.name, name: p.name, title: p.path || undefined, tokens: p.tokens }))} />
          ) : (
            <p className="m-0 text-sm text-ink2">Chưa có dự án nào {per.per}.</p>
          )}
        </Panel>
      </div>
    </>
  );
}
