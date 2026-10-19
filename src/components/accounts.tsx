// How one account and its quota windows are drawn, in the popover and in
// the main window's tables: AccountName, WindowCell, WindowRow, NowStrip,
// NowCard and InUse, from the redesign.
// Every number is the share used: the bars fill as the quota goes.

import { Card, CardTitle } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import type { Agent } from '../lib/model';
import type { CellView, ChipView, NowView, UsedTone } from '../lib/view';
import { AgentIcon } from './logos';
import { Chip, Dot, Meter } from './parts';

/** The service's logo, small: Claude, Antigravity (Gemini), Codex. */
const ID_AGENT: Record<string, Agent> = { c: 'claude', g: 'agy', x: 'codex' };

export function IdMark({ id, className }: { id: string; className?: string }) {
  return <AgentIcon agent={ID_AGENT[id]} className={cn('size-3', className)} />;
}

/** A number's colour when its quota runs short. */
function toneInk(tone: UsedTone | string): string | undefined {
  return tone === 'warn' ? 'text-warn-ink!' : tone === 'bad' ? 'text-bad-ink!' : undefined;
}

/**
 * Where an account's name is shown: its name, a sub-line, and chips.
 * `card`: on a popover card, where the chips go to the right.
 */
export function AccountName(props: { name: string; sub: string; full: string; chips?: ChipView[]; card?: boolean }) {
  return (
    <span className="an grid min-w-0 gap-px">
      <b className="overflow-hidden text-ellipsis whitespace-nowrap text-body font-semi" title={props.full || undefined}>
        {props.name}
      </b>
      {props.sub || props.chips?.length ? (
        <small className={cn('flex min-w-0 items-center gap-[5px] whitespace-nowrap text-xs text-ink2', props.card && 'overflow-hidden text-ellipsis')}>
          {props.sub ? <span className="overflow-hidden text-ellipsis">{props.sub}</span> : null}
          {props.chips?.map((c) => (
            <Chip key={c.t} chip={c} className={cn('flex-none overflow-hidden text-ellipsis', props.card && 'ml-auto')} />
          ))}
        </small>
      ) : null}
    </span>
  );
}

/**
 * One window in a table cell: the share used, a bar, and when it resets.
 * `tight`: Tổng quan's narrower columns, where the reset time is smaller.
 */
export function WindowCell({ cell, dim, tight }: { cell: CellView; dim?: boolean; tight?: boolean }) {
  if (cell.none) {
    return (
      <span className="wc min-w-0 leading-[1.15] tabular-nums" title={cell.title}>
        <b className="block text-sm font-reg text-ink3">—</b>
      </span>
    );
  }
  const tone = dim ? 'muted' : cell.tone;
  return (
    <span className="wc min-w-0 leading-[1.15] tabular-nums" title={cell.title}>
      <b className={cn('block text-sm font-semi', tone === 'warn' ? 'text-warn-ink' : tone === 'bad' ? 'text-bad-ink' : 'text-ink')}>{cell.text}</b>
      <Meter percent={cell.used} tone={tone} className="mt-[3px] mb-0.5 h-(--meter-h-s)" />
      <small className={cn('block overflow-hidden text-ellipsis whitespace-nowrap text-ink2', tight ? 'text-cap' : 'text-xs')}>{cell.reset}</small>
    </span>
  );
}

/** One window on a popover card: its name, a bar, the share used and when it resets. */
export function WindowRow({ cell, wide, dim }: { cell: CellView; wide?: boolean; dim?: boolean }) {
  const tone = dim ? 'muted' : cell.tone;
  return (
    <span
      className={cn(
        'mt-1 grid items-center gap-[7px] text-xs text-ink2 tabular-nums',
        wide ? 'grid-cols-[92px_minmax(0,1fr)_34px_60px]' : 'grid-cols-[52px_minmax(0,1fr)_34px_60px]',
      )}
      title={cell.title}
    >
      <span>{cell.label}</span>
      <Meter percent={cell.used} tone={tone} className="m-0 h-(--meter-h-s)" />
      <b className={cn('text-right', cell.none ? 'font-reg text-ink3' : 'font-semi text-ink', toneInk(tone))}>{cell.text}</b>
      <span className={cn('whitespace-nowrap', cell.none && 'font-reg text-ink3')}>{cell.reset}</span>
    </span>
  );
}

/** The flex name in Đang dùng, as the prototype drew it. */
const NOW_NAME = 'flex min-w-0 flex-wrap items-center gap-[5px] overflow-hidden text-ellipsis whitespace-nowrap font-semi';

/**
 * Đang dùng in the popover: one line per service, the account in use, its
 * share used and when that resets. The window shows on hover.
 */
export function NowStrip({ now }: { now: NowView }) {
  if (!now.rows.length) return null;
  return (
    <div className="mx-2.5 mt-0.5 mb-1.5 grid gap-[3px] rounded-tile bg-fill px-2.5 pt-[5px] pb-1.5" role="list" aria-label="Đang dùng">
      {now.rows.map((r) => (
        <div
          className="grid min-h-[19px] grid-cols-[62px_minmax(0,1fr)_34px_60px] items-center gap-[7px] text-xs text-ink2 tabular-nums"
          role="listitem"
          key={r.id}
          title={r.title}
          aria-label={r.label}
        >
          <span className="inline-flex items-center gap-[5px] text-ink2" aria-hidden="true">
            <IdMark id={r.id} className="m-0" />
            {r.svc}
          </span>
          <span className="flex min-w-0 items-baseline gap-1.5" aria-hidden="true">
            <b className={cn(NOW_NAME, 'text-sm text-ink')}>{r.name}</b>
            {r.extra ? <b className={cn('flex-none whitespace-nowrap font-semi', toneInk(r.extra.tone))}>C/GPT {r.extra.text}</b> : null}
          </span>
          <b className={cn('text-right', r.used == null ? 'font-reg text-ink3' : 'font-semi text-ink', toneInk(r.tone))} aria-hidden="true">
            {r.text}
          </b>
          <span className="overflow-hidden text-ellipsis whitespace-nowrap text-left" aria-hidden="true">
            {r.reset}
          </span>
        </div>
      ))}
    </div>
  );
}

/** Đang dùng on Tổng quan: a tile per service with the number large. */
export function NowCard({ now }: { now: NowView }) {
  if (!now.rows.length) return null;
  return (
    <Card bare>
      <CardTitle bare>Đang dùng</CardTitle>
      <div className="grid auto-cols-[minmax(0,1fr)] grid-flow-col gap-2.5" role="list" aria-label="Đang dùng">
        {now.rows.map((r) => (
          <div className="grid min-w-0 gap-0.5 rounded-tile bg-fill px-3 py-2.5 tabular-nums" role="listitem" key={r.id} title={r.title} aria-label={r.label}>
            <small className="inline-flex items-center gap-1.5 text-xs text-ink2" aria-hidden="true">
              <IdMark id={r.id} className="m-0" />
              {r.svc}
            </small>
            <b className={cn(NOW_NAME, 'mt-1')} aria-hidden="true">
              {r.name}
            </b>
            <small className="overflow-hidden text-ellipsis whitespace-nowrap text-cap text-ink2" aria-hidden="true">
              {r.sub || ' '}
            </small>
            <b className={cn('mt-1.5 text-big leading-[1.1]', r.used == null ? 'font-reg text-ink3' : 'font-bold', toneInk(r.tone))} aria-hidden="true">
              {r.text}
            </b>
            <Meter percent={r.used} tone={r.tone} className="mt-[3px] mb-0.5" />
            <small className="text-cap text-ink2" aria-hidden="true">
              {r.used == null ? r.note : 'đã dùng ' + r.unit + (r.reset ? ' · ' + r.reset : '')}
              {r.extra ? (
                <>
                  {' · '}
                  <b className={toneInk(r.extra.tone)}>C/GPT {r.extra.text}</b>
                </>
              ) : null}
            </small>
          </div>
        ))}
      </div>
    </Card>
  );
}

/** "Đang dùng": a status, not a button. A dot and words, no fill, no border, nothing to press. */
export function InUse() {
  return (
    <span className="inline-flex cursor-default items-center gap-1.5 whitespace-nowrap px-0.5 py-1 text-sm font-semi text-ok-ink">
      <Dot tone="ok" />
      Đang dùng
    </span>
  );
}
