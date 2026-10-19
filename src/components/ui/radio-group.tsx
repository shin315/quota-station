import * as React from 'react';
import { RadioGroup as RadioGroupPrimitive } from 'radix-ui';

import { cn } from '@/lib/utils';

// Choose one (G10): each choice a tile with a round mark and a second line,
// so the choice never rests on colour alone. Arrow keys move the choice; Tab
// reaches the chosen one.

function RadioGroup({ className, ...props }: React.ComponentProps<typeof RadioGroupPrimitive.Root>) {
  return <RadioGroupPrimitive.Root data-slot="radio-group" loop className={cn('grid grid-cols-2 gap-1.5', className)} {...props} />;
}

function RadioGroupItem({ className, children, ...props }: React.ComponentProps<typeof RadioGroupPrimitive.Item>) {
  return (
    <RadioGroupPrimitive.Item
      data-slot="radio-group-item"
      className={cn(
        'group/radio grid cursor-pointer grid-cols-[14px_minmax(0,1fr)] items-start gap-x-[7px] rounded-lg border-0 bg-fill px-2 py-[7px] text-left text-sm shadow-[inset_0_0_0_.5px_var(--card-line)] focus-visible:-outline-offset-2',
        'data-[state=checked]:bg-sel data-[state=checked]:shadow-[inset_0_0_0_1.5px_var(--accent)] disabled:cursor-default disabled:opacity-45',
        className,
      )}
      {...props}
    >
      <span
        aria-hidden="true"
        className="row-span-2 mt-0.5 size-3 rounded-full bg-card shadow-[inset_0_0_0_1.5px_var(--ink2)] group-data-[state=checked]/radio:bg-on-accent group-data-[state=checked]/radio:shadow-[inset_0_0_0_4px_var(--accent)]"
      />
      {children}
    </RadioGroupPrimitive.Item>
  );
}

export { RadioGroup, RadioGroupItem };
