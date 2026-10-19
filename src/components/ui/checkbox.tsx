import * as React from 'react';
import { Checkbox as CheckboxPrimitive } from 'radix-ui';

import { cn } from '@/lib/utils';

/** The macOS box: a light square with a hairline, blue with a white tick when ticked. */
function CheckboxBox({ className }: { className?: string }) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        'grid size-4 flex-[0_0_16px] place-items-center rounded-sm transition-[background] duration-(--dur-1) ease-qs',
        'bg-[color-mix(in_srgb,var(--card)_70%,transparent)] shadow-[inset_0_0_0_1px_color-mix(in_srgb,var(--ink)_30%,transparent),0_.5px_1px_rgba(0,0,0,.12)]',
        'group-data-[state=checked]/cb:bg-accent group-data-[state=checked]/cb:shadow-[0_.5px_1px_rgba(0,0,0,.2)]',
        className,
      )}
    >
      <CheckboxPrimitive.Indicator className="h-1 w-2 -translate-y-px -rotate-45 border-b-2 border-l-2 border-white" />
    </span>
  );
}

/**
 * A checkbox on its own, next to its label: the size of macOS's small box.
 * The label wraps it, so pressing the words ticks it too.
 */
function Checkbox({ className, ...props }: React.ComponentProps<typeof CheckboxPrimitive.Root>) {
  return (
    <CheckboxPrimitive.Root
      data-slot="checkbox"
      className={cn(
        'group/cb mt-0.5 inline-grid size-3.5 cursor-pointer place-items-center rounded-[3px] border-0 bg-transparent p-0 disabled:cursor-default disabled:opacity-50',
        className,
      )}
      {...props}
    >
      <CheckboxBox className="size-3.5 flex-[0_0_14px] rounded-[3px]" />
    </CheckboxPrimitive.Root>
  );
}

export { Checkbox, CheckboxBox };
