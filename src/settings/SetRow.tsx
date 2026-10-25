// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// A row of a Cài đặt card (settings-R2, 25/10/2026): its name and one line
// under it on the left, its control at the card's right edge, so every
// switch, field and segmented control on the tab ends on the same line. What
// a field says back (read, saved, refused) goes under the field, within its
// width. Where the page has under 600 px the control goes under its name; a
// switch or a chip (`inline`) stays beside it.

import type { ReactNode } from 'react';
import { cn } from '@/lib/utils';

/** A field's or a segmented control's width: the same on every row. */
export const SET_CTL = 'w-[320px] max-w-full @max-[600px]/main:w-full';

export function SetRow(props: {
  label: ReactNode;
  /** The label's id, when it names a group (aria-labelledby). */
  labelId?: string;
  /** The field the label names. */
  htmlFor?: string;
  /** The line (or lines) under the name. */
  hint?: ReactNode;
  /** Under the control: what the field says back. */
  under?: ReactNode;
  /** A switch, a chip or a button: it stays beside its name where the page is narrow. */
  inline?: boolean;
  /** The control's own lines (the other Macs) start on the left of its block. */
  block?: boolean;
  children: ReactNode;
}) {
  const Label = props.htmlFor ? 'label' : 'div';
  return (
    <div
      className={cn(
        'srow grid grid-cols-[minmax(10rem,1fr)_auto] items-center gap-x-6 gap-y-1 border-t-[.5px] border-card-line py-2.5 first:border-t-0 first:pt-0.5 last:pb-0.5',
        props.inline ? '@max-[600px]/main:grid-cols-[minmax(0,1fr)_auto] @max-[600px]/main:gap-x-3' : '@max-[600px]/main:grid-cols-[minmax(0,1fr)] @max-[600px]/main:gap-y-1.5',
      )}
    >
      <div className={cn('min-w-0', props.block && 'self-start pt-1')}>
        <Label className="block text-sm font-semi" id={props.labelId} htmlFor={props.htmlFor}>
          {props.label}
        </Label>
        {props.hint ? <div className="mt-0.5 grid gap-0.5 text-xs text-ink2 [&_[data-tone]]:mt-0">{props.hint}</div> : null}
      </div>
      <div
        className={cn(
          'flex min-w-0 flex-wrap items-center gap-2',
          props.block ? 'justify-self-end' : 'justify-end',
          !props.inline && '@max-[600px]/main:justify-start @max-[600px]/main:justify-self-stretch',
        )}
      >
        {props.children}
      </div>
      {props.under ? <div className={cn('col-start-2 w-0 min-w-full', !props.inline && '@max-[600px]/main:col-start-1')}>{props.under}</div> : null}
    </div>
  );
}
