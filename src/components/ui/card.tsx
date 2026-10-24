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

function CardTitle({ className, bare, ...props }: React.ComponentProps<'h4'> & { bare?: boolean }) {
  return (
    <h4
      data-slot="card-title"
      className={cn('mt-0 mb-1.5 flex items-center gap-1.5 text-sm font-bold text-ink2', bare && 'mx-0.5 mb-2', className)}
      {...props}
    />
  );
}

/** A line of text in a card. */
function CardText({ className, ...props }: React.ComponentProps<'p'>) {
  return <p data-slot="card-text" className={cn('mt-0 mb-2 text-sm text-ink2', className)} {...props} />;
}

export { Card, CardText, CardTitle };
