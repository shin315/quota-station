// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// Pieces the agent and backup dialogs share: the lines a write changes (a
// diff), a command the app runs, the steps of a flow, the small print under
// them, and the lists to tick.

import type { ReactNode } from 'react';
import { cn } from '@/lib/utils';
import { Checkbox } from '@/components/ui/checkbox';
import { t } from '../lib/i18n';
import { CODE } from './parts';

/** The small print under a preview: what happens around it. */
export const TECH = 'mt-1.5 mb-0 text-xs text-ink2';

/** How a line of a diff is drawn: kept, added (+), removed (−), or the gap between hunks. */
export type LineOp = 'ctx' | 'add' | 'del' | 'gap';

const LINE: Record<LineOp, string> = {
  ctx: 'text-ink2',
  add: "bg-ok/13 before:text-ok-ink before:content-['+']",
  del: "bg-bad/11 before:text-bad-ink before:content-['−']",
  gap: 'text-ink3',
};

/**
 * A diff: a head with the file (or what is compared) and a word at the right,
 * then the lines; a file read whole has no head. In a dialog it scrolls past
 * 46% of the window's height. `flat`: inside a group that is already a
 * panel, so no panel of its own, set in under the group's row.
 */
export function DiffView({ head, right, flat, children }: { head?: ReactNode; right?: ReactNode; flat?: boolean; children: ReactNode }) {
  return (
    <div
      className={cn(
        'mt-0 mb-2 overflow-hidden rounded-tile bg-fill font-mono text-[11.5px]/[1.55] in-data-[kind=dialog]:max-h-[46vh] in-data-[kind=dialog]:overflow-auto',
        flat && 'ml-[26px] rounded-none border-t-[.5px] border-card-line bg-transparent [&>div]:px-0 [&_pre>span]:pr-0',
      )}
    >
      {head !== undefined ? (
        <div className="flex items-center gap-2 border-b-[.5px] border-card-line px-2.5 py-[5px] font-sans text-xs/[1.4] font-semi text-ink2">
          {head}
          <span className="ml-auto font-reg">{right}</span>
        </div>
      ) : null}
      <pre className="m-0 py-1.5 whitespace-pre-wrap [overflow-wrap:anywhere]">{children}</pre>
    </div>
  );
}

/** The file a diff is about, in the head. */
export function DiffFile({ children }: { children: ReactNode }) {
  return <span className={CODE}>{children}</span>;
}

/** One line of a diff; `n`: its number, shown before it; `lit` marks it (a line of a file read whole has no `op`). */
export function DiffLine({ op, n, lit, children }: { op?: LineOp; n?: number; lit?: boolean; children: ReactNode }) {
  return (
    <span
      data-n={n}
      data-op={op}
      className={cn(
        'relative block pr-2.5 pl-[22px] text-ink before:absolute before:left-2 before:text-ink3',
        op && LINE[op],
        n !== undefined && 'pl-[38px] before:left-1.5 before:w-[22px] before:text-right before:content-[attr(data-n)]',
        lit && 'bg-warn/18',
      )}
    >
      {children}
    </span>
  );
}

/** A diff line's tag that a key was there and is hidden. */
export function KeyMasked({ children }: { children?: ReactNode }) {
  return <span className="ml-1.5 whitespace-nowrap rounded-chip bg-accent/14 px-1.5 font-sans text-cap text-info-ink">{children ?? t('khoá, đã che', 'key, hidden')}</span>;
}

/** A command the app runs, after a `$`. */
export function Command({ children }: { children: ReactNode }) {
  return (
    <div className="mt-0 mb-2 flex items-start gap-2 rounded-tile bg-fill px-2.5 py-[7px] font-mono text-[11.5px]/[1.5]">
      <span className="text-ink3" aria-hidden="true">
        $
      </span>
      <code className="flex-1 [overflow-wrap:anywhere]">{children}</code>
    </div>
  );
}

/** The steps of a flow, the one at hand lit, those done ticked. */
export function Steps({ names, at }: { names: string[]; at: number }) {
  return (
    <ol className="mt-0 mb-3 flex list-none flex-wrap gap-1 p-0 text-xs text-ink2" aria-label={t('Các bước', 'Steps')}>
      {names.map((t, i) => (
        <li
          key={t}
          className={cn('flex items-center gap-[5px] rounded-chip bg-fill py-0.5 pr-2 pl-[3px]', i === at && 'bg-(--sel) font-semi text-info-ink')}
          aria-current={i === at ? 'step' : undefined}
        >
          <i
            className={cn(
              'grid size-4 place-items-center rounded-full bg-fill2 text-[10px] font-heavy not-italic',
              i === at && 'bg-accent text-on-accent',
              i < at && 'bg-ok/22 text-ok-ink',
            )}
          >
            {i < at ? '✓' : i + 1}
          </i>
          {t}
        </li>
      ))}
    </ol>
  );
}

/** A checkbox and its words; pressing the words ticks it too. The line under them is `small`. */
export function CheckLine(props: {
  checked: boolean;
  disabled?: boolean;
  title?: string;
  className?: string;
  onChange(on: boolean): void;
  children: ReactNode;
}) {
  return (
    <label className={cn('mt-0 mb-2.5 flex cursor-pointer items-start gap-2 text-sm [&_small]:block [&_small]:text-xs [&_small]:text-ink2', props.className)} title={props.title}>
      <Checkbox checked={props.checked} disabled={props.disabled} onCheckedChange={(on) => props.onChange(on === true)} />
      {props.children}
    </label>
  );
}

/**
 * A row of a list to tick: the box, the name and the line under it, and a
 * word at the end. `off`: it cannot be ticked here.
 */
export function CheckRow(props: { checked: boolean; disabled?: boolean; off?: boolean; className?: string; onChange(on: boolean): void; children: ReactNode }) {
  return (
    <label
      className={cn(
        'grid cursor-pointer grid-cols-[18px_minmax(0,1fr)_auto] items-start gap-2 border-t-[.5px] border-card-line py-1.5 text-sm first:border-t-0',
        '[&_[data-slot=badge]]:self-center [&_b]:font-semi [&_small]:block [&_small]:text-xs [&_small]:text-ink2',
        props.off && 'cursor-default text-ink2',
        props.className,
      )}
    >
      <Checkbox checked={props.checked} disabled={props.disabled} onCheckedChange={(on) => props.onChange(on === true)} />
      {props.children}
    </label>
  );
}

/** A row of a list of saved things: when, what, and where or its buttons. */
export const PACK_ROW = 'grid grid-cols-[130px_minmax(0,1fr)_auto] items-center gap-2.5 border-t-[.5px] border-card-line py-[7px] text-sm first:border-t-0 [h4+&]:border-t-0';
/** The small grey words of a pack row. */
export const PACK_META = 'flex flex-wrap items-center gap-1.5 text-xs text-ink2 tabular-nums';

/** A row of settings or of saved sets: its name and line, then its buttons. Under a card's title it has no line above. */
export const SETTING_ROW =
  'grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-3 gap-y-1 border-t-[.5px] border-card-line py-[9px] text-sm [h4+&]:border-t-0 [&_b]:font-semi [&_small]:block [&_small]:text-xs [&_small]:text-ink2';
/** The buttons at the end of a row. */
export const ROW_ACTIONS = 'flex flex-none items-center gap-2.5';

/** Two or more versions side by side: a row per setting; the cells that differ tinted (`COMPARE_DIFF`). */
export const COMPARE =
  'mb-1 grid grid-cols-[140px_minmax(0,1fr)_minmax(0,1fr)] gap-x-2.5 gap-y-0 text-sm [&>*]:min-w-0 [&>*]:border-t-[.5px] [&>*]:border-card-line [&>*]:px-0 [&>*]:py-[5px] [&>*]:[overflow-wrap:anywhere]';
export const COMPARE_HEAD = 'flex items-center gap-[5px] border-t-0! text-xs font-semi text-ink2';
export const COMPARE_DIFF = 'bg-warn/12';

/** A group of rows to tick, on one panel. */
export const GROUP_PANEL = 'mt-0 mb-2 rounded-tile bg-fill px-2.5 py-0.5';

/** Steps as they run and how each went: a mark (✓, `!`, `×`, running, or waiting, which greys the line), the words, a button. */
export function Progress({ lines, busy }: { lines: [string, ReactNode, ReactNode, ReactNode?][]; busy?: boolean }) {
  return (
    <ul className="mt-0 mb-2 grid list-none gap-[7px] p-0 text-sm" aria-busy={busy || undefined}>
      {lines.map(([tone, title, sub, action], i) => (
        <li key={typeof title === 'string' ? title : i} className={cn('grid grid-cols-[18px_minmax(0,1fr)_auto] items-start gap-2', tone === 'wait' && 'text-ink2')}>
          <span className={cn('mt-px grid size-4 place-items-center rounded-full text-[10px] font-black', PROGRESS_MARK[tone] ?? 'bg-ok/20 text-ok-ink')} aria-hidden="true">
            {tone === 'x' ? '×' : tone === 'w' ? '!' : tone === 'run' || tone === 'wait' ? '' : '✓'}
          </span>
          <span>
            {title}
            {sub ? <small className="block text-xs text-ink2">{sub}</small> : null}
          </span>
          {action ? <span>{action}</span> : null}
        </li>
      ))}
    </ul>
  );
}

const PROGRESS_MARK: Record<string, string> = {
  w: 'bg-warn/22 text-warn-ink',
  x: 'bg-bad/16 text-bad-ink',
  wait: 'bg-fill2 text-ink3',
  run: 'animate-qs-pulse bg-accent/22 text-ok-ink',
};

/** A key to paste: its name and where it is kept, then the field that never shows it again. */
export const SECRET_ROW =
  'grid grid-cols-[minmax(0,1fr)_230px] items-center gap-x-3 gap-y-1.5 border-t-[.5px] border-card-line py-2 text-xs text-ink2 first-of-type:border-t-0 [&_b]:block [&_b]:text-sm [&_b]:font-semi [&_small]:block [&_small]:text-xs [&_small]:text-ink2';
