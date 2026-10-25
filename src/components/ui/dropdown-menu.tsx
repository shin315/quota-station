// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2023 shadcn (shadcn/ui)
// SPDX-License-Identifier: MIT

import * as React from 'react';
import { DropdownMenu as DropdownMenuPrimitive } from 'radix-ui';

import { cn } from '@/lib/utils';

function DropdownMenu(props: React.ComponentProps<typeof DropdownMenuPrimitive.Root>) {
  return <DropdownMenuPrimitive.Root data-slot="dropdown-menu" {...props} />;
}

function DropdownMenuTrigger(props: React.ComponentProps<typeof DropdownMenuPrimitive.Trigger>) {
  return <DropdownMenuPrimitive.Trigger data-slot="dropdown-menu-trigger" {...props} />;
}

/**
 * A macOS menu: glass, a hairline, the chosen item blue. It stays in place
 * in the page rather than a portal, so a menu in a dialog is part of the
 * dialog's layer. `onOpenAutoFocus`: Radix's Menu takes it but the dropdown's
 * types leave it out; it is how a menu opened by the pointer can still put
 * the focus on its first item.
 */
function DropdownMenuContent({
  className,
  sideOffset = 2,
  onOpenAutoFocus,
  ...props
}: React.ComponentProps<typeof DropdownMenuPrimitive.Content> & { onOpenAutoFocus?: (event: Event) => void }) {
  return (
    <DropdownMenuPrimitive.Content
      data-slot="dropdown-menu-content"
      sideOffset={sideOffset}
      {...({ onOpenAutoFocus } as object)}
      className={cn(
        'material-menu z-20 min-w-[196px] rounded-[10px] p-[5px] text-left',
        className,
      )}
      {...props}
    />
  );
}

/** The menu's head: a name and a line under it, not an item. */
function DropdownMenuLabel({ className, ...props }: React.ComponentProps<typeof DropdownMenuPrimitive.Label>) {
  return (
    <DropdownMenuPrimitive.Label
      data-slot="dropdown-menu-label"
      className={cn('mb-1 border-b-[.5px] border-card-line px-2 pt-1 pb-1.5', className)}
      {...props}
    />
  );
}

function DropdownMenuItem({ className, ...props }: React.ComponentProps<typeof DropdownMenuPrimitive.Item>) {
  return (
    <DropdownMenuPrimitive.Item
      data-slot="dropdown-menu-item"
      className={cn(
        'block w-full cursor-pointer whitespace-nowrap rounded-md border-0 bg-transparent px-2 py-[5px] text-left text-sm text-ink outline-0',
        'data-[highlighted]:bg-accent data-[highlighted]:text-on-accent',
        'disabled:cursor-default disabled:text-ink3 data-[disabled]:cursor-default data-[disabled]:text-ink3',
        className,
      )}
      {...props}
    />
  );
}

function DropdownMenuSeparator({ className, ...props }: React.ComponentProps<typeof DropdownMenuPrimitive.Separator>) {
  return <DropdownMenuPrimitive.Separator data-slot="dropdown-menu-separator" className={cn('mx-1.5 my-1 h-[.5px] bg-card-line', className)} {...props} />;
}

export { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger };
