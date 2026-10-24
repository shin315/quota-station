// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2023 shadcn (shadcn/ui)
// SPDX-License-Identifier: MIT

import * as React from 'react';
import { Switch as SwitchPrimitive } from 'radix-ui';

import { cn } from '@/lib/utils';

/** The design system's switch: green when on, a white knob that slides. */
function Switch({ className, ...props }: React.ComponentProps<typeof SwitchPrimitive.Root>) {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      className={cn(
        'relative h-(--tog-h) w-(--tog-w) flex-[0_0_auto] cursor-pointer rounded-[10px] border-0 bg-fill2 p-0',
        'data-[state=checked]:bg-ok disabled:cursor-default disabled:opacity-45',
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className="absolute top-0.5 left-0.5 block size-[15px] rounded-full bg-white shadow-[0_1px_2px_rgba(0,0,0,.3)] transition-[left] duration-(--dur-1) ease-qs data-[state=checked]:left-[15px]"
      />
    </SwitchPrimitive.Root>
  );
}

export { Switch };
