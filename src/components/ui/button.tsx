import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { Slot } from 'radix-ui';

import { cn } from '@/lib/utils';

// macOS push buttons: grey by default, blue for the action that goes ahead,
// red-tinted to remove or put back something; `link` is the blue text link.
const buttonVariants = cva('cursor-pointer border-0 disabled:cursor-default', {
  variants: {
    variant: {
      default: 'whitespace-nowrap rounded-ctl bg-fill2 font-med text-ink hover:brightness-[.97] disabled:opacity-45 disabled:filter-none',
      primary: 'whitespace-nowrap rounded-ctl bg-accent font-med text-on-accent hover:brightness-[.97] disabled:opacity-45 disabled:filter-none',
      danger: 'whitespace-nowrap rounded-ctl bg-bad/14 font-med text-bad-ink hover:brightness-[.97] disabled:opacity-45 disabled:filter-none',
      link: 'bg-transparent p-0 text-accent-ink disabled:text-ink3',
      linkBad: 'bg-transparent p-0 text-bad-ink disabled:text-ink3',
    },
    size: {
      default: 'px-2.5 py-1 text-sm',
      lg: 'px-3.5 py-[5px] text-body',
      sm: 'h-6 px-2.5 py-0 text-xs',
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
