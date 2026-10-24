// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2023 shadcn (shadcn/ui)
// SPDX-License-Identifier: MIT

import * as React from 'react';

import { cn } from '@/lib/utils';

// A group in the main window: the second layer of glass on the window's
// glass, one step more opaque, with a hairline edge (Q13), and never a panel
// inside a panel. `bare`: a group of cards (Đang dùng's tiles, Cảnh báo's
// recent alerts) drops its own panel so the cards stand on the glass.

function Card({ className, bare, ...props }: React.ComponentProps<'div'> & { bare?: boolean }) {
  return (
    <div
      data-slot="card"
      className={cn('min-w-0 rounded-card', bare ? 'bg-transparent p-0' : 'material-card px-4 pt-3.5 pb-4', className)}
      {...props}
    />
  );
}

/**
 * A card's head: its icon or the service's logo, then its name. `big`: a
 * service's card, its name at the size of a section. `bare`: over a group of
 * cards, small.
 */
function CardTitle({ className, bare, big, ...props }: React.ComponentProps<'h4'> & { bare?: boolean; big?: boolean }) {
  return (
    <h4
      data-slot="card-title"
      className={cn(
        'mt-0 mb-3 flex min-w-0 items-center gap-2.5 font-heading tracking-head text-heading [&>svg]:size-[26px] [&>svg]:flex-none [&>svg]:stroke-[1.7]',
        big ? 'text-h2' : 'text-h3',
        bare && 'mx-0.5 mb-2.5 text-[15px] font-semi tracking-normal',
        className,
      )}
      {...props}
    />
  );
}

/** A line of text in a card. */
function CardText({ className, ...props }: React.ComponentProps<'p'>) {
  return <p data-slot="card-text" className={cn('mt-0 mb-2 text-sm text-ink2', className)} {...props} />;
}

export { Card, CardText, CardTitle };
