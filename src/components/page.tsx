// A tab of the main window: its title row (which drags the window, as a
// title bar would), the grids its cards sit in, and the list rows they share.

import type { ComponentProps, ReactNode } from 'react';
import { cn } from '@/lib/utils';

/** The title row: the tab's name, how fresh it is, its buttons. */
export function TabHead({ className, ...props }: ComponentProps<'div'>) {
  return <div className={cn('flex flex-wrap items-center gap-2.5', className)} data-tauri-drag-region="deep" {...props} />;
}

export function TabTitle({ className, ...props }: ComponentProps<'h2'>) {
  return <h2 className={cn('m-0 flex-1 text-h2 tracking-[-.01em]', className)} {...props} />;
}

/** The line next to the title: this Mac and how fresh the numbers are. */
export function TabSub({ className, ...props }: ComponentProps<'small'>) {
  return <small className={cn('flex items-center gap-1.5 text-sm text-ink2', className)} {...props} />;
}

/** Two columns of cards, the left one wider. */
export const GRID = 'grid grid-cols-[minmax(0,1.85fr)_minmax(0,1fr)] items-start gap-3';
/** Two equal columns of cards. */
export const GRID2 = 'grid grid-cols-[1fr_1fr] items-start gap-3';
/** Cards one under another in a column. */
export const STACK = 'grid min-w-0 content-start gap-3';

/** One row of a form: its name on the left, the controls on the right, a blue edge once changed. */
export function rowClass(changed = false): string {
  return cn(
    'grid grid-cols-[190px_minmax(0,1fr)] items-start gap-3.5 border-t-[.5px] border-card-line py-2.5 first:border-t-0 first:pt-0.5',
    changed && '-ml-2 pl-2 shadow-[inset_3px_0_0_var(--accent)]',
  );
}

/**
 * A row of an accounts table; the table sets its columns (--wt-cols). The
 * row in use is lit, and the hairlines around it give way.
 */
export const TABLE_ROW =
  'wt-r grid grid-cols-(--wt-cols) items-center gap-2.5 border-t-[.5px] border-card-line px-1.5 py-2 text-sm first:border-t-0 [.on+&]:border-t-transparent';
/** The lit row of an accounts table: the one in use. */
export const TABLE_ROW_ON = 'on rounded-lg border-t-transparent bg-[color-mix(in_srgb,var(--sel)_70%,transparent)]';

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
