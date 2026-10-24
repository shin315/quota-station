// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2023 shadcn (shadcn/ui)
// SPDX-License-Identifier: MIT

import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { Dialog as DialogPrimitive } from 'radix-ui';

import { cn } from '@/lib/utils';

// The confirmation sheet and the dialog: a card on a scrim, inside the
// window it belongs to (no portal). The app decides when one opens and
// closes, Escape included; the dialog keeps focus inside while it is open,
// starts on the chosen option or the first control, and gives focus back
// where it was when it closes (G10).

function Dialog(props: React.ComponentProps<typeof DialogPrimitive.Root>) {
  return <DialogPrimitive.Root data-slot="dialog" {...props} />;
}

/**
 * The scrim under a dialog. A plain layer, not Radix's Overlay: that one
 * also locks scrolling by restyling <body>, and the windows never scroll.
 */
function DialogOverlay({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot="dialog-overlay" className={cn('absolute inset-0 bg-scrim', className)} {...props} />;
}

const dialogContentVariants = cva('material-dlg overflow-y-auto shadow-dlg', {
  variants: {
    kind: {
      /** In the popover: slid up from its bottom edge. */
      sheet: 'absolute right-2 bottom-2 left-2 z-[21] max-h-[calc(100%-16px)] rounded-dlg p-4',
      /** In the main window: in the middle. */
      dialog: 'max-h-[calc(100%-32px)] rounded-dlg px-5 pt-5 pb-[18px]',
    },
    size: {
      narrow: 'w-[380px]',
      wide: 'w-[440px]',
      lg: 'w-[540px]',
      xl: 'w-[680px]',
    },
  },
  compoundVariants: [{ kind: 'sheet', className: 'w-auto' }],
  defaultVariants: { kind: 'dialog', size: 'narrow' },
});

type DialogKind = NonNullable<VariantProps<typeof dialogContentVariants>['kind']>;
const DialogKindContext = React.createContext<DialogKind>('dialog');

const FOCUSABLE = 'button:not([disabled]):not([tabindex="-1"]), [href], input:not([disabled]), textarea:not([disabled])';

function DialogContent({
  className,
  kind = 'dialog',
  size,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Content> & VariantProps<typeof dialogContentVariants>) {
  const before = React.useRef<HTMLElement | null>(null);
  return (
    <DialogKindContext.Provider value={kind ?? 'dialog'}>
      <DialogPrimitive.Content
        data-slot="dialog-content"
        data-kind={kind}
        aria-modal="true"
        aria-describedby={undefined}
        tabIndex={-1}
        className={cn(dialogContentVariants({ kind, size }), className)}
        onOpenAutoFocus={(e) => {
          e.preventDefault();
          before.current = document.activeElement as HTMLElement | null;
          const el = e.target as HTMLElement;
          const first = el.querySelector<HTMLElement>('[role="radio"][aria-checked="true"]') ?? el.querySelector<HTMLElement>(FOCUSABLE);
          (first ?? el).focus();
        }}
        onCloseAutoFocus={(e) => {
          e.preventDefault();
          before.current?.focus?.();
        }}
        onEscapeKeyDown={(e) => e.preventDefault()}
        onPointerDownOutside={(e) => e.preventDefault()}
        onInteractOutside={(e) => e.preventDefault()}
        {...props}
      />
    </DialogKindContext.Provider>
  );
}

/** The dialog's name: the sheet's title sits a little closer to what follows. */
function DialogTitle({ className, ...props }: React.ComponentProps<typeof DialogPrimitive.Title>) {
  const kind = React.useContext(DialogKindContext);
  return (
    <DialogPrimitive.Title asChild>
      <h3 data-slot="dialog-title" className={cn('mt-0 text-sheet', kind === 'sheet' ? 'mb-1' : 'mb-1.5', className)} {...props} />
    </DialogPrimitive.Title>
  );
}

/** The line under the title. */
function DialogLead({ className, ...props }: React.ComponentProps<'p'>) {
  return <p data-slot="dialog-lead" className={cn('mt-0 mb-2 text-sm text-ink2', className)} {...props} />;
}

/** The buttons at the end, to the right. */
function DialogButtons({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot="dialog-buttons" className={cn('flex flex-wrap justify-end gap-2', className)} {...props} />;
}

/** A heading inside a dialog or a sheet: small, bold and grey, with its icon. */
function DialogSection({ className, ...props }: React.ComponentProps<'div'>) {
  const kind = React.useContext(DialogKindContext);
  return (
    <div
      data-slot="dialog-section"
      className={cn(
        'mb-1 flex items-center gap-1.5 text-xs font-bold text-ink2',
        '[&_.agl]:size-3.5',
        kind === 'sheet' ? 'mt-2.5 first-of-type:mt-0 [&_svg]:size-3.5' : 'mt-2 [&_svg]:size-[13px]',
        className,
      )}
      {...props}
    />
  );
}

export { Dialog, DialogButtons, DialogContent, DialogLead, DialogOverlay, DialogSection, DialogTitle };
