// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2023 shadcn (shadcn/ui)
// SPDX-License-Identifier: MIT

import * as React from 'react';
import { Popover as PopoverPrimitive } from 'radix-ui';

import { cn } from '@/lib/utils';

// A small panel beside what opened it: the menus' glass, a hairline. It is
// put at the window's root (the `.qs` element), not in the page, so it can
// open from inside a line of text and still be valid HTML, and it takes the
// window's look from that root. Escape and a click outside close it, and
// the focus goes back to its button.

function Popover(props: React.ComponentProps<typeof PopoverPrimitive.Root>) {
  return <PopoverPrimitive.Root data-slot="popover" {...props} />;
}

function PopoverTrigger(props: React.ComponentProps<typeof PopoverPrimitive.Trigger>) {
  return <PopoverPrimitive.Trigger data-slot="popover-trigger" {...props} />;
}

/** The window's root, which carries its look; the body outside a window. */
function windowRoot(from: Element | null): HTMLElement | null {
  return (from?.closest('.qs') as HTMLElement | null) ?? document.querySelector<HTMLElement>('.qs') ?? document.body;
}

function PopoverContent({ className, sideOffset = 4, align = 'start', ...props }: React.ComponentProps<typeof PopoverPrimitive.Content>) {
  const [root, setRoot] = React.useState<HTMLElement | null>(null);
  // The root is found from where the popover's anchor sits, once it is in the page.
  const anchor = React.useCallback((el: HTMLSpanElement | null) => setRoot(windowRoot(el)), []);
  return (
    <>
      <span ref={anchor} hidden />
      <PopoverPrimitive.Portal container={root}>
        <PopoverPrimitive.Content
          data-slot="popover-content"
          sideOffset={sideOffset}
          align={align}
          collisionPadding={12}
          className={cn('material-menu z-40 w-max max-w-[min(360px,calc(100vw-24px))] rounded-[10px] px-3 py-2.5 text-left text-sm text-ink2 outline-0', className)}
          {...props}
        />
      </PopoverPrimitive.Portal>
    </>
  );
}

export { Popover, PopoverContent, PopoverTrigger };
