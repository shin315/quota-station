// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { Toggle as TogglePrimitive } from 'radix-ui';

import { cn } from '@/lib/utils';

// macOS's segmented control: a grey track, the chosen segment raised on it.
// Each segment is a toggle button (aria-pressed) in a group, so every
// segment stays in the Tab order as before.
const segmentedVariants = cva('gap-0.5 rounded-lg bg-fill2 p-0.5', {
  variants: {
    variant: {
      /** Equal segments filling the row. */
      fill: 'grid auto-cols-[minmax(0,1fr)] grid-flow-col',
      /** As wide as its words. */
      inline: 'inline-grid auto-cols-[minmax(0,1fr)] grid-flow-col',
      /** The menu bar's service picker: each segment with its mark. */
      menubar: 'inline-flex',
    },
  },
  defaultVariants: { variant: 'fill' },
});

const segmentVariants = cva(
  'cursor-pointer whitespace-nowrap rounded-md border-0 bg-transparent data-[state=on]:bg-card',
  {
    variants: {
      variant: {
        fill: 'px-1 py-1 text-s data-[state=on]:font-semi data-[state=on]:shadow-[0_1px_3px_rgba(0,0,0,.18)] dark:data-[state=on]:bg-[#5a5a5f]',
        inline:
          'px-1 py-1 text-s data-[state=on]:cursor-default data-[state=on]:font-semi data-[state=on]:shadow-[0_1px_3px_rgba(0,0,0,.18)] dark:data-[state=on]:bg-[#5a5a5f]',
        menubar:
          'inline-flex items-center gap-1.5 px-3 py-1 text-[12.5px] text-ink [font-family:inherit] disabled:text-ink3 data-[state=on]:cursor-default data-[state=on]:font-semi data-[state=on]:shadow-[0_1px_3px_rgba(0,0,0,.2)]',
      },
    },
    defaultVariants: { variant: 'fill' },
  },
);

type SegmentedVariant = NonNullable<VariantProps<typeof segmentedVariants>['variant']>;
const SegmentedContext = React.createContext<SegmentedVariant>('fill');

function Segmented({ className, variant = 'fill', ...props }: React.ComponentProps<'div'> & VariantProps<typeof segmentedVariants>) {
  return (
    <SegmentedContext.Provider value={variant ?? 'fill'}>
      <div data-slot="segmented" role="group" className={cn(segmentedVariants({ variant }), className)} {...props} />
    </SegmentedContext.Provider>
  );
}

/**
 * One segment. With `pressed` it is a toggle (aria-pressed); without, a plain
 * button that does its job at once (the pause lengths).
 */
function Segment({
  className,
  pressed,
  onClick,
  ...props
}: Omit<React.ComponentProps<'button'>, 'onClick'> & { pressed?: boolean; onClick?: () => void }) {
  const variant = React.useContext(SegmentedContext);
  const cls = cn(segmentVariants({ variant }), className);
  if (pressed === undefined) return <button data-slot="segment" type="button" className={cls} onClick={onClick} {...props} />;
  return <TogglePrimitive.Root data-slot="segment" className={cls} pressed={pressed} onPressedChange={() => onClick?.()} {...props} />;
}

export { Segment, Segmented, segmentedVariants };
