// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// A tab of the main window: its title row (which drags the window, as a
// title bar would), the grids its cards sit in, and the list rows they share.

import type { ComponentProps, ReactNode } from 'react';
import { cn } from '@/lib/utils';

/** The title row: the tab's name, how fresh it is, its buttons; they wrap under the name where the page is narrow. */
export function TabHead({ className, ...props }: ComponentProps<'div'>) {
  return <div className={cn('flex flex-wrap items-center gap-2.5', className)} data-tauri-drag-region="deep" {...props} />;
}

export function TabTitle({ className, ...props }: ComponentProps<'h2'>) {
  return <h2 className={cn('m-0 flex-1 text-page tracking-[-.01em]', className)} {...props} />;
}

/** The line next to the title: this Mac and how fresh the numbers are. */
export function TabSub({ className, ...props }: ComponentProps<'small'>) {
  return <small className={cn('flex items-center gap-1.5 text-sm text-ink2', className)} {...props} />;
}

/** Two columns of cards, the left one wider; one column where the page is narrow. */
export const GRID = 'grid grid-cols-[minmax(0,1.85fr)_minmax(0,1fr)] items-start gap-3 @max-[600px]/main:grid-cols-[minmax(0,1fr)]';
/**
 * Two equal columns of cards; one column where the page has under 680 px
 * (the window less the sidebar), and a card alone takes the whole row.
 */
export const GRID2 =
  'grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] items-start gap-3 @max-[680px]/main:grid-cols-[minmax(0,1fr)] [&>:only-child]:col-span-full';
/** Cards one under another in a column. */
export const STACK = 'grid min-w-0 content-start gap-3';

/**
 * One row of a form: its name on the left (up to 15rem, so "Chuyển khi 5 giờ
 * dùng tới" keeps one line), the controls on the right, a blue edge once
 * changed. The name goes over its controls where the page has under 600 px;
 * in a dialog the name keeps 190 px.
 * `inline`: a row whose control is a switch, a status or a short value:
 * where the page is narrow it stays at the row's right edge, beside the
 * name, and what follows it (a hint, a button) runs the row's width under both.
 */
export function rowClass(changed = false, inline = false): string {
  return cn(
    'grid grid-cols-[190px_minmax(0,1fr)] items-start gap-3.5 border-t-[.5px] border-card-line py-2.5 first:border-t-0 first:pt-0.5 @max-[600px]/main:grid-cols-[minmax(0,1fr)] @max-[600px]/main:gap-y-1.5 in-data-[slot=dialog-content]:grid-cols-[190px_minmax(0,1fr)]',
    inline && ROW_INLINE,
    changed && '-ml-2 pl-2 shadow-[inset_3px_0_0_var(--accent)]',
  );
}

/** See rowClass's `inline`. */
const ROW_INLINE =
  '@max-[600px]/main:grid-cols-[minmax(0,1fr)_auto] @max-[600px]/main:gap-y-1 @max-[600px]/main:[&>:last-child]:contents @max-[600px]/main:[&>:last-child>:first-child]:col-start-2 @max-[600px]/main:[&>:last-child>:first-child]:row-start-1 @max-[600px]/main:[&>:last-child>:first-child]:self-center @max-[600px]/main:[&>:last-child>:first-child]:justify-self-end @max-[600px]/main:[&>:last-child>:not(:first-child)]:col-span-full';

/**
 * A row of an accounts table: it takes the table's columns (TABLE, --wt-cols),
 * so a column sized to its content (`auto`, the buttons) is as wide as the
 * widest row's. The row in use is lit, and the hairlines around it give way.
 * Where the page has under 640 px a row is a card of its own: the name and
 * the buttons on top, the windows under them (TABLE_CELLS), each with its name.
 */
export const TABLE_ROW =
  'wt-r col-span-full grid grid-cols-subgrid items-center border-t-[.5px] border-card-line px-1.5 py-2 text-sm first:border-t-0 [.on+&]:border-t-transparent @max-[640px]/main:grid-cols-[minmax(9rem,1fr)_auto_auto] @max-[640px]/main:gap-x-2.5 @max-[640px]/main:gap-y-2 @max-[640px]/main:[&>:first-child]:col-start-1 @max-[640px]/main:[&>:first-child]:row-start-1';
/** A row's windows: columns of the table, or the card's second line where the page is narrow. */
export const TABLE_CELLS =
  'contents @max-[640px]/main:col-span-full @max-[640px]/main:row-start-2 @max-[640px]/main:grid @max-[640px]/main:auto-cols-[minmax(0,1fr)] @max-[640px]/main:grid-flow-col @max-[640px]/main:gap-3';
/** A row's buttons and its ⋯, on the card's top line where the page is narrow. */
export const TABLE_END = '@max-[640px]/main:row-start-1';
/** The lit row of an accounts table: the one in use. */
export const TABLE_ROW_ON = 'on rounded-lg border-t-transparent bg-[color-mix(in_srgb,var(--sel)_70%,transparent)]';
/**
 * An accounts table: its columns (--wt-cols) and their gap (--wt-gap); it is
 * never narrower than its columns' least widths add up to. One column of
 * cards where the page is narrow.
 */
export const TABLE = 'grid min-w-min grid-cols-(--wt-cols) gap-x-[var(--wt-gap,10px)] @max-[640px]/main:min-w-0 @max-[640px]/main:grid-cols-[minmax(0,1fr)]';
/** Around a table: one that still does not fit scrolls sideways inside it. */
export const TABLE_BOX = 'overflow-x-auto overscroll-x-contain';
/** A table beside another column (Tổng quan's rail). */
export const TABLE_TIGHT = '[--wt-gap:10px]';
/** The table's head row: the windows' names. */
export const TABLE_HEAD = TABLE_ROW + ' h pt-0 pb-1 text-xs whitespace-nowrap text-ink3 @max-[640px]/main:hidden';

/** A form row's name. */
export const LABEL = 'fl pt-[3px] text-sm font-semi';

/** A row of a list in a card: its words on the left, its buttons on the right. */
export const LIST_ROW = 'arow grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-2.5 border-t-[.5px] border-card-line py-[9px] text-sm first:border-t-0';

/** A count next to a card's title, light. */
export const COUNT = 'font-reg text-ink3';

/** The empty state of a card: a bold line and a word under it. */
export function Empty({ className, ...props }: ComponentProps<'div'>) {
  return <div className={cn('px-2.5 py-10 text-center text-sm text-ink2 [&>b]:mb-1 [&>b]:block [&>b]:text-body [&>b]:text-ink', className)} {...props} />;
}

/** The empty state's large icon, over its words. */
export function EmptyIcon({ children }: { children: ReactNode }) {
  return (
    <span className="mx-auto mb-2 grid size-11 place-items-center rounded-[12px] bg-fill text-accent-ink" aria-hidden="true">
      {children}
    </span>
  );
}

/** One short line under a table: an icon, the words, and its buttons. */
export function InfoLine({ icon, children, actions }: { icon: ReactNode; children: ReactNode; actions: ReactNode }) {
  return (
    <div className="grid grid-cols-[18px_minmax(0,1fr)_auto] items-center gap-2 border-t-[.5px] border-card-line px-0.5 py-1.5 text-sm text-ink2 first:border-t-0 [&_b]:font-semi [&_b]:text-ink">
      <span className="inline-flex text-ink3" aria-hidden="true">
        {icon}
      </span>
      <span>{children}</span>
      <span className="flex items-center gap-2.5">{actions}</span>
    </div>
  );
}
