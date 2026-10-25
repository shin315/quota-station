// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2023 shadcn (shadcn/ui)
// SPDX-License-Identifier: MIT

import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';

import { cn } from '@/lib/utils';

// The design system's chips (round, on rows and names) and pills (square, in
// the history): a short word on a light fill of its tone.
const badgeVariants = cva('whitespace-nowrap text-cap', {
  variants: {
    shape: {
      chip: 'rounded-chip px-1.5 leading-4 tabular-nums',
      pill: 'inline-block rounded-md px-1.5 py-px font-semi',
    },
    tone: {
      default: 'bg-fill2 text-ink2',
      on: 'bg-accent text-on-accent',
      ok: 'bg-ok/18 text-ok-ink',
      warn: 'bg-warn/20 text-warn-ink',
      bad: 'bg-bad/15 text-bad-ink',
      info: 'bg-accent/14 text-info-ink',
      tag: 'bg-accent/14 text-info-ink',
      c: 'bg-id-claude-tint text-id-claude-ink',
      g: 'bg-id-agy-tint text-id-agy-ink',
      x: 'bg-id-codex-tint text-id-codex-ink',
    },
  },
  defaultVariants: {
    shape: 'chip',
    tone: 'default',
  },
});

type BadgeTone = NonNullable<VariantProps<typeof badgeVariants>['tone']>;

/** A chip's kind as the view names it (`stale` is the red of `bad`); anything else is the grey chip. */
function badgeTone(kind: string | undefined): BadgeTone {
  if (kind === 'stale') return 'bad';
  return kind && kind in TONES ? (kind as BadgeTone) : 'default';
}

const TONES: Record<BadgeTone, true> = { default: true, on: true, ok: true, warn: true, bad: true, info: true, tag: true, c: true, g: true, x: true };

function Badge({ className, shape, tone, ...props }: React.ComponentProps<'span'> & VariantProps<typeof badgeVariants>) {
  return <span data-slot="badge" data-tone={tone ?? 'default'} className={cn(badgeVariants({ shape, tone }), className)} {...props} />;
}

export { Badge, badgeTone, badgeVariants };
