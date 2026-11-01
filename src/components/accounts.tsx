// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// How one account and its quota windows are drawn in the main window's
// tables: AccountName, WindowCell and InUse.
// The popover draws its own cards (popover/Popover.tsx).
// Every number is the share used: the bars fill as the quota goes.

import { cn } from '@/lib/utils';
import type { Agent } from '../lib/model';
import type { CellView, ChipView } from '../lib/view';
import { AgentIcon } from './logos';
import { t } from '../lib/i18n';
import { Chip, Dot, Meter } from './parts';

/** The service's logo, small: Claude, Antigravity (Gemini), Codex. */
const ID_AGENT: Record<string, Agent> = { c: 'claude', g: 'agy', x: 'codex' };

export function IdMark({ id, className }: { id: string; className?: string }) {
  return <AgentIcon agent={ID_AGENT[id]} className={cn('size-3', className)} />;
}

/**
 * Where an account's name is shown: its name, a sub-line, and chips.
 * `card`: on a popover card, where the chips go to the right.
 */
export function AccountName(props: { name: string; sub: string; full: string; chips?: ChipView[]; card?: boolean }) {
  return (
    <span className="an grid min-w-0 gap-px">
      <b className="overflow-hidden text-body font-semi text-ellipsis whitespace-nowrap" title={props.full || undefined}>
        {props.name}
      </b>
      {props.sub || props.chips?.length ? (
        <small className={cn('flex min-w-0 items-center gap-[5px] overflow-hidden whitespace-nowrap text-xs text-ink2 @max-[640px]/main:flex-wrap', props.card && 'text-ellipsis')}>
          {/* One line: "Cá nhân · 08–23" never breaks inside its value. */}
          {props.sub ? (
            <span className="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap" title={props.sub}>
              {props.sub}
            </span>
          ) : null}
          {props.chips?.map((c) => (
            <Chip key={c.t} chip={c} className={cn('flex-none overflow-hidden text-ellipsis', props.card && 'ml-auto')} />
          ))}
        </small>
      ) : null}
    </span>
  );
}

/** A window's name over its number, only where the table's head row is gone (a narrow page). */
function CellName({ label }: { label: string }) {
  return (
    <small className="hidden truncate text-xs text-ink3 @max-[640px]/main:block" aria-hidden="true">
      {label}
    </small>
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
        <CellName label={cell.label} />
        <b className="block text-sm font-reg text-ink3">—</b>
      </span>
    );
  }
  const tone = dim ? 'muted' : cell.tone;
  return (
    <span className="wc min-w-0 leading-[1.15] tabular-nums" title={cell.title}>
      <CellName label={cell.label} />
      <b className={cn('block text-sm font-semi', tone === 'warn' ? 'text-warn-ink' : tone === 'bad' ? 'text-bad-ink' : 'text-ink')}>{cell.text}</b>
      <Meter percent={cell.used} tone={tone} className="mt-[3px] mb-0.5 h-(--meter-h-s)" />
      <small className={cn('block overflow-hidden text-ellipsis whitespace-nowrap text-ink2', tight ? 'text-cap' : 'text-xs')}>{cell.reset}</small>
    </span>
  );
}

/** "Đang dùng": a status, not a button. A dot and words, no fill, no border, nothing to press. */
export function InUse() {
  return (
    <span className="inline-flex cursor-default items-center gap-1.5 whitespace-nowrap px-0.5 py-1 text-sm font-semi text-ok-ink">
      <Dot tone="ok" />
      {t('Đang dùng', 'In use')}
    </span>
  );
}
