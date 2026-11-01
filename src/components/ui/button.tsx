// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2023 shadcn (shadcn/ui)
// SPDX-License-Identifier: MIT

import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { Slot } from 'radix-ui';

import { cn } from '@/lib/utils';

// macOS push buttons: grey by default, blue for the action that goes ahead,
// red-tinted to remove or put back something; `tint` is a blue-tinted one,
// `quiet` only text until hovered; `link` is the blue text link.
const BOX =
  'inline-flex items-center justify-center gap-1 whitespace-nowrap rounded-ctl border-0 font-med hover:brightness-[.97] disabled:opacity-45 disabled:filter-none [&_svg]:size-3.5';

const buttonVariants = cva('cursor-pointer border-0 disabled:cursor-default', {
  variants: {
    variant: {
      default: BOX + ' bg-fill2 text-ink',
      primary: BOX + ' bg-accent text-on-accent',
      danger: BOX + ' bg-bad/14 text-bad-ink',
      tint: BOX + ' bg-accent/14 text-info-ink',
      quiet: BOX + ' bg-transparent text-ink2 not-disabled:hover:bg-fill not-disabled:hover:text-ink',
      link: 'bg-transparent p-0 text-accent-ink disabled:text-ink3',
      linkBad: 'bg-transparent p-0 text-bad-ink disabled:text-ink3',
    },
    size: {
      default: 'px-2.5 py-1 text-sm',
      lg: 'px-3.5 py-[5px] text-body',
      sm: 'h-6 px-2.5 py-0 text-xs',
      /** A button that is only an icon: square. */
      icon: 'size-7 p-0 [&_svg]:size-4',
      link: 'text-sm',
      linkSm: 'text-xs',
    },
  },
  defaultVariants: {
    variant: 'default',
    size: 'default',
  },
});

function Button({
  className,
  variant = 'default',
  size = 'default',
  asChild = false,
  type = 'button',
  ...props
}: React.ComponentProps<'button'> &
  VariantProps<typeof buttonVariants> & {
    asChild?: boolean;
  }) {
  const Comp = asChild ? Slot.Root : 'button';
  return (
    <Comp
      data-slot="button"
      data-variant={variant}
      data-size={size}
      type={asChild ? undefined : type}
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  );
}

export { Button, buttonVariants };
