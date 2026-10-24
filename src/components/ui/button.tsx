// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2023 shadcn (shadcn/ui)
// SPDX-License-Identifier: MIT

import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { Slot } from 'radix-ui';

import { cn } from '@/lib/utils';

// macOS push buttons: grey by default, blue for the action that goes ahead,
// red-tinted to remove or put back something; `link` is the blue text link.
// The prototype's buttons (D04): 32 high (28 small), 8 round, a hairline
// edge; the primary one blue with white text that gets darker on hover and
// press so the text keeps 4.5:1; the quiet one only text until hovered.
const BOX =
  'inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-ctl border font-med leading-[1.2] transition-colors duration-(--dur-1) disabled:opacity-45 [&_svg]:size-4';

const buttonVariants = cva('cursor-pointer disabled:cursor-default', {
  variants: {
    variant: {
      default: BOX + ' border-line-2 bg-ctl text-ink not-disabled:hover:bg-ctl-hover not-disabled:active:brightness-[.96]',
      primary: BOX + ' border-transparent bg-accent text-on-accent not-disabled:hover:bg-accent-hover not-disabled:active:bg-accent-press',
      danger: BOX + ' border-transparent bg-destr text-destr-ink not-disabled:active:brightness-[.96]',
      /** A blue-tinted action on a neutral banner (D07). */
      tint: BOX + ' border-transparent bg-tint text-tint-ink not-disabled:active:brightness-[.96]',
      quiet: BOX + ' border-transparent bg-transparent text-ink2 not-disabled:hover:bg-ctl not-disabled:hover:text-ink',
      link: 'border-0 bg-transparent p-0 text-accent-ink underline underline-offset-2 hover:decoration-2 disabled:text-ink3 disabled:no-underline',
      linkBad: 'border-0 bg-transparent p-0 text-bad-ink underline underline-offset-2 hover:decoration-2 disabled:text-ink3 disabled:no-underline',
    },
    size: {
      default: 'min-h-8 px-3.5 text-body',
      /** A dialog's or a sheet's buttons: the design draws them at the one size of the rest. */
      lg: 'min-h-8 px-3.5 text-body',
      sm: 'min-h-7 px-2.5 text-sm',
      /** A button that is only an icon: square. */
      icon: 'size-8 p-0 [&_svg]:size-[18px]',
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
