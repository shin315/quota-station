// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2023 shadcn (shadcn/ui)
// SPDX-License-Identifier: MIT

import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';

import { cn } from '@/lib/utils';

// The design system's chips (round, on rows and names) and pills (square, in
// the history): a short word on a light fill of its tone.
// The prototype's chips (D01, D03): a passive label, never a button: no
// edge, no shadow, nothing on hover. "đang dùng" is the passive chip with a
// green dot, a read-only tag is blue, a state worth a look is tinted.
const badgeVariants = cva('inline-flex items-center gap-[5px] whitespace-nowrap align-middle font-semi tabular-nums', {
  variants: {
    shape: {
      chip: 'h-5 rounded-chip px-2 text-[11.5px] leading-none',
      pill: 'h-[18px] rounded-chip px-1.5 text-[10.5px] leading-none',
    },
    tone: {
      default: 'bg-card-2 text-ink2',
      on: "bg-passive text-passive-ink before:size-1.5 before:rounded-full before:bg-ok before:content-['']",
      ok: "bg-passive text-passive-ink before:size-1.5 before:rounded-full before:bg-ok before:content-['']",
      warn: 'bg-warn/15 text-warn-ink',
      bad: 'bg-bn-bad text-bad-ink',
      info: 'bg-tint text-tint-ink',
      tag: 'bg-tag text-tag-ink',
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
