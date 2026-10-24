// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// How one account and its quota windows are drawn in the main window's
// tables: AccountName, WindowCell, NowCard and InUse.
// The popover draws its own cards (popover/Popover.tsx).
// Every number is the share used: the bars fill as the quota goes.

import { Card, CardTitle } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import type { Agent } from '../lib/model';
import type { CellView, ChipView, NowView, UsedTone } from '../lib/view';
import { AgentIcon } from './logos';
import { t } from '../lib/i18n';
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
 * `card`: on a popover card, where the chips go to the right. `dot`: a
 * table's row, with the dot that is green on the account in use.
 */
export function AccountName(props: { name: string; sub: string; full: string; chips?: ChipView[]; card?: boolean; dot?: 'on' | 'off' }) {
  const name = (
    <span className="an grid min-w-0 gap-px">
      <b className="overflow-hidden text-[15px] font-semi text-ellipsis whitespace-nowrap text-heading" title={props.full || undefined}>
        {props.name}
      </b>
      {props.sub || props.chips?.length ? (
        <small className={cn('flex min-w-0 flex-wrap items-center gap-[5px] text-[13px] text-ink2', props.card && 'flex-nowrap overflow-hidden text-ellipsis whitespace-nowrap')}>
          {props.sub ? <span className="min-w-0 overflow-hidden text-ellipsis">{props.sub}</span> : null}
          {props.chips?.map((c) => (
            <Chip key={c.t} chip={c} className={cn('flex-none overflow-hidden text-ellipsis', props.card && 'ml-auto')} />
          ))}
        </small>
      ) : null}
    </span>
  );
  if (!props.dot) return name;
  return (
    <span className="flex min-w-0 items-start gap-2.5">
      <Dot tone={props.dot === 'on' ? 'ok' : 'off'} className="mt-1.5 size-2.5 flex-none" />
      {name}
    </span>
  );
}

/** One window in a table cell: the share used, a bar under it, and when it resets. */
export function WindowCell({ cell, dim }: { cell: CellView; dim?: boolean; tight?: boolean }) {
  if (cell.none) {
    return (
      <span className="wc min-w-0 tabular-nums" title={cell.title}>
        <b className="block text-body font-reg text-ink3">—</b>
      </span>
    );
  }
  const tone = dim ? 'muted' : cell.tone;
  return (
    <span className="wc grid min-w-0 content-center gap-1 tabular-nums" title={cell.title}>
      <b className={cn('block text-body font-semi', tone === 'warn' ? 'text-warn-ink' : tone === 'bad' ? 'text-bad-ink' : 'text-ink')}>{cell.text}</b>
      <Meter percent={cell.used} tone={tone} className="max-w-[132px]" />
      <small className="block overflow-hidden text-[13px] text-ellipsis whitespace-nowrap text-ink2">{cell.reset}</small>
    </span>
  );
}

/** Đang dùng on Tổng quan: a tile per service, its logo, account and number. */
export function NowCard({ now }: { now: NowView }) {
  if (!now.rows.length) return null;
  return (
    <Card className="pt-3">
      <CardTitle className="mb-2.5 text-[15px] font-semi tracking-normal">{t('Đang dùng', 'In use')}</CardTitle>
      <div className="grid grid-cols-[repeat(auto-fit,minmax(200px,1fr))] gap-4" role="list" aria-label={t('Đang dùng', 'In use')}>
        {now.rows.map((r) => (
          <div
            className="grid min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-3 rounded-card border border-line bg-tile px-3.5 py-2.5 leading-[1.28] tabular-nums [grid-template-areas:'logo_t_big'_'line_line_line']"
            role="listitem"
            key={r.id}
            title={r.title}
            aria-label={r.label}
          >
            <IdMark id={r.id} className="size-[34px] [grid-area:logo]" />
            <span className="grid min-w-0 gap-px [grid-area:t]" aria-hidden="true">
              <b className="text-[15px] font-semi text-heading">{r.svc}</b>
              <span className="overflow-hidden text-body text-ellipsis whitespace-nowrap text-ink2">
                {r.name}
                {r.sub ? <small className="text-[length:inherit] text-ink3"> · {r.sub}</small> : null}
              </span>
            </span>
            <b
              className={cn('text-big leading-[1.1] tracking-head [grid-area:big]', r.used == null ? 'font-reg text-ink3' : 'font-bold text-heading', toneInk(r.tone))}
              aria-hidden="true"
            >
              {r.text}
            </b>
            <small className="justify-self-end text-[13px] whitespace-nowrap text-ink2 [grid-area:line]" aria-hidden="true">
              {r.used == null ? r.note : r.reset ? t('Hạn mức ' + r.unit + ' · đặt lại sau ' + r.reset, r.unit + ' quota · resets in ' + r.reset) : t('Hạn mức ' + r.unit + ' · chưa dùng', r.unit + ' quota · unused')}
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

/** "Đang dùng": a status, not a button. A green dot and words on a quiet pill, nothing to press. */
export function InUse() {
  return (
    <span className="inline-flex cursor-default items-center gap-[7px] rounded-chip bg-passive px-3 py-1.5 text-sm font-med whitespace-nowrap text-passive-ink">
      <Dot tone="ok" />
      {t('Đang dùng', 'In use')}
    </span>
  );
}
