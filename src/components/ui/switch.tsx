// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2023 shadcn (shadcn/ui)
// SPDX-License-Identifier: MIT

import * as React from 'react';
import { Switch as SwitchPrimitive } from 'radix-ui';

import { cn } from '@/lib/utils';

/** The design system's switch (RS): 40 by 22, blue when on, a white knob that slides. */
function Switch({ className, ...props }: React.ComponentProps<typeof SwitchPrimitive.Root>) {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      className={cn(
        'relative h-[22px] w-10 flex-[0_0_auto] cursor-pointer rounded-[11px] border-0 bg-toggle-off p-0 transition-colors duration-(--dur-2)',
        'data-[state=checked]:bg-toggle-on disabled:cursor-default disabled:opacity-45',
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className="absolute top-0.5 left-0.5 block size-[18px] rounded-full bg-knob shadow-[0_1px_2px_rgba(0,0,0,.3)] transition-transform duration-(--dur-2) ease-qs data-[state=checked]:translate-x-[18px]"
      />
    </SwitchPrimitive.Root>
  );
}

export { Switch };
