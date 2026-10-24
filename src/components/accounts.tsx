// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// How one account and its quota windows are drawn, in the popover and in
// the main window's tables: AccountName, WindowCell, NowCard and InUse.
// The popover draws its own cards (popover/Popover.tsx).
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

/** The flex name in Đang dùng, as the prototype drew it. */
const NOW_NAME = 'flex min-w-0 flex-wrap items-center gap-[5px] overflow-hidden text-ellipsis whitespace-nowrap font-semi';

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
